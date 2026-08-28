#![recursion_limit = "512"]

use std::collections::{BTreeMap, BTreeSet, HashMap};

use components::Association;
#[cfg(feature = "ssr")]
use leptos::prelude::LeptosOptions;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub mod components;
mod types;
mod utils;

pub mod app;
pub mod translation;

pub mod application_types;
pub mod persistance;
pub mod translation_page;
pub mod local_store;
pub mod properties_page;
pub mod matching_page;

pub const BUTTON_CLASS: &str = "btn";

pub const BUTTON_PRIMARY_CLASS: &str = "btn-primary";

pub const BUTTON_DANGER_CLASS: &str = "btn-danger";

#[cfg(feature = "ssr")]
#[derive(Clone, Debug)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use leptos::mount::mount_to_body;

    use crate::app::*;
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[cfg(feature = "ssr")]
use crate::persistance::AwsPersistance;

#[cfg(feature = "ssr")]
static DB: std::sync::OnceLock<AwsPersistance> = std::sync::OnceLock::new();

#[cfg(feature = "ssr")]
pub async fn init_db() {
    DB.set(AwsPersistance::init().await).unwrap();
}
#[cfg(feature = "ssr")]
pub async fn get_db<'a>() -> &'a AwsPersistance {
    DB.get().unwrap()
}

#[derive(Deserialize, Serialize)]
pub struct UserInfo {
    pub sub: String,
    pub email_verified: String,
    pub email: String,
    pub username: String,
}

pub async fn get_user_info(hash: HashMap<String, String>) -> UserInfo {
    let client = Client::new();

    let body = client
        .get("https://typing.auth.us-east-1.amazoncognito.com/oauth2/userInfo")
        .header(
            reqwest::header::AUTHORIZATION,
            format!(
                "{} {}",
                hash.get("token_type").unwrap(),
                hash.get("access_token").unwrap()
            ),
        );
    body.send().await.unwrap().json::<UserInfo>().await.unwrap()
}

pub fn parse_hash(hash: String) -> HashMap<String, String> {
    let hash = &hash[1..];
    let hash = hash
        .split("&")
        .fold(HashMap::<String, String>::new(), |mut acc, item| {
            let parts: Vec<&str> = item.split("=").collect();
            acc.insert(parts[0].to_string(), parts[1].to_string());
            acc
        });
    hash
}

pub type TypePairs = BTreeMap<usize, BTreeSet<Association>>;
