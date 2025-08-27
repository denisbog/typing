use std::ops::Sub;
use std::time::Duration;
use std::{collections::BTreeSet, hash::Hash};

use leptos::either::Either;
use leptos::html::Div;
use leptos::logging::log;
use leptos::logging::warn;
use leptos::prelude::*;
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
            WordState::Pair => "relative flex p-1 lg:mt-1 bg-blue-100",
            WordState::Highlighted => "relative flex p-1 lg:mt-1 bg-red-100",
            WordState::HighlightedPair => "relative flex p-1 lg:mt-1 bg-blue-200",
            WordState::Clicked => "relative flex p-1 lg:mt-1 underline",
            WordState::ClickedSelected => "relative flex p-1 lg:mt-1 underline bg-yellow-100",
            WordState::None => "relative flex p-1 lg:mt-1",
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
#[component]
pub fn Sentance(
    paragraph: Paragraph,
    index: usize,
    total: usize,
    pairs: ReadSignal<TypePairs>,
    set_pairs: WriteSignal<TypePairs>,
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
                        class="absolute -top-2 -right-2 italic text-xs lg:text-md underline cursor-pointer z-10 bg-yellow-200 p-1 shadow-md rounded"
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
                    class="absolute -top-2 -right-2 italic text-xs lg:text-md underline cursor-pointer z-10 bg-red-200 p-1 shadow-md rounded"
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
        pub chars: usize,
    }

    impl TypingStat {
        pub fn new() -> Self {
            TypingStat {
                chars: 0,
                timer: wasm_timer::Instant::now(),
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
        pub fn get_wpm(&self) -> f32 {
            if self.chars > 0 {
                self.chars as f32 / 5.0 / self.timer.elapsed().as_secs_f32() * 60.0
            } else {
                0.0
            }
        }
    }

    let (store, set_store) = signal(TypeState::from_str(&paragraph.original));
    let (typing, set_typing) = signal(Option::<TypingStat>::None);
    let class = move || {
        if sentace_state.read().enable_selection {
            "p-2 cursor-default"
        } else {
            "p-2"
        }
    };

    let div_ref = NodeRef::<Div>::new();
    let (coordinates, set_coordinates) = signal(EffectPosition { x: 0.0, y: 0.0 });
    Effect::new(move |_| {
        if let Some(div) = div_ref.get() {
            // Cast NodeRef to web_sys::Element
            let element = div;
            // Get bounding client rect
            let rect = element.get_bounding_client_rect();
            let x = rect.x();
            let y = rect.y();
            // Get scroll offsets for document coordinates
            let doc_x = x + window().scroll_x().unwrap_or(0.0);
            let doc_y = y + window().scroll_y().unwrap_or(0.0);
            set_coordinates.set(EffectPosition { x: doc_x, y: doc_y });
        }
    });

    AnimationContext::provide();

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

    view! {
        <div class="flex flex-col justify-center min-h-lvh lg:h-min snap-start parent" id=index + 1>
            <div class=class>
                <div
                    class="lg:p-2 flex flex-wrap lg:text-3xl font-mono focus:outline-none"
                    tabindex=1
                    on:keydown=move |event| {
                        let key = event.key_code();
                        let mut local_store = store.get_untracked();
                        if key == 8 {
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
                        } else if key == 32 && local_store.word_index < local_store.words.len() - 1 {
                            if let Some(word) = local_store.words.get_mut(local_store.word_index) {
                                if word.char_index == word.characters.len() {
                                    event.prevent_default();
                                    local_store.word_index += 1;
                                    set_store.set(local_store);
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
                        set_typing.set(Some(TypingStat::new()));
                        if !sentace_state.read().enable_selection {
                            set_store.update(|store| store.focus = true)
                        }
                    }

                    on:focusout=move |_event| { set_store.update(|store| store.focus = false) }

                    on:keypress=move |event| {
                        let key = event.key_code();
                        let mut local_store = store.get();
                        if local_store.word_index < local_store.words.len() {
                            let word = local_store.words.get_mut(local_store.word_index).unwrap();
                            if word.char_index < word.characters.len() {
                                word.characters
                                    .get_mut(word.char_index)
                                    .unwrap()
                                    .typed(char::from_u32(key).unwrap());
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
                            }
                        }
                        event.prevent_default();
                    }
                >

                    <div class="absolute animate-blink" style=caret_position>
                        <span class="text-xl lg:text-3xl font-extrabold font-mono text-yellow-500 font-bold">
                            _
                        </span>
                    </div>
                    <div class="pr-2">{index + 1} {"/"} {total} {")"}</div>

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
                                    let class = move || TypingState::get_style_for_word_state(
                                        sentace_state
                                            .read()
                                            .get_state_for_word(word_index, EvaluationFor::Original),
                                    );
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
                                                key=move |(index, _c)| {
                                                    format!("{}", index)
                                                }

                                                children=move |(char_index, c)| {
                                                    let class = if let Some(typed_char) = c.typed_char {
                                                        log!("compare {} with {}", typed_char, c.reference_char);
                                                        if compare(typed_char, c.reference_char) {
                                                            if store.read().word_index == word_index {
                                                                "text-gray-700 underline"
                                                            } else {
                                                                "text-gray-700"
                                                            }
                                                        } else {
                                                            "text-red-400 italic underline"
                                                        }
                                                    } else {
                                                        if store.read().word_index == word_index
                                                            && store.read().focus
                                                        {
                                                            "underline"
                                                        } else {
                                                            ""
                                                        }
                                                    };
                                                    let local_store = store.get();
                                                    let word_state = local_store
                                                        .words
                                                        .get(local_store.word_index)
                                                        .unwrap();
                                                    if store.read().word_index == word_index
                                                        && store.read().focus
                                                        && (word_state.char_index == char_index || char_index == 0
                                                            || (word_state.characters.len() == word_state.char_index
                                                                && (char_index + 1) == word_state.char_index))
                                                    {
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
                                                            <div class="absolute -top-2 lg:-top-4 right-1 text-red-600 italic text-xs lg:text-md bg-blue-200 shadow-md rounded px-1 border-solid-1 font-sans">
                                                                {index}
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
                <div class="pr-2 flex flex-col items-end">
                    WPM:
                    {move || {
                        typing
                            .read()
                            .as_ref()
                            .map_or_else(
                                || "not in focus".to_string(),
                                |typing| format!("{:.2} ", typing.get_wpm()),
                            )
                    }}

                </div>
                <div class="px-2 lg:px-5 lg:p-3 flex flex-wrap text-gray-500 italic cursor-default">

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
                                                    <div class="absolute -top-2 lg:-top-4 right-1 text-red-600 italic text-xs lg:text-md bg-blue-200 shadow-md rounded px-1 border-solid-1 font-sans">
                                                        {index}
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
                view! {
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
                }
            }

        </div>
    }
}
