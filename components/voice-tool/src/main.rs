use anyhow::Context;
use base64::Engine;
use clap::Parser;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_dynamo::from_items;
use std::{collections::HashMap, path::Path, path::PathBuf};
use tokio::fs;

use voice_tool::Article;

const API_URL: &str = "https://api.mistral.ai/v1";
const TTS_MODEL: &str = "voxtral-mini-tts-2603";
const TRANSCRIPTION_MODEL: &str = "voxtral-mini-2602";
const VOICE_ID: &str = "gb_oliver_neutral";

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "translation")]
    table: String,

    #[arg(long, default_value = "translated-index")]
    index: String,

    #[arg(long, default_value = ".")]
    prefix: String,

    #[arg(long, default_value = "generated")]
    out: String,

    #[arg(long, default_value_t = 1)]
    start: usize,

    #[arg(long, default_value_t = usize::MAX)]
    limit: usize,

    #[arg(long)]
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TtsResponse {
    audio_data: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExistingParagraphMetadata {
    generated_at: String,
    source: String,
    article_index: usize,
    article_number: usize,
    paragraph_index: usize,
    paragraph_number: usize,
    title: String,
    original: String,
    article_dir: String,
    paragraph_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParagraphFiles {
    metadata: String,
    text: String,
    audio: String,
    transcription: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParagraphManifestEntry {
    paragraph_index: usize,
    paragraph_number: usize,
    folder: String,
    path: String,
    files: ParagraphFiles,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArticleManifest {
    generated_at: String,
    source: String,
    article_index: usize,
    article_number: usize,
    title: String,
    directory: String,
    path: String,
    paragraph_count: usize,
    paragraphs: Vec<ParagraphManifestEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RootIndex {
    generated_at: String,
    source: String,
    articles: Vec<RootArticleSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RootArticleSummary {
    article_index: usize,
    article_number: usize,
    title: String,
    directory: String,
    metadata_file: String,
    paragraph_count: usize,
}

fn slugify(value: &str) -> String {
    let slug: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect();

    slug.split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(80)
        .collect::<String>()
        .trim_matches('-')
        .to_string()
        .if_empty_then("article")
}

trait IfEmptyThen {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl IfEmptyThen for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

fn pad(n: usize, width: usize) -> String {
    format!("{:0width$}", n, width = width)
}

fn article_dir_name(article_number: usize, title: &str) -> String {
    format!("{}-{}", pad(article_number, 3), slugify(title))
}

fn paragraph_folder_name(paragraph_number: usize) -> String {
    format!("paragraph-{}", pad(paragraph_number, 3))
}

async fn ensure_dir(dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dir).await?;
    Ok(())
}

async fn read_json<T: for<'de> Deserialize<'de>>(file: &Path) -> anyhow::Result<T> {
    Ok(serde_json::from_str(&fs::read_to_string(file).await?).context("failed to parse json")?)
}

async fn tts_and_transcribe(
    client: &reqwest::Client,
    text: &str,
    out_dir: &Path,
    api_key: &str,
) -> anyhow::Result<()> {
    let speech_res = client
        .post(format!("{API_URL}/audio/speech"))
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "input": text,
            "model": TTS_MODEL,
            "response_format": "mp3",
            "voice_id": VOICE_ID,
        }))
        .send()
        .await?;

    if !speech_res.status().is_success() {
        anyhow::bail!(
            "TTS failed ({}): {}",
            speech_res.status(),
            speech_res.text().await?
        );
    }

    let speech_json: TtsResponse = speech_res.json().await?;
    let audio_base64 = speech_json
        .audio_data
        .context("TTS response did not include audio_data")?;

    let audio_buffer = base64::engine::general_purpose::STANDARD
        .decode(audio_base64)
        .context("failed to decode audio_data")?;
    fs::write(out_dir.join("output.mp3"), &audio_buffer).await?;

    let form = Form::new()
        .text("model", TRANSCRIPTION_MODEL.to_string())
        .text("response_format", "verbose_json")
        .text("timestamp_granularities[]", "word")
        .part(
            "file",
            Part::bytes(audio_buffer)
                .file_name("output.mp3")
                .mime_str("audio/mpeg")?,
        );

    let trans_res = client
        .post(format!("{API_URL}/audio/transcriptions"))
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await?;

    if !trans_res.status().is_success() {
        anyhow::bail!(
            "Transcription failed ({}): {}",
            trans_res.status(),
            trans_res.text().await?
        );
    }

    let transcription: serde_json::Value = trans_res.json().await?;
    fs::write(
        out_dir.join("transcription.json"),
        serde_json::to_vec_pretty(&transcription)?,
    )
    .await?;

    Ok(())
}

async fn fetch_articles(
    client: &aws_sdk_dynamodb::Client,
    args: &Args,
) -> anyhow::Result<Vec<Article>> {
    let mut paginator = client
        .query()
        .table_name(&args.table)
        .index_name(&args.index)
        .key_condition_expression("translated = :translated")
        .expression_attribute_values(
            ":translated",
            aws_sdk_dynamodb::types::AttributeValue::S("true".to_string()),
        )
        .into_paginator()
        .send();

    let mut articles = Vec::new();
    while let Some(page) = paginator.next().await {
        let page = page?;
        if let Some(items) = page.items {
            let mut page_articles: Vec<Article> = from_items(items)?;
            articles.append(&mut page_articles);
        }
    }
    Ok(articles)
}

fn is_guardrail_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.contains("guardrail_violation")
        || msg.contains("Request blocked by guardrail policy")
        || msg.contains("403 Forbidden")
}

async fn update_article_status(
    client: &aws_sdk_dynamodb::Client,
    table: &str,
    article: &Article,
    audio_directory: &str,
) -> anyhow::Result<()> {
    use aws_sdk_dynamodb::types::{AttributeValue, AttributeValueUpdate};

    let mut key = HashMap::new();
    key.insert("user_id".to_string(), AttributeValue::S(article.user_id.clone()));
    key.insert(
        "created_at".to_string(),
        AttributeValue::N(article.created_at.to_string()),
    );

    client
        .update_item()
        .table_name(table)
        .set_key(Some(key))
        .attribute_updates(
            "translated",
            AttributeValueUpdate::builder()
                .value(AttributeValue::S("voice".to_string()))
                .build(),
        )
        .attribute_updates(
            "audio_directory",
            AttributeValueUpdate::builder()
                .value(AttributeValue::S(audio_directory.to_string()))
                .build(),
        )
        .send()
        .await?;

    Ok(())
}

async fn read_existing_paragraph_record(
    article_dir_name: &str,
    paragraph_dir: &Path,
) -> anyhow::Result<ParagraphManifestEntry> {
    let metadata: ExistingParagraphMetadata =
        read_json(&paragraph_dir.join("metadata.json")).await?;
    Ok(ParagraphManifestEntry {
        paragraph_index: metadata.paragraph_index.saturating_sub(1),
        paragraph_number: metadata.paragraph_number,
        folder: paragraph_folder_name(metadata.paragraph_number),
        path: format!(
            "{}/{}",
            article_dir_name,
            paragraph_folder_name(metadata.paragraph_number)
        ),
        files: ParagraphFiles {
            metadata: "metadata.json".to_string(),
            text: "text.txt".to_string(),
            audio: "output.mp3".to_string(),
            transcription: "transcription.json".to_string(),
        },
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let api_key = args
        .api_key
        .clone()
        .or_else(|| std::env::var("MISTRAL_API_KEY").ok())
        .context("Missing MISTRAL_API_KEY environment variable")?;

    let output_root = PathBuf::from(&args.out);
    ensure_dir(&output_root).await?;

    println!("fetch items from dynamodb");
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_dynamodb::Client::new(&config);
    let http_client = reqwest::Client::new();

    let mut articles = fetch_articles(&client, &args).await?;
    let total = articles.len();
    let start_offset = args.start.saturating_sub(1);
    articles = articles.into_iter().skip(start_offset).collect();
    if args.limit != usize::MAX {
        articles.truncate(args.limit);
    }

    let generated_at = chrono::Utc::now().to_rfc3339();
    let source = format!(
        "dynamodb://{}?index={}&translated=true",
        args.table, args.index
    );
    let mut root_index = RootIndex {
        generated_at: generated_at.clone(),
        source: source.clone(),
        articles: Vec::new(),
    };

    let mut processed = 0usize;

    for (offset, article) in articles.into_iter().enumerate() {
        let article_number = args.start + offset;
        let article_index = article_number.saturating_sub(1);
        let title = if article.title.trim().is_empty() {
            format!("Article {}", article_number)
        } else {
            article.title.clone()
        };
        let article_dir_name = article_dir_name(article_number, &title);
        let article_dir = output_root.join(&article_dir_name);
        let article_audio_directory = PathBuf::from(&args.prefix)
            .join(&args.out)
            .join(&article_dir_name)
            .to_string_lossy()
            .to_string();
        ensure_dir(&article_dir).await?;

        let mut paragraph_records = Vec::new();
        let paragraphs: Vec<String> = article
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.original.trim().to_string())
            .filter(|text| !text.is_empty())
            .collect();

        println!(
            "processing: {} {} ({}/{})",
            article.user_id,
            article.created_at,
            processed + 1,
            total
        );

        for (paragraph_number, original) in paragraphs.into_iter().enumerate() {
            let paragraph_number = paragraph_number + 1;
            let folder = paragraph_folder_name(paragraph_number);
            let paragraph_dir = article_dir.join(&folder);

            if fs::try_exists(&paragraph_dir).await? {
                paragraph_records
                    .push(read_existing_paragraph_record(&article_dir_name, &paragraph_dir).await?);
                continue;
            }

            ensure_dir(&paragraph_dir).await?;
            fs::write(
                paragraph_dir.join("metadata.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "generatedAt": generated_at.clone(),
                    "source": source.clone(),
                    "articleIndex": article_number,
                    "articleNumber": article_number,
                    "paragraphIndex": paragraph_number,
                    "paragraphNumber": paragraph_number,
                    "title": title.clone(),
                    "original": original.clone(),
                    "articleDir": article_dir_name.clone(),
                    "paragraphDir": folder.clone(),
                }))?,
            )
            .await?;
            fs::write(paragraph_dir.join("text.txt"), &original).await?;

            println!("generating {}", paragraph_dir.display());
            match tts_and_transcribe(&http_client, &original, &paragraph_dir, &api_key).await {
                Ok(()) => {
                    paragraph_records.push(ParagraphManifestEntry {
                        paragraph_index: paragraph_number - 1,
                        paragraph_number,
                        folder: folder.clone(),
                        path: format!("{}/{}", article_dir_name, folder),
                        files: ParagraphFiles {
                            metadata: "metadata.json".to_string(),
                            text: "text.txt".to_string(),
                            audio: "output.mp3".to_string(),
                            transcription: "transcription.json".to_string(),
                        },
                    });
                }
                Err(err) if is_guardrail_error(&err) => {
                    eprintln!(
                        "warning: skipping guardrail-blocked paragraph {} for article {} {}: {}",
                        paragraph_number,
                        article.user_id,
                        article.created_at,
                        err
                    );
                    continue;
                }
                Err(err) => return Err(err),
            }

        }

        let article_metadata = ArticleManifest {
            generated_at: generated_at.clone(),
            source: source.clone(),
            article_index,
            article_number,
            title: title.clone(),
            directory: article_dir_name.clone(),
            path: article_dir_name.clone(),
            paragraph_count: paragraph_records.len(),
            paragraphs: paragraph_records,
        };
        fs::write(
            article_dir.join("metadata.json"),
            serde_json::to_vec_pretty(&article_metadata)?,
        )
        .await?;

        root_index.articles.push(RootArticleSummary {
            article_index,
            article_number,
            title,
            directory: article_dir_name,
            metadata_file: "metadata.json".to_string(),
            paragraph_count: article_metadata.paragraph_count,
        });

        update_article_status(
            &client,
            &args.table,
            &article,
            &article_audio_directory,
        )
        .await?;

        processed += 1;
    }

    fs::write(
        output_root.join("index.json"),
        serde_json::to_vec_pretty(&root_index)?,
    )
    .await?;

    println!(
        "Done. Generated {} article(s) in {}",
        processed,
        output_root.display()
    );
    Ok(())
}
