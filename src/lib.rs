#![recursion_limit = "512"]

use std::collections::{BTreeMap, BTreeSet, HashMap};

use components::Association;
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

pub const BUTTON_CLASS: &str = "inline-flex items-center justify-center text-sm font-medium text-zinc-200 px-4 py-2 rounded-lg \
     bg-zinc-800/80 border border-zinc-700/50 shadow-sm backdrop-blur \
     hover:bg-indigo-600/80 hover:border-indigo-500/50 hover:text-white \
     transition-all duration-150 cursor-pointer \
     text-nowrap whitespace-nowrap select-none";

pub const BUTTON_PRIMARY_CLASS: &str = "inline-flex items-center justify-center text-sm font-semibold text-white px-4 py-2 rounded-lg \
     bg-gradient-to-r from-indigo-600 to-violet-600 shadow-lg shadow-indigo-500/25 \
     hover:from-indigo-500 hover:to-violet-500 hover:shadow-indigo-500/40 \
     transition-all duration-150 cursor-pointer \
     text-nowrap whitespace-nowrap select-none";

pub const BUTTON_DANGER_CLASS: &str = "inline-flex items-center justify-center text-sm font-medium text-rose-300 px-4 py-2 rounded-lg \
     bg-rose-500/10 border border-rose-500/20 \
     hover:bg-rose-500/20 hover:text-rose-200 \
     transition-all duration-150 cursor-pointer \
     text-nowrap whitespace-nowrap select-none";

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
