use leptos::{logging, server, ServerFnError};
use serde::{Deserialize, Serialize};

use crate::{
    application_types::{Article, Data},
    TypePairs,
};

#[server(FetchData, "/store")]
pub async fn get_data(id: String) -> Result<Data, ServerFnError> {
    logging::log!("fetching data");
    use crate::persistance::Persistance;
    let articles = crate::get_db().await.get_items_for_user(&id).await;
    Ok(Data { articles })
}

#[server(StoreArticle, "/store")]
pub async fn store_article(article: Article) -> Result<(), ServerFnError> {
    let mut article = article;
    article.created_at = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    use crate::persistance::Persistance;
    crate::get_db().await.put_item_for_user(article).await;
    Ok(())
}

#[server(DeleteArticle, "/store")]
pub async fn delete_article(article: Article) -> Result<(), ServerFnError> {
    use crate::persistance::Persistance;
    crate::get_db().await.delete_item_for_user(article).await;
    Ok(())
}
#[server(StorePairs, "/store")]
pub async fn store_pairs(article: Article, data: TypePairs) -> Result<(), ServerFnError> {
    Ok(())
}

#[server(FetchPairs, "/store")]
pub async fn get_pairs(id: String) -> Result<TypePairs, ServerFnError> {
    Ok(TypePairs::new())
}
