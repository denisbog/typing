use leptos::{server, ServerFnError};
use serde::{Deserialize, Serialize};

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
