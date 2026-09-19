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
    /// article (add/edit/delete, including the crawler and translation tool)
    /// increments this counter, and the same value is stamped onto the
    /// article. The UI compares this with its cached copy before fetching the
    /// full library from the server.
    pub version: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Article {
    pub user_id: String,
    pub created_at: u64,
    pub translated: String,
    pub title: String,
    pub audio_directory: Option<String>,
    pub paragraphs: Vec<Paragraph>,
    /// Library version at which this article was last written. Mirrors
    /// [`UserPreferences::version`] at write time.
    #[serde(default)]
    pub version: u64,
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
            version: 0,
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
            version: 0,
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
