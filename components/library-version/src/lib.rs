//! Shared per-user "library version" counter.
//!
//! The counter lives in the `translation_preferences` DynamoDB table (partition
//! key `user_id`) and is incremented atomically by every component that mutates
//! an article: the web app, the Spiegel crawler, the translation tool and the
//! voice tool. The web UI also reads it to decide whether its cached article
//! list is still current, and passes it back when fetching so the server only
//! returns articles changed since then.
//!
//! Keeping this in one crate means every producer of article changes uses the
//! exact same atomic `ADD` semantics and the same table name.

use std::collections::HashMap;

use aws_sdk_dynamodb::types::{AttributeValue, ReturnValue};
use aws_sdk_dynamodb::Client;

/// DynamoDB table holding the per-user preferences row, including the `version`
/// counter.
pub const PREFERENCES_TABLE: &str = "translation_preferences";

/// Attribute name of the counter inside [`PREFERENCES_TABLE`].
pub const VERSION_ATTRIBUTE: &str = "version";

/// Atomically increment (creating the row/attribute if absent) the library
/// version for `user_id` and return the new value.
///
/// `ADD` is used rather than a read-modify-write so concurrent writers (the UI,
/// crawler, translation tool, voice tool) can never lose an increment.
pub async fn bump_version_for_user(client: &Client, user_id: &str) -> u64 {
    let mut key = HashMap::new();
    key.insert(
        "user_id".to_string(),
        AttributeValue::S(user_id.to_string()),
    );
    let response = client
        .update_item()
        .table_name(PREFERENCES_TABLE)
        .set_key(Some(key))
        .update_expression("ADD #v :one")
        .expression_attribute_names("#v", VERSION_ATTRIBUTE)
        .expression_attribute_values(":one", AttributeValue::N("1".to_string()))
        .return_values(ReturnValue::UpdatedNew)
        .send()
        .await
        .expect("failed to bump the library version");
    version_from_attributes(response.attributes)
}

/// Read the current library version for `user_id`, or `0` when it was never
/// set. The read is strongly consistent so a just-committed write is visible.
pub async fn current_version_for_user(client: &Client, user_id: &str) -> u64 {
    let mut key = HashMap::new();
    key.insert(
        "user_id".to_string(),
        AttributeValue::S(user_id.to_string()),
    );
    let response = client
        .get_item()
        .table_name(PREFERENCES_TABLE)
        .set_key(Some(key))
        .consistent_read(true)
        .send()
        .await
        .expect("failed to read the library version");
    version_from_attributes(response.item)
}

fn version_from_attributes(attributes: Option<HashMap<String, AttributeValue>>) -> u64 {
    attributes
        .and_then(|attributes| {
            attributes
                .get(VERSION_ATTRIBUTE)
                .and_then(|value| value.as_n().ok())
                .and_then(|value| value.parse::<u64>().ok())
        })
        .unwrap_or(0)
}
