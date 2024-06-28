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
#[component]
pub fn Popup(
    text: &'static str,
    translation: &'static str,
    display: Option<WriteSignal<Option<(&'static str, &'static str)>>>,
) -> impl IntoView {
    let (store, set_store) = create_signal(TypeState::from_str(text));
    let (pair, set_pair) = create_signal(false);
    let (original_selected, set_original_selected) = create_signal(BTreeSet::<usize>::new());
    let (translation_selected, set_translation_selected) = create_signal(HashSet::<usize>::new());

    let (pairs, set_pairs) = create_signal(BTreeSet::<Association>::new());

    let (clicked, set_clicked) = create_signal(Clicked::None);
    let (clicked_highlight, set_clicked_highlight) = create_signal(ClickedHeighlight::None);

    let pair_button = move || {
        if pair() {
            view! {
                <div>
                    <div
                        class="absolute -top-2 -right-2 italic text-base underline md:text-xl cursor-pointer z-10"
                        on:click=move |_event| {
                            logging::log!("current pairs {:?}", pairs.get_untracked());
                            set_pairs
                                .update(|item| {
                                    if original_selected.get_untracked().len() > 0
                                        && translation_selected.get_untracked().len() > 0
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
        if original_selected.get_untracked().len() > 0
            && translation_selected.get_untracked().len() > 0
        {
            set_pair.set(true);
        } else {
            set_pair.set(false);
        }
    };
    let highlight_original = move |index: usize| -> bool {
        pairs
            .get()
            .iter()
            .flat_map(|item| item.original.iter())
            .any(|item| *item == index)
    };
    let highlight_translation = move |index: usize| -> bool {
        pairs
            .get()
            .iter()
            .flat_map(|item| item.translation.iter())
            .any(|item| *item == index)
    };
    let highlight_original_index = move |index: usize| -> Option<usize> {
        pairs
            .get()
            .iter()
            .enumerate()
            .find(|(_pair_index, item)| item.original.iter().any(|item| *item == index))
            .map_or_else(|| None, |(pair_index, _item)| Some(pair_index))
    };
    let highlight_translation_index = move |index: usize| -> Option<usize> {
        pairs
            .get()
            .iter()
            .enumerate()
            .find(|(_pair_index, item)| item.translation.iter().any(|item| *item == index))
            .map_or_else(|| None, |(pair_index, _item)| Some(pair_index))
    };
    let translation_words: Vec<&str> = translation.split(" ").collect();
    view! {
        <div
            on:click=move |_| {
                if let Some(action) = display {
                    action(Some((text, translation)))
                }
            }

            class="p-3 flex flex-wrap text-4xl lg:text-3xl text-gray-500 focus:bg-gray-300 font-mono"
            tabindex=1
            on:keydown=move |event| {
                let key = event.key_code();
                let mut local_store = store.get();
                logging::log!("key down {}", key);
                let word = local_store.data.get_mut(local_store.word_index).unwrap();
                if key == 8 {
                    if word.char_index > 0 {
                        word.char_index -= 1;
                        let temp = word.data.get_mut(word.char_index).unwrap();
                        temp.backspace();
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
                    (64..=93) | (97..=122) | 44 | 45 | 46 | 58 | 59 => {
                        let mut local_store = store.get();
                        if local_store.word_index < local_store.data.len() {
                            logging::log!("current index {}", local_store.word_index);
                            logging::log!(
                                "current word index {}", local_store.data.get(local_store
                                .word_index).unwrap().char_index
                            );
                            let word = local_store.data.get_mut(local_store.word_index).unwrap();
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
                                if !highlight_original(word_index)
                                    && original_selected.get().contains(&word_index)
                                {
                                    "relative flex px-2 py-1 underline"
                                } else {
                                    "relative flex px-2 py-1"
                                }
                            };
                            let highlight = move || {
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
                                        .contains(&word_index)
                                    {
                                        return "bg-green-200";
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
                                        .contains(&word_index)
                                    {
                                        return "bg-green-200";
                                    }
                                }
                                if highlight_original(word_index) { "bg-green-100" } else { "" }
                            };
                            let hightlight_index = move || {
                                if let Some(index) = highlight_original_index(word_index) {
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
                                        if let Some(selected_index) = highlight_original_index(
                                            word_index,
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
                                            if highlight_original_index(word_index).is_none() {
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
                                                    return pair_button.into_view()
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
                key=move |&(index, _item)| index
                children=move |(index, item)| {
                    let class = move || {
                        if !highlight_translation(index)
                            && translation_selected.get().contains(&index)
                        {
                            "relative p-1 underline"
                        } else {
                            "relative p-1"
                        }
                    };
                    let highlight = move || {
                        if let ClickedHeighlight::SelectedOriginal(
                            clicked_highlight,
                            _clicked_highlight_word_index,
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
                                return "bg-green-200";
                            }
                        }
                        if let ClickedHeighlight::SelectedTranslation(
                            clicked_highlight,
                            _clicked_highlight_word_index,
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
                                return "bg-green-200";
                            }
                        }
                        if highlight_translation(index) { "bg-green-100" } else { "" }
                    };
                    let class = move || format!("{} {}", class(), highlight());
                    let hightlight_index = move || {
                        if let Some(index) = highlight_translation_index(index) {
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
                                if let Some(selected_index) = highlight_translation_index(index) {
                                    logging::log!("click on selection  {}", index);
                                    set_clicked_highlight(
                                        ClickedHeighlight::SelectedTranslation(
                                            selected_index,
                                            index,
                                        ),
                                    );
                                }
                                if translation_selected.get_untracked().contains(&index) {
                                    set_translation_selected
                                        .update(|data| {
                                            data.remove(&index);
                                        });
                                } else {
                                    if highlight_translation_index(index).is_none() {
                                        set_translation_selected
                                            .update(|data| {
                                                data.insert(index);
                                            });
                                        set_clicked.set(Clicked::Translation(index));
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
                                        if pair() && clicked_index == index {
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
                                    if clicked_highligth_word_index == index {
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
    }
}
