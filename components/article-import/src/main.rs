//! Import article folders produced by `~/llm/crawler` + `~/llm/transcribe`
//! into the `translation` DynamoDB table.
//!
//! The crawler writes one folder per article:
//!
//! ```text
//! 2026-10-01_mutmasslicher-anschlagsplan-der-hamas-botschaft-mit-blut/
//!   article.json              metadata, blocks (para/head/image), images, audio
//!   images/01_….webp
//!   audio/….mp3
//!   transcribe/transcript.json      sentence segments with start/end times
//!   transcribe/translation.json     index-aligned English sentences
//! ```
//!
//! The heavy payloads are uploaded to the cloud store by hand; this tool only
//! creates the DynamoDB record the web app reads. It aligns the article's
//! paragraphs and headings with the sentence-level transcript so each paragraph
//! keeps its translated text, its audio span and — for word pairing — its
//! original wording.
//!
//! ```bash
//! article-import --user-id <uuid> --prefix https://cdn.example.com/articles \
//!     --root ~/llm/crawler/articles-ihre-artikel-new
//! ```

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use article_model::{Article, Block, Image, Paragraph, ParagraphKind};
use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(
    name = "article-import",
    version,
    about = "Create DynamoDB records for crawler/transcribe article folders"
)]
struct Args {
    /// A single article folder (repeatable).
    #[arg(short = 'd', long = "dir", value_name = "PATH")]
    dirs: Vec<PathBuf>,

    /// A root directory whose subfolders are article folders (repeatable).
    #[arg(short = 'r', long = "root", value_name = "PATH")]
    roots: Vec<PathBuf>,

    /// Owner of the imported articles.
    #[arg(long = "user-id", env = "TYPING_USER_ID")]
    user_id: String,

    /// Base URL of the cloud store holding the article folders. Each record's
    /// `data_directory` becomes `<prefix>/<folder-name>`.
    #[arg(long = "prefix", default_value = "", value_name = "URL")]
    prefix: String,

    /// DynamoDB table to write to.
    #[arg(long = "table", default_value = "translation", value_name = "NAME")]
    table: String,

    /// Resolve and print what would be imported without writing anything.
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Print each imported article as JSON (implies no DynamoDB write).
    #[arg(long = "dump-json")]
    dump_json: bool,
}

/// The subset of `article.json` this tool needs.
#[derive(Debug, Deserialize)]
struct CrawledArticle {
    #[serde(default)]
    url: String,
    #[serde(default)]
    canonical_url: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    kicker: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    section: String,
    #[serde(default)]
    authors: Vec<String>,
    #[serde(default)]
    published: String,
    #[serde(default)]
    modified: String,
    #[serde(default)]
    blocks: Vec<CrawledBlock>,
    #[serde(default)]
    images: Vec<CrawledImage>,
    #[serde(default)]
    audio: Option<CrawledAudio>,
}

#[derive(Debug, Deserialize)]
struct CrawledBlock {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    level: u8,
    #[serde(default)]
    index: usize,
}

#[derive(Debug, Deserialize)]
struct CrawledImage {
    #[serde(default)]
    alt: String,
    #[serde(default)]
    caption: String,
    #[serde(default)]
    credit: String,
    #[serde(default)]
    file: String,
}

#[derive(Debug, Deserialize)]
struct CrawledAudio {
    #[serde(default)]
    file: String,
    #[serde(default)]
    duration_ms: u64,
}

#[derive(Debug, Deserialize)]
struct Transcript {
    #[serde(default)]
    segments: Vec<Segment>,
}

#[derive(Debug, Deserialize)]
struct Segment {
    start: f64,
    end: f64,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct Translation {
    #[serde(default)]
    sentences: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let dirs = collect_dirs(&args)?;
    if dirs.is_empty() {
        anyhow::bail!("no article folders found (use --dir or --root)");
    }

    let prefix = args.prefix.trim_end_matches('/').to_string();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the unix epoch")?
        .as_millis() as u64;

    let client = if args.dry_run || args.dump_json {
        None
    } else {
        let config = aws_config::load_from_env().await;
        Some(aws_sdk_dynamodb::Client::new(&config))
    };

    // One version bump for the whole batch, exactly like the old crawler: every
    // article imported by this run shares the same library revision.
    let version = match &client {
        Some(client) => library_version::bump_version_for_user(client, &args.user_id).await,
        None => 0,
    };

    for (offset, dir) in dirs.iter().enumerate() {
        let folder = dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .with_context(|| format!("cannot determine folder name of {}", dir.display()))?;
        let mut article = build_article(dir, &prefix, &folder, &args.user_id)?;
        article.created_at = now + offset as u64;
        article.version = version;

        if let Some(client) = &client {
            client
                .put_item()
                .table_name(&args.table)
                .set_item(Some(serde_dynamo::to_item(&article)?))
                .send()
                .await
                .with_context(|| format!("failed to write {}", dir.display()))?;
            println!(
                "imported {} ({} paragraphs, {} blocks, translation: {})",
                folder,
                article.paragraphs.len(),
                article.blocks.len(),
                article.translated
            );
        } else if args.dump_json {
            println!("{}", serde_json::to_string_pretty(&article)?);
        } else {
            println!(
                "would import {} ({} paragraphs, {} blocks, translation: {})",
                folder,
                article.paragraphs.len(),
                article.blocks.len(),
                article.translated
            );
        }
    }

    Ok(())
}

/// Expand `--dir`/`--root` into the list of article folders to import.
fn collect_dirs(args: &Args) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    for dir in &args.dirs {
        if dir.join("article.json").is_file() {
            dirs.push(dir.clone());
        } else {
            anyhow::bail!("{} does not contain article.json", dir.display());
        }
    }
    for root in &args.roots {
        let mut children: Vec<PathBuf> = std::fs::read_dir(root)
            .with_context(|| format!("cannot read {}", root.display()))?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir() && path.join("article.json").is_file())
            .collect();
        children.sort();
        dirs.extend(children);
    }
    dirs.sort();
    dirs.dedup();
    Ok(dirs)
}

