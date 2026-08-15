use std::ops::Sub;
use std::time::Duration;
use std::{collections::BTreeSet, hash::Hash};

use leptos::either::Either;
use leptos::html::Div;
use leptos::logging::log;
use leptos::logging::warn;
use leptos::prelude::*;
#[cfg(feature = "hydrate")]
use leptos::task::spawn_local;
use leptos_animation::{easing, AnimatedSignal, AnimationTarget};
use leptos_animation::{AnimationContext, AnimationMode};
use serde::Deserialize;
use serde::Serialize;

use crate::application_types::Paragraph;
use crate::types::TypeState;
use crate::utils::compare;
use crate::TypePairs;
use core::hash::Hasher;

use crate::BUTTON_CLASS;

#[cfg(feature = "hydrate")]
use leptos::web_sys;
#[cfg(feature = "hydrate")]
use wasm_bindgen::JsValue;
#[cfg(feature = "hydrate")]
use wasm_bindgen_futures::JsFuture;

#[cfg(feature = "hydrate")]
async fn copy_to_clipboard(text: String) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let clipboard = window.navigator().clipboard();
    JsFuture::from(clipboard.write_text(&text)).await?;
    Ok(())
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Association {
    pub start_position: usize,
    pub original: BTreeSet<usize>,
    pub translation: BTreeSet<usize>,
}

impl Association {
    fn new(original: BTreeSet<usize>, translation: BTreeSet<usize>) -> Self {
        Association {
            start_position: *original.iter().next().unwrap(),
            original,
            translation,
        }
    }
}
impl PartialOrd for Association {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.start_position.partial_cmp(&other.start_position)
    }
}
impl Ord for Association {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start_position.cmp(&other.start_position)
    }
}
impl Hash for Association {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.original.iter().for_each(|item| item.hash(state));
        self.translation.iter().for_each(|item| item.hash(state));
    }
}
#[derive(Debug, Clone)]
enum Clicked {
    SelectedOriginal(usize, usize),
    SelectedTranslation(usize, usize),

    Original(usize),
    Translation(usize),
    None,
}
#[derive(Debug, Clone)]
enum EvaluationFor {
    Original,
    Translation,
}
enum WordState {
    /// selected word part of pair
    Pair,
    /// selected word part of highlighted pair
    Highlighted,
    /// word part of highlighted pair
    HighlightedPair,
    /// new selection
    Clicked,
    ClickedSelected,
    /// normal word render
    None,
}

#[derive(Debug, Clone)]
struct TypingState {
    original_selected: BTreeSet<usize>,
    translated_selected: BTreeSet<usize>,
    pairs: BTreeSet<Association>,
    clicked: Clicked,
    enable_selection: bool,
}

impl TypingState {
    fn default() -> Self {
        Self {
            original_selected: BTreeSet::new(),
            translated_selected: BTreeSet::new(),
            pairs: BTreeSet::new(),
            clicked: Clicked::None,
            enable_selection: false,
        }
    }

    fn set_initial_pairs(&mut self, pairs: BTreeSet<Association>) {
        self.pairs = pairs;
    }

    fn get_current_pairs(&self) -> BTreeSet<Association> {
        self.pairs.clone()
    }

    fn get_pair_index_for_word_if_any(
        &self,
        index: usize,
        evaluation_for: EvaluationFor,
    ) -> Option<usize> {
        match evaluation_for {
            EvaluationFor::Original => self
                .pairs
                .iter()
                .enumerate()
                .find(|(_pair_index, item)| item.original.iter().any(|item| *item == index))
                .map_or_else(|| None, |(pair_index, _item)| Some(pair_index)),
            EvaluationFor::Translation => self
                .pairs
                .iter()
                .enumerate()
                .find(|(_pair_index, item)| item.translation.iter().any(|item| *item == index))
                .map_or_else(|| None, |(pair_index, _item)| Some(pair_index)),
        }
    }

