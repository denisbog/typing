use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}

#[derive(Clone, PartialEq)]
struct CharState {
    index: usize,
    reference_char: char,
    typed_char: Option<char>,
}

impl CharState {
    fn new(index: usize, reference_char: char) -> Self {
        CharState {
            index,
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
struct TypeState {
    index: usize,
    data: Vec<CharState>,
}

impl TypeState {
    fn from_str(value: &'static str) -> Self {
        TypeState {
            index: 0,
            data: value
                .chars()
                .enumerate()
                .map(|(index, reference_char)| CharState::new(index, reference_char))
                .collect(),
        }
    }
}
#[component]
fn App() -> impl IntoView {
    let (store, set_store) = create_signal(TypeState::from_str("Mit seinem Nein zu Koalitionen."));
    view! {
        <div
            class="board auto flex"
            tabindex=1
            on:keydown=move |event| {
                let key = event.key_code();
                let mut local_store = store.get();
                logging::log!("current index {}", local_store.index);
                if key == 8 && local_store.index > 0 {
                    local_store.index -= 1;
                    let temp = local_store.data.get_mut(local_store.index).unwrap();
                    temp.backspace();
                    set_store(local_store);
                } else if key == 32 && local_store.index < local_store.data.len() {
                    let temp = local_store.data.get_mut(local_store.index).unwrap();
                    temp.typed(char::from_u32(key).unwrap());
                    local_store.index += 1;
                    set_store(local_store);
                }
            }

            on:keypress=move |event| {
                let key = event.key_code();
                match key {
                    (64..=93) | (97..=122) => {
                        let mut local_store = store.get();
                        if local_store.index < local_store.data.len() {
                            logging::log!("current index {}", local_store.index);
                            let temp = local_store.data.get_mut(local_store.index).unwrap();
                            temp.typed(char::from_u32(key).unwrap());
                            local_store.index += 1;
                            set_store(local_store);
                        }
                    }
                    _ => {}
                };
                logging::log!("{:?}", & key);
            }
        >

            {
                view! {
                    <For
                        each=move || store.get().data.into_iter().enumerate()
                        key=|(index, c)| {
                            format!("{}-{}", index.clone(), c.typed_char.unwrap_or('-'))
                        }

                        children=move |(_index, c)| {
                            if let Some(typed_char) = c.typed_char {
                                if typed_char == c.reference_char {
                                    return view! {
                                        <div class="p-2 m-1 shadow-lg">{c.reference_char}</div>
                                    };
                                } else {
                                    return view! {
                                        <div class="p-2 m-1 shadow-lg relative text-gray-400">
                                            {c.reference_char}
                                            <div class="absolute -top-2 -right-1 text-red-600">
                                                <p>{c.typed_char}</p>
                                            </div>
                                        </div>
                                    };
                                }
                            }
                            view! {
                                <div class="p-2 m-1 shadow-lg text-amber-600">
                                    {c.reference_char} {c.typed_char}
                                </div>
                            }
                        }
                    />
                }
            }

        </div>
    }
}
