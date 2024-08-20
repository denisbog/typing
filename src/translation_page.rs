use crate::application_types::Pair;
use crate::components::Association;
use crate::translation::delete_article;
use crate::translation::store_pairs;
use crate::TypePairs;
use crate::BUTTON_CLASS;
use crate::{application_types::Data, components::Sentance};
use leptos::*;
use leptos_router::use_params;
use leptos_router::Params;

#[derive(Params, PartialEq)]
pub struct ArticleParams {
    id: Option<usize>,
}

#[component]
pub fn TranslationPage(data: ReadSignal<Data>, set_data: WriteSignal<Data>) -> impl IntoView {
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
                            <div class="grid grid-cols-2 lg:grid-cols-8 gap-4">

                                {item
                                    .paragraphs
                                    .iter()
                                    .filter(|paragraph| paragraph.pairs.is_some())
                                    .map(|paragraph| {
                                        let words_original = paragraph
                                            .original
                                            .split(" ")
                                            .map(str::to_string)
                                            .collect::<Vec<String>>();
                                        if let Some(translation) = &paragraph.translation {
                                            let words_translation = translation
                                                .split(" ")
                                                .map(str::to_string)
                                                .collect::<Vec<String>>();
                                            paragraph
                                                .pairs
                                                .clone()
                                                .unwrap()
                                                .iter()
                                                .map(|pair| {
                                                    let pair_original = pair
                                                        .original
                                                        .iter()
                                                        .map(|index| { words_original[*index].clone() })
                                                        .map(|word| {
                                                            view! { <div class="flex p-1 italic">{word}</div> }
                                                        })
                                                        .collect_view();
                                                    let pair_translated = pair
                                                        .translation
                                                        .iter()
                                                        .map(|index| { words_translation[*index].clone() })
                                                        .map(|word| {
                                                            view! { <div class="flex p-1 italic">{word}</div> }
                                                        })
                                                        .collect_view();
                                                    view! {
                                                        <div class="flex justify-end text-gray-500">
                                                            {pair_original}
                                                        </div>
                                                        <div class="flex text-green-700">{pair_translated}</div>
                                                    }
                                                })
                                                .collect_view()
                                        } else {
                                            view! { "no translation available" }.into_view()
                                        }
                                    })
                                    .collect_view()}

                            </div>
                        </div>
                        <div
                            class=BUTTON_CLASS
                            on:click=move |_event| {
                                spawn_local(async move {
                                    let article_to_remove = data
                                        .get_untracked()
                                        .articles
                                        .get(index)
                                        .unwrap()
                                        .clone();
                                    let _ = store_pairs(article_to_remove).await;
                                });
                            }
                        >

                            Save Pairs
                        </div>
                        <div
                            class=BUTTON_CLASS
                            on:click=move |_event| {
                                let article_to_remove = data
                                    .get_untracked()
                                    .articles
                                    .get(index)
                                    .unwrap()
                                    .clone();
                                spawn_local(async move {
                                    delete_article(article_to_remove).await.unwrap();
                                });
                                set_data
                                    .update(|item| {
                                        item.articles.remove(index);
                                    });
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
pub fn ArticlePage(data: ReadSignal<Data>, set_data: WriteSignal<Data>) -> impl IntoView {
    let params = use_params::<ArticleParams>();
    let article_id = params.with(|param| param.as_ref().unwrap().id).unwrap();

    let (pairs, set_pairs) = create_signal(TypePairs::new());
    logging::log!("init pairs");
    set_pairs.update(|state| {
        if let Some(article) = data.get().articles.get(article_id) {
            article
                .clone()
                .paragraphs
                .into_iter()
                .enumerate()
                .for_each(|(index, paragraph)| {
                    if paragraph.pairs.is_some() {
                        let associations = paragraph
                            .pairs
                            .unwrap()
                            .into_iter()
                            .map(|pair| Association {
                                start_position: pair.original[0],
                                original: pair.original.into_iter().collect(),
                                translation: pair.translation.into_iter().collect(),
                            })
                            .collect();

                        logging::log!("inserted associations: {:?}", associations);
                        state.insert(index, associations);
                    }
                });
        }
    });

    let on_back = move || {
        logging::log!("pairs updated");
        set_data.update(|state| {
            if let Some(article) = state.articles.get_mut(article_id) {
                pairs.get().into_iter().for_each(|(key, value)| {
                    logging::log!("key: {}", key);
                    let pairs = value
                        .into_iter()
                        .map(|item| {
                            let original: Vec<usize> = item.original.into_iter().collect();
                            let translation: Vec<usize> = item.translation.into_iter().collect();
                            Pair {
                                original,
                                translation,
                            }
                        })
                        .collect();
                    article.paragraphs[key].pairs = Some(pairs);
                });
            }
        });
        let navigate = leptos_router::use_navigate();
        navigate("/", Default::default());
    };

    let views = move || {
        if let Some(article) = data.get().articles.get(article_id) {
            let total = article.paragraphs.len();
            let link = article
                .paragraphs
                .clone()
                .into_iter()
                .enumerate()
                .map(|(index, _item)| {
                    view! {
                        <a class="pl-1" href=format!("#{}", index + 1)>
                            {index + 1}
                        </a>
                    }
                })
                .collect_view();
            let paragraphs = article
                .paragraphs
                .clone()
                .into_iter()
                .enumerate()
                .map(|(index, item)| {
                    view! { <Sentance paragraph=item index total pairs set_pairs/> }
                })
                .collect_view();

            view! {
                {paragraphs}
                <div class="fixed bottom-2 p-2 text-gray-500 bg-gray-100 shadow-md cursor-default flex">
                    "jump: " <div class="pl-1 underline cursor-pointer" on:click=move |_| on_back()>
                        home
                    </div> {link}
                </div>
            }
            .into_view()
        } else {
            view! {}.into_view()
        }
    };
    view! { <div class="w-screen lg:w-3/4 flex flex-col ">{views}</div> }
}
