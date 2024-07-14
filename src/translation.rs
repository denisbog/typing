use leptos::{logging, server, ServerFnError};
use serde::{Deserialize, Serialize};

use crate::{application_types::Data, TypePairs};
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

#[server(Store, "/store")]
pub async fn store_data(id: String, data: Data) -> Result<(), ServerFnError> {
    crate::get_db()
        .await
        .insert(id.as_bytes(), serde_json::to_vec(&data).unwrap())
        .unwrap();
    Ok(())
}

#[server(StorePairs, "/store")]
pub async fn store_pairs(id: String, data: TypePairs) -> Result<(), ServerFnError> {
    let id = format!("{}-pairs", id);
    crate::get_db()
        .await
        .insert(id.as_bytes(), serde_json::to_vec(&data).unwrap())
        .unwrap();
    Ok(())
}
#[server(FetchData, "/store")]
pub async fn get_data(id: String) -> Result<Data, ServerFnError> {
    logging::log!("fetching data");
    Ok(serde_json::from_slice(
        std::str::from_utf8(&crate::get_db().await.get(id.as_bytes()).unwrap().unwrap())
            .unwrap()
            .as_bytes(),
    )
    .unwrap())
}

#[server(FetchPairs, "/store")]
pub async fn get_pairs(id: String) -> Result<TypePairs, ServerFnError> {
    let id = format!("{}-pairs", id);
    logging::log!("fetching pairs {}", id);
    let response = if let Ok(Some(pairs)) = crate::get_db().await.get(id.as_bytes()) {
        serde_json::from_slice(std::str::from_utf8(&pairs).unwrap().as_bytes()).unwrap()
    } else {
        TypePairs::new()
    };
    logging::log!("fetching pairs {}. return", id);
    Ok(response)
}
#[server(Api, "/api")]
pub async fn get_translations(
    request: TranslationRequest,
) -> Result<TranslationResponse, ServerFnError> {
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
