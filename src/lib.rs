use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[cfg(feature = "ssr")]
use axum::extract::FromRef;

use components::Association;
use leptos::LeptosOptions;
#[cfg(feature = "ssr")]
use tokio::sync::Mutex;

pub mod components;
mod types;
mod utils;

pub mod app;
pub mod error_template;
pub mod translation;

pub mod application_types;
pub mod translation_page;

#[cfg(feature = "ssr")]
pub mod fileserv;

#[cfg(feature = "ssr")]
#[derive(FromRef, Clone, Debug)]
pub struct AppState {
    pub sled: Arc<Mutex<sled::Db>>,
    pub leptos_options: LeptosOptions,
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}

#[cfg(feature = "ssr")]
static DB: std::sync::OnceLock<sled::Db> = std::sync::OnceLock::new();

#[cfg(feature = "ssr")]
pub async fn init_db() {
    let db: sled::Db = sled::open("./my_db").unwrap();
    DB.set(db).unwrap();
}
#[cfg(feature = "ssr")]
pub async fn get_db<'a>() -> &'a sled::Db {
    DB.get().unwrap()
}

pub type TypePairs = BTreeMap<usize, BTreeMap<usize, BTreeSet<Association>>>;
