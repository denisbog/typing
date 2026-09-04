use std::ops::Sub;
use std::time::Duration;
use std::collections::BTreeSet;

use crate::application_types::Pair;
use crate::components::Association;
use crate::translation::delete_article;
use crate::translation::store_pairs;
use crate::{application_types::{Data}, components::{Sentance, TypingSpeedPanel}, BUTTON_DANGER_CLASS};
use crate::TypePairs;
use crate::BUTTON_CLASS;
use leptos::either::Either;
use leptos::html::Div;
use leptos::logging::log;
use leptos::task::spawn_local;
use leptos_animation::easing;
use leptos_animation::AnimatedSignal;
use leptos_animation::AnimationContext;
use leptos_animation::AnimationMode;
use leptos_animation::AnimationTarget;
use leptos_router::hooks::{use_location, use_params};
use leptos_router::params::Params;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys;
use serde_json::Value;
use crate::application_types::{Article, Paragraph};
use leptos_use::use_intersection_observer_with_options;
use leptos_use::UseIntersectionObserverOptions;

#[cfg(feature = "hydrate")]
use wasm_bindgen::{closure::Closure, JsValue};
#[cfg(feature = "hydrate")]
use wasm_bindgen_futures::JsFuture;
#[cfg(feature = "hydrate")]
use web_sys::HtmlAudioElement;

#[derive(Params, PartialEq)]
pub struct ArticleParams {
    id: Option<usize>,
}

#[derive(Clone)]
struct EffectPosition {
    x: f64,
    y: f64,
}

impl Sub for EffectPosition {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        EffectPosition {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeechCursor {
    pub paragraph: usize,
    pub word: usize,
}

#[cfg(feature = "hydrate")]
#[derive(Clone, Copy)]
pub struct PlaybackState {
    pub audio: StoredValue<Option<HtmlAudioElement>, LocalStorage>,
    pub cue_starts: StoredValue<Vec<f64>, LocalStorage>,
    pub is_playing: ReadSignal<bool>,
    pub set_is_playing: WriteSignal<bool>,
    pub article_title: ReadSignal<Option<String>>,
    pub set_article_title: WriteSignal<Option<String>>,
    pub article_index: ReadSignal<Option<usize>>,
    pub set_article_index: WriteSignal<Option<usize>>,
    pub paragraph: ReadSignal<Option<usize>>,
    pub set_paragraph: WriteSignal<Option<usize>>,
    pub selected_voice: ReadSignal<Option<String>>,
    pub set_selected_voice: WriteSignal<Option<String>>,
    pub speech_cursor: ReadSignal<Option<SpeechCursor>>,
    pub set_speech_cursor: WriteSignal<Option<SpeechCursor>>,
    pub current_paragraph_only: ReadSignal<bool>,
    pub set_current_paragraph_only: WriteSignal<bool>,
}

#[cfg(not(feature = "hydrate"))]
#[derive(Clone, Copy)]
pub struct PlaybackState;

#[cfg(feature = "hydrate")]
#[derive(Clone, Debug)]
struct SpeechCue {
    word_index: usize,
    start: f64,
    end: f64,
}

#[cfg(feature = "hydrate")]
fn trim_trailing_slash(value: &str) -> String {
    value.trim_end_matches('/').to_string()
}

#[cfg(feature = "hydrate")]
fn paragraph_base_url(base: &str, paragraph_index: usize, voice: Option<&str>) -> String {
    match voice {
        Some(voice) => format!(
            "{}/paragraph-{paragraph_index:03}/{}",
            trim_trailing_slash(base),
            voice
        ),
        None => format!("{}/paragraph-{paragraph_index:03}", trim_trailing_slash(base)),
    }
}

#[cfg(feature = "hydrate")]
async fn fetch_text(url: &str) -> Result<String, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let response = JsFuture::from(window.fetch_with_str(url)).await?;
    let response: web_sys::Response = response.dyn_into()?;
    let text = JsFuture::from(response.text()?).await?;
    text.as_string().ok_or_else(|| JsValue::from_str("missing response text"))
}

#[cfg(feature = "hydrate")]
async fn url_exists(url: &str) -> Result<bool, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let mut init = web_sys::RequestInit::new();
    init.method("HEAD");
    init.mode(web_sys::RequestMode::Cors);
    let request = web_sys::Request::new_with_str_and_init(url, &init)?;
    let response = JsFuture::from(window.fetch_with_request(&request)).await?;
    let response: web_sys::Response = response.dyn_into()?;
    Ok(response.ok())
}

