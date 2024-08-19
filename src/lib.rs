use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "ssr")]
use axum::extract::FromRef;

use components::Association;
use leptos::LeptosOptions;

pub mod components;
mod types;
mod utils;

pub mod app;
pub mod error_template;
pub mod translation;

pub mod application_types;
pub mod persistance;
pub mod translation_page;

pub const BUTTON_CLASS: &'static str =
    "size-fit text-md lg:text-xl m-2 p-2 shadow-md rounded bg-gray-300 cursor-pointer";

#[cfg(feature = "ssr")]
pub mod fileserv;

#[cfg(feature = "ssr")]
#[derive(FromRef, Clone, Debug)]
pub struct AppState {
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

pub type TypePairs = BTreeMap<usize, BTreeMap<usize, BTreeSet<Association>>>;
