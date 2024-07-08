use crate::translation::{get_translations, TranslationRequest};

#[derive(Default, Clone)]
pub struct Data {
    pub articles: Vec<Article>,
}

#[derive(Default, Clone)]
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
                pairs: Vec::<Pair>::new(),
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
                pairs: Vec::<Pair>::new(),
            })
            .collect();

        Article { title, paragraphs }
    }
}

#[derive(Default, Clone)]
pub struct Paragraph {
    pub original: String,
    pub translation: String,
    pub pairs: Vec<Pair>,
}

#[derive(Default, Clone)]
pub struct Pair {
    pub orignal: Vec<usize>,
    pub traslation: Vec<usize>,
}
