use std::collections::BTreeSet;
use std::collections::HashSet;
use std::hash::Hash;

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
#[derive(Clone)]
enum Clicked {
    Original(usize),
    Translation(usize),
    None,
}
#[derive(Clone)]
enum ClickedHeighlight {
    SelectedOriginal(usize, usize),
    SelectedTranslation(usize, usize),
    None,
}
enum EvaluationFor {
    Original,
    Translation,
}
#[component]
pub fn Sentance(text: String, translation: String) -> impl IntoView {
    let (pair, set_pair) = create_signal(false);
    let (original_selected, set_original_selected) = create_signal(BTreeSet::<usize>::new());
    let (translation_selected, set_translation_selected) = create_signal(HashSet::<usize>::new());

    let (pairs, set_pairs) = create_signal(BTreeSet::<Association>::new());

    let (clicked, set_clicked) = create_signal(Clicked::None);
    let (clicked_highlight, set_clicked_highlight) = create_signal(ClickedHeighlight::None);

    let pair_button = move || {
        if pair() {
            view! {
                <div class="snap-start">
                    <div
                        class="absolute -top-2 -right-2 italic text-base underline md:text-xl cursor-pointer z-10"
                        on:click=move |_event| {
                            logging::log!("current pairs {:?}", pairs.get_untracked());
                            set_pairs
                                .update(|item| {
                                    if !original_selected.get_untracked().is_empty()
                                        && !translation_selected.get_untracked().is_empty()
                                    {
                                        item.insert(
                                            Association::new(
                                                original_selected.get_untracked(),
                                                translation_selected.get_untracked(),
                                            ),
                                        );
                                    }
                                });
                            set_original_selected
                                .update(|p| {
                                    p.clear();
                                });
                            set_translation_selected
                                .update(|p| {
                                    p.clear();
                                });
                            set_pair.set(false);
                            set_clicked_highlight.set(ClickedHeighlight::None);
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
                    class="absolute -top-2 -right-2 italic text-base underline md:text-xl cursor-pointer z-10"
                    on:click=move |_event| {
                        set_clicked_highlight.set(ClickedHeighlight::None);
                        set_pairs
                            .update(|item| {
                                let selected = item.iter().nth(pair_to_remove).unwrap().clone();
                                item.remove(&selected);
                            });
                    }
                >

                    remove
                </div>

            </div>
        }
            .into_view()
    };
    let update_pair = move || {
        if !original_selected.get_untracked().is_empty()
            && !translation_selected.get_untracked().is_empty()
        {
            set_pair.set(true);
        } else {
            set_pair.set(false);
        }
    };
    let highlight_index = move |index: usize, evaluation_for: EvaluationFor| -> Option<usize> {
        match evaluation_for {
            EvaluationFor::Original => pairs
                .get()
                .iter()
                .enumerate()
                .find(|(_pair_index, item)| item.original.iter().any(|item| *item == index))
                .map_or_else(|| None, |(pair_index, _item)| Some(pair_index)),
            EvaluationFor::Translation => pairs
                .get()
                .iter()
                .enumerate()
                .find(|(_pair_index, item)| item.translation.iter().any(|item| *item == index))
                .map_or_else(|| None, |(pair_index, _item)| Some(pair_index)),
        }
    };

    let highlight_pair = move |index: usize, evaluation_for: EvaluationFor| match evaluation_for {
        EvaluationFor::Original => {
            if let ClickedHeighlight::SelectedOriginal(
                clicked_highlight,
                _clicked_highligth_word_index,
            ) = clicked_highlight.get()
            {
                if pairs
                    .get()
                    .iter()
                    .nth(clicked_highlight)
                    .unwrap()
                    .original
                    .contains(&index)
                {
                    return true;
                }
            }
            if let ClickedHeighlight::SelectedTranslation(
                clicked_highlight,
                _clicked_highligth_word_index,
            ) = clicked_highlight.get()
            {
                if pairs
                    .get()
                    .iter()
                    .nth(clicked_highlight)
                    .unwrap()
                    .original
                    .contains(&index)
                {
                    return true;
                }
            }
            return false;
        }
        EvaluationFor::Translation => {
            if let ClickedHeighlight::SelectedOriginal(
                clicked_highlight,
                _clicked_highligth_word_index,
            ) = clicked_highlight.get()
            {
                if pairs
                    .get()
                    .iter()
                    .nth(clicked_highlight)
                    .unwrap()
                    .translation
                    .contains(&index)
                {
                    return true;
                }
            }
            if let ClickedHeighlight::SelectedTranslation(
                clicked_highlight,
                _clicked_highligth_word_index,
            ) = clicked_highlight.get()
            {
                if pairs
                    .get()
                    .iter()
                    .nth(clicked_highlight)
                    .unwrap()
                    .translation
                    .contains(&index)
                {
                    return true;
                }
            }
            return false;
        }
    };
    let highlight_word = move |index: usize, evaluation_for: EvaluationFor| match evaluation_for {
        EvaluationFor::Original => pairs
            .get()
            .iter()
            .flat_map(|item| item.original.iter())
            .any(|item| *item == index),
        EvaluationFor::Translation => pairs
            .get()
            .iter()
            .flat_map(|item| item.translation.iter())
            .any(|item| *item == index),
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
                                    let class = move || {
                                        if !highlight_word(word_index, EvaluationFor::Original)
                                            && original_selected.get().contains(&word_index)
                                        {
                                            "relative flex px-2 py-1 underline"
                                        } else {
                                            "relative flex px-2 py-1"
                                        }
                                    };
                                    let highlight = move || {
                                        if highlight_pair(word_index, EvaluationFor::Original) {
                                            return "outline";
                                        }
                                        if highlight_word(word_index, EvaluationFor::Original) {
                                            "outline-dashed"
                                        } else {
                                            ""
                                        }
                                    };
                                    let hightlight_index = move || {
                                        if let Some(index) = highlight_index(
                                            word_index,
                                            EvaluationFor::Original,
                                        ) {
                                            view! {
                                                <div class="absolute -top-0 -right-0 text-red-600 italic text-base md:text-xl">
                                                    {index}
                                                </div>
                                            }
                                        } else {
                                            view! { <div class="absolute"></div> }
                                        }
                                    };
                                    let class = move || format!("{} {}", class(), highlight());
                                    view! {
                                        <div
                                            class=class
                                            on:click=move |_| {
                                                if let Some(selected_index) = highlight_index(
                                                    word_index,
                                                    EvaluationFor::Original,
                                                ) {
                                                    logging::log!("click on selection  {}", word_index);
                                                    set_clicked_highlight(
                                                        ClickedHeighlight::SelectedOriginal(
                                                            selected_index,
                                                            word_index,
                                                        ),
                                                    );
                                                }
                                                if original_selected.get_untracked().contains(&word_index) {
                                                    set_original_selected
                                                        .update(|data| {
                                                            data.remove(&word_index);
                                                        });
                                                } else {
                                                    if highlight_index(word_index, EvaluationFor::Original)
                                                        .is_none()
                                                    {
                                                        set_original_selected
                                                            .update(|data| {
                                                                data.insert(word_index);
                                                            });
                                                        set_clicked.set(Clicked::Original(word_index));
                                                    }
                                                    update_pair();
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

                                            {hightlight_index}
                                            {move || {
                                                match clicked.get() {
                                                    Clicked::Original(clicked_index) => {
                                                        if pair() && clicked_index == word_index {
                                                            pair_button.into_view()
                                                        } else {
                                                            view! {}.into_view()
                                                        }
                                                    }
                                                    _ => view! {}.into_view(),
                                                }
                                            }}

                                            {move || {
                                                if let ClickedHeighlight::SelectedOriginal(
                                                    clicked_highlight,
                                                    clicked_highligth_word_index,
                                                ) = clicked_highlight.get()
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
                <div class="px-8 p-5 flex flex-wrap text-4xl lg:text-3xl text-gray-500 italic">
                    <For
                        each=move || translation_words.clone().into_iter().enumerate()
                        key=move |(index, _item)| index.clone()
                        children=move |(word_index, item)| {
                            let class = move || {
                                if !highlight_word(word_index, EvaluationFor::Translation)
                                    && translation_selected.get().contains(&word_index)
                                {
                                    "relative flex px-2 py-1 underline"
                                } else {
                                    "relative flex px-2 py-1"
                                }
                            };
                            let highlight = move || {
                                if highlight_pair(word_index, EvaluationFor::Translation) {
                                    return "outline";
                                }
                                if highlight_word(word_index, EvaluationFor::Translation) {
                                    "outline-dashed"
                                } else {
                                    ""
                                }
                            };
                            let class = move || format!("{} {}", class(), highlight());
                            let hightlight_index = move || {
                                if let Some(index) = highlight_index(
                                    word_index,
                                    EvaluationFor::Translation,
                                ) {
                                    view! {
                                        <div class="absolute -top-0 -right-0 text-red-600 italic text-base md:text-xl">
                                            {index}
                                        </div>
                                    }
                                } else {
                                    view! { <div class="absolute"></div> }
                                }
                            };
                            view! {
                                <div
                                    class=class
                                    on:click=move |_| {
                                        if let Some(selected_index) = highlight_index(
                                            word_index,
                                            EvaluationFor::Translation,
                                        ) {
                                            logging::log!("click on selection  {}", word_index);
                                            set_clicked_highlight(
                                                ClickedHeighlight::SelectedTranslation(
                                                    selected_index,
                                                    word_index,
                                                ),
                                            );
                                        }
                                        if translation_selected
                                            .get_untracked()
                                            .contains(&word_index)
                                        {
                                            set_translation_selected
                                                .update(|data| {
                                                    data.remove(&word_index);
                                                });
                                        } else {
                                            if highlight_index(word_index, EvaluationFor::Translation)
                                                .is_none()
                                            {
                                                set_translation_selected
                                                    .update(|data| {
                                                        data.insert(word_index);
                                                    });
                                                set_clicked.set(Clicked::Translation(word_index));
                                            }
                                        };
                                        update_pair();
                                    }
                                >

                                    {item}
                                    {hightlight_index}

                                    {move || {
                                        match clicked.get() {
                                            Clicked::Translation(clicked_index) => {
                                                if pair() && clicked_index == word_index {
                                                    pair_button.into_view()
                                                } else {
                                                    view! {}.into_view()
                                                }
                                            }
                                            _ => view! {}.into_view(),
                                        }
                                    }}

                                    {move || {
                                        if let ClickedHeighlight::SelectedTranslation(
                                            clicked_highlight,
                                            clicked_highligth_word_index,
                                        ) = clicked_highlight.get()
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
        </div>
    }
}