#[cfg(feature = "hydrate")]
async fn fetch_available_voices(base_directory: &str) -> Result<Vec<String>, JsValue> {
    let metadata_url = format!("{}/metadata.json", trim_trailing_slash(base_directory));
    let metadata_text = fetch_text(&metadata_url).await?;
    let metadata: Value = serde_json::from_str(&metadata_text).unwrap_or(Value::Null);
    Ok(metadata
        .get("voices")
        .and_then(Value::as_array)
        .map(|voices| {
            voices
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default())
}

#[cfg(feature = "hydrate")]
fn push_speech_cue(cues: &mut Vec<SpeechCue>, word_index: usize, start: f64, end: f64) {
    cues.push(SpeechCue {
        word_index,
        start,
        end,
    });
}

#[cfg(feature = "hydrate")]
fn parse_cue_entry(entry: &Value, word_index: &mut usize, cues: &mut Vec<SpeechCue>) {
    if let Some(words) = entry.get("words").and_then(Value::as_array) {
        words.iter().for_each(|word| parse_cue_entry(word, word_index, cues));
        return;
    }

    let start = entry.get("start").and_then(Value::as_f64);
    let end = entry.get("end").and_then(Value::as_f64).or_else(|| {
        entry
            .get("duration")
            .and_then(Value::as_f64)
            .and_then(|duration| start.map(|start| start + duration))
    });

    let text = entry
        .get("word")
        .or_else(|| entry.get("text"))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");

    if text.is_empty() {
        return;
    }

    let words = text
        .split_whitespace()
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();

    if words.is_empty() {
        return;
    }

    let inferred_start = start.unwrap_or(*word_index as f64 * 0.35);
    let inferred_end = end.unwrap_or(inferred_start + (words.len() as f64 * 0.35).max(0.15));
    let duration = (inferred_end - inferred_start).max(0.05);
    let step = duration / words.len() as f64;

    words.iter().enumerate().for_each(|(offset, _word)| {
        push_speech_cue(
            cues,
            *word_index,
            inferred_start + (offset as f64 * step),
            inferred_start + ((offset as f64 + 1.0) * step),
        );
        *word_index += 1;
    });
}

#[cfg(feature = "hydrate")]
fn build_speech_cues(value: Value) -> Vec<SpeechCue> {
    let entries = match value {
        Value::Array(entries) => entries,
        Value::Object(mut map) => {
            for key in ["segments", "words", "items"] {
                if let Some(Value::Array(entries)) = map.remove(key) {
                    return entries
                        .iter()
                        .fold((Vec::new(), 0usize), |(mut cues, mut word_index), entry| {
                            parse_cue_entry(entry, &mut word_index, &mut cues);
                            (cues, word_index)
                        })
                        .0;
                }
            }
            vec![Value::Object(map)]
        }
        _ => vec![],
    };

    entries
        .iter()
        .fold((Vec::new(), 0usize), |(mut cues, mut word_index), entry| {
            parse_cue_entry(entry, &mut word_index, &mut cues);
            (cues, word_index)
        })
        .0
}

#[cfg(feature = "hydrate")]
fn active_speech_cursor(cues: &[SpeechCue], current_time: f64, paragraph: usize) -> Option<SpeechCursor> {
    cues.iter()
        .rev()
        .find(|cue| current_time >= cue.start && current_time <= cue.end)
        .or_else(|| cues.iter().rev().find(|cue| current_time >= cue.start))
        .map(|cue| SpeechCursor {
            paragraph,
            word: cue.word_index,
        })
}

#[cfg(feature = "hydrate")]
async fn start_paragraph_audio(
    article: Article,
    article_index: usize,
    base_directory: String,
    paragraph_index: usize,
    playback: PlaybackState,
    voice: Option<String>,
) -> Result<(), JsValue> {
    if paragraph_index >= article.paragraphs.len() {
        playback.set_speech_cursor.set(None);
        playback.set_is_playing.set(false);
        return Ok(());
    }
    let paragraph_url = paragraph_base_url(&base_directory, paragraph_index + 1, voice.as_deref());
    let audio_url = format!("{}/output.mp3", paragraph_url);
    if !url_exists(&audio_url).await.unwrap_or(false) {
        let next_paragraph = paragraph_index + 1;
        if !playback.current_paragraph_only.get_untracked()
            && next_paragraph < article.paragraphs.len()
        {
            let article = article.clone();
            let base = base_directory.clone();
            let playback = playback;
            let voice = voice.clone();
            spawn_local(async move {
                let _ = start_paragraph_audio(article, article_index, base, next_paragraph, playback, voice).await;
            });
        } else {
            playback.set_speech_cursor.set(None);
            playback.set_is_playing.set(false);
            playback.set_article_title.set(None);
            playback.set_article_index.set(None);
            playback.set_paragraph.set(None);
        }
        return Ok(());
    }
    let transcription_url = format!("{}/transcription.json", paragraph_url);
    let transcription_text = fetch_text(&transcription_url).await?;
    let transcription_json: Value = serde_json::from_str(&transcription_text).unwrap_or(Value::Null);
    let cues = build_speech_cues(transcription_json);
    playback
        .cue_starts
        .set_value(cues.iter().map(|cue| cue.start).collect());

    if let Some(previous) = playback.audio.get_value() {
        let _ = previous.pause();
        // previous.set_src("");
    }
    playback.audio.set_value(None);

    let audio = HtmlAudioElement::new()?;
    audio.set_preload("auto");
    audio.set_src(&audio_url);
    playback.audio.set_value(Some(audio.clone()));
    playback.set_is_playing.set(true);
    playback.set_article_title.set(Some(article.title.clone()));
    playback.set_article_index.set(Some(article_index));
    playback.set_paragraph.set(Some(paragraph_index));

    let paragraph_for_cursor = paragraph_index;
    let cues_for_timeupdate = cues.clone();
    let audio_for_timeupdate = audio.clone();
    let playback_for_timeupdate = playback;
    let ontimeupdate = Closure::wrap(Box::new(move || {
        let cursor = active_speech_cursor(
            &cues_for_timeupdate,
            audio_for_timeupdate.current_time(),
            paragraph_for_cursor,
        );
        playback_for_timeupdate.set_speech_cursor.set(cursor);
    }) as Box<dyn FnMut()>);
    audio.set_ontimeupdate(Some(ontimeupdate.as_ref().unchecked_ref()));
    ontimeupdate.forget();

    let article_for_end = article.clone();
    let base_for_end = base_directory.clone();
    let playback_for_end = playback;
    let onended = Closure::wrap(Box::new(move || {
        playback_for_end.set_speech_cursor.set(None);
        playback_for_end.set_is_playing.set(false);
        let next_paragraph = paragraph_index + 1;
        if !playback_for_end.current_paragraph_only.get_untracked()
            && next_paragraph < article_for_end.paragraphs.len()
        {
            let article = article_for_end.clone();
            let base = base_for_end.clone();
            let playback = playback_for_end;
            let voice = voice.clone();
            spawn_local(async move {
                let _ = start_paragraph_audio(article, article_index, base, next_paragraph, playback, voice).await;
            });
        } else if !playback_for_end.current_paragraph_only.get_untracked() {
            playback_for_end.set_article_title.set(None);
            playback_for_end.set_article_index.set(None);
            playback_for_end.set_paragraph.set(None);
            if let Some(current) = playback_for_end.audio.get_value() {
                current.pause().ok();
                // current.set_src("");
            }
            playback_for_end.audio.set_value(None);
        }
    }) as Box<dyn FnMut()>);
    audio.set_onended(Some(onended.as_ref().unchecked_ref()));
    onended.forget();
// audio.play().unwrap();
    if JsFuture::from(audio.play()?).await.is_err() {
        // playback.set_is_playing.set(false);
        // playback.set_paragraph.set(None);
        // playback.set_speech_cursor.set(None);
        return Err(JsValue::from_str("audio playback failed"));
    }
    Ok(())
}

/// Extract every saved pair (word-index lists of original + translation) from
/// an article, in order. Used to tell whether an article's live pairs still
/// differ from what is persisted on the server.
fn article_pairs(article: &crate::application_types::Article) -> Vec<(Vec<usize>, Vec<usize>)> {
    article
        .paragraphs
        .iter()
        .filter_map(|paragraph| paragraph.pairs.as_ref())
        .flatten()
        .map(|pair| (pair.original.clone(), pair.translation.clone()))
        .collect()
}

#[component]
pub fn TranslationPage(
    data: ReadSignal<Data>,
    set_data: WriteSignal<Data>,
    saved: ReadSignal<Data>,
    set_saved: WriteSignal<Data>,
) -> impl IntoView {
    let (search, set_search) = signal(crate::local_store::saved_search());
    let search_query = move || search.get().trim().to_lowercase();
    let (favorites, set_favorites) = signal(crate::local_store::favorites());

    // Keys used to identify a single article for favorites. created_at doubles
    // as the unique primary key per user in the backend.
    fn favorite_key(article: &crate::application_types::Article) -> String {
        article.created_at.to_string()
    }

    let toggle_favorite = move |key: String| {
        let mut favs = favorites.get_untracked();
        if favs.contains(&key) {
            favs.remove(&key);
        } else {
            favs.insert(key);
        }
        set_favorites.set(favs);
        crate::local_store::save_favorites(&favorites.get_untracked());
    };

    let views = move || {
        let query = search_query();
        // Keep the original article index (used for routes / save / delete)
        // but only render cards that match the search query.
        let mut indexed: Vec<(usize, crate::application_types::Article)> = data
            .get()
            .articles
            .clone()
            .into_iter()
            .enumerate()
            .collect();
        indexed.retain(|(_, item)| {
            if query.is_empty() {
                return true;
            }
            if item.title.to_lowercase().contains(&query) {
                return true;
            }
            item.paragraphs.iter().any(|paragraph| {
                paragraph.original.to_lowercase().contains(&query)
                    || paragraph
                        .translation
                        .as_ref()
                        .is_some_and(|t| t.to_lowercase().contains(&query))
            })
        });
        // Snapshot of each article's pairs as last persisted on the server,
        // keyed by created_at (the article's unique primary key).
        let saved_pairs_by_key: std::collections::HashMap<String, Vec<(Vec<usize>, Vec<usize>)>> =
            saved
                .get()
                .articles
                .iter()
                .map(|article| (article.created_at.to_string(), article_pairs(article)))
                .collect();
        // Number of pairs in the live article that are not yet on the server.
        let unsaved_by_index: std::collections::HashMap<usize, usize> = indexed
            .iter()
            .map(|(idx, item)| {
                let local_pairs = article_pairs(item);
                let unsaved = match saved_pairs_by_key.get(&item.created_at.to_string()) {
                    Some(saved_pairs) => local_pairs
                        .iter()
                        .filter(|lp| !saved_pairs.contains(lp))
                        .count(),
                    // Article unknown to the server: every pair counts as unsaved.
                    None => local_pairs.len(),
                };
                (*idx, unsaved)
            })
            .collect();
        // Articles with unsaved pairs first, then favorites, then the rest —
        // stable within each group so routes/actions don't reorder unexpectedly.
        let favs = favorites.get();
        indexed.sort_by_key(|(idx, item)| {
            let is_unsaved = unsaved_by_index.get(idx).copied().unwrap_or(0) > 0;
            let is_fav = favs.contains(&favorite_key(item));
            (
                if is_unsaved { 0u8 } else { 1u8 },
                if is_fav { 0u8 } else { 1u8 },
                *idx,
            )
        });
        indexed
            .into_iter()
            .map(move |(index, item)| {
                let (saving, set_saving) = signal(false);
                let (deleting, set_deleting) = signal(false);
                let paragraph_count = item.paragraphs.len();
                let translated_count = item
                    .paragraphs
                    .iter()
                    .filter(|paragraph| paragraph.translation.is_some())
                    .count();
                let pair_count = item
                    .paragraphs
                    .iter()
                    .filter_map(|paragraph| paragraph.pairs.as_ref())
                    .map(Vec::len)
                    .sum::<usize>();
                let unsaved_count = unsaved_by_index.get(&index).copied().unwrap_or(0);
                let has_unsaved = unsaved_count > 0;
                let progress = if paragraph_count == 0 {
                    0
                } else {
                    translated_count * 100 / paragraph_count
                };
                let fav_key = favorite_key(&item);
                let class_fav_key = fav_key.clone();
                let title_fav_key = fav_key.clone();
                let click_fav_key = fav_key.clone();
                let title = item.title.trim();
                let (headline, summary) = title
                    .split_once("||")
                    .map(|(headline, summary)| {
                        (headline.trim().to_string(), Some(summary.trim().to_string()))
                    })
                    .unwrap_or_else(|| (title.to_string(), None));

                view! {
                    <article class="article-list-card article-card group">
                        <a
                            class="article-link"
                            href=format!("/article/{}", index)
                            on:click=move |_| window().scroll_to_with_x_and_y(0.0, 0.0)
                        >
                            <div class="article-link-head">
                                <div class="article-link-badges">
                                    <span class="article-index">
                                        {format!("Article {:02}", index + 1)}
                                    </span>
                                    <span class="dot-sep"></span>
                                    <span class="article-meta">
                                        {format!("{} paragraphs", paragraph_count)}
                                    </span>
                                    {item
                                        .audio_directory
                                        .as_ref()
                                        .map(|_| {
                                            view! {
                                                <span
                                                    class="article-voice-badge"
                                                    title="This article has audio"
                                                >
                                                    "🔊"
                                                </span>
                                            }
                                        })}

                                    {has_unsaved
                                        .then(|| {
                                            view! {
                                                <span
                                                    class="article-unsaved-badge"
                                                    title="Pairs edited in this article aren't saved to the server yet"
                                                >
                                                    {format!("{} unsaved", unsaved_count)}
                                                </span>
                                            }
                                        })}

                                </div>
                                <span class="article-arrow">"↗"</span>
                            </div>

                            <div class="article-card-body">
                                <h3 class="article-title">{headline}</h3>
                                {summary
                                    .map(|summary| {
                                        view! { <p class="article-summary">{summary}</p> }
                                    })}

                            </div>

                            <div class="article-card-foot">
                                <div class="article-coverage-label">
                                    <span>"Translation coverage"</span>
                                    <span class="article-coverage-value">
                                        {format!("{}%", progress)}
                                    </span>
                                </div>
                                <div class="article-coverage-track">
                                    <div
                                        class="article-coverage-bar"
                                        style=format!("width: {}%", progress)
                                    ></div>
                                </div>
                            </div>
                        </a>

                        <div class="article-actions">
                            <div class="article-pairs">
                                <span class="article-pairs-count">{pair_count}</span>
                                <span>"saved pairs"</span>
                            </div>
                            <div class="article-action-buttons">
                                <a
                                    class=BUTTON_CLASS
                                    href=format!("/article/{}/match", index)
                                    title="Match this article’s saved pairs"
                                >
                                    <span class="btn-icon">"⧉"</span>
                                    <span class="btn-label">"Match"</span>
                                </a>
                                <a
                                    class=BUTTON_CLASS
                                    href=format!("/article/{}/rebuild", index)
                                    title="Rebuild this article’s original text"
                                >
                                    <span class="btn-icon">"↶"</span>
                                    <span class="btn-label">"Rebuild"</span>
                                </a>
                                <button
                                    class=move || {
                                        if favorites.get().contains(&class_fav_key) {
                                            format!("{} btn-fav is-fav", BUTTON_CLASS)
                                        } else {
                                            format!("{} btn-fav", BUTTON_CLASS)
                                        }
                                    }

                                    title=move || {
                                        if favorites.get().contains(&title_fav_key) {
                                            "Remove from favorites"
                                        } else {
                                            "Mark as favorite"
                                        }
                                    }

                                    on:click=move |_event| {
                                        toggle_favorite(click_fav_key.clone());
                                    }
                                >

                                    {move || {
                                        if favorites.get().contains(&fav_key) {
                                            view! { <span>"★"</span> }.into_any()
                                        } else {
                                            view! { <span>"☆"</span> }.into_any()
                                        }
                                    }}

                                </button>
                                <button
                                    class=move || {
                                        if saving.get() {
                                            format!("{} opacity-50 cursor-wait", BUTTON_CLASS)
                                        } else {
                                            BUTTON_CLASS.to_string()
                                        }
                                    }

                                    title="Save this article’s word pairs"
                                    disabled=move || saving.get()
                                    on:click=move |_event| {
                                        set_saving.set(true);
                                        spawn_local(async move {
                                            let article_to_store = data
                                                .get_untracked()
                                                .articles
                                                .get(index)
                                                .unwrap()
                                                .clone();
                                            let _ = store_pairs(article_to_store.clone()).await;
                                            set_saved
                                                .update(|snapshot| {
                                                    snapshot
                                                        .articles
                                                        .retain(|article| {
                                                            article.created_at != article_to_store.created_at
                                                        });
                                                    snapshot.articles.push(article_to_store.clone());
                                                });
                                            set_saving.set(false);
                                        });
                                    }
                                >

                                    {move || {
                                        if saving.get() {
                                            view! {
                                                <span class="btn-spinner"></span>
                                                "Saving"
                                            }
                                                .into_any()
                                        } else {
                                            view! { "Save" }.into_any()
                                        }
                                    }}

                                </button>
                                <button
                                    class=move || {
                                        if deleting.get() {
                                            format!("{} opacity-50 cursor-wait", BUTTON_DANGER_CLASS)
                                        } else {
                                            BUTTON_DANGER_CLASS.to_string()
                                        }
                                    }

                                    title="Delete article"
                                    disabled=move || deleting.get()
                                    on:click=move |_event| {
                                        let article_to_remove = data
                                            .get_untracked()
                                            .articles
                                            .get(index)
                                            .unwrap()
                                            .clone();
                                        set_deleting.set(true);
                                        spawn_local(async move {
                                            let _ = delete_article(article_to_remove).await;
                                            set_deleting.set(false);
                                            set_data
                                                .update(|item| {
                                                    item.articles.remove(index);
                                                });
                                        });
                                    }
                                >

                                    {move || {
                                        if deleting.get() {
                                            view! {
                                                <span class="btn-spinner"></span>
                                                "Deleting"
                                            }
                                                .into_any()
                                        } else {
                                            view! { "Delete" }.into_any()
                                        }
                                    }}

                                </button>
                            </div>
                        </div>
                    </article>
                }
            })
            .collect_view()
    };

    view! {
        <Show
            when=move || !data.get().articles.is_empty()
            fallback=move || {
                view! {
                    <div class="library-empty glass-panel">
                        <span class="library-empty-icon">"＋"</span>
                        <h3 class="library-empty-title">"Your library is ready"</h3>
                        <p class="library-empty-sub">
                            "Add your first article to create a focused typing and translation practice session."
                        </p>
                    </div>
                }
            }
        >

            <div class="article-search">
                <span class="article-search-icon">"🔍"</span>
                <input
                    type="search"
                    class="article-search-input"
                    placeholder="Search articles by title, text or translation…"
                    prop:value=search
                    on:input=move |event| {
                        let value = event_target_value(&event);
                        set_search.set(value.clone());
                        crate::local_store::save_search(&value);
                    }
                />

                {move || {
                    if search_query().is_empty() {
                        view! {}.into_any()
                    } else {
                        view! {
                            <button
                                class="article-search-clear"
                                aria-label="Clear search"
                                on:click=move |_| {
                                    set_search.set(String::new());
                                    crate::local_store::save_search("");
                                }
                            >

                                "×"
                            </button>
                        }
                            .into_any()
                    }
                }}

            </div>

            <Show
                when=move || {
                    let query = search_query();
                    query.is_empty()
                        || data
                            .get()
                            .articles
                            .iter()
                            .any(|item| {
                                item.title.to_lowercase().contains(&query)
                                    || item
                                        .paragraphs
                                        .iter()
                                        .any(|paragraph| {
                                            paragraph.original.to_lowercase().contains(&query)
                                                || paragraph
                                                    .translation
                                                    .as_ref()
                                                    .is_some_and(|t| { t.to_lowercase().contains(&query) })
                                        })
                            })
                }

                fallback=move || {
                    view! {
                        <div class="library-empty glass-panel">
                            <h3 class="library-empty-title">"No articles match your search"</h3>
                            <p class="library-empty-sub">
                                "Try a different keyword, or clear the search to see everything."
                            </p>
                        </div>
                    }
                }
            >

                <div class="library-grid">{views}</div>
            </Show>
        </Show>
    }
}

/// How many paragraphs at the top of an article are interactive immediately.
/// Everything below that is a lightweight placeholder until it scrolls near
/// the viewport.
const INITIALLY_MOUNTED: usize = 3;

/// Distance from the viewport (px) at which a paragraph starts loading, so it
/// is already interactive by the time the reader scrolls to it.
const PARAGRAPH_LOAD_MARGIN: &str = "800px";

/// Renders one interactive [`Sentance`] lazily.
///
/// Articles can hold ~100 paragraphs. Each `Sentance` carries its own typing
/// state, per-word character spans, timers and effects, so instantiating all
/// of them up front makes the first paint (and the hydration pass) do a lot of
/// wasted work. This wrapper keeps a cheap, readable placeholder on screen and
/// only mounts the full `Sentance` once its sentinel scrolls near the viewport.
///
/// Mounting is one-way: a paragraph that has been mounted stays mounted, so the
/// in-progress typing state is never discarded when the reader scrolls back up,
/// and `children` (which builds the `Sentance`) is only ever invoked once per
/// paragraph.
#[component]
fn LazyParagraph(
    /// Paragraph data, used for the placeholder text while unmounted.
    paragraph: Paragraph,
    /// 0-based paragraph index.
    index: usize,
    /// Total number of paragraphs in the article.
    total: usize,
    is_mobile: Signal<bool>,
    pairing_mode: ReadSignal<bool>,
    children: ChildrenFn,
) -> impl IntoView {
    let (mounted, set_mounted) = signal(index < INITIALLY_MOUNTED);

    // Sentinel wrapper is always in the DOM for the paragraph's whole lifetime,
    // so the IntersectionObserver never needs to be recreated. `mounted` is
    // sticky (only ever set to true), so `children()` below runs at most once.
    let sentinel = NodeRef::<Div>::new();

    let _observer = use_intersection_observer_with_options(
        sentinel,
        move |entries, _| {
            if entries.iter().any(|entry| {
                let rect = entry.bounding_client_rect();
                (rect.width() > 0.0 || rect.height() > 0.0) && entry.is_intersecting()
            }) {
                set_mounted.set(true);
            }
        },
        UseIntersectionObserverOptions::default().root_margin(PARAGRAPH_LOAD_MARGIN),
    );

    let paragraph_for_placeholder = paragraph.clone();
    view! {
        <div node_ref=sentinel>
            {move || {
                if mounted.get() {
                    Either::Left(children())
                } else {
                    let placeholder = view! {
                        <section class="sentence-section parent" id=index + 1>
                            <div class="sentence-card article-card">
                                <div class="sentence-header">
                                    <div class="sentence-header-left">
                                        <span class="badge-index">
                                            {format!("Paragraph {:02}", index + 1)}
                                        </span>
                                        <span class="dot-sep"></span>
                                        <span class="badge-count">
                                            {format!("{:02} / {:02}", index + 1, total)}
                                        </span>
                                    </div>
                                    <span class=move || {
                                        if pairing_mode.get() {
                                            "mode-badge mode-badge--active"
                                        } else {
                                            "mode-badge"
                                        }
                                    }>
                                        {move || {
                                            if is_mobile.get() {
                                                "Reading mode"
                                            } else if pairing_mode.get() {
                                                "Pairing mode"
                                            } else {
                                                "Typing mode"
                                            }
                                        }}

                                    </span>
                                </div>
                                <div class="sentence-grid">
                                    <div class="sentence-col-original">
                                        <div class="sentence-label-row">
                                            <span class="sentence-label">"Original · German"</span>
                                        </div>
                                        <div class="sentence-placeholder-text">
                                            {paragraph_for_placeholder.original.clone()}
                                        </div>
                                    </div>
                                    {paragraph_for_placeholder
                                        .translation
                                        .as_ref()
                                        .map(|translation| {
                                            view! {
                                                <div class="sentence-col-translation">
                                                    <div class="sentence-label-row">
                                                        <span class="sentence-label">"Translation · English"</span>
                                                    </div>
                                                    <div class="sentence-placeholder-text">
                                                        {translation.clone()}
                                                    </div>
                                                </div>
                                            }
                                                .into_any()
                                        })}

                                </div>
                            </div>
                        </section>
                    };
                    Either::Right(placeholder.into_any())
                }
            }}

        </div>
    }
}

#[component]
pub fn ArticlePage(
    data: ReadSignal<Data>,
    set_data: WriteSignal<Data>,
    saved: ReadSignal<Data>,
    playback: PlaybackState,
) -> impl IntoView {
    let params = use_params::<ArticleParams>();
    // The route param never changes while this page is mounted (navigating to
    // another article re-creates the component), so this is a one-time read.
    // `with_untracked` makes that explicit and avoids the "access outside a
    // reactive tracking context" warning.
    let article_id = params
        .with_untracked(|param| param.as_ref().ok().and_then(|p| p.id))
        .unwrap();
    log!("render article");
    #[cfg(feature = "hydrate")]
    let location = use_location();
    let on_back = move |pairs: ReadSignal<TypePairs>| {
        log!("pairs updated");
        set_data.update(|state| {
            if let Some(article) = state.articles.get_mut(article_id) {
                pairs.get().into_iter().for_each(|(key, value)| {
                    log!("key: {}", key);
                    let pairs = value
                        .into_iter()
                        .map(|item| {
                            let original: Vec<usize> = item.original.into_iter().collect();
                            let translation: Vec<usize> = item.translation.into_iter().collect();
                            Pair {
                                original,
                                translation,
                            }
                        })
                        .collect();
                    article.paragraphs[key].pairs = Some(pairs);
                });
            }
        });
    };

    AnimationContext::provide();

    let div_ref = NodeRef::<Div>::new();
    let (coordinates, set_coordinates) = signal(EffectPosition { x: 0.0, y: 0.0 });
    #[cfg(feature = "hydrate")]
    let speech_cursor = playback.speech_cursor;
    #[cfg(feature = "hydrate")]
    let audio_current_article = playback.article_index;
    #[cfg(feature = "hydrate")]
    let audio_current_paragraph = playback.paragraph;
    #[cfg(feature = "hydrate")]
    let audio_is_playing = playback.is_playing;
    #[cfg(not(feature = "hydrate"))]
    let (speech_cursor, _set_speech_cursor) = signal(Option::<SpeechCursor>::None);
    #[cfg(not(feature = "hydrate"))]
    let (audio_current_article, _set_audio_current_article) = signal(Option::<usize>::None);
    #[cfg(not(feature = "hydrate"))]
    let (audio_current_paragraph, _set_audio_current_paragraph) = signal(Option::<usize>::None);
    #[cfg(not(feature = "hydrate"))]
    let (audio_is_playing, _set_audio_is_playing) = signal(false);

    Effect::new(move |_| {
        if let Some(div) = div_ref.get() {
            let element = div;
            let rect = element.get_bounding_client_rect();
            let x = rect.x();
            let y = rect.y();
            let doc_x = x + window().scroll_x().unwrap_or(0.0);
            let doc_y = y + window().scroll_y().unwrap_or(0.0);
            set_coordinates.set(EffectPosition { x: doc_x, y: doc_y });
        }
    });

    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        let hash = location.hash.get();
        let hash = hash.trim_start_matches('#');
        if hash.is_empty() {
            if let Some(window) = web_sys::window() {
                window.scroll_to_with_x_and_y(0.0, 0.0);
            }
            return;
        }
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            if let Some(target) = document.get_element_by_id(hash) {
                target.scroll_into_view();
            }
        }
    });

    let is_mobile = leptos_use::use_media_query("(max-width: 1024px)");

    let views = move || {
        let animated_value = AnimatedSignal::new(
            move || AnimationTarget::<EffectPosition> {
                target: coordinates.get().into(),
                duration: Duration::from_secs_f64(0.25),
                easing: easing::CUBIC_OUT,
                mode: AnimationMode::Start,
            },
            |from, to, progress| EffectPosition {
                x: (to.x - from.x) * progress + from.x,
                y: (to.y - from.y) * progress + from.y,
            },
        );
        let caret_position = move || {
            let temp = animated_value.read();
            format!("left: {}px; top:{}px", temp.x, temp.y)
        };
        if let Some(article) = data.get().articles.get(article_id) {
            let mut article_pairs = TypePairs::new();
            article
                .clone()
                .paragraphs
                .into_iter()
                .enumerate()
                .for_each(|(index, paragraph)| {
                    if paragraph.pairs.is_some() {
                        let associations = paragraph
                            .pairs
                            .unwrap()
                            .into_iter()
                            .map(|pair| Association {
                                start_position: pair.original[0],
                                original: pair.original.into_iter().collect(),
                                translation: pair.translation.into_iter().collect(),
                            })
                            .collect();

                        log!("inserted associations: {:?}", associations);
                        article_pairs.insert(index, associations);
                    }
                });

            let (pairs, set_pairs) = signal(article_pairs);

            // Server-known pairs for this article, per paragraph, as
            // (original, translation) word-index sets. Used to tell which pairs
            // created here are brand-new and not yet saved to the server.
            let saved_article_pairs: Vec<Vec<(BTreeSet<usize>, BTreeSet<usize>)>> = saved
                .get()
                .articles
                .iter()
                .find(|item| item.created_at == article.created_at)
                .map(|item| {
                    item.paragraphs
                        .iter()
                        .map(|paragraph| {
                            paragraph
                                .pairs
                                .as_ref()
                                .map(|pairs| {
                                    pairs
                                        .iter()
                                        .map(|pair| {
                                            (
                                                pair.original.iter().copied().collect::<BTreeSet<_>>(),
                                                pair
                                                    .translation
                                                    .iter()
                                                    .copied()
                                                    .collect::<BTreeSet<_>>(),
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default()
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Pairing/typing mode is a single article-wide setting: toggling it
            // on any paragraph switches every paragraph together.
            let (pairing_mode, set_pairing_mode) = signal(false);

            let total = article.paragraphs.len();
            let (typing_speed_paragraph, set_typing_speed_paragraph) =
                signal(Option::<usize>::None);
            let (typing_speed_samples, set_typing_speed_samples) = signal(Vec::<f32>::new());
            let (completed_typing_speeds, set_completed_typing_speeds) =
                signal(vec![None; total]);
            let (mistyped_characters, set_mistyped_characters) = signal(0usize);
            let link = article
                .paragraphs
                .clone()
                .into_iter()
                .enumerate()
                .filter(|(index, _item)| *index == 0 || (*index + 1) % 5 == 0)
                .map(|(index, _item)| {
                    view! {
                        <a class="jump-link" href=format!("#{}", index + 1)>
                            {index + 1}
                        </a>
                    }
                })
                .collect_view();
            let has_audio_directory = article.audio_directory.is_some();
            let audio_directory = article.audio_directory.clone().unwrap_or_default();
            let audio_directory_for_audio = audio_directory.clone();
            let audio_directory_for_current = audio_directory.clone();
            let audio_directory_for_voice_change = audio_directory.clone();
            let article_for_current = article.clone();
            let article_for_voice = article.clone();
            #[cfg(feature = "hydrate")]
            let (available_voices, set_available_voices) = signal(Vec::<String>::new());
            #[cfg(feature = "hydrate")]
            if has_audio_directory {
                let audio_directory_for_voices = audio_directory.clone();
                spawn_local(async move {
                    if let Ok(voices) = fetch_available_voices(&audio_directory_for_voices).await {
                        // Fall back to "default" when the globally preferred voice
                        // is not available for this particular article.
                        if let Some(pref) = playback.selected_voice.get_untracked().as_deref() {
                            if !voices.iter().any(|v| v == pref) {
                                playback.set_selected_voice.set(None);
                            }
                        }
                        crate::local_store::merge_voices(&voices);
                        set_available_voices.set(voices);
                    }
                });
            }
            #[cfg(feature = "hydrate")]
            let on_audio_click = if has_audio_directory {
                let article_for_audio = article.clone();
                Some(UnsyncCallback::new(move |paragraph_index: usize| {
                    let active_article = playback.article_index.get();
                    let active_paragraph = playback.paragraph.get();
                    if active_article == Some(article_id) && active_paragraph == Some(paragraph_index) {
                        if let Some(audio) = playback.audio.get_value() {
                            if playback.is_playing.get() {
                                audio.pause().ok();
                                playback.set_is_playing.set(false);
                            } else {
                                audio.play().ok();
                                playback.set_is_playing.set(true);
                            }
                        }
                        // playback.set_paragraph.set(None);
                        // set_speech_cursor.set(None);
                    } else {
                        let article = article_for_audio.clone();
                        let directory = audio_directory_for_audio.clone();
                        let playback = playback;
                        let voice = playback.selected_voice.get();
                        spawn_local(async move {
                            let _ = start_paragraph_audio(
                                article,
                                article_id,
                                directory,
                                paragraph_index,
                                playback,
                                voice,
                            )
                            .await;
                        });
                    }
                }))
            } else {
                None
            };
            #[cfg(not(feature = "hydrate"))]
            let on_audio_click: Option<UnsyncCallback<usize>> = None;
            #[cfg(feature = "hydrate")]
            let on_replay_word = if has_audio_directory {
                Some(UnsyncCallback::new(move |(paragraph_index, word_index): (usize, usize)| {
                    if playback.article_index.get_untracked() != Some(article_id)
                        || playback.paragraph.get_untracked() != Some(paragraph_index)
                    {
                        return;
                    }
                    let Some(start) = playback.cue_starts.get_value().get(word_index).copied() else {
                        return;
                    };
                    if let Some(audio) = playback.audio.get_value() {
                        audio.set_current_time(start);
                        if audio.play().is_ok() {
                            playback.set_is_playing.set(true);
                            playback.set_speech_cursor.set(Some(SpeechCursor {
                                paragraph: paragraph_index,
                                word: word_index,
                            }));
                        }
                    }
                }))
            } else {
                None
            };
            #[cfg(not(feature = "hydrate"))]
            let on_replay_word: Option<UnsyncCallback<(usize, usize)>> = None;
            #[cfg(feature = "hydrate")]
            let on_current_paragraph = if has_audio_directory {
                Some(UnsyncCallback::new(move |paragraph_index: usize| {
                    if !playback.current_paragraph_only.get_untracked() {
                        return;
                    }
                    if playback.article_index.get_untracked() == Some(article_id)
                        && playback.paragraph.get_untracked() == Some(paragraph_index)
                    {
                        if let Some(audio) = playback.audio.get_value() {
                            if audio.play().is_ok() {
                                playback.set_is_playing.set(true);
                            }
                        }
                        return;
                    }

                    let article = article_for_current.clone();
                    let directory = audio_directory_for_current.clone();
                    let voice = playback.selected_voice.get_untracked();
                    spawn_local(async move {
                        let _ = start_paragraph_audio(
                            article,
                            article_id,
                            directory,
                            paragraph_index,
                            playback,
                            voice,
                        )
                        .await;
                    });
                }))
            } else {
                None
            };
            #[cfg(not(feature = "hydrate"))]
            let on_current_paragraph: Option<UnsyncCallback<usize>> = None;
            let paragraphs = article
                .paragraphs
                .clone()
                .into_iter()
                .enumerate()
                .map(|(index, item)| {
                    // Owned per-iteration copies: the `LazyParagraph` children
                    // closure is `move` and reusable, so it can't borrow from
                    // this `FnMut` closure's by-reference captures.
                    let audio_directory = audio_directory.clone();
                    let saved_article_pairs = saved_article_pairs.clone();
                    view! {
                        <LazyParagraph paragraph=item.clone() index total is_mobile pairing_mode>
                            <Sentance
                                paragraph=item.clone()
                                article_index=article_id
                                index
                                total
                                pairs
                                set_pairs
                                div_ref
                                speech_cursor
                                audio_directory=if has_audio_directory {
                                    Some(audio_directory.clone())
                                } else {
                                    None
                                }

                                on_audio_click=on_audio_click.clone()
                                on_replay_word=on_replay_word.clone()
                                on_current_paragraph=on_current_paragraph.clone()
                                audio_current_article
                                audio_current_paragraph
                                audio_is_playing
                                is_mobile
                                typing_speed_paragraph
                                set_typing_speed_paragraph
                                typing_speed_samples
                                set_typing_speed_samples
                                completed_typing_speeds
                                set_completed_typing_speeds
                                set_mistyped_characters
                                pairing_mode
                                set_pairing_mode
                                saved_pairs=saved_article_pairs
                                    .get(index)
                                    .cloned()
                                    .unwrap_or_default()
                            />
                        </LazyParagraph>
                    }
                })
                .collect_view();

            #[cfg(feature = "hydrate")]
            let voice_dropdown = if has_audio_directory {
                view! {
                    <select
                        class="voice-select"
                        prop:value=move || {
                            playback.selected_voice.get().unwrap_or_else(|| "default".to_string())
                        }

                        on:change=move |event| {
                            let value = event_target_value(&event);
                            let voice = if value == "default" { None } else { Some(value) };
                            playback.set_selected_voice.set(voice.clone());
                            if has_audio_directory
                                && playback.article_index.get() == Some(article_id)
                                && playback.paragraph.get().is_some()
                            {
                                let paragraph_index = playback.paragraph.get().unwrap();
                                if let Some(audio) = playback.audio.get_value() {
                                    audio.pause().ok();
                                }
                                playback.set_is_playing.set(false);
                                playback.set_speech_cursor.set(None);
                                let article = article_for_voice.clone();
                                let directory = audio_directory_for_voice_change.clone();
                                let playback = playback;
                                spawn_local(async move {
                                    let _ = start_paragraph_audio(
                                            article,
                                            article_id,
                                            directory,
                                            paragraph_index,
                                            playback,
                                            voice,
                                        )
                                        .await;
                                });
                            }
                        }
                    >

                        <option value="default">"default"</option>
                        {move || {
                            available_voices
                                .get()
                                .into_iter()
                                .map(|voice| {
                                    let value = voice.clone();
                                    if playback
                                        .selected_voice
                                        .get()
                                        .unwrap_or_else(|| "default".to_string()) == voice
                                    {
                                        view! {
                                            // workaround for the selection issue, options are being
                                            // rendered after the select value is set, we need to force the
                                            // selction maker on the selected item
                                            <option value=value selected>
                                                {voice}
                                            </option>
                                        }
                                            .into_any()
                                    } else {
                                        view! {
                                            // workaround for the selection issue, options are being
                                            // rendered after the select value is set, we need to force the
                                            // selction maker on the selected item

                                            // workaround for the selection issue, options are being
                                            // rendered after the select value is set, we need to force the
                                            // selction maker on the selected item

                                            // workaround for the selection issue, options are being
                                            // rendered after the select value is set, we need to force the
                                            // selction maker on the selected item
                                            <option value=value>{voice}</option>
                                        }
                                            .into_any()
                                    }
                                })
                                .collect_view()
                        }}

                    </select>
                }
                .into_any()
            } else {
                view! { <div class="hidden"></div> }.into_any()
            };
            #[cfg(not(feature = "hydrate"))]
            let voice_dropdown = view! { <div class="hidden"></div> }.into_any();
            #[cfg(feature = "hydrate")]
            let current_paragraph_toggle = view! {
                <label class="current-paragraph-toggle">
                    <input
                        class="current-paragraph-checkbox"
                        type="checkbox"
                        prop:checked=move || playback.current_paragraph_only.get()
                        on:change=move |event| {
                            playback.set_current_paragraph_only.set(event_target_checked(&event));
                        }
                    />

                    "Current paragraph only"
                </label>
            }
            .into_any();
            #[cfg(not(feature = "hydrate"))]
            let current_paragraph_toggle = view! { <div class="hidden"></div> }.into_any();

            Either::Left(view! {
                <header class="page-header">
                    <div class="page-header-inner">
                        <a href="/" class="back-link group" on:click=move |_| on_back(pairs)>
                            <span class="back-icon">"←"</span>
                            <span class="back-link-label">"Library"</span>
                        </a>
                        <div class="page-title-wrap">
                            <div class="page-title-eyebrow">"Now practicing"</div>
                            <div class="page-title">{article.title.clone()}</div>
                        </div>
                        <div class="page-count">
                            <div class="page-count-num">{total}</div>
                            <div class="page-count-label">"Paragraphs"</div>
                        </div>
                    </div>
                </header>
                {paragraphs}
                <TypingSpeedPanel
                    typing_speed_paragraph
                    typing_speed_samples
                    completed_typing_speeds
                    mistyped_characters
                />
                <div class="jump-bar">
                    <a
                        class="jump-back group"
                        href="/"
                        title="Return to the articles list"
                        on:click=move |_| on_back(pairs)
                    >
                        <span class="jump-back-icon">"←"</span>
                        <span class="jump-back-label">"Articles"</span>
                    </a>
                    <div class="jump-bar-options">{voice_dropdown} {current_paragraph_toggle}</div>
                    <span class="jump-label">"Jump to"</span>
                    <nav class="jump-nav">{link}</nav>
                </div>
                <div
                    class=move || {
                        if typing_speed_paragraph.get().is_some() { "caret" } else { "hidden" }
                    }

                    style=caret_position
                >
                    <span class="caret-char">_</span>
                </div>
            })
        } else {
            Either::Right(())
        }
    };
    view! { <div class="article-page">{views}</div> }
}
