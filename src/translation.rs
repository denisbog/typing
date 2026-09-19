use leptos::prelude::*;
use leptos::{logging, server};
use server_fn::codec::Json;

use crate::{
    application_types::{Article, Data, UserPreferences},
    persistance::Persistance,
};

#[server(FetchData, "/store", input = Json)]
pub async fn get_data(id: String, since_version: u64) -> Result<(Data, u64), ServerFnError> {
    logging::log!("fetching data for id {} since {}", id, since_version);
    use crate::persistance::Persistance;
    let db = crate::get_db().await;
    // Read the version *before* the articles. If a write lands in between, the
    // articles are newer than the version, so the client records a version
    // that is behind the data: the next refresh will then fetch again. Reading
    // it after the articles would let the client claim a revision it does not
    // actually have and skip a needed refresh.
    let version = db.get_version_for_user(&id).await;
    // `since_version == 0` means the client has no snapshot yet and needs the
    // full library; otherwise only articles changed after that revision are
    // returned (including soft-deletes, which the client applies as removals).
    let articles = db.get_items_for_user(&id, since_version).await;
    Ok((Data { articles }, version))
}

#[server(StoreArticle, "/store", input = Json)]
pub async fn store_article(article: Article) -> Result<u64, ServerFnError> {
    let mut article = article;
    article.created_at = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    use crate::persistance::Persistance;
    Ok(crate::get_db().await.put_item_for_user(article).await)
}

#[server(DeleteArticle, "/store", input = Json)]
pub async fn delete_article(article: Article) -> Result<u64, ServerFnError> {
    use crate::persistance::Persistance;
    Ok(crate::get_db().await.delete_item_for_user(article).await)
}
#[server(StorePairs, "/store", input = Json)]
pub async fn store_pairs(article: Article) -> Result<u64, ServerFnError> {
    Ok(crate::get_db()
        .await
        .update_pairs_for_article(article)
        .await)
}

/// Lightweight library-version probe used by the UI before downloading the
/// full article list. Returns `0` for a user that has no preferences row yet.
#[server(GetVersion, "/store", input = Json)]
pub async fn get_version(user_id: String) -> Result<u64, ServerFnError> {
    use crate::persistance::Persistance;
    Ok(crate::get_db().await.get_version_for_user(&user_id).await)
}

#[server(GetPreferences, "/store", input = Json)]
pub async fn get_preferences(
    user_id: String,
) -> Result<Option<UserPreferences>, ServerFnError> {
    use crate::persistance::Persistance;
    Ok(crate::get_db().await.get_preferences_for_user(&user_id).await)
}

#[server(SavePreferences, "/store", input = Json)]
pub async fn save_preferences(preferences: UserPreferences) -> Result<(), ServerFnError> {
    use crate::persistance::Persistance;
    crate::get_db()
        .await
        .put_preferences_for_user(preferences)
        .await;
    Ok(())
}
