use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}

#[derive(Clone, PartialEq)]
struct CharState {
    char_index: usize,
    reference_char: char,
    typed_char: Option<char>,
}

impl CharState {
    fn new(char_index: usize, reference_char: char) -> Self {
        CharState {
            char_index,
            reference_char,
            typed_char: None,
        }
    }
    fn typed(&mut self, typed_char: char) {
        self.typed_char = Some(typed_char);
    }
    fn backspace(&mut self) {
        self.typed_char = None
    }
}
#[derive(Clone)]
struct WordState {
    char_index: usize,
    data: Vec<CharState>,
}
#[derive(Clone)]
struct TypeState {
    word_index: usize,
    data: Vec<WordState>,
}

impl TypeState {
    fn from_str(value: &'static str) -> Self {
        TypeState {
            word_index: 0,
            data: value
                .split(' ')
                .map(|part| WordState {
                    char_index: 0,
                    data: part
                        .chars()
                        .enumerate()
                        .map(|(index, reference_char)| CharState::new(index, reference_char))
                        .collect(),
                })
                .collect(),
        }
    }
}
#[component]
fn App() -> impl IntoView {
    let (store, set_store) = create_signal(TypeState::from_str("Dass überhaupt noch Euro und Dollar in Moskau landen und gegen Rubel getauscht werden können, ist dabei wenig überraschend. Russland erwirtschaftet Jahr für Jahr hohe Überschüsse durch den Verkauf von Öl, Gas und anderen Rohstoffen. Viele davon werden auch nach Kriegsbeginn noch in westlichen Währungen bezahlt, auch wenn der Anteil asiatischer Währungen wie des chinesischen Yuan steigt. Russlands Exporteure also haben Dollar und Euro, während andere Firmen im Ausland dringend benötigte Einfuhren kaufen müssen. Auch von diesen müssen viele immer noch in Euro und Dollar bezahlt werden. Grob vereinfacht gesagt: Russlands Rohstoffexporteure verkaufen in Moskau ihre Dollars, die Importeure wiederum kaufen sie dort. Einer der zentralen Handelspunkte für solche Geschäfte war bislang: die Moskauer Börse."));
    view! {
        <div
            class="w-screen p-10 flex flex-wrap text-2xl text-gray-500 focus:bg-gray-300 font-mono"
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
                    local_store.word_index += 1;
                    set_store(local_store);
                }
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
                            logging::log!("inserting {}", char::from_u32(key).unwrap());
                            word.data
                                .get_mut(word.char_index)
                                .unwrap()
                                .typed(char::from_u32(key).unwrap());
                            word.char_index += 1;
                            set_store(local_store);
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
                        key=|(index, c)| { format!("{}-{}", index, c.char_index) }

                        children=move |(_index, c)| {
                            view! {
                                <div class="flex px-2 py-1">
                                    <For
                                        each=move || c.clone().data.into_iter().enumerate()
                                        key=|(index, c)| {
                                            format!("{}-{}", index.clone(), c.typed_char.unwrap_or('~'))
                                        }

                                        children=move |(_index, c)| {
                                            if let Some(typed_char) = c.typed_char {
                                                if compare(typed_char, c.reference_char) {
                                                    return view! {
                                                        <div class="min-w-4 text-gray-900">{c.reference_char}</div>
                                                    };
                                                } else {
                                                    return view! {
                                                        <div class="relative text-gray-400 min-w-4">
                                                            {c.reference_char}
                                                            <div class="absolute -top-0 -right-0 text-red-600 italic text-base">
                                                                <p>{c.typed_char}</p>
                                                            </div>
                                                        </div>
                                                    };
                                                }
                                            }
                                            view! {
                                                <div class="min-w-4">{c.reference_char} {c.typed_char}</div>
                                            }
                                        }
                                    />

                                </div>
                            }
                        }
                    />
                }
            }

        </div>
    }
}

fn compare(t: char, r: char) -> bool {
    if t == r.to_lowercase().next().unwrap() {
        true
    } else if t == r {
        true
    } else if t == 'S' && r == 'ß' {
        true
    } else if t == 'U' && r == 'Ü' {
        true
    } else if t == 'A' && r == 'Ä' {
        true
    } else if t == 'O' && r == 'Ö' {
        true
    } else if t == 's' && r == 'ß' {
        true
    } else if t == 'u' && r == 'ü' {
        true
    } else if t == 'a' && r == 'ä' {
        true
    } else if t == 'o' && r == 'ö' {
        true
    } else if t == 's' && r == 'ß' {
        true
    } else if t == 'u' && r == 'Ü' {
        true
    } else if t == 'a' && r == 'Ä' {
        true
    } else if t == 'o' && r == 'Ö' {
        true
    } else {
        false
    }
}
