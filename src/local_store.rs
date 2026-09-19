//! Client-side local persistence (localStorage) for offline caching and
//! user preferences. Compiled only for the hydrate (wasm/client) target.

use crate::application_types::{Data, UserPreferences};

const DATA_KEY: &str = "typing.data.cache";
// Snapshot of the library as last known to be stored on the server. Kept apart
// from DATA_KEY (the live working copy) so pair edits that have not been saved
// to the server yet are still recognised as unsaved after reopening the app.
const SAVED_DATA_KEY: &str = "typing.data.saved";
const SYNC_KEY: &str = "typing.data.sync_ts";
const VOICES_KEY: &str = "typing.known_voices";
const SEARCH_KEY: &str = "typing.search";
// Article-list view state: the selected creation-date sort and the active
// "favorites" / "missing voice" filters.
const ARTICLE_SORT_KEY: &str = "typing.article_sort";
const ARTICLE_FILTERS_KEY: &str = "typing.article_filters";
// Library-version boundary of the most recent server sync. Articles whose
// `version` is greater were changed in that sync (or since), powering the
// "updated since previous sync" filter.
const LAST_SYNC_VERSION_KEY: &str = "typing.data.sync_version";
// The whole set of user preferences is persisted as one JSON blob under this
// single key (see load_preferences / save_preferences).
const PREFS_KEY: &str = "typing.preferences";
// Legacy per-key storage used before preferences were stored as a single blob.
// Kept only to migrate existing clients' local data on first load.
const LEGACY_VOICE_KEY: &str = "typing.preferred_voice";
const LEGACY_FAVORITES_KEY: &str = "typing.favorites";
const LEGACY_CURRENT_PARAGRAPH_ONLY_KEY: &str = "typing.current_paragraph_only";
const LEGACY_GROUP_MATCHING_BY_PARAGRAPH_KEY: &str = "typing.group_matching_by_paragraph";

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

/// Persist the library as last known to be stored on the server.
#[cfg(feature = "hydrate")]
pub fn cache_saved_data(data: &Data) {
    if let Ok(json) = serde_json::to_string(data) {
        set_storage(SAVED_DATA_KEY, &json);
    }
}