    /// check if clicked or if activate hightligh
    ///
    fn set_selection_click(&mut self, index: usize, evaluation_for: EvaluationFor) {
        if !self.enable_selection {
            return;
        }
        log!("click in new state {}", index);
        match evaluation_for {
            EvaluationFor::Original => {
                if let Clicked::SelectedOriginal(clicked_selected_index, clicked_index) =
                    self.clicked
                {
                    if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Original)
                    {
                        if selected_index == clicked_selected_index {
                            log!("highlight selection {}", index);
                            self.clicked = Clicked::SelectedOriginal(selected_index, index);
                            self.original_selected.clear();
                            return;
                        } else {
                            self.clicked = Clicked::None;
                        }
                    }
                }

                if let Some(selected_index) =
                    self.get_pair_index_for_word_if_any(index, EvaluationFor::Original)
                {
                    self.clicked = Clicked::SelectedOriginal(selected_index, index);
                    self.original_selected.clear();
                    self.translated_selected.clear();
                    return;
                }

                if !self.original_selected.contains(&index) {
                    log!("click inserting {}", index);
                    self.original_selected.insert(index);
                    if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Original)
                    {
                        log!("highlight selection {}", index);
                        self.clicked = Clicked::SelectedOriginal(selected_index, index);
                    } else {
                        log!("click on selection {}", index);
                        self.clicked = Clicked::Original(index);
                    }
                } else if let Clicked::Original(clicked_index) = self.clicked {
                    if clicked_index == index {
                        log!("remove from selection {}", index);
                        self.original_selected.remove(&index);
                        self.clicked = Clicked::None;
                    } else {
                        self.clicked = Clicked::Original(index);
                    }
                } else {
                    self.clicked = Clicked::Original(index);
                };

                log!("click {:?}", self.clicked);
                log!("original selected {:?}", self.original_selected);
            }
            EvaluationFor::Translation => {
                if let Clicked::SelectedTranslation(clicked_selected_index, clicked_index) =
                    self.clicked
                {
                    if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Translation)
                    {
                        if selected_index == clicked_selected_index {
                            log!("highlight selection {}", index);
                            self.clicked = Clicked::SelectedTranslation(selected_index, index);
                            self.translated_selected.clear();
                            return;
                        } else {
                            self.clicked = Clicked::None;
                        }
                    }
                }

                if let Some(selected_index) =
                    self.get_pair_index_for_word_if_any(index, EvaluationFor::Translation)
                {
                    self.clicked = Clicked::SelectedTranslation(selected_index, index);
                    self.original_selected.clear();
                    self.translated_selected.clear();
                    return;
                }

                if !self.translated_selected.contains(&index) {
                    log!("click inserting {}", index);
                    self.translated_selected.insert(index);
                    if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Translation)
                    {
                        log!("highlight selection {}", index);
                        self.clicked = Clicked::SelectedTranslation(selected_index, index);
                    } else {
                        log!("click on selection {}", index);
                        self.clicked = Clicked::Translation(index);
                    }
                } else if let Clicked::Translation(clicked_index) = self.clicked {
                    if clicked_index == index {
                        log!("remove from selection {}", index);
                        self.translated_selected.remove(&index);
                        self.clicked = Clicked::None;
                    } else {
                        self.clicked = Clicked::Translation(index);
                    }
                } else {
                    self.clicked = Clicked::Translation(index);
                };

                log!("click {:?}", self.clicked);
                log!("translation selected {:?}", self.translated_selected);
            }
        };
    }

    fn get_state_for_word(&self, index: usize, evaluation_for: EvaluationFor) -> WordState {
        match evaluation_for {
            EvaluationFor::Original => {
                if let Clicked::SelectedOriginal(clicked_selected_index, clicked_index) =
                    self.clicked
                {
                    if clicked_index == index {
                        return WordState::Highlighted;
                    } else if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Original)
                    {
                        if clicked_selected_index == selected_index {
                            return WordState::HighlightedPair;
                        }
                    }
                }

                if let Clicked::SelectedTranslation(clicked_selected_index, clicked_index) =
                    self.clicked
                {
                    if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Original)
                    {
                        if clicked_selected_index == selected_index {
                            return WordState::HighlightedPair;
                        }
                    }
                }

                if let Clicked::Original(clicked_index) = self.clicked {
                    if clicked_index == index {
                        return WordState::ClickedSelected;
                    }
                }

                if let Some(selected_index) =
                    self.get_pair_index_for_word_if_any(index, EvaluationFor::Original)
                {
                    return WordState::Pair;
                }

                if self.original_selected.contains(&index) {
                    WordState::Clicked
                } else {
                    WordState::None
                }
            }
            EvaluationFor::Translation => {
                if let Clicked::SelectedTranslation(clicked_selected_index, clicked_index) =
                    self.clicked
                {
                    if clicked_index == index {
                        return WordState::Highlighted;
                    } else if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Translation)
                    {
                        if clicked_selected_index == selected_index {
                            return WordState::HighlightedPair;
                        }
                    }
                }

                if let Clicked::SelectedOriginal(clicked_selected_index, clicked_index) =
                    self.clicked
                {
                    if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Translation)
                    {
                        if clicked_selected_index == selected_index {
                            return WordState::HighlightedPair;
                        }
                    }
                }

                if let Clicked::Translation(clicked_index) = self.clicked {
                    if clicked_index == index {
                        return WordState::ClickedSelected;
                    }
                }

                if let Some(selected_index) =
                    self.get_pair_index_for_word_if_any(index, EvaluationFor::Translation)
                {
                    return WordState::Pair;
                }

                if self.translated_selected.contains(&index) {
                    WordState::Clicked
                } else {
                    WordState::None
                }
            }
        }
    }

    fn pair_enabled(&self) -> bool {
        log!("evaluate if pair action is enabled {:?}", self.pairs);
        !self.original_selected.is_empty() && !self.translated_selected.is_empty()
    }
    fn pair(&mut self) {
        if !self.original_selected.is_empty() && !self.translated_selected.is_empty() {
            self.pairs.insert(Association::new(
                self.original_selected.clone(),
                self.translated_selected.clone(),
            ));
            log!("new pair {:?}", self.pairs);
            self.original_selected.clear();
            self.translated_selected.clear();
            self.clicked = Clicked::None;
        }
    }

    fn remove(&mut self, pair_to_remove: usize) {
        let selected = self.pairs.iter().nth(pair_to_remove).unwrap().clone();
        self.pairs.remove(&selected);
        self.clicked = Clicked::None;
    }

    fn get_style_for_word_state(word_state: WordState) -> &'static str {
        match word_state {
            WordState::Pair => "relative flex px-1.5 py-0.5 lg:mt-1 rounded-md bg-emerald-500/10 text-emerald-300",
            WordState::Highlighted => "relative flex px-1.5 py-0.5 lg:mt-1 rounded-md bg-rose-500/15 text-rose-200",
            WordState::HighlightedPair => "relative flex px-1.5 py-0.5 lg:mt-1 rounded-md bg-sky-500/15 text-sky-200",
            WordState::Clicked => "relative flex px-1.5 py-0.5 lg:mt-1 rounded-md underline decoration-indigo-400 decoration-2 underline-offset-4",
            WordState::ClickedSelected => "relative flex px-1.5 py-0.5 lg:mt-1 rounded-md bg-amber-500/10 underline decoration-amber-400 decoration-2 underline-offset-4",
            WordState::None => "relative flex px-1.5 py-0.5 lg:mt-1 rounded-md transition-colors hover:bg-white/5",
        }
    }

    fn toogle_enable_pair(&mut self) {
        if self.enable_selection {
            self.original_selected.clear();
            self.translated_selected.clear();
            self.clicked = Clicked::None;
        }

        self.enable_selection = !self.enable_selection;
    }
}

