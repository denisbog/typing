use std::sync::Arc;

#[cfg(feature = "ssr")]
use axum::extract::FromRef;

use leptos::LeptosOptions;

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
    pub sled: Arc<std::sync::Mutex<sled::Db>>,
    pub leptos_options: LeptosOptions,
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
