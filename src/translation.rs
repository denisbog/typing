use leptos::prelude::*;
use leptos::{logging, server};
use server_fn::codec::Json;

use crate::{
    application_types::{Article, Data, UserPreferences},
    persistance::Persistance,
};

#[server(FetchData, "/store", input = Json)]
pub async fn get_data(id: String) -> Result<Data, ServerFnError> {
    logging::log!("fetching data for id {}", id);
    use crate::persistance::Persistance;
    let articles = crate::get_db().await.get_items_for_user(&id).await;
    Ok(Data { articles })
}

#[server(StoreArticle, "/store", input = Json)]
pub async fn store_article(article: Article) -> Result<(), ServerFnError> {
    let mut article = article;
    article.created_at = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    use crate::persistance::Persistance;
    crate::get_db().await.put_item_for_user(article).await;
    Ok(())
}

#[server(DeleteArticle, "/store", input = Json)]
pub async fn delete_article(article: Article) -> Result<(), ServerFnError> {
    use crate::persistance::Persistance;
    crate::get_db().await.delete_item_for_user(article).await;
    Ok(())
}
#[server(StorePairs, "/store", input = Json)]
pub async fn store_pairs(article: Article) -> Result<(), ServerFnError> {
    crate::get_db()
        .await
        .update_pairs_for_article(article)
        .await;
    Ok(())
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
