use leptos::{logging, server, ServerFnError};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct TranslationRequest {
    pub src: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslationResponse {
    pub translated: Vec<String>,
}
#[server(Api, "/api")]
pub async fn get_translations(original: String) -> Result<TranslationResponse, ServerFnError> {
    logging::log!("translations {}", original);

    let request = TranslationRequest {
        src: original.split("\n").map(str::to_string).collect(),
    };
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
