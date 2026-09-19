#[cfg(feature = "ssr")]
use std::collections::HashMap;

#[cfg(feature = "ssr")]
use aws_sdk_dynamodb::{types::AttributeValue, types::AttributeValueUpdate, Client};
#[cfg(feature = "ssr")]
use serde_dynamo::{from_item, from_items};

use crate::application_types::{Article, UserPreferences};

pub trait Persistance {
    /// Articles owned by `user_id`. When `since_version` is non-zero only
    /// articles whose `version` is greater are returned (the delta since the
    /// client's snapshot), including soft-deleted ones so the client can drop
    /// them. A `since_version` of `0` returns the full live library.
    fn get_items_for_user(
        &self,
        user_id: &str,
        since_version: u64,
    ) -> impl std::future::Future<Output = Vec<Article>> + Send;
    /// Store a new/updated article and return the new library version stamped
    /// onto it.
    fn put_item_for_user(&self, item: Article) -> impl std::future::Future<Output = u64> + Send;
    /// Persist an article's paragraph pairs and return the new library version.
    fn update_pairs_for_article(
        &self,
        item: Article,
    ) -> impl std::future::Future<Output = u64> + Send;
    /// Soft-delete an article and return the new library version.
    fn delete_item_for_user(&self, item: Article) -> impl std::future::Future<Output = u64> + Send;
    /// Atomically increment the user's library version and return the new value.
    fn bump_version_for_user(&self, user_id: &str) -> impl std::future::Future<Output = u64> + Send;
    /// Current library version for the user (`0` when never set).
    fn get_version_for_user(&self, user_id: &str) -> impl std::future::Future<Output = u64> + Send;
    fn get_preferences_for_user(
        &self,
        user_id: &str,
    ) -> impl std::future::Future<Output = Option<UserPreferences>> + Send;
    fn put_preferences_for_user(
        &self,
        preferences: UserPreferences,
    ) -> impl std::future::Future<Output = ()> + Send;
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
    async fn get_items_for_user(&self, user_id: &str, since_version: u64) -> Vec<Article> {
        let mut items: Vec<Article> = Vec::new();
        let mut exclusive_start_key = None;
        loop {
            let mut request = self
                .client
                .query()
                .table_name("translation")
                .key_condition_expression("user_id = :user_id")
                .expression_attribute_values(":user_id", AttributeValue::S(user_id.to_string()));
            // Incremental syncs only need articles changed since the version the
            // client already holds. Soft-deleted rows are included here (their
            // version was bumped by the delete) so the client can remove them;
            // full syncs filter them out below.
            if since_version > 0 {
                request = request
                    .filter_expression("#v > :since")
                    .expression_attribute_names("#v", "version")
                    .expression_attribute_values(":since", AttributeValue::N(since_version.to_string()));
            }
            if let Some(key) = exclusive_start_key.take() {
                request = request.set_exclusive_start_key(Some(key));
            }
            let response = request.send().await.unwrap();
            if let Some(page) = response.items {
                items.extend(from_items(page).unwrap());
            }
            match response.last_evaluated_key {
                Some(key) => exclusive_start_key = Some(key),
                None => break,
            }
        }
        if since_version == 0 {
            items.retain(|item| item.translated != "deleted");
        }
        items
    }

    async fn put_item_for_user(&self, item: Article) -> u64 {
        let version = self.bump_version_for_user(&item.user_id).await;
        let mut item = item;
        item.version = version;
        self.client
            .put_item()
            .table_name("translation")
            .set_item(Some(serde_dynamo::to_item(item).unwrap()))
            .send()
            .await
            .unwrap();
        version
    }

    async fn delete_item_for_user(&self, item: Article) -> u64 {
        let version = self.bump_version_for_user(&item.user_id).await;
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
            .update_expression("SET translated = :deleted, #v = :version")
            .expression_attribute_names("#v", "version")
            .expression_attribute_values(":deleted", AttributeValue::S("deleted".to_string()))
            .expression_attribute_values(":version", AttributeValue::N(version.to_string()))
            .send()
            .await
            .unwrap();
        version
    }

    async fn update_pairs_for_article(&self, item: Article) -> u64 {
        let version = self.bump_version_for_user(&item.user_id).await;
        let mut item = item;
        item.version = version;
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
            .attribute_updates(
                "version",
                AttributeValueUpdate::builder()
                    .value(AttributeValue::N(version.to_string()))
                    .build(),
            )
            .send()
            .await
            .unwrap();
        version
    }

    /// Increment (and create if absent) the per-user `version` attribute in the
    /// `translation_preferences` table, returning the resulting value. Shared
    /// with the crawler / translation tool / voice tool via the
    /// `library-version` crate so all writers use the same atomic update.
    async fn bump_version_for_user(&self, user_id: &str) -> u64 {
        library_version::bump_version_for_user(&self.client, user_id).await
    }

    async fn get_version_for_user(&self, user_id: &str) -> u64 {
        library_version::current_version_for_user(&self.client, user_id).await
    }

    async fn get_preferences_for_user(&self, user_id: &str) -> Option<UserPreferences> {
        let mut key = HashMap::new();
        key.insert(
            "user_id".to_string(),
            AttributeValue::S(user_id.to_string()),
        );
        let response = self
            .client
            .get_item()
            .table_name("translation_preferences")
            .set_key(Some(key))
            // The library version is used to decide whether cached data is
            // stale, so it must not be served from a stale replica.
            .consistent_read(true)
            .send()
            .await
            .unwrap();
        response.item.map(|item| from_item(item).unwrap())
    }

    /// Save the user's editable preferences without touching the library
    /// `version`: the version is server-owned (bumped by article writes), so a
    /// stale client copy must never overwrite it. Only the attributes the
    /// client actually controls are written.
    async fn put_preferences_for_user(&self, preferences: UserPreferences) {
        let mut key = HashMap::new();
        key.insert(
            "user_id".to_string(),
            AttributeValue::S(preferences.user_id.clone()),
        );
        let item: HashMap<String, AttributeValue> =
            serde_dynamo::to_item(preferences).unwrap();
        self.client
            .update_item()
            .table_name("translation_preferences")
            .set_key(Some(key))
            .update_expression(
                "SET voice = :voice, current_paragraph_only = :current_paragraph_only, \
                 group_matching_by_paragraph = :group_matching_by_paragraph, \
                 favorites = :favorites",
            )
            .expression_attribute_values(":voice", item.get("voice").cloned().unwrap())
            .expression_attribute_values(
                ":current_paragraph_only",
                item.get("current_paragraph_only").cloned().unwrap(),
            )
            .expression_attribute_values(
                ":group_matching_by_paragraph",
                item.get("group_matching_by_paragraph").cloned().unwrap(),
            )
            .expression_attribute_values(
                ":favorites",
                item.get("favorites").cloned().unwrap(),
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
            audio_directory: None,
            paragraphs: vec![Paragraph::default()],
            version: 0,
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
