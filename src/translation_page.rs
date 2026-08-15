use std::ops::Sub;
use std::time::Duration;

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
use crate::application_types::Article;

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

#[component]
pub fn TranslationPage(data: ReadSignal<Data>, set_data: WriteSignal<Data>) -> impl IntoView {
    let views = move || {
        {
            data.get()
            .articles
            .clone()
            .into_iter()
            .enumerate()
            .map(move |(index, item)| {
                view! {
                    <div class="group flex flex-col gap-4 rounded-2xl border border-white/5 bg-white/[0.03] p-5 shadow-lg shadow-black/20 backdrop-blur transition-all duration-200 hover:border-indigo-500/30 hover:bg-white/[0.05] animate-fade-in">
                        <div class="flex w-full flex-col gap-4">
                            <a class="flex w-full flex-row items-center gap-3" href=format!("/article/{}", index)>
                                <span class="flex h-9 min-w-[40px] items-center justify-center rounded-lg border border-indigo-500/20 bg-indigo-500/10 px-2 font-mono text-sm font-semibold text-indigo-300">
                                    {item.paragraphs.len()}
                                </span>
                                <span class="flex text-lg font-semibold text-zinc-100 transition-colors group-hover:text-white">{item.title}</span>
                            </a>
                            <div class="grid grid-cols-2 gap-x-4 gap-y-1 lg:grid-cols-8">
                                {item
                                    .paragraphs
                                    .iter()
                                    .filter(|paragraph| paragraph.pairs.is_some())
                                    .map(|paragraph| {
                                        let words_original = paragraph
                                            .original
                                            .split(" ")
                                            .map(str::to_string)
                                            .collect::<Vec<String>>();
                                        if let Some(translation) = &paragraph.translation {
                                            let words_translation = translation
                                                .split(" ")
                                                .map(str::to_string)
                                                .collect::<Vec<String>>();
                                            Either::Left(
                                                paragraph
                                                    .pairs
                                                    .clone()
                                                    .unwrap()
                                                    .iter()
                                                    .map(|pair| {
                                                        let pair_original = pair
                                                            .original
                                                            .iter()
                                                            .map(|index| { words_original[*index].clone() })
                                                            .map(|word| {
                                                                view! { <div class="flex p-1 italic">{word}</div> }
                                                            })
                                                            .collect_view();
                                                        let pair_translated = pair
                                                            .translation
                                                            .iter()
                                                            .map(|index| { words_translation[*index].clone() })
                                                            .map(|word| {
                                                                view! { <div class="flex p-1 italic">{word}</div> }
                                                            })
                                                            .collect_view();
                                                        view! {
                                                            <div class="flex justify-end text-zinc-500/80">
                                                                {pair_original}
                                                            </div>
                                                            <div class="flex text-emerald-400/90">{pair_translated}</div>
                                                        }
                                                    })
                                                    .collect_view(),
                                            )
                                        } else {
                                            Either::Right(view! { "no translation available" })
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                        <div class="flex flex-wrap gap-2 border-t border-white/5 pt-4">
                            <div
                                class=BUTTON_CLASS
                                on:click=move |_event| {
                                    spawn_local(async move {
                                        let article_to_remove = data
                                            .get_untracked()
                                            .articles
                                            .get(index)
                                            .unwrap()
                                            .clone();
                                        let _ = store_pairs(article_to_remove).await;
                                    });
                                }
                            >

                                "Save Pairs"
                            </div>
                            <div
                                class=BUTTON_DANGER_CLASS
                                on:click=move |_event| {
                                    let article_to_remove = data
                                        .get_untracked()
                                        .articles
                                        .get(index)
                                        .unwrap()
                                        .clone();
                                    spawn_local(async move {
                                        delete_article(article_to_remove).await.unwrap();
                                    });
                                    set_data
                                        .update(|item| {
                                            item.articles.remove(index);
                                        });
                                }
                            >

                                "Delete"
                            </div>
                        </div>
                    </div>
                }
            }).collect_view()
        }
    };
    view! { <div class="w-full flex flex-col gap-5">{views}</div> }
}

#[component]
pub fn ArticlePage(
    data: ReadSignal<Data>,
    set_data: WriteSignal<Data>,
    playback: PlaybackState,
) -> impl IntoView {
    let params = use_params::<ArticleParams>();
    let article_id = params.with(|param| param.as_ref().unwrap().id).unwrap();
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
            return;
        }
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            if let Some(target) = document.get_element_by_id(hash) {
                target.scroll_into_view();
            }
        }
    });

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
                .map(|(index, _item)| {
                    view! {
                        <a class="flex h-6 w-6 items-center justify-center rounded-md bg-white/5 font-mono text-xs text-zinc-400 transition-colors hover:bg-indigo-500/20 hover:text-indigo-300" href=format!("#{}", index + 1)>
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
                    view! {
                        <Sentance
                            paragraph=item
                            article_index=article_id
                            index
                            total
                            pairs
                            set_pairs
                            div_ref
                            speech_cursor
                            audio_directory=if has_audio_directory { Some(audio_directory.clone()) } else { None }
                            on_audio_click=on_audio_click.clone()
                            on_replay_word=on_replay_word.clone()
                            on_current_paragraph=on_current_paragraph.clone()
                            audio_current_article
                            audio_current_paragraph
                            audio_is_playing
                            typing_speed_paragraph
                            set_typing_speed_paragraph
                            typing_speed_samples
                            set_typing_speed_samples
                            completed_typing_speeds
                            set_completed_typing_speeds
                            set_mistyped_characters
                        />
                    }
                })
                .collect_view();

            #[cfg(feature = "hydrate")]
            let voice_dropdown = if has_audio_directory {
                view! {
                    <select
                        class="rounded-lg border border-zinc-700/80 bg-zinc-950/80 px-2 py-1.5 text-sm text-zinc-300 transition-colors focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20"
                        prop:value=move || playback.selected_voice.get().unwrap_or_else(|| "default".to_string())
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

                                    // workaround for the selection issue, options are being
                                    // rendered after the select value is set, we need to force the
                                    // selction maker on the selected item
                                    if playback.selected_voice.get().unwrap_or_else(|| "default".to_string()) == voice {
                                        view! { <option value=value selected>{voice}</option> }.into_any()
                                    }else {
                                       view! { <option value=value>{voice}</option> }.into_any()
                                    }
                                    // view! { <option value=value>{voice}</option> }
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
                <label class="flex cursor-pointer items-center gap-2 text-sm text-zinc-400 hover:text-zinc-200 transition-colors">
                    <input
                        class="h-4 w-4 accent-indigo-500"
                        type="checkbox"
                        prop:checked=move || playback.current_paragraph_only.get()
                        on:change=move |event| {
                            playback
                                .set_current_paragraph_only
                                .set(event_target_checked(&event));
                        }
                    />
                    "Current paragraph only"
                </label>
            }
            .into_any();
            #[cfg(not(feature = "hydrate"))]
            let current_paragraph_toggle = view! { <div class="hidden"></div> }.into_any();

            Either::Left(view! {
                <div class="sticky top-0 z-30 w-full border-b border-white/5 bg-[#08080d]/70 backdrop-blur-xl animate-fade-in">
                    <div class="mx-auto flex w-full max-w-4xl items-center justify-between gap-4 px-4 py-3">
                        <a href="/" class="text-sm text-zinc-400 transition-colors hover:text-indigo-300">"← Home"</a>
                        <div class="truncate text-lg font-semibold text-zinc-100">{article.title.clone()}</div>
                        <div class="font-mono text-sm text-zinc-500">{total} " paragraphs"</div>
                    </div>
                </div>
                {paragraphs}
                <TypingSpeedPanel
                    typing_speed_paragraph
                    typing_speed_samples
                    completed_typing_speeds
                    mistyped_characters
                />
                <div class="fixed bottom-4 left-1/2 z-40 flex -translate-x-1/2 flex-wrap items-center gap-2 rounded-full border border-white/10 bg-zinc-900/80 px-4 py-2 text-sm text-zinc-300 shadow-2xl shadow-black/50 backdrop-blur-xl animate-slide-up">
                    {voice_dropdown}
                    {current_paragraph_toggle}
                    <span class="text-zinc-500">"jump:"</span>
                    <div class="underline decoration-indigo-400/50 underline-offset-4 cursor-pointer transition-colors hover:text-indigo-300" on:click=move |_| on_back(pairs)>
                        <a href="/">home</a>
                    </div> {link}
                </div>
                <div class="absolute animate-blink z-20" style=caret_position>
                    <span class="text-xl lg:text-2xl font-extrabold font-mono text-indigo-400">
                        _
                    </span>
                </div>
            })
        } else {
            Either::Right(())
        }
    };
    view! {
        <div class="mx-auto w-full max-w-5xl flex flex-col">
            {views}
        </div>
    }
}