/// Convert one on-disk article folder into a DynamoDB [`Article`].
fn build_article(dir: &Path, prefix: &str, folder: &str, user_id: &str) -> Result<Article> {
    let crawled: CrawledArticle = read_json(&dir.join("article.json"))?;
    let transcript: Transcript =
        read_json(&dir.join("transcribe/transcript.json")).unwrap_or(Transcript {
            segments: Vec::new(),
        });
    let translation: Translation =
        read_json(&dir.join("transcribe/translation.json")).unwrap_or(Translation {
            sentences: Vec::new(),
        });

    let aligned = align(&crawled, &transcript.segments, &translation.sentences);

    let mut paragraphs = Vec::new();
    let mut blocks = Vec::new();
    for (block_index, block) in crawled.blocks.iter().enumerate() {
        match block.kind.as_str() {
            "image" => {
                if let Some(image) = crawled.images.get(block.index) {
                    blocks.push(Block::image(Image {
                        src: image.file.clone(),
                        alt: image.alt.clone(),
                        caption: image.caption.clone(),
                        credit: image.credit.clone(),
                    }));
                }
            }
            _ => {
                let is_heading = block.kind == "head";
                let index = paragraphs.len();
                let aligned = aligned.get(block_index);
                let paragraph = Paragraph {
                    original: block.text.clone(),
                    translation: aligned.and_then(|item| item.translation.clone()),
                    pairs: None,
                    kind: if is_heading {
                        ParagraphKind::Head
                    } else {
                        ParagraphKind::Para
                    },
                    level: block.level,
                    start: aligned.and_then(|item| item.start),
                    end: aligned.and_then(|item| item.end),
                    word_start: aligned.and_then(|item| item.word_start),
                };
                paragraphs.push(paragraph);
                if is_heading {
                    blocks.push(Block::heading(index, block.level));
                } else {
                    blocks.push(Block::paragraph(index));
                }
            }
        }
    }

    let translated = if paragraphs
        .iter()
        .any(|paragraph| paragraph.translation.is_some())
    {
        "true"
    } else {
        "false"
    };

    let data_directory = if prefix.is_empty() {
        None
    } else {
        Some(format!("{prefix}/{folder}"))
    };

    Ok(Article {
        user_id: user_id.to_string(),
        created_at: 0,
        translated: translated.to_string(),
        title: crawled.title,
        subtitle: crawled.kicker,
        description: crawled.description,
        section: crawled.section,
        authors: crawled.authors,
        published: crawled.published,
        modified: crawled.modified,
        source_url: if crawled.canonical_url.is_empty() {
            crawled.url
        } else {
            crawled.canonical_url
        },
        source_id: crawled.id,
        data_directory,
        audio: crawled.audio.as_ref().and_then(|audio| {
            if audio.file.is_empty() {
                None
            } else {
                Some(audio.file.clone())
            }
        }),
        audio_duration: crawled
            .audio
            .as_ref()
            .filter(|audio| audio.duration_ms > 0)
            .map(|audio| audio.duration_ms as f64 / 1000.0),
        blocks,
        paragraphs,
        version: 0,
    })
}

/// One block matched to its sentence range in the transcript.
#[derive(Debug, Default, Clone)]
struct AlignedBlock {
    translation: Option<String>,
    start: Option<f64>,
    end: Option<f64>,
    word_start: Option<usize>,
}

