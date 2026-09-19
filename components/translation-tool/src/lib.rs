use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Article {
    pub user_id: String,
    pub created_at: u64,
    pub translated: String,
    pub title: String,
    pub paragraphs: Vec<Paragraph>,
    /// Library version this article was written at. Bumped by the translation
    /// tool when it stores the translated paragraphs.
    #[serde(default)]
    pub version: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paragraph {
    pub original: String,
    pub translation: Option<String>,
}