/// Load the locally cached "stored on server" snapshot, if any.
#[cfg(feature = "hydrate")]
pub fn cached_saved_data() -> Option<Data> {
    let raw = get_storage(SAVED_DATA_KEY)?;
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

/// The last article-search query, persisted so it survives navigation.
pub fn save_search(query: &str) {
    #[cfg(feature = "hydrate")]
    set_storage(SEARCH_KEY, query);
    #[cfg(not(feature = "hydrate"))]
    let _ = query;
}

pub fn saved_search() -> String {
    #[cfg(feature = "hydrate")]
    {
        return get_storage(SEARCH_KEY).unwrap_or_default();
    }
    #[cfg(not(feature = "hydrate"))]
    {
        String::new()
    }
}

/// Persist the selected article-list sort ("default" | "newest" | "oldest").
pub fn save_article_sort(sort: &str) {
    #[cfg(feature = "hydrate")]
    set_storage(ARTICLE_SORT_KEY, sort);
    #[cfg(not(feature = "hydrate"))]
    let _ = sort;
}

/// The last article-list sort selection, defaulting to "default".
pub fn saved_article_sort() -> String {
    #[cfg(feature = "hydrate")]
    {
        return get_storage(ARTICLE_SORT_KEY).unwrap_or_default();
    }
    #[cfg(not(feature = "hydrate"))]
    {
        String::new()
    }
}

/// Persist the active article-list filters as a compact
/// `favorites,missing,recent,pairs` tuple of `0`/`1` flags.
pub fn save_article_filters(
    only_favorites: bool,
    only_missing_voice: bool,
    only_recent: bool,
    only_with_pairs: bool,
) {
    #[cfg(feature = "hydrate")]
    set_storage(
        ARTICLE_FILTERS_KEY,
        &format!(
            "{},{},{},{}",
            only_favorites as u8,
            only_missing_voice as u8,
            only_recent as u8,
            only_with_pairs as u8
        ),
    );
    #[cfg(not(feature = "hydrate"))]
    let _ = (only_favorites, only_missing_voice, only_recent, only_with_pairs);
}

/// The last active article-list filters as
/// `(only_favorites, only_missing_voice, only_recent, only_with_pairs)`. Older
/// clients stored fewer values; missing flags default to `false`.
pub fn saved_article_filters() -> (bool, bool, bool, bool) {
    #[cfg(feature = "hydrate")]
    {
        let raw = get_storage(ARTICLE_FILTERS_KEY).unwrap_or_default();
        let mut parts = raw.split(',');
        let favorites = parts.next().is_some_and(|v| v == "1");
        let missing = parts.next().is_some_and(|v| v == "1");
        let recent = parts.next().is_some_and(|v| v == "1");
        let pairs = parts.next().is_some_and(|v| v == "1");
        return (favorites, missing, recent, pairs);
    }
    #[cfg(not(feature = "hydrate"))]
    {
        (false, false, false, false)
    }
}

/// Persist the library-version boundary of the most recent sync.
pub fn save_last_sync_version(version: u64) {
    #[cfg(feature = "hydrate")]
    set_storage(LAST_SYNC_VERSION_KEY, &version.to_string());
    #[cfg(not(feature = "hydrate"))]
    let _ = version;
}

/// The library-version boundary of the most recent sync (`0` when none yet).
pub fn cached_last_sync_version() -> u64 {
    #[cfg(feature = "hydrate")]
    {
        return get_storage(LAST_SYNC_VERSION_KEY)
            .and_then(|raw| raw.parse::<u64>().ok())
            .unwrap_or(0);
    }
    #[cfg(not(feature = "hydrate"))]
    {
        0
    }
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

/// Load the full set of user preferences from the local (offline) cache.
///
/// Preferences are stored as a single JSON blob. If no blob exists yet (e.g.
/// an older client), they are migrated from the legacy per-key storage and
/// written back as a blob so the migration only happens once.
pub fn load_preferences() -> UserPreferences {
    #[cfg(feature = "hydrate")]
    {
        if let Some(raw) = get_storage(PREFS_KEY) {
            if let Ok(mut p) = serde_json::from_str::<UserPreferences>(&raw) {
                // Identity comes from the session, never from the cache.
                p.user_id = String::new();
                return p;
            }
        }
        // Legacy migration from the old individual keys.
        let p = UserPreferences {
            user_id: String::new(),
            voice: get_storage(LEGACY_VOICE_KEY).unwrap_or_default(),
            current_paragraph_only: get_storage(LEGACY_CURRENT_PARAGRAPH_ONLY_KEY)
                .is_some_and(|v| v == "1"),
            group_matching_by_paragraph: get_storage(LEGACY_GROUP_MATCHING_BY_PARAGRAPH_KEY)
                .is_some_and(|v| v == "1"),
            favorites: get_storage(LEGACY_FAVORITES_KEY)
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default(),
            version: 0,
        };
        save_preferences(&p);
        p
    }
    #[cfg(not(feature = "hydrate"))]
    {
        UserPreferences::default()
    }
}

/// Persist the full set of user preferences to the local (offline) cache as a
/// single JSON blob.
pub fn save_preferences(preferences: &UserPreferences) {
    #[cfg(feature = "hydrate")]
    if let Ok(json) = serde_json::to_string(preferences) {
        set_storage(PREFS_KEY, &json);
    }
    #[cfg(not(feature = "hydrate"))]
    let _ = preferences;
}

/// Format a unix-ms timestamp as a readable local date string (no time).
/// Returns an empty string for the epoch (0), which is used as a placeholder
/// for articles that have not been synced to the server yet.
pub fn format_date(ts: u64) -> String {
    if ts == 0 {
        return String::new();
    }
    #[cfg(feature = "hydrate")]
    {
        return js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ts as f64))
            .to_locale_date_string("en-GB", &js_sys::Object::new())
            .as_string()
            .unwrap_or_else(|| ts.to_string());
    }
    #[cfg(not(feature = "hydrate"))]
    {
        ts.to_string()
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
