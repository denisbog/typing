use std::collections::HashSet;

use leptos::*;

use crate::types::TypeState;
use crate::utils::compare;
#[component]
pub fn Popup(
    text: &'static str,
    translation: &'static str,
    display: Option<WriteSignal<Option<(&'static str, &'static str)>>>,
) -> impl IntoView {
    let (store, set_store) = create_signal(TypeState::from_str(text));
    let (pair, set_pair) = create_signal(true);
    let (translation_selected, set_translation_selected) = create_signal(HashSet::<usize>::new());
    let (original_selected, set_original_selected) = create_signal(HashSet::<usize>::new());

    let (pairs, set_pairs) = create_signal(HashSet::<(HashSet<usize>, HashSet<usize>)>::new());
    let insertPairf = move |a: HashSet<usize>, b: HashSet<usize>| {
        logging::log!("inserting new pari");
    };

    let pair_button = move || {
        if pair() {
            view! {
                <input
                    type="button"
                    value="pair"
                    on:click={
                        logging::log!("pairing {:?}", original_selected.get_untracked());
                        logging::log!("pairing {:?}", translation_selected.get_untracked());
                        set_original_selected
                            .update(|p| {
                                p.clear();
                            });
                        set_translation_selected
                            .update(|p| {
                                p.clear();
                            });
                        move |_| set_pair.set(false)
                    }
                />
            }
        } else {
            view! { <input type="button" value="done" on:click=move |_| set_pair.set(true)/> }
        }
    };

    let translation_words: Vec<&str> = translation.split(" ").collect();
    view! {
        {pair_button}
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
                        key=move |(index, c)| {
                            format!("{}-{}", index, c.char_index)
                        }

                        children=move |(word_index, c)| {
                            let class = move || {
                                if original_selected.get().contains(&word_index) {
                                    "flex px-2 py-1 underline"
                                } else {
                                    "flex px-2 py-1"
                                }
                            };
                            view! {
                                <div
                                    class=class
                                    on:click=move |_| {
                                        logging::log!("selected");
                                        if original_selected.get_untracked().contains(&word_index) {
                                            set_original_selected
                                                .update(|data| {
                                                    data.remove(&word_index);
                                                });
                                        } else {
                                            set_original_selected
                                                .update(|data| {
                                                    data.insert(word_index);
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
                                                    let class = move || if store.get().word_index ==word_index {
                                                        "min-w-4 text-gray-900 underline"
                                                    } else {
                                                        "min-w-4 text-gray-900"
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
                                                if store.get().word_index == word_index && store.get().focus {
                                                    "min-w-4 underline"
                                                } else {
                                                    "min-w-4"
                                                }
                                            };
                                            view! { <div class=class>{c.reference_char}</div> }
                                        }
                                    />

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
                        if translation_selected.get().contains(&index) {
                            "p-1 underline"
                        } else {
                            "p-1"
                        }
                    };
                    view! {
                        <div
                            class=class
                            on:click=move |_| {
                                if translation_selected.get_untracked().contains(&index) {
                                    set_translation_selected
                                        .update(|data| {
                                            data.remove(&index);
                                        });
                                } else {
                                    set_translation_selected
                                        .update(|data| {
                                            data.insert(index);
                                        });
                                }
                            }
                        >

                            {item}
                        </div>
                    }
                }
            />

        </div>
    }
}
