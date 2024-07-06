use std::collections::BTreeSet;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::Mutex;
use std::usize;

use leptos::*;

use crate::types::TypeState;
use crate::utils::compare;
use core::hash::Hasher;

#[derive(Eq, PartialEq, Clone, Debug)]
struct Association {
    start_position: usize,
    original: BTreeSet<usize>,
    translation: HashSet<usize>,
}

impl Association {
    fn new(original: BTreeSet<usize>, translation: HashSet<usize>) -> Self {
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
    Pair(usize),
    /// selected word part of highlighted pair
    Highlighted(usize),
    /// word part of a selection
    SelectedPair(usize),
    /// word part of highlighted pair
    HighlightedPair(usize),
    /// new selection
    Clicked,
    ClickedSelected,
    /// normal word render
    None,
}

struct TypingState {
    original_selected: BTreeSet<usize>,
    translated_selected: HashSet<usize>,
    pairs: BTreeSet<Association>,
    clicked: Clicked,
}

impl TypingState {
    fn default() -> Self {
        Self {
            original_selected: BTreeSet::new(),
            translated_selected: HashSet::new(),
            pairs: BTreeSet::new(),
            clicked: Clicked::None,
        }
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
                            return ();
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
                    return ();
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
                            return ();
                        } else {
                            self.clicked = Clicked::None;
                        }
                    }
                }

                if let Some(selected_index) =
                    self.get_pair_index_for_word_if_any(index, EvaluationFor::Translation)
                {
                    self.clicked = Clicked::SelectedTranslation(selected_index, index);
                    self.translated_selected.clear();
                    return ();
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
        match evaluation_for {
            EvaluationFor::Original => {
                if let Clicked::SelectedOriginal(clicked_selected_index, clicked_index) =
                    self.clicked
                {
                    if clicked_index == index {
                        return WordState::Highlighted(clicked_selected_index);
                    } else if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Original)
                    {
                        if clicked_selected_index == selected_index {
                            return WordState::HighlightedPair(selected_index);
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
                    return WordState::Pair(selected_index);
                }

                if self.original_selected.contains(&index) {
                    return WordState::Clicked;
                } else {
                    return WordState::None;
                }
            }
            EvaluationFor::Translation => {
                if let Clicked::SelectedTranslation(clicked_selected_index, clicked_index) =
                    self.clicked
                {
                    if clicked_index == index {
                        return WordState::Highlighted(clicked_selected_index);
                    } else if let Some(selected_index) =
                        self.get_pair_index_for_word_if_any(index, EvaluationFor::Translation)
                    {
                        if clicked_selected_index == selected_index {
                            return WordState::HighlightedPair(selected_index);
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
                    return WordState::Pair(selected_index);
                }

                if self.translated_selected.contains(&index) {
                    return WordState::Clicked;
                } else {
                    return WordState::None;
                }
            }
        };
    }

    fn pair_enabled(&self) -> bool {
        logging::log!("evaluate if pari action is enabled {:?}", self.pairs);
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
}
#[component]
pub fn Sentance(text: String, translation: String) -> impl IntoView {
    let state = Arc::new(Mutex::new(TypingState::default()));

    let (original_tick, set_original_tick) = create_signal(state.clone());
    let (translation_tick, set_translation_tick) = create_signal(state.clone());

    let pair_button = move || {
        if original_tick.get().lock().unwrap().pair_enabled() {
            view! {
                <div class="snap-start">
                    <div
                        class="absolute -top-2 -right-2 italic text-base underline md:text-xl cursor-pointer z-10"
                        on:click=move |_event| {
                            original_tick.get().lock().unwrap().pair();
                            set_translation_tick.update(|_state| {});
                            set_original_tick.update(|_state| {});
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

    let delete_button = move |state: Arc<Mutex<TypingState>>, pair_to_remove: usize| {
        view! {
            <div>
                <div
                    class="absolute -top-2 -right-2 italic text-base underline md:text-xl cursor-pointer z-10"
                    on:click=move |_event| {
                        state.lock().unwrap().remove(pair_to_remove);
                        set_translation_tick.update(|_state| {});
                        set_original_tick.update(|_state| {});
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
    view! {
        <div class="flex flex-col justify-center min-h-lvh lg:h-min snap-start">
            <div class="outline-dashed">
                <div
                    class="p-3 flex flex-wrap text-5xl lg:text-3xl text-gray-500 font-mono focus:outline-none"
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
                            set_store(local_store);
                        } else if (key == 32) && local_store.word_index < local_store.data.len() {
                            event.prevent_default();
                            local_store.word_index += 1;
                            set_store(local_store);
                        }
                    }

                    on:focus=move |_event| { set_store.update(|store| store.focus = true) }

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
                                        set_store(local_store);
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
                                    let class = move || match original_tick
                                        .get()
                                        .lock()
                                        .unwrap()
                                        .get_state_for_word(word_index, EvaluationFor::Original)
                                    {
                                        WordState::Pair(_) => {
                                            "relative flex px-2 p-1 outline-dashed bg-blue-200"
                                        }
                                        WordState::Highlighted(_) => {
                                            "relative flex px-2 p-1 bg-red-200"
                                        }
                                        WordState::SelectedPair(_) => {
                                            "relative flex px-2 p-1 line-through bg-green-200"
                                        }
                                        WordState::HighlightedPair(_) => {
                                            "relative flex px-2 p-1 outline bg-orange-200"
                                        }
                                        WordState::Clicked => "relative flex px-2 p-1 underline",
                                        WordState::ClickedSelected => {
                                            "relative flex px-2 p-1 underline bg-yellow-200"
                                        }
                                        WordState::None => "relative flex px-2 p-1",
                                    };
                                    let refresh_other =move || match original_tick
                                        .get()
                                        .lock()
                                        .unwrap()
                                        .get_state_for_word(word_index, EvaluationFor::Original)
                                    {
                                        WordState::ClickedSelected => true,
                                        _ => false,
                                    };
                                    if refresh_other(){
                                logging::log!("update others");
                                        set_translation_tick.update(|_state| {});
                                    }
                                    view! {
                                        // let hightlight_index = move || {
                                        // if let Some(index) = original_tick.get().lock().unwrap().get_pair_index_for_word_if_any(
                                        // word_index,
                                        // EvaluationFor::Original,
                                        // ) {
                                        // view! {
                                        // <div class="absolute -top-0 -right-0 text-red-600 italic text-base md:text-xl">
                                        // {index}
                                        // </div>
                                        // }
                                        // } else {
                                        // view! { <div class="absolute"></div> }
                                        // }
                                        // };
                                        <div
                                            class=class
                                            on:click=move |_| {
                                                set_original_tick
                                                    .update(|state| {
                                                        state
                                                            .lock()
                                                            .unwrap()
                                                            .set_selection_click(word_index, EvaluationFor::Original);
                                                    });
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
                                                                    "min-w-4 text-gray-900 underline"
                                                                } else {
                                                                    "min-w-4 text-gray-900"
                                                                }
                                                            };
                                                            return view! { <div class=class>{c.reference_char}</div> };
                                                        } else {
                                                            return view! {
                                                                <div class="relative text-gray-400 min-w-4 underline">
                                                                    {c.reference_char}
                                                                    <div class="absolute -top-0 -right-0 text-red-600 italic text-base md:text-3xl">
                                                                        <p>{c.typed_char}</p>
                                                                    </div>
                                                                </div>
                                                            };
                                                        }
                                                    }
                                                    let class = move || {
                                                        if store.get().word_index == word_index && store.get().focus
                                                        {
                                                            "min-w-4 underline"
                                                        } else {
                                                            "min-w-4"
                                                        }
                                                    };
                                                    view! { <div class=class>{c.reference_char}</div> }
                                                }
                                            />

                                            // {hightlight_index}

                                            {move || {
                                                let pair = match original_tick.get().lock().unwrap().clicked
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
                                                ) = original_tick.get().lock().unwrap().clicked
                                                {
                                                    if clicked_highligth_word_index == word_index {
                                                        return delete_button(
                                                            original_tick.get(),
                                                            clicked_highlight,
                                                        );
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
                <div class="px-8 p-5 flex flex-wrap text-4xl lg:text-3xl text-gray-500 italic">
                    <For
                        each=move || translation_words.clone().into_iter().enumerate()
                        key=move |(index, _item)| index.clone()
                        children=move |(word_index, item)| {
                            let class = move || match translation_tick
                                .get()
                                .lock()
                                .unwrap()
                                .get_state_for_word(word_index, EvaluationFor::Translation)
                            {
                                WordState::Pair(_) => {
                                    "relative flex px-2 p-1 outline-dashed bg-blue-200"
                                }
                                WordState::Highlighted(_) => "relative flex px-2 p-1 bg-red-200",
                                WordState::SelectedPair(_) => {
                                    "relative flex px-2 p-1 line-through bg-green-200"
                                }
                                WordState::HighlightedPair(_) => {
                                    "relative flex px-2 p-1 outline bg-orange-200"
                                }
                                WordState::Clicked => "relative flex px-2 p-1 underline",
                                WordState::ClickedSelected => {
                                    "relative flex px-2 p-1 underline bg-yellow-200"
                                }
                                WordState::None => "relative flex px-2 p-1",
                            };
                            view! {
                                // let hightlight_index = move || {
                                // if let Some(index) = translation_tick.get().lock().unwrap().get_pair_index_for_word_if_any(
                                // word_index,
                                // EvaluationFor::Translation,
                                // ) {
                                // view! {
                                // <div class="absolute -top-0 -right-0 text-red-600 italic text-base md:text-xl">
                                // {index}
                                // </div>
                                // }
                                // } else {
                                // view! { <div class="absolute"></div> }
                                // }
                                // };
                                <div
                                    class=class
                                    on:click=move |_| {
                                        set_translation_tick
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
                                >

                                    {item}
                                    // {hightlight_index}

                                    {move || {
                                        let pair = match translation_tick
                                            .get()
                                            .lock()
                                            .unwrap()
                                            .clicked
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
                                        ) = translation_tick.get().lock().unwrap().clicked
                                        {
                                            if clicked_highligth_word_index == word_index {
                                                return delete_button(
                                                    translation_tick.get(),
                                                    clicked_highlight,
                                                );
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
        </div>
    }
}
