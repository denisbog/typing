use leptos::*;

use crate::types::TypeState;
use crate::utils::compare;
#[component]
pub fn Sentance(text: &'static str, translation: &'static str) -> impl IntoView {
    let (store, set_store) = create_signal(TypeState::from_str(text));
    view! {
        <div
            class="w-screen p-3 flex flex-wrap text-2xl md:text-4xl text-gray-500 focus:bg-gray-300 font-mono"
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

            on:focus=move |_event| {
                logging::log!("focus gained");
                set_store.update(|store| store.focus = true)
            }

            on:focusout=move |_event| {
                logging::log!("focus lost");
                set_store.update(|store| store.focus = false)
            }

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
                let current_word = move |index| index == store.get_untracked().word_index;
                let focus = move || store.get_untracked().focus;
                view! {
                    <For
                        each=move || store.get().data.into_iter().enumerate()
                        key=move |(index, c)| {
                            let marker = if current_word(*index) { "selected" } else { "" };
                            format!("{}-{}-{}-{}", index, c.char_index, marker, focus())
                        }

                        children=move |(word_index, c)| {
                            view! {
                                <div class="flex px-2 py-1">
                                    <For
                                        each=move || c.clone().data.into_iter().enumerate()
                                        key=|(index, c)| {
                                            format!("{}-{}", index, c.typed_char.unwrap_or('~'))
                                        }

                                        children=move |(_index, c)| {
                                            if let Some(typed_char) = c.typed_char {
                                                if compare(typed_char, c.reference_char) {
                                                    let class = if current_word(word_index) {
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
                                            let class = if current_word(word_index) && focus() {
                                                "min-w-4 underline"
                                            } else {
                                                "min-w-4"
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
        <div class="w-screen px-8 p-5 flex flex-wrap text-2xl md:text-4xl text-gray-500 italic">
            {translation}
        </div>
    }
}
