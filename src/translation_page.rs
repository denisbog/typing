use std::collections::HashMap;

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
    pairs: ReadSignal<HashMap<usize, HashMap<usize, Vec<(Vec<usize>, Vec<usize>)>>>>,
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
                                            .map(|(selection_original, selection_translated)| {
                                                let pair_original = selection_original
                                                    .iter()
                                                    .map(|index| { words_original[*index].clone() })
                                                    .collect::<String>();
                                                let pair_translated = selection_translated
                                                    .iter()
                                                    .map(|index| { words_translation[*index].clone() })
                                                    .collect::<String>();
                                                view! {
                                                    <div class="flex">
                                                        <div>{pair_original}</div>
                                                        <div>{pair_translated}</div>
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
                            class="w-fit text-3xl lg:text-2xl m-2 p-2 shadow-md rounded bg-gray-300 cursor-pointer"
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

pub fn update_data(data: ReadSignal<Data>, set_data: WriteSignal<Data>) -> impl IntoView {}

#[component]
pub fn ArticlePage(
    data: ReadSignal<Data>,
    set_data: WriteSignal<Data>,
    delete_article: impl Fn(usize),
    set_pairs: WriteSignal<HashMap<usize, HashMap<usize, Vec<(Vec<usize>, Vec<usize>)>>>>,
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
                view! { <Sentance text=item.original translation=item.translation article_id index set_pairs/> }
            })
            .collect_view()
        } else {
            view! {}.into_view()
        }
    };
    view! {
        <div class="w-screen lg:w-3/4 flex flex-col relative">
            <div>
                <div class="w-fit text-3xl lg:text-2xl m-2 p-2 shadow-md rounded bg-gray-300 cursor-pointer">
                    Save Pairs
                </div>
            </div>
            {views}
        </div>
    }
}
