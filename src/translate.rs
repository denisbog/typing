use std::collections::HashMap;

use aws_sdk_dynamodb::types::{AttributeValue, AttributeValueUpdate};
use serde::{Deserialize, Serialize};
use serde_dynamo::from_items;
use typing::application_types::{Article, Paragraph};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub src: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslationResponse {
    pub translated: Vec<String>,
}

impl TranslationRequest {
    pub fn from_str(original: &String) -> Self {
        Self {
            src: original
                .split("\n")
                .map(str::to_string)
                .collect::<Vec<String>>(),
        }
    }
}
pub async fn get_translations(request: TranslationRequest) -> Result<TranslationResponse, String> {
    let client = reqwest::Client::new();
    let res: TranslationResponse = client
        .post("http://localhost:5000/translate")
        .json(&request)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    Ok(res)
}

#[tokio::main]
async fn main() {
    println!("translating");
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_dynamodb::Client::new(&config);
    let response = client
        .query()
        .table_name("translation")
        .index_name("translated-index")
        .key_condition_expression("translated = :translated")
        .expression_attribute_values(":translated", AttributeValue::S("false".to_string()))
        .send()
        .await
        .unwrap();

    let mut items: Vec<Article> = from_items(response.items.unwrap()).unwrap();
    println!("items {:?}", items);
    let items: Vec<&mut Article> = futures::future::join_all(items.iter_mut().map(|item| async {
        let paragraphs: Vec<String> = item
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.original.clone())
            .collect();
        let translations = get_translations(TranslationRequest {
            src: paragraphs.clone(),
        })
        .await
        .unwrap();
        item.paragraphs = paragraphs
            .into_iter()
            .zip(translations.translated.into_iter())
            .map(|(original, translation)| Paragraph {
                original,
                translation: Some(translation),
                pairs: None,
            })
            .collect();
        item
    }))
    .await;

    let config = aws_config::load_from_env().await;
    let client = aws_sdk_dynamodb::Client::new(&config);

    futures::future::join_all(items.into_iter().map(|item| async {
        let mut key = HashMap::new();
        key.insert(
            "user_id".to_string(),
            AttributeValue::S(item.user_id.clone()),
        );
        key.insert(
            "created_at".to_string(),
            AttributeValue::N(item.created_at.to_string()),
        );

        let temp: HashMap<String, AttributeValue> = serde_dynamo::to_item(item).unwrap();

        client
            .update_item()
            .table_name("translation")
            .set_key(Some(key))
            .attribute_updates(
                "translated",
                AttributeValueUpdate::builder()
                    .value(AttributeValue::S("true".to_string()))
                    .build(),
            )
            .attribute_updates(
                "paragraphs",
                AttributeValueUpdate::builder()
                    .value(temp.get("paragraphs").unwrap().clone())
                    .build(),
            )
            .send()
            .await
            .unwrap();
    }))
    .await;
}
