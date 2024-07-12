use leptos::{logging, server, use_context, ServerFnError};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use crate::AppState;

use crate::application_types::Data;
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
    let app_state = use_context::<AppState>();
    app_state
        .unwrap()
        .sled
        .lock()
        .await
        .insert(id.as_bytes(), serde_json::to_vec(&data).unwrap())
        .unwrap();
    Ok(())
}

#[server(FetchData, "/store")]
pub async fn get_data(id: String) -> Result<Data, ServerFnError> {
    logging::log!("getting Data");
    let app_state = use_context::<AppState>();
    Ok(serde_json::from_slice(
        std::str::from_utf8(
            &app_state
                .unwrap()
                .sled
                .lock()
                .await
                .get(id.as_bytes())
                .unwrap()
                .unwrap(),
        )
        .unwrap()
        .as_bytes(),
    )
    .unwrap())
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
