use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub articles: Vec<Article>,
}

/// Per-user preferences and favourite articles, persisted to the
/// `translation_preferences` DynamoDB table (partition key `user_id`).
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPreferences {
    pub user_id: String,
    pub voice: String,
    pub current_paragraph_only: bool,
    pub group_matching_by_paragraph: bool,
    pub favorites: std::collections::HashSet<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Article {
    pub user_id: String,
    pub created_at: u64,
    pub translated: String,
    pub title: String,
    pub audio_directory: Option<String>,
    pub paragraphs: Vec<Paragraph>,
}

impl Article {
    pub fn from_pair(user_id: String, original: Vec<String>, translation: Vec<String>) -> Self {
        let title = original.first().unwrap().to_string();
        let paragraphs: Vec<Paragraph> = original
            .into_iter()
            .zip(translation)
            .map(|(original, translation)| Paragraph {
                original,
                translation: Some(translation),
                pairs: None,
            })
            .collect();
        Article {
            user_id,
            created_at: 0,
            translated: "false".to_string(),
            title,
            audio_directory: None,
            paragraphs,
        }
    }
    pub fn from_str(user_id: String, original: Vec<String>) -> Self {
        let title = original.first().unwrap().to_string();
        let paragraphs: Vec<Paragraph> = original
            .into_iter()
            .map(|original| Paragraph {
                original,
                translation: None,
                pairs: None,
            })
            .collect();

        Article {
            user_id,
            created_at: 0,
            translated: "false".to_string(),
            title,
            audio_directory: None,
            paragraphs,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paragraph {
    pub original: String,
    pub translation: Option<String>,
    pub pairs: Option<Vec<Pair>>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pair {
    pub original: Vec<usize>,
    pub translation: Vec<usize>,
}
