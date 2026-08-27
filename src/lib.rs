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

pub const BUTTON_CLASS: &str = "inline-flex items-center justify-center gap-1.5 rounded-md border border-white/[0.08] \
     bg-white/[0.045] px-3 py-1.5 text-sm font-semibold text-slate-400 \
     transition-colors duration-150 hover:border-white/25 hover:bg-white/[0.08] hover:text-white \
     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-400/50 cursor-pointer \
     text-nowrap whitespace-nowrap select-none disabled:pointer-events-none disabled:opacity-40";

pub const BUTTON_PRIMARY_CLASS: &str = "inline-flex items-center justify-center gap-1.5 rounded-md border border-white/10 \
     bg-slate-200 px-3 py-1.5 text-sm font-bold text-slate-900 \
     transition-colors duration-150 hover:bg-white \
     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-300/50 cursor-pointer \
     text-nowrap whitespace-nowrap select-none disabled:pointer-events-none disabled:opacity-40";

pub const BUTTON_DANGER_CLASS: &str = "inline-flex items-center justify-center gap-1.5 rounded-md border border-white/[0.08] \
     bg-white/[0.045] px-3 py-1.5 text-sm font-semibold text-slate-400 \
     transition-colors duration-150 hover:border-rose-300/30 hover:bg-white/[0.08] hover:text-rose-200 \
     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-400/50 cursor-pointer \
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