/// Match each text block in order to a run of consecutive transcript sentences.
///
/// The transcript is force-aligned to the article's own text, so a block's
/// wording is the concatenation of the sentences covering it. Walking a single
/// forward pointer and comparing normalized text keeps preamble sentences
/// (title, kicker, byline, image captions) out of the way without having to
/// model them.
fn align(
    crawled: &CrawledArticle,
    sentences: &[Segment],
    translations: &[String],
) -> Vec<AlignedBlock> {
    // Word index of the first word of each sentence, for `word_start`.
    let mut word_starts = Vec::with_capacity(sentences.len() + 1);
    let mut running = 0usize;
    for sentence in sentences {
        word_starts.push(running);
        running += sentence.text.split_whitespace().count();
    }
    word_starts.push(running);

    let mut pointer = 0usize;
    let mut result = Vec::new();
    for block in &crawled.blocks {
        if block.kind == "image" {
            result.push(AlignedBlock::default());
            continue;
        }
        let target = normalize(&block.text);
        if target.is_empty() {
            result.push(AlignedBlock::default());
            continue;
        }

        let Some(start) = (pointer..sentences.len())
            .find(|&index| is_prefix(&target, &normalize(&sentences[index].text)))
        else {
            result.push(AlignedBlock::default());
            continue;
        };

        let mut accumulated = String::new();
        let mut end = start;
        for index in start..sentences.len() {
            let sentence = normalize(&sentences[index].text);
            let candidate = if accumulated.is_empty() {
                sentence
            } else {
                format!("{accumulated} {sentence}")
            };
            if candidate == target || is_prefix(&target, &candidate) {
                accumulated = candidate;
                end = index;
                if accumulated == target {
                    break;
                }
            } else {
                break;
            }
        }

        let translation = translations
            .get(start..=end)
            .map(|slice| slice.join(" "))
            .filter(|text| !text.trim().is_empty());

        result.push(AlignedBlock {
            translation,
            start: sentences.get(start).map(|sentence| sentence.start),
            end: sentences.get(end).map(|sentence| sentence.end),
            word_start: word_starts.get(start).copied(),
        });
        pointer = end + 1;
    }
    result
}

/// `true` when `sentence` is the whole text or a word-boundary prefix of it.
fn is_prefix(text: &str, sentence: &str) -> bool {
    !sentence.is_empty() && (text == sentence || text.starts_with(&format!("{sentence} ")))
}

/// Lowercase, drop punctuation and collapse whitespace, so the article text and
/// the recognizer's transcript can be compared.
fn normalize(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut previous_space = false;
    for character in value.chars() {
        let character = character.to_lowercase().next().unwrap_or(character);
        if character.is_alphanumeric() {
            normalized.push(character);
            previous_space = false;
        } else if !previous_space && !normalized.is_empty() {
            normalized.push(' ');
            previous_space = true;
        }
    }
    normalized.trim_end().to_string()
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("cannot parse {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sentence(start: f64, end: f64, text: &str) -> Segment {
        Segment {
            start,
            end,
            text: text.to_string(),
        }
    }

    #[test]
    fn aligns_blocks_to_the_matching_sentences() {
        let crawled = CrawledArticle {
            blocks: vec![
                CrawledBlock {
                    kind: "para".to_string(),
                    text: "Erster Satz. Zweiter Satz.".to_string(),
                    level: 0,
                    index: 0,
                },
                CrawledBlock {
                    kind: "head".to_string(),
                    text: "Eine Überschrift".to_string(),
                    level: 3,
                    index: 0,
                },
            ],
            ..empty_article()
        };
        let sentences = vec![
            sentence(0.0, 1.0, "Titel"),
            sentence(1.0, 2.0, "Erster Satz."),
            sentence(2.0, 3.0, "Zweiter Satz."),
            sentence(3.0, 4.0, "Eine Überschrift"),
        ];
        let translations = vec![
            "Title".to_string(),
            "First sentence.".to_string(),
            "Second sentence.".to_string(),
            "A heading".to_string(),
        ];

        let aligned = align(&crawled, &sentences, &translations);
        assert_eq!(aligned.len(), 2);
        assert_eq!(aligned[0].start, Some(1.0));
        assert_eq!(aligned[0].end, Some(3.0));
        assert_eq!(
            aligned[0].translation.as_deref(),
            Some("First sentence. Second sentence.")
        );
        assert_eq!(aligned[1].word_start, Some(5));
    }

    #[test]
    fn normalize_ignores_punctuation_and_case() {
        assert_eq!(
            normalize("Der 7.-Oktober-Massaker!"),
            "der 7 oktober massaker"
        );
    }

    fn empty_article() -> CrawledArticle {
        CrawledArticle {
            url: String::new(),
            canonical_url: String::new(),
            id: String::new(),
            title: String::new(),
            kicker: String::new(),
            description: String::new(),
            section: String::new(),
            authors: Vec::new(),
            published: String::new(),
            modified: String::new(),
            blocks: Vec::new(),
            images: Vec::new(),
            audio: None,
        }
    }
}
