use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Article {
    pub user_id: String,
    pub created_at: u64,
    pub translated: String,
    pub title: String,
    pub paragraphs: Vec<Paragraph>,
    pub audio_directory: Option<String>,
    /// Library version at which this article was last written. Stamped by the
    /// voice tool after incrementing the user's version counter.
    #[serde(default)]
    pub version: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paragraph {
    pub original: String,
    pub translation: Option<String>,
}
