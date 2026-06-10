use std::collections::HashMap;

#[cfg(feature = "ssr")]
use aws_sdk_dynamodb::{types::AttributeValue, Client};
#[cfg(feature = "ssr")]
use serde_dynamo::from_items;

use crate::application_types::Article;

pub trait Persistance {
    fn get_items_for_user(
        &self,
        user_id: &str,
    ) -> impl std::future::Future<Output = Vec<Article>> + Send;
    fn put_item_for_user(&self, item: Article) -> impl std::future::Future<Output = ()> + Send;
    fn update_pairs_for_article(
        &self,
        item: Article,
    ) -> impl std::future::Future<Output = ()> + Send;
    fn delete_item_for_user(&self, item: Article) -> impl std::future::Future<Output = ()> + Send;
}

#[derive(Debug)]
#[cfg(feature = "ssr")]
pub struct AwsPersistance {
    client: Client,
}

#[cfg(feature = "ssr")]
impl AwsPersistance {
    pub async fn init() -> Self {
        let config = aws_config::load_from_env().await;
        AwsPersistance {
            client: aws_sdk_dynamodb::Client::new(&config),
        }
    }
}

#[cfg(feature = "ssr")]
impl Persistance for AwsPersistance {
    async fn get_items_for_user(&self, user_id: &str) -> Vec<Article> {
        let mut response = self
            .client
            .query()
            .table_name("translation")
            .key_condition_expression("user_id = :user_id")
            .expression_attribute_values(":user_id", AttributeValue::S(user_id.to_string()))
            .send()
            .await
            .unwrap();
        let mut items: Vec<Article> = from_items(response.items.unwrap()).unwrap();
        while response.last_evaluated_key.is_some() {
            response = self
                .client
                .query()
                .table_name("translation")
                .key_condition_expression("user_id = :user_id")
                .expression_attribute_values(":user_id", AttributeValue::S(user_id.to_string()))
                .set_exclusive_start_key(response.last_evaluated_key)
                .send()
                .await
                .unwrap();
            items.extend(from_items(response.items.unwrap()).unwrap());
        }
        items.into_iter().filter(|item| item.translated != "deleted").collect()
    }

    async fn put_item_for_user(&self, item: Article) {
        self.client
            .put_item()
            .table_name("translation")
            .set_item(Some(serde_dynamo::to_item(item).unwrap()))
            .send()
            .await
            .unwrap();
    }

    async fn delete_item_for_user(&self, item: Article) {
        let mut key = HashMap::new();
        key.insert("user_id".to_string(), AttributeValue::S(item.user_id));
        key.insert(
            "created_at".to_string(),
            AttributeValue::N(item.created_at.to_string()),
        );
        self.client
            .update_item()
            .table_name("translation")
            .set_key(Some(key))
            .update_expression("SET translated = :deleted")
            .expression_attribute_values(":deleted", AttributeValue::S("deleted".to_string()))
            .send()
            .await
            .unwrap();
    }

    async fn update_pairs_for_article(&self, item: Article) {
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
        self.client
            .update_item()
            .table_name("translation")
            .set_key(Some(key))
            .attribute_updates(
                "paragraphs",
                AttributeValueUpdate::builder()
                    .value(temp.get("paragraphs").unwrap().clone())
                    .build(),
            )
            .send()
            .await
            .unwrap();
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use aws_sdk_dynamodb::types::AttributeValue;
    use serde_dynamo::{from_item, from_items};

    use crate::{
        application_types::{Article, Paragraph},
        persistance::{AwsPersistance, Persistance},
    };

    #[tokio::test]
    async fn insert_new_item() {
        let item = Article {
            user_id: "user_id".to_string(),
            created_at: 0,
            translated: "false".to_string(),
            title: "test".to_string(),
            paragraphs: vec![Paragraph::default()],
        };

        let persistance = AwsPersistance::init().await;
        persistance.put_item_for_user(item).await;
    }

    #[tokio::test]
    async fn query_item() {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_dynamodb::Client::new(&config);
        let response = client
            .query()
            .table_name("translation")
            .key_condition_expression("user_id = :user_id")
            .expression_attribute_values(":user_id", AttributeValue::S("user_id".to_string()))
            .send()
            .await
            .unwrap();

        let item: Vec<Article> = from_items(response.items.unwrap()).unwrap();

        println!("items: {:?}", item);
    }
    #[tokio::test]
    async fn get_item() {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_dynamodb::Client::new(&config);
        let mut key = HashMap::new();

        key.insert(
            "user_id".to_string(),
            AttributeValue::S("user_id".to_string()),
        );
        key.insert(
            "created_at".to_string(),
            AttributeValue::N("123".to_string()),
        );
        let response = client
            .get_item()
            .table_name("translation")
            .set_key(Some(key))
            .send()
            .await
            .unwrap();

        let item: Article = from_item(response.item.unwrap()).unwrap();

        println!("tables: {:?}", item);
    }
}