#[component]
pub fn Sentance(
    paragraph: Paragraph,
    article_index: usize,
    index: usize,
    total: usize,
    pairs: ReadSignal<TypePairs>,
    set_pairs: WriteSignal<TypePairs>,
    div_ref: NodeRef<Div>,
    speech_cursor: ReadSignal<Option<crate::translation_page::SpeechCursor>>,
    audio_directory: Option<String>,
    on_audio_click: Option<UnsyncCallback<usize>>,
    on_replay_word: Option<UnsyncCallback<(usize, usize)>>,
    on_current_paragraph: Option<UnsyncCallback<usize>>,
    audio_current_article: ReadSignal<Option<usize>>,
    audio_current_paragraph: ReadSignal<Option<usize>>,
    audio_is_playing: ReadSignal<bool>,
    typing_speed_paragraph: ReadSignal<Option<usize>>,
    set_typing_speed_paragraph: WriteSignal<Option<usize>>,
    typing_speed_samples: ReadSignal<Vec<f32>>,
    set_typing_speed_samples: WriteSignal<Vec<f32>>,
    completed_typing_speeds: ReadSignal<Vec<Option<f32>>>,
    set_completed_typing_speeds: WriteSignal<Vec<Option<f32>>>,
    set_mistyped_characters: WriteSignal<usize>,
) -> impl IntoView {
    let mut typing_state = TypingState::default();
    if let Some(pairs_for_paragraph) = pairs.read().get(&index) {
        typing_state.set_initial_pairs(pairs_for_paragraph.clone());
    }
    let (sentace_state, set_sentace_state) = signal(typing_state);

    let pair_button = move || {
        if sentace_state.read().pair_enabled() {
            Either::Left(view! {
                <div class="snap-start">
                    <div
                        class="absolute -top-2.5 -right-2 z-10 cursor-pointer rounded-md bg-amber-400 px-1.5 py-px font-sans text-xs font-semibold text-zinc-900 shadow-md shadow-amber-400/30 transition-colors hover:bg-amber-300"
                        on:click=move |_event| {
                            set_sentace_state
                                .update(|state| {
                                    state.pair();
                                    set_pairs
                                        .update(|pairs| {
                                            pairs.insert(index, state.get_current_pairs());
                                        });
                                });
                        }
                    >

                        pair
                    </div>

                </div>
            })
        } else {
            Either::Right(())
        }
    };

    let delete_button = move |pair_to_remove: usize| {
        view! {
            <div>
                <div
                    class="absolute -top-2.5 -right-2 z-10 cursor-pointer rounded-md bg-rose-500 px-1.5 py-px font-sans text-xs font-semibold text-white shadow-md shadow-rose-500/30 transition-colors hover:bg-rose-400"
                    on:click=move |_event| {
                        set_sentace_state
                            .update(|state| {
                                state.remove(pair_to_remove);
                                set_pairs
                                    .update(|pairs| {
                                        pairs.insert(index, state.get_current_pairs());
                                    });
                            });
                    }
                >

                    remove
                </div>

            </div>
        }
            .into_view()
    };

    let translation_words: Vec<String> = if let Some(translation) = paragraph.translation {
        translation.clone().split(" ").map(str::to_string).collect()
    } else {
        vec![]
    };

    #[derive(Clone)]
    struct TypingStat {
        pub timer: wasm_timer::Instant,
        pub paused_at: Option<wasm_timer::Instant>,
        pub paused_total: Duration,
        pub chars: usize,
    }

    impl TypingStat {
        pub fn new() -> Self {
            TypingStat {
                chars: 0,
                timer: wasm_timer::Instant::now(),
                paused_at: None,
                paused_total: Duration::ZERO,
            }
        }
        pub fn tick(&mut self) {
            self.chars += 1;
        }

        pub fn untick(&mut self) {
            if self.chars > 0 {
                self.chars -= 1;
            } else {
                warn!("trying to decrement 0 position");
            }
        }
        pub fn pause(&mut self) {
            if self.paused_at.is_none() {
                self.paused_at = Some(wasm_timer::Instant::now());
            }
        }
        pub fn resume(&mut self) {
            if let Some(paused_at) = self.paused_at.take() {
                self.paused_total += paused_at.elapsed();
            }
        }
        pub fn get_elapsed(&self) -> Duration {
            let mut elapsed = self.timer.elapsed();
            elapsed = elapsed.saturating_sub(self.paused_total);
            if let Some(paused_at) = self.paused_at {
                elapsed = elapsed.saturating_sub(paused_at.elapsed());
            }
            elapsed
        }
        pub fn get_wpm(&self) -> f32 {
            let elapsed = self.get_elapsed().as_secs_f32();
            if self.chars > 0 && elapsed > 0.0 {
                self.chars as f32 / 5.0 / elapsed * 60.0
            } else {
                0.0
            }
        }
    }

    let (store, set_store) = signal(TypeState::from_str(&paragraph.original));
    let replay_word = on_replay_word.clone();
    let paragraph_changed = on_current_paragraph.clone();
    let (typing, set_typing) = signal(Option::<TypingStat>::None);
    let (mistyped, set_mistyped) = signal(0usize);
    let (copy_notice, _set_copy_notice) = signal(false);
    let record_current_speed = move || {
        if let Some(typing) = typing.read().as_ref() {
            set_typing_speed_samples.update(|samples| samples.push(typing.get_wpm()));
        }
    };
    let finalize_current_speed = move |paragraph_index: usize| {
        let final_wpm = typing.read().as_ref().map_or(0.0, |typing| typing.get_wpm());
        set_completed_typing_speeds.update(|speeds| {
            if paragraph_index < speeds.len() {
                speeds[paragraph_index] = Some(final_wpm);
            }
        });
    };
    let switch_to_paragraph = move |paragraph_index: usize| {
        let current_paragraph = typing_speed_paragraph.get();
        if current_paragraph != Some(paragraph_index) {
            set_typing_speed_paragraph.set(Some(paragraph_index));
            set_typing_speed_samples.set(vec![0.0]);
            set_typing.set(Some(TypingStat::new()));
            set_mistyped_characters.set(mistyped.get_untracked());
            if let Some(paragraph_changed) = paragraph_changed.clone() {
                paragraph_changed.run(paragraph_index);
            }
        } else {
            if typing.read().is_none() {
                set_typing.set(Some(TypingStat::new()));
            }
            set_typing.update(|typing| {
                typing.as_mut().map(|typing| typing.resume());
            });
        }
    };
    let class = move || {
        if sentace_state.read().enable_selection {
            "w-full max-w-4xl flex cursor-default rounded-2xl border border-indigo-500/30 bg-indigo-500/[0.04] p-4 lg:p-6 shadow-lg shadow-indigo-500/5 transition-colors duration-300"
        } else {
            "w-full max-w-4xl flex rounded-2xl border border-white/5 bg-white/[0.02] p-4 lg:p-6 transition-colors duration-300 hover:border-white/10"
        }
    };

    view! {
        <div class="flex flex-col items-center justify-center min-h-lvh lg:h-min snap-start parent gap-2" id=index + 1>
            <div class=class>
                <div
                    class="flex flex-wrap font-mono text-lg lg:text-2xl leading-relaxed focus:outline-none"
                    tabindex=1
                    on:keydown=move |event| {
                        let key = event.key_code();
                        let mut local_store = store.get_untracked();
                        if key == 8 && event.ctrl_key() {
                            // Ctrl+Backspace moves the cursor to the beginning of the
                            // current word without changing the typed text.
                            if let Some(word) = local_store.words.get_mut(local_store.word_index) {
                                let typed_characters = word
                                    .characters
                                    .iter()
                                    .filter(|character| character.typed_char.is_some())
                                    .count();
                                word.char_index = 0;
                                word.characters.iter_mut().for_each(|character| {
                                    character.typed_char = None;
                                });
                                set_typing.update(|typing| {
                                    if let Some(typing) = typing.as_mut() {
                                        for _ in 0..typed_characters {
                                            typing.untick();
                                        }
                                    }
                                });
                            }
                            let word_index = local_store.word_index;
                            set_store.set(local_store);
                            if let Some(replay_word) = replay_word.clone() {
                                replay_word.run((index, word_index));
                            }
                            event.prevent_default();
                        } else if key == 8 {
                            if let Some(word) = local_store.words.get_mut(local_store.word_index) {
                                if word.char_index > 0 {
                                    word.char_index -= 1;
                                    let temp = word.characters.get_mut(word.char_index).unwrap();
                                    temp.backspace();
                                    set_typing
                                        .update(|typing| {
                                            typing
                                                .as_mut()
                                                .map(|typing| {
                                                    typing.untick();
                                                });
                                        });
                                } else if local_store.word_index > 0 {
                                    local_store.word_index -= 1;
                                    set_typing
                                        .update(|typing| {
                                            typing
                                                .as_mut()
                                                .map(|typing| {
                                                    typing.untick();
                                                });
                                        });
                                }
                            } else if local_store.word_index > 0 {
                                local_store.word_index -= 1;
                            }
                            set_store.set(local_store);
                            record_current_speed();
                        } else if key == 32 && local_store.word_index < local_store.words.len() - 1
                        {
                            if let Some(word) = local_store.words.get_mut(local_store.word_index) {
                                if word.char_index == word.characters.len() {
                                    event.prevent_default();
                                    local_store.word_index += 1;
                                    set_store.set(local_store);
                                    record_current_speed();
                                    set_typing
                                        .update(|typing| {
                                            typing
                                                .as_mut()
                                                .map(|typing| {
                                                    typing.tick();
                                                });
                                        });
                                }
                            }
                        }
                    }

                    on:focus=move |_event| {
                        switch_to_paragraph(index);
                        if !sentace_state.read().enable_selection {
                            set_store.update(|store| store.focus = true)
                        }
                    }

                    on:focusout=move |_event| {
                        set_store.update(|store| store.focus = false);
                        finalize_current_speed(index);
                        set_typing.update(|typing| {
                            typing.as_mut().map(|typing| typing.pause());
                        });
                    }

                    on:keypress=move |event| {
                        let key = event.key_code();
                        let mut local_store = store.get();
                        if local_store.word_index < local_store.words.len() {
                            let word = local_store.words.get_mut(local_store.word_index).unwrap();
                            if word.char_index < word.characters.len() {
                                let typed_char = char::from_u32(key).unwrap();
                                let character = word.characters.get_mut(word.char_index).unwrap();
                                if !compare(typed_char, character.reference_char) {
                                    set_mistyped.update(|count| *count += 1);
                                    set_mistyped_characters.set(mistyped.get_untracked());
                                }
                                character.typed(typed_char);
                                word.char_index += 1;
                                set_typing
                                    .update(|typing| {
                                        typing
                                            .as_mut()
                                            .map(|typing| {
                                                typing.tick();
                                            });
                                    });
                                typing
                                    .read()
                                    .as_ref()
                                    .map(|typing| {
                                        log!(
                                            "timmings chars: {}, timer: {}", typing.chars, typing.timer
                                            .elapsed().as_secs_f32()
                                        );
                                    });
                                set_store.set(local_store);
                                record_current_speed();
                            }
                        }
                        event.prevent_default();
                    }
                >

                    <div class="pr-3 pt-1 lg:pt-2 font-sans text-sm lg:text-base tabular-nums text-indigo-300/50">{index + 1} {"/"} {total} {")"}</div>

                    {
                        view! {
                            <For
                                each=move || store.get().words.into_iter().enumerate()
                                key=move |(index, w)| {
                                    let current_word_index = store.read().word_index == *index
                                        && store.read().focus;
                                    format!("{}-{}-{}", index, w.char_index, current_word_index)
                                }

                                children=move |(word_index, w)| {
                                    let speech_active = move || {
                                        audio_current_article.get() == Some(article_index)
                                            && speech_cursor
                                                .get()
                                                .is_some_and(|cursor| {
                                                    cursor.paragraph == index
                                                        && cursor.word == word_index
                                                })
                                    };
                                    let class = move || {
                                        let mut class = TypingState::get_style_for_word_state(
                                            sentace_state
                                                .read()
                                                .get_state_for_word(word_index, EvaluationFor::Original),
                                        )
                                        .to_string();
                                        if speech_active() {
                                            class.push_str(" underline decoration-cyan-400 decoration-2 underline-offset-4");
                                        }
                                        class
                                    };
                                    view! {
                                        <div
                                            class=class
                                            on:click=move |_| {
                                                if sentace_state.read().enable_selection {
                                                    set_sentace_state
                                                        .update(|state| {
                                                            state
                                                                .set_selection_click(word_index, EvaluationFor::Original);
                                                        });
                                                }
                                            }
                                        >

                                            <For
                                                each=move || w.clone().characters.into_iter().enumerate()
                                                key=move |(index, _c)| { format!("{}", index) }

                                                children=move |(char_index, c)| {
                                                    let class = if let Some(typed_char) = c.typed_char {
                                                        log!("compare {} with {}", typed_char, c.reference_char);
                                                        if compare(typed_char, c.reference_char) {
                                                            if store.read().word_index == word_index {
                                                                "text-indigo-300/70 underline decoration-indigo-400/40 underline-offset-4"
                                                            } else {
                                                                "text-indigo-300/50"
                                                            }
                                                        } else {
                                                            "text-rose-400 italic underline decoration-rose-400/60 decoration-wavy underline-offset-4"
                                                        }
                                                    } else {
                                                        if store.read().word_index == word_index
                                                            && store.read().focus
                                                        {
                                                            "underline decoration-indigo-400 decoration-2 underline-offset-4"
                                                        } else {
                                                            ""
                                                        }
                                                    };
                                                    let local_store = store.get();
                                                    let word_state = local_store
                                                        .words
                                                        .get(local_store.word_index)
                                                        .unwrap();
                                                    let typing_cursor_active = store.read().word_index == word_index
                                                        && store.read().focus
                                                        && (word_state.char_index == char_index || char_index == 0
                                                            || (word_state.characters.len() == word_state.char_index
                                                                && (char_index + 1) == word_state.char_index));
                                                    if typing_cursor_active {
                                                        view! {
                                                            <div class=class node_ref=div_ref>
                                                                {c.reference_char}
                                                            </div>
                                                        }
                                                            .into_any()
                                                    } else {
                                                        view! { <div class=class>{c.reference_char}</div> }
                                                            .into_any()
                                                    }
                                                }
                                            />

                                            {move || {
                                                if let Some(index) = sentace_state
                                                    .read()
                                                    .get_pair_index_for_word_if_any(
                                                        word_index,
                                                        EvaluationFor::Original,
                                                    )
                                                {
                                                    Either::Left(
                                                        view! {
                                                            <div class="absolute -top-2.5 lg:-top-3 right-0.5 rounded border border-indigo-500/30 bg-zinc-900 px-1 py-px font-sans text-[10px] leading-none font-semibold text-indigo-300 shadow">
                                                                {index + 1}
                                                            </div>
                                                        },
                                                    )
                                                } else {
                                                    Either::Right(view! { <div class="absolute"></div> })
                                                }
                                            }}

                                            {move || {
                                                let pair = match sentace_state.read().clicked {
                                                    Clicked::Original(clicked_word_index) => {
                                                        clicked_word_index == word_index
                                                    }
                                                    _ => false,
                                                };
                                                if pair {
                                                    Either::Left(pair_button())
                                                } else {
                                                    Either::Right(())
                                                }
                                            }}

                                            {move || {
                                                if let Clicked::SelectedOriginal(
                                                    clicked_highlight,
                                                    clicked_highligth_word_index,
                                                ) = sentace_state.read().clicked
                                                {
                                                    if clicked_highligth_word_index == word_index {
                                                        return delete_button(clicked_highlight).into_any();
                                                    }
                                                }
                                                ().into_any()
                                            }}

                                        </div>
                                    }
                                }
                            />
                        }
                    }

                </div>
                <div class="flex flex-wrap px-5 lg:px-8 pb-2 text-zinc-500/90 italic">

                    <For
                        each=move || translation_words.clone().into_iter().enumerate()
                        key=move |(index, _item)| *index
                        children=move |(word_index, item)| {
                            let class = move || TypingState::get_style_for_word_state(
                                sentace_state
                                    .read()
                                    .get_state_for_word(word_index, EvaluationFor::Translation),
                            );
                            view! {
                                <div
                                    class=class
                                    on:click=move |_| {
                                        if sentace_state.read().enable_selection {
                                            set_sentace_state
                                                .update(|state| {
                                                    state
                                                        .set_selection_click(
                                                            word_index,
                                                            EvaluationFor::Translation,
                                                        );
                                                });
                                        }
                                    }
                                >

                                    {item}

                                    {move || {
                                        if let Some(index) = sentace_state
                                            .read()
                                            .get_pair_index_for_word_if_any(
                                                word_index,
                                                EvaluationFor::Translation,
                                            )
                                        {
                                            Either::Left(
                                                view! {
                                                    <div class="absolute -top-2.5 lg:-top-3 right-0.5 rounded border border-indigo-500/30 bg-zinc-900 px-1 py-px font-sans text-[10px] leading-none font-semibold text-indigo-300 shadow">
                                                        {index + 1}
                                                    </div>
                                                },
                                            )
                                        } else {
                                            Either::Right(view! { <div class="absolute"></div> })
                                        }
                                    }}

                                    {move || {
                                        let pair = match sentace_state.read().clicked {
                                            Clicked::Translation(clicked_word_index) => {
                                                clicked_word_index == word_index
                                            }
                                            _ => false,
                                        };
                                        if pair {
                                            Either::Left(pair_button())
                                        } else {
                                            Either::Right(())
                                        }
                                    }}

                                    {move || {
                                        if let Clicked::SelectedTranslation(
                                            clicked_highlight,
                                            clicked_highligth_word_index,
                                        ) = sentace_state.read().clicked
                                        {
                                            if clicked_highligth_word_index == word_index {
                                                return delete_button(clicked_highlight).into_any();
                                            }
                                        }
                                        ().into_any()
                                    }}

                                </div>
                            }
                        }
                    />

                </div>
            </div>

            {
                let label = move || {
                    if sentace_state.read().enable_selection {
                        "click to enable typing"
                    } else {
                        "click to enable pairing"
                    }
                };
                let audio_disabled = audio_directory.is_none() || on_audio_click.is_none();
                let audio_click = on_audio_click.clone();
                let audio_label = move || {
                    if audio_current_article.get() == Some(article_index)
                        && audio_current_paragraph.get() == Some(index)
                    {
                        if audio_is_playing.get() {
                            "Pause audio"
                        } else {
                            "Resume audio"
                        }
                    } else {
                        "Play audio"
                    }
                };
                #[cfg(feature = "hydrate")]
                let copy_original = move || {
                    view! {
                        <button
                            class=BUTTON_CLASS
                            title="Copy the original paragraph text"
                            disabled=move || copy_notice.get()
                            on:click=move |_| {
                                let text = paragraph.original.clone();
                                _set_copy_notice.set(true);
                                spawn_local(async move {
                                    let _ = copy_to_clipboard(text).await;
                                    let _ = wasm_timer::Delay::new(Duration::from_millis(1500)).await;
                                    _set_copy_notice.set(false);
                                });
                            }
                        >
                            {move || if copy_notice.get() { "Copied to clipboard" } else { "Copy original" }}
                        </button>
                    }
                };
                #[cfg(not(feature = "hydrate"))]
                let copy_original = || {
                    view! {
                        <button class=BUTTON_CLASS disabled=true title="Copy the original paragraph text">
                            "Copy original"
                        </button>
                    }
                };
                view! {
                    <div class="flex items-center justify-center flex-wrap gap-2 px-4 pb-8">
                        <div
                            class=BUTTON_CLASS
                            on:click=move |_event| {
                                set_sentace_state
                                    .update(|state| {
                                        state.toogle_enable_pair();
                                    });
                            }
                        >

                            {label}
                        </div>
                        <button
                            class=move || {
                                if audio_disabled {
                                    format!("{} opacity-50 cursor-not-allowed", BUTTON_CLASS)
                                } else {
                                    BUTTON_CLASS.to_string()
                                }
                            }
                            disabled=move || audio_disabled
                            on:click=move |_| {
                                if let Some(on_audio_click) = audio_click.clone() {
                                    on_audio_click.run(index);
                                }
                            }
                        >
                            {audio_label}
                        </button>
                        {copy_original()}
                        <div class="rounded-md border border-white/5 bg-white/5 px-2.5 py-1.5 font-mono text-xs text-zinc-400">

                            {move || {
                                typing
                                    .read()
                                    .as_ref()
                                    .map_or_else(
                                        || "not in focus".to_string(),
                                        |typing| format!("{:.2} ", typing.get_wpm()),
                                    )
                            }}
                            <span class="text-zinc-500">(wpm)</span>
                        </div>
                    </div>
                }
            }

        </div>
    }
}

