use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub articles: Vec<Article>,
}

/// Per-user preferences and favourite articles, persisted to the
/// `translation_preferences` DynamoDB table (partition key `user_id`).
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserPreferences {
    pub user_id: String,
    pub voice: String,
    pub current_paragraph_only: bool,
    pub group_matching_by_paragraph: bool,
    pub favorites: std::collections::HashSet<String>,
    /// Monotonic version of the user's article library. Every write to an
    /// article (add/edit/delete, including the import tool and translation
    /// step) increments this counter, and the same value is stamped onto the
    /// article. The UI compares this with its cached copy before fetching the
    /// full library from the server.
    pub version: u64,
}

/// One article in the user's library, stored in the `translation` DynamoDB
/// table and identified by (`user_id`, `created_at`).
///
/// The shape mirrors the on-disk output of the `~/llm/crawler` tool (its
/// `article.json`, `images/` and `audio/` folders) enriched with the
/// sentence-level translation and timing produced by `~/llm/transcribe`.
/// The heavy payloads (images, audio, word-level transcription) live in the
/// cloud store; this record keeps the article's text, its translation, the
/// interleaved display blocks and the per-paragraph media offsets.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Article {
    pub user_id: String,
    pub created_at: u64,
    /// Translation state: `"false"` (untranslated), `"true"`/`"voice"`
    /// (translated), or `"deleted"` (soft-deleted tombstone).
    pub translated: String,
    /// Headline, without the kicker or lead.
    pub title: String,
    /// Kicker / overline, e.g. `"DER SPIEGEL"`.
    pub subtitle: String,
    /// Lead / teaser paragraph shown on the article card.
    pub description: String,
    /// Section, e.g. `"Panorama"`.
    pub section: String,
    /// Byline authors.
    pub authors: Vec<String>,
    /// ISO-8601 publication timestamp.
    pub published: String,
    /// ISO-8601 last-modified timestamp.
    pub modified: String,
    /// Canonical article URL.
    pub source_url: String,
    /// SPIEGEL document id (the UUID in the URL).
    pub source_id: String,
    /// Base URL of the article folder in the cloud store. Images resolve as
    /// `<data_directory>/<image.src>`, audio as `<data_directory>/<audio>` and
    /// the word-level transcription as
    /// `<data_directory>/transcribe/transcription.json`.
    pub data_directory: Option<String>,
    /// Audio file path relative to `data_directory`.
    pub audio: Option<String>,
    /// Audio duration in seconds, when known.
    pub audio_duration: Option<f64>,
    /// Ordered display blocks (paragraphs, headings and images).
    pub blocks: Vec<Block>,
    /// Text units used for typing and word pairing. `block.paragraph` indexes
    /// into this list.
    pub paragraphs: Vec<Paragraph>,
    /// Library version at which this article was last written. Mirrors
    /// [`UserPreferences::version`] at write time.
    pub version: u64,
}

impl Article {
    /// Build a minimal article from a list of original paragraphs, used by the
    /// manual "add article" form in the UI.
    pub fn from_str(user_id: String, original: Vec<String>) -> Self {
        let title = original.first().cloned().unwrap_or_default();
        let paragraphs: Vec<Paragraph> = original
            .into_iter()
            .map(|original| Paragraph {
                original,
                translation: None,
                pairs: None,
                kind: ParagraphKind::Para,
                level: 0,
                start: None,
                end: None,
                word_start: None,
            })
            .collect();
        let blocks = paragraphs
            .iter()
            .enumerate()
            .map(|(index, _)| Block::paragraph(index))
            .collect();

        Article {
            user_id,
            created_at: 0,
            translated: "false".to_string(),
            title,
            blocks,
            paragraphs,
            ..Article::default()
        }
    }

    /// Build an article from parallel original/translation paragraph lists.
    pub fn from_pair(user_id: String, original: Vec<String>, translation: Vec<String>) -> Self {
        let title = original.first().cloned().unwrap_or_default();
        let paragraphs: Vec<Paragraph> = original
            .into_iter()
            .zip(translation)
            .map(|(original, translation)| Paragraph {
                original,
                translation: Some(translation),
                pairs: None,
                kind: ParagraphKind::Para,
                level: 0,
                start: None,
                end: None,
                word_start: None,
            })
            .collect();
        let blocks = paragraphs
            .iter()
            .enumerate()
            .map(|(index, _)| Block::paragraph(index))
            .collect();

        Article {
            user_id,
            created_at: 0,
            translated: "false".to_string(),
            title,
            blocks,
            paragraphs,
            ..Article::default()
        }
    }
}

/// An entry in the ordered article body. Blocks reference a text paragraph by
/// index so the same paragraph data drives both display and pairing.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Block {
    /// `"paragraph"` or `"image"`.
    pub kind: BlockKind,
    /// Index into [`Article::paragraphs`] for text blocks.
    pub paragraph: Option<usize>,
    /// Heading level for paragraph blocks that are headings (0 = body text).
    pub level: u8,
    /// Image payload for `kind == "image"`.
    pub image: Option<Image>,
}

impl Block {
    pub fn paragraph(index: usize) -> Self {
        Block {
            kind: BlockKind::Paragraph,
            paragraph: Some(index),
            level: 0,
            image: None,
        }
    }

    pub fn heading(index: usize, level: u8) -> Self {
        Block {
            kind: BlockKind::Paragraph,
            paragraph: Some(index),
            level,
            image: None,
        }
    }

    pub fn image(image: Image) -> Self {
        Block {
            kind: BlockKind::Image,
            paragraph: None,
            level: 0,
            image: Some(image),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockKind {
    #[default]
    Paragraph,
    Image,
}

/// An image belonging to an article, stored by its path relative to
/// [`Article::data_directory`].
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Image {
    /// Path relative to the article's `data_directory`, e.g.
    /// `images/01_…_w2048.webp`.
    pub src: String,
    pub alt: String,
    pub caption: String,
    pub credit: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParagraphKind {
    #[default]
    Para,
    Head,
}

/// A text unit: one paragraph or heading of the article together with its
/// translation, saved word pairs and the audio timing that anchors it to the
/// article's audio track.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Paragraph {
    pub original: String,
    pub translation: Option<String>,
    pub pairs: Option<Vec<Pair>>,
    pub kind: ParagraphKind,
    /// Heading level for [`ParagraphKind::Head`].
    pub level: u8,
    /// Start of this unit in the article audio, in seconds.
    pub start: Option<f64>,
    /// End of this unit in the article audio, in seconds.
    pub end: Option<f64>,
    /// Index of this unit's first word in the article's word-level
    /// transcription, used to slice the speech cues for word highlighting.
    pub word_start: Option<usize>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pair {
    pub original: Vec<usize>,
    pub translation: Vec<usize>,
}
