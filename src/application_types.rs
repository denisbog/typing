use serde::{Deserialize, Serialize};

use crate::translation::{get_translations, TranslationRequest};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub articles: Vec<Article>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Article {
    pub title: String,
    pub paragraphs: Vec<Paragraph>,
}

impl Article {
    pub fn from_pair(original: Vec<String>, translation: Vec<String>) -> Self {
        let title = original.iter().nth(0).unwrap().to_string();
        let paragraphs: Vec<Paragraph> = original
            .into_iter()
            .zip(translation)
            .map(|(original, translation)| Paragraph {
                original,
                translation,
                pairs: None,
            })
            .collect();
        Article { title, paragraphs }
    }
    pub async fn from_str(original: String) -> Self {
        let request = TranslationRequest::from_str(&original);
        let response = get_translations(request.clone()).await.unwrap();

        let title = request.src.iter().nth(0).unwrap().to_string();
        let paragraphs: Vec<Paragraph> = request
            .src
            .into_iter()
            .zip(response.translated)
            .map(|(original, translation)| Paragraph {
                original,
                translation,
                pairs: None,
            })
            .collect();

        Article { title, paragraphs }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paragraph {
    pub original: String,
    pub translation: String,
    pub pairs: Option<Vec<Pair>>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pair {
    pub orignal: Vec<usize>,
    pub traslation: Vec<usize>,
}
