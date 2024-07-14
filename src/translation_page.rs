use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::components::Association;
use crate::{application_types::Data, components::Sentance};
use leptos::*;
use leptos_router::use_params;
use leptos_router::Params;

#[derive(Params, PartialEq)]
pub struct ArticleParams {
    id: Option<usize>,
}

#[component]
pub fn TranslationPage(
    data: ReadSignal<Data>,
    set_data: WriteSignal<Data>,
    pairs: ReadSignal<BTreeMap<usize, BTreeMap<usize, BTreeSet<Association>>>>,
    set_pairs: WriteSignal<BTreeMap<usize, BTreeMap<usize, BTreeSet<Association>>>>,
) -> impl IntoView {
    let params = use_params::<ArticleParams>();
    params.with(|param| {
        if let Ok(item) = param {
            logging::log!("params {:?}", item.id);
        }
    });
    let views = move || {
        data.get()
            .articles
            .clone()
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                view! {
                    <div class="flex p-2">
                        <div class="flex flex-col w-full">
                            <a class="w-full" href=format!("/article/{}", index)>
                                {item.title}
                            </a>
                            {if let Some(article_pairs) = pairs.get().get(&index) {
                                article_pairs
                                    .iter()
                                    .map(|(&paragraph_index, paragraph_selection)| {
                                        let paragraph = item
                                            .paragraphs
                                            .get(paragraph_index)
                                            .unwrap();
                                        let words_original = paragraph
                                            .original
                                            .split(" ")
                                            .map(str::to_string)
                                            .collect::<Vec<String>>();
                                        let words_translation = paragraph
                                            .translation
                                            .split(" ")
                                            .map(str::to_string)
                                            .collect::<Vec<String>>();
                                        paragraph_selection
                                            .iter()
                                            .map(|association| {
                                                let pair_original = association
                                                    .original
                                                    .iter()
                                                    .map(|index| { words_original[*index].clone() })
                                                    .map(|word| {
                                                        view! { <div class="flex p-1 italic">{word}</div> }
                                                    })
                                                    .collect_view();
                                                let pair_translated = association
                                                    .translation
                                                    .iter()
                                                    .map(|index| { words_translation[*index].clone() })
                                                    .map(|word| {
                                                        view! { <div class="flex p-1 italic">{word}</div> }
                                                    })
                                                    .collect_view();
                                                view! {
                                                    <div class="grid grid-cols-2 gap-4">
                                                        <div class="flex justify-end text-gray-500">{pair_original}</div>
                                                        <div class="flex text-green-700">{pair_translated}</div>
                                                    </div>
                                                }
                                            })
                                            .collect_view()
                                    })
                                    .collect_view()
                            } else {
                                view! {}.into_view()
                            }}

                        </div>
                        <div
                            class="flex size-fit text-3xl lg:text-2xl m-2 p-2 shadow-md rounded bg-gray-300 cursor-pointer"
                            on:click=move |_event| {
                                set_data
                                    .update(|item| {
                                        item.articles.remove(index);
                                    })
                            }
                        >

                            Delete
                        </div>
                    </div>
                }
            })
            .collect_view()
    };
    view! { <div class="w-screen lg:w-3/4 flex flex-col">{views}</div> }
}

#[component]
pub fn ArticlePage(
    data: ReadSignal<Data>,
    set_data: WriteSignal<Data>,
    pairs: ReadSignal<BTreeMap<usize, BTreeMap<usize, BTreeSet<Association>>>>,
    set_pairs: WriteSignal<BTreeMap<usize, BTreeMap<usize, BTreeSet<Association>>>>,
) -> impl IntoView {
    let params = use_params::<ArticleParams>();
    let article_id = params.with(|param| param.as_ref().unwrap().id).unwrap();

    let views = move || {
        if let Some(pargrah) = data.get().articles.get(article_id) {
            pargrah
                .paragraphs
                .clone()
                .into_iter()
                .enumerate()
                .map(|(index, item)| {
                    view! {
                        <Sentance
                            text=item.original
                            translation=item.translation
                            article_id
                            index
                            pairs
                            set_pairs
                        />
                    }
                })
                .collect_view()
        } else {
            view! {}.into_view()
        }
    };
    view! { <div class="w-screen lg:w-3/4 flex flex-col relative">{views}</div> }
}
