use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::Mutex;

use leptos::*;
use serde::Deserialize;
use serde::Serialize;

use crate::types::TypeState;
use crate::utils::compare;
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
        logging::log!("click in new state {}", index);
        match evaluation_for {
            EvaluationFor::Original => {
                if let Clicked::SelectedOriginal(clicked_selected_index, clicked_index) =
                    self.clicked
                {
                    if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Original)
                    {
                        if selected_index == clicked_selected_index {
                            logging::log!("highlight selection {}", index);
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
                    logging::log!("click inserting {}", index);
                    self.original_selected.insert(index);
                    if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Original)
                    {
                        logging::log!("highlight selection {}", index);
                        self.clicked = Clicked::SelectedOriginal(selected_index, index);
                    } else {
                        logging::log!("click on selection {}", index);
                        self.clicked = Clicked::Original(index);
                    }
                } else if let Clicked::Original(clicked_index) = self.clicked {
                    if clicked_index == index {
                        logging::log!("remove from selection {}", index);
                        self.original_selected.remove(&index);
                        self.clicked = Clicked::None;
                    } else {
                        self.clicked = Clicked::Original(index);
                    }
                } else {
                    self.clicked = Clicked::Original(index);
                };

                logging::log!("click {:?}", self.clicked);
                logging::log!("original selected {:?}", self.original_selected);
            }
            EvaluationFor::Translation => {
                if let Clicked::SelectedTranslation(clicked_selected_index, clicked_index) =
                    self.clicked
                {
                    if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Translation)
                    {
                        if selected_index == clicked_selected_index {
                            logging::log!("highlight selection {}", index);
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
                    logging::log!("click inserting {}", index);
                    self.translated_selected.insert(index);
                    if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Translation)
                    {
                        logging::log!("highlight selection {}", index);
                        self.clicked = Clicked::SelectedTranslation(selected_index, index);
                    } else {
                        logging::log!("click on selection {}", index);
                        self.clicked = Clicked::Translation(index);
                    }
                } else if let Clicked::Translation(clicked_index) = self.clicked {
                    if clicked_index == index {
                        logging::log!("remove from selection {}", index);
                        self.translated_selected.remove(&index);
                        self.clicked = Clicked::None;
                    } else {
                        self.clicked = Clicked::Translation(index);
                    }
                } else {
                    self.clicked = Clicked::Translation(index);
                };

                logging::log!("click {:?}", self.clicked);
                logging::log!("translation selected {:?}", self.translated_selected);
            }
        };
    }

    fn get_state_for_word(&self, index: usize, evaluation_for: EvaluationFor) -> WordState {
        logging::log!("get state for word");
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
        logging::log!("evaluate if pair action is enabled {:?}", self.pairs);
        !self.original_selected.is_empty() && !self.translated_selected.is_empty()
    }
    fn pair(&mut self) {
        if !self.original_selected.is_empty() && !self.translated_selected.is_empty() {
            self.pairs.insert(Association::new(
                self.original_selected.clone(),
                self.translated_selected.clone(),
            ));
            logging::log!("new pair {:?}", self.pairs);
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
            WordState::Pair => "relative flex lg:px-2 p-1 lg:mt-1 bg-blue-100",
            WordState::Highlighted => "relative flex lg:px-2 p-1 lg:mt-1 bg-red-100",
            WordState::HighlightedPair => "relative flex lg:px-2 p-1 lg:mt-1 bg-blue-200",
            WordState::Clicked => "relative flex lg:px-2 p-1 lg:mt-1 underline",
            WordState::ClickedSelected => {
                "relative flex lg:px-2 p-1 lg:mt-1 underline bg-yellow-100"
            }
            WordState::None => "relative flex lg:px-2 p-1 lg:mt-1",
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
    text: String,
    translation: String,
    article_id: usize,
    index: usize,
    pairs: ReadSignal<BTreeMap<usize, BTreeMap<usize, BTreeSet<Association>>>>,
    set_pairs: WriteSignal<BTreeMap<usize, BTreeMap<usize, BTreeSet<Association>>>>,
) -> impl IntoView {
    let (sentace_state, set_sentace_state) =
        create_signal(Arc::new(Mutex::new(TypingState::default())));

    if let Some(pairs_for_article) = pairs.get().get(&article_id) {
        if let Some(pairs_for_paragraph) = pairs_for_article.get(&index) {
            set_sentace_state.update(|state| {
                state
                    .lock()
                    .unwrap()
                    .set_initial_pairs(pairs_for_paragraph.clone())
            })
        }
    }

    let pair_button = move || {
        if sentace_state.get().lock().unwrap().pair_enabled() {
            view! {
                <div class="snap-start">
                    <div
                        class="absolute -top-2 -right-2 italic text-xs lg:text-md underline cursor-pointer z-10 bg-yellow-200 p-1 shadow-md rounded"
                        on:click=move |_event| {
                            set_sentace_state
                                .update(|state| {
                                    let mut state = state.lock().unwrap();
                                    state.pair();
                                    set_pairs
                                        .update(|pairs| {
                                            pairs
                                                .entry(article_id)
                                                .or_default()
                                                .insert(index, state.get_current_pairs());
                                        });
                                });
                        }
                    >

                        pair
                    </div>

                </div>
            }
            .into_view()
        } else {
            view! {}.into_view()
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
                                state.lock().unwrap().remove(pair_to_remove);
                                set_pairs
                                    .update(|pairs| {
                                        pairs
                                            .entry(article_id)
                                            .or_default()
                                            .insert(index, state.lock().unwrap().get_current_pairs());
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

    let translation_words: Vec<String> =
        translation.clone().split(" ").map(str::to_string).collect();
    let (store, set_store) = create_signal(TypeState::from_str(&text));
    let class = move || {
        if sentace_state.get().lock().unwrap().enable_selection {
            "outline-dashed p-2 cursor-default"
        } else {
            "outline-dashed p-2"
        }
    };
    view! {
        <div class="flex flex-col justify-center min-h-lvh lg:h-min snap-start">
            <div class=class>
                <div
                    class="lg:p-2 flex flex-wrap lg:text-2xl text-gray-500 font-mono focus:outline-none"
                    tabindex=1
                    on:keydown=move |event| {
                        let key = event.key_code();
                        let mut local_store = store.get_untracked();
                        logging::log!("key down {} {}", key, local_store.word_index);
                        if key == 8 {
                            if let Some(word) = local_store.data.get_mut(local_store.word_index) {
                                if word.char_index > 0 {
                                    word.char_index -= 1;
                                    let temp = word.data.get_mut(word.char_index).unwrap();
                                    temp.backspace();
                                } else if local_store.word_index > 0 {
                                    local_store.word_index -= 1;
                                }
                            } else if local_store.word_index > 0 {
                                local_store.word_index -= 1;
                            }
                            set_store.set(local_store);
                        } else if (key == 32) && local_store.word_index < local_store.data.len() {
                            event.prevent_default();
                            local_store.word_index += 1;
                            set_store.set(local_store);
                        }
                    }

                    on:focus=move |_event| {
                        if !sentace_state.get().lock().unwrap().enable_selection {
                            set_store.update(|store| store.focus = true)
                        }
                    }

                    on:focusout=move |_event| { set_store.update(|store| store.focus = false) }

                    on:keypress=move |event| {
                        let key = event.key_code();
                        match key {
                            (64..=93) | (97..=122) | 34 | 39 | 44 | 45 | 46 | 58 | 59 => {
                                let mut local_store = store.get();
                                if local_store.word_index < local_store.data.len() {
                                    logging::log!("current index {}", local_store.word_index);
                                    logging::log!(
                                        "current word index {}", local_store.data.get(local_store
                                        .word_index).unwrap().char_index
                                    );
                                    let word = local_store
                                        .data
                                        .get_mut(local_store.word_index)
                                        .unwrap();
                                    if word.char_index < word.data.len() {
                                        logging::log!("inserting {}", char::from_u32(key).unwrap());
                                        word.data
                                            .get_mut(word.char_index)
                                            .unwrap()
                                            .typed(char::from_u32(key).unwrap());
                                        word.char_index += 1;
                                        set_store.set(local_store);
                                    }
                                }
                            }
                            _ => {}
                        };
                        logging::log!("keypress {:?}", & key);
                    }
                >

                    {
                        view! {
                            <For
                                each=move || store.get().data.into_iter().enumerate()
                                key=move |(index, c)| { format!("{}-{}", index, c.char_index) }

                                children=move |(word_index, c)| {
                                    let class = move || TypingState::get_style_for_word_state(
                                        sentace_state
                                            .get()
                                            .lock()
                                            .unwrap()
                                            .get_state_for_word(word_index, EvaluationFor::Original),
                                    );
                                    view! {
                                        <div
                                            class=class
                                            on:click=move |_| {
                                                if sentace_state.get().lock().unwrap().enable_selection {
                                                    set_sentace_state
                                                        .update(|state| {
                                                            state
                                                                .lock()
                                                                .unwrap()
                                                                .set_selection_click(word_index, EvaluationFor::Original);
                                                        });
                                                }
                                            }
                                        >

                                            <For
                                                each=move || c.clone().data.into_iter().enumerate()
                                                key=|(index, c)| {
                                                    format!("{}-{}", index, c.typed_char.unwrap_or('~'))
                                                }

                                                children=move |(_index, c)| {
                                                    if let Some(typed_char) = c.typed_char {
                                                        if compare(typed_char, c.reference_char) {
                                                            let class = move || {
                                                                if store.get().word_index == word_index {
                                                                    "lg:min-w-4 text-gray-900 underline"
                                                                } else {
                                                                    "lg:min-w-4 text-gray-900"
                                                                }
                                                            };
                                                            return view! { <div class=class>{c.reference_char}</div> };
                                                        } else {
                                                            return view! {
                                                                <div class="relative text-gray-400 lg:min-w-4 underline">
                                                                    {c.reference_char}
                                                                    <div class="absolute -top-0 -right-0 text-red-700 italic underline">
                                                                        <p>{c.typed_char}</p>
                                                                    </div>
                                                                </div>
                                                            };
                                                        }
                                                    }
                                                    let class = move || {
                                                        if store.get().word_index == word_index && store.get().focus
                                                        {
                                                            "lg:min-w-4 underline"
                                                        } else {
                                                            "lg:min-w-4"
                                                        }
                                                    };
                                                    view! { <div class=class>{c.reference_char}</div> }
                                                }
                                            />

                                            {move || {
                                                if let Some(index) = sentace_state
                                                    .get()
                                                    .lock()
                                                    .unwrap()
                                                    .get_pair_index_for_word_if_any(
                                                        word_index,
                                                        EvaluationFor::Original,
                                                    )
                                                {
                                                    view! {
                                                        <div class="absolute -top-2 lg:-top-4 right-1 text-red-600 italic text-xs lg:text-md bg-blue-200 shadow-md rounded px-1 border-solid-1 font-sans">
                                                            {index}
                                                        </div>
                                                    }
                                                } else {
                                                    view! { <div class="absolute"></div> }
                                                }
                                            }}

                                            {move || {
                                                let pair = match sentace_state.get().lock().unwrap().clicked
                                                {
                                                    Clicked::Original(clicked_word_index) => {
                                                        clicked_word_index == word_index
                                                    }
                                                    _ => false,
                                                };
                                                if pair {
                                                    pair_button().into_view()
                                                } else {
                                                    view! {}.into_view()
                                                }
                                            }}

                                            {move || {
                                                if let Clicked::SelectedOriginal(
                                                    clicked_highlight,
                                                    clicked_highligth_word_index,
                                                ) = sentace_state.get().lock().unwrap().clicked
                                                {
                                                    if clicked_highligth_word_index == word_index {
                                                        return delete_button(clicked_highlight);
                                                    }
                                                }
                                                view! {}.into_view()
                                            }}

                                        </div>
                                    }
                                }
                            />
                        }
                    }

                </div>
                <div class="px-2 lg:px-5 lg:p-3 flex flex-wrap text-gray-500 italic cursor-default">

                    <For
                        each=move || translation_words.clone().into_iter().enumerate()
                        key=move |(index, _item)| *index
                        children=move |(word_index, item)| {
                            let class = move || TypingState::get_style_for_word_state(
                                sentace_state
                                    .get()
                                    .lock()
                                    .unwrap()
                                    .get_state_for_word(word_index, EvaluationFor::Translation),
                            );
                            view! {
                                <div
                                    class=class
                                    on:click=move |_| {
                                        if sentace_state.get().lock().unwrap().enable_selection {
                                            set_sentace_state
                                                .update(|state| {
                                                    state
                                                        .lock()
                                                        .unwrap()
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
                                            .get()
                                            .lock()
                                            .unwrap()
                                            .get_pair_index_for_word_if_any(
                                                word_index,
                                                EvaluationFor::Translation,
                                            )
                                        {
                                            view! {
                                                <div class="absolute -top-2 lg:-top-4 right-1 text-red-600 italic text-xs lg:text-md bg-blue-200 shadow-md rounded px-1 border-solid-1 font-sans">
                                                    {index}
                                                </div>
                                            }
                                        } else {
                                            view! { <div class="absolute"></div> }
                                        }
                                    }}

                                    {move || {
                                        let pair = match sentace_state.get().lock().unwrap().clicked
                                        {
                                            Clicked::Translation(clicked_word_index) => {
                                                clicked_word_index == word_index
                                            }
                                            _ => false,
                                        };
                                        if pair {
                                            pair_button().into_view()
                                        } else {
                                            view! {}.into_view()
                                        }
                                    }}

                                    {move || {
                                        if let Clicked::SelectedTranslation(
                                            clicked_highlight,
                                            clicked_highligth_word_index,
                                        ) = sentace_state.get().lock().unwrap().clicked
                                        {
                                            if clicked_highligth_word_index == word_index {
                                                return delete_button(clicked_highlight);
                                            }
                                        }
                                        view! {}.into_view()
                                    }}

                                </div>
                            }
                        }
                    />

                </div>
            </div>

            {
                let label = move || {
                    if sentace_state.get().lock().unwrap().enable_selection {
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
                                    state.lock().unwrap().toogle_enable_pair();
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
