//! Client-side local persistence (localStorage) for offline caching and
//! user preferences. Compiled only for the hydrate (wasm/client) target.

use crate::application_types::Data;

const DATA_KEY: &str = "typing.data.cache";
const SYNC_KEY: &str = "typing.data.sync_ts";
const VOICE_KEY: &str = "typing.preferred_voice";
const VOICES_KEY: &str = "typing.known_voices";

#[cfg(feature = "hydrate")]
fn set_storage(key: &str, value: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(key, value);
    }
}

#[cfg(feature = "hydrate")]
fn get_storage(key: &str) -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()
        .flatten()?
        .get_item(key)
        .ok()
        .flatten()
}

/// Persist the full article library locally (offline cache).
#[cfg(feature = "hydrate")]
pub fn cache_data(data: &Data) {
    if let Ok(json) = serde_json::to_string(data) {
        set_storage(DATA_KEY, &json);
    }
}

/// Load the locally cached library, if any.
#[cfg(feature = "hydrate")]
pub fn cached_data() -> Option<Data> {
    let raw = get_storage(DATA_KEY)?;
    serde_json::from_str(&raw).ok()
}

/// Timestamp (unix ms) of the last successful server sync.
pub fn cached_last_sync() -> Option<u64> {
    #[cfg(feature = "hydrate")]
    {
        return get_storage(SYNC_KEY)?.parse::<u64>().ok();
    }
    #[cfg(not(feature = "hydrate"))]
    {
        None
    }
}

/// Save data plus the sync timestamp atomically.
#[cfg(feature = "hydrate")]
pub fn cache_data_with_sync(data: &Data, sync_ts: u64) {
    cache_data(data);
    set_storage(SYNC_KEY, &sync_ts.to_string());
}

/// The union of voice names discovered so far (cached so the voice list is
/// available offline, e.g. "merz").
pub fn cached_voices() -> Vec<String> {
    #[cfg(feature = "hydrate")]
    {
        return get_storage(VOICES_KEY)
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
    }
    #[cfg(not(feature = "hydrate"))]
    {
        Vec::new()
    }
}

/// Merge newly discovered voices into the cached list and persist it.
pub fn merge_voices(new: &[String]) {
    let mut all = cached_voices();
    all.extend(new.iter().cloned());
    all.sort();
    all.dedup();
    #[cfg(feature = "hydrate")]
    if let Ok(json) = serde_json::to_string(&all) {
        set_storage(VOICES_KEY, &json);
    }
    #[cfg(not(feature = "hydrate"))]
    let _ = &all;
}

/// Current time in unix milliseconds.
pub fn now_ms() -> u64 {
    #[cfg(feature = "hydrate")]
    {
        // std::time::SystemTime is not implemented on wasm32-unknown-unknown,
        // so use the JS clock on the client.
        return js_sys::Date::now() as u64;
    }
    #[cfg(not(feature = "hydrate"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Preferred global voice (set from the Properties page).
pub fn save_preferred_voice(voice: &str) {
    #[cfg(feature = "hydrate")]
    set_storage(VOICE_KEY, voice);
    #[cfg(not(feature = "hydrate"))]
    let _ = voice;
}

pub fn preferred_voice() -> Option<String> {
    #[cfg(feature = "hydrate")]
    {
        return get_storage(VOICE_KEY).filter(|v| !v.is_empty());
    }
    #[cfg(not(feature = "hydrate"))]
    {
        None
    }
}

/// Format a unix-ms timestamp as a readable local date/time string.
pub fn format_sync_time(ts: u64) -> String {
    #[cfg(feature = "hydrate")]
    {
        return js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ts as f64))
            .to_locale_string("en-US", &js_sys::Object::new())
            .as_string()
            .unwrap_or_else(|| ts.to_string());
    }
    #[cfg(not(feature = "hydrate"))]
    {
        ts.to_string()
    }
}