#[component]
pub fn TypingSpeedPanel(
    typing_speed_paragraph: ReadSignal<Option<usize>>,
    typing_speed_samples: ReadSignal<Vec<f32>>,
    completed_typing_speeds: ReadSignal<Vec<Option<f32>>>,
    mistyped_characters: ReadSignal<usize>,
) -> impl IntoView {
    let current_speed = move || typing_speed_samples.get().last().copied().unwrap_or(0.0);
    let average_previous = move || {
        let paragraph_index = typing_speed_paragraph.get()?;
        let speeds = completed_typing_speeds.get();
        let previous: Vec<f32> = speeds.iter().take(paragraph_index).flatten().copied().collect();
        if previous.is_empty() {
            None
        } else {
            Some(previous.iter().sum::<f32>() / previous.len() as f32)
        }
    };
    let chart_max_speed = move || {
        let samples = typing_speed_samples.get();
        let samples_max = samples.iter().copied().fold(1.0_f32, |acc, item| acc.max(item));
        let baseline = average_previous().map(|avg| avg + 10.0).unwrap_or(0.0);
        samples_max.max(baseline).max(1.0)
    };
    let chart_points = move || {
        let samples = typing_speed_samples.get();
        if samples.is_empty() {
            return String::new();
        }
        let width = 240.0;
        let height = 100.0;
        let max_speed = chart_max_speed();
        let last_index = samples.len().saturating_sub(1).max(1) as f32;
        samples
            .iter()
            .enumerate()
            .map(|(index, speed)| {
                let x = (index as f32 / last_index) * width;
                let y = height - ((speed / max_speed) * height);
                format!("{x:.2},{y:.2}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let average_line_y = move || {
        average_previous().map(|avg| {
            let height = 100.0;
            let y = height - (((avg) / chart_max_speed()) * height);
            y.clamp(0.0, height)
        })
    };

    view! {
        <Show
            when=move || typing_speed_paragraph.get().is_some() && !typing_speed_samples.get().is_empty()
            fallback=move || view! { <div class="hidden"></div> }
        >
            <div class="fixed top-2 right-2 z-30 overflow-hidden rounded-xl border border-white/10 bg-zinc-900/80 shadow-2xl shadow-black/40 backdrop-blur-xl">
                <div class="relative w-full">
                    <svg viewBox="0 0 240 100" class="w-full h-24 block overflow-visible">
                        <rect x="0" y="0" width="240" height="100" rx="8" class="fill-[#0c0c14] stroke-white/10" stroke-width="1"/>
                        <Show when=move || average_line_y().is_some() fallback=move || view! { <div class="hidden"></div> }>
                            <line
                                x1="0"
                                x2="240"
                                y1=move || average_line_y().unwrap_or(0.0)
                                y2=move || average_line_y().unwrap_or(0.0)
                                stroke="#9ca3af"
                                stroke-width="1"
                                stroke-dasharray="4 4"
                            />
                        </Show>
                        <polyline fill="none" stroke="#818cf8" stroke-width="2" stroke-linejoin="round" stroke-linecap="round" points=chart_points/>
                    </svg>
                    <div class="absolute inset-0 pointer-events-none text-[10px] text-zinc-300 p-2">
                        <div class="absolute bottom-1 left-2 rounded-md bg-black/50 px-2 py-0.5 font-mono text-indigo-300">
                            {move || format!("{:.1} WPM", current_speed())}
                        </div>
                        <div class="absolute top-1 right-2 rounded-md bg-black/50 px-2 py-0.5 font-mono text-rose-400">
                            {move || format!("{} mistyped", mistyped_characters.get())}
                        </div>
                        <div class="absolute bottom-1 right-2 rounded-md bg-black/50 px-2 py-0.5 font-mono">
                            {move || average_previous().map(|avg| format!("avg prev {:.1} WPM", avg)).unwrap_or_else(|| "avg prev —".to_string())}
                        </div>
                    </div>
                </div>
            </div>
        </Show>
    }
}
