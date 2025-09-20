use std::ops::Sub;
use std::time::Duration;

use crate::application_types::Pair;
use crate::components::Association;
use crate::translation::delete_article;
use crate::translation::store_pairs;
use crate::TypePairs;
use crate::BUTTON_CLASS;
use crate::{application_types::Data, components::Sentance};
use leptos::either::Either;
use leptos::html::Div;
use leptos::logging::log;
use leptos::task::spawn_local;
use leptos_animation::easing;
use leptos_animation::AnimatedSignal;
use leptos_animation::AnimationContext;
use leptos_animation::AnimationMode;
use leptos_animation::AnimationTarget;
use leptos_router::hooks::use_params;
use leptos_router::params::Params;

use leptos::prelude::*;

#[derive(Params, PartialEq)]
pub struct ArticleParams {
    id: Option<usize>,
}

#[derive(Clone)]
struct EffectPosition {
    x: f64,
    y: f64,
}

impl Sub for EffectPosition {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        EffectPosition {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[component]
pub fn TranslationPage(data: ReadSignal<Data>, set_data: WriteSignal<Data>) -> impl IntoView {
    let views = move || {
        {
            data.get()
            .articles
            .clone()
            .into_iter()
            .enumerate()
            .map(move |(index, item)| {
                view! {
                    <div class="flex p-2 snap-start">
                        <div class="flex flex-col w-full">
                            <a class="flex w-full flex-row" href=format!("/article/{}", index)>
                                <span class="flex p-1 m-1 bg-zinc-900 min-w-[40px] font-mono text-gray-500 rounded shadow-md justify-center">
                                    {item.paragraphs.len()}
                                </span>
                                <span class="flex">{item.title}</span>
                            </a>
                            <div class="grid grid-cols-2 lg:grid-cols-8">
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
                                            Either::Left(
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
                                                    .collect_view(),
                                            )
                                        } else {
                                            Either::Right(view! { "no translation available" })
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
            }).collect_view()
        }
    };
    view! { <div class="w-screen lg:w-3/4 flex flex-col">{views}</div> }
}

#[component]
pub fn ArticlePage(data: ReadSignal<Data>, set_data: WriteSignal<Data>) -> impl IntoView {
    let params = use_params::<ArticleParams>();
    let article_id = params.with(|param| param.as_ref().unwrap().id).unwrap();
    log!("render article");
    let on_back = move |pairs: ReadSignal<TypePairs>| {
        log!("pairs updated");
        set_data.update(|state| {
            if let Some(article) = state.articles.get_mut(article_id) {
                pairs.get().into_iter().for_each(|(key, value)| {
                    log!("key: {}", key);
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
    };

    AnimationContext::provide();

    let div_ref = NodeRef::<Div>::new();
    let (coordinates, set_coordinates) = signal(EffectPosition { x: 0.0, y: 0.0 });
    Effect::new(move |_| {
        if let Some(div) = div_ref.get() {
            // Cast NodeRef to web_sys::Element
            let element = div;
            // Get bounding client rect
            let rect = element.get_bounding_client_rect();
            let x = rect.x();
            let y = rect.y();
            // Get scroll offsets for document coordinates
            let doc_x = x + window().scroll_x().unwrap_or(0.0);
            let doc_y = y + window().scroll_y().unwrap_or(0.0);
            set_coordinates.set(EffectPosition { x: doc_x, y: doc_y });
        }
    });

    let views = move || {
        let animated_value = AnimatedSignal::new(
            move || AnimationTarget::<EffectPosition> {
                target: coordinates.get().into(),
                duration: Duration::from_secs_f64(0.25),
                easing: easing::CUBIC_OUT,
                mode: AnimationMode::Start,
            },
            |from, to, progress| EffectPosition {
                x: (to.x - from.x) * progress + from.x,
                y: (to.y - from.y) * progress + from.y,
            },
        );
        let caret_position = move || {
            let temp = animated_value.read();
            format!("left: {}px; top:{}px", temp.x, temp.y)
        };
        if let Some(article) = data.get().articles.get(article_id) {
            let mut article_pairs = TypePairs::new();
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

                        log!("inserted associations: {:?}", associations);
                        article_pairs.insert(index, associations);
                    }
                });

            let (pairs, set_pairs) = signal(article_pairs);

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
                    view! { <Sentance paragraph=item index total pairs set_pairs div_ref/> }
                })
                .collect_view();
            Either::Left(view! {
                {paragraphs}
                <div class="fixed bottom-2 p-2 bg-zinc-900 shadow-md cursor-default flex">
                    "jump: "
                    <div class="pl-1 underline cursor-pointer" on:click=move |_| on_back(pairs)>
                        <a href="/">home</a>
                    </div> {link}
                </div>
                <div class="absolute animate-blink z-20" style=caret_position>
                    <span class="text-xl lg:text-3xl font-extrabold font-mono text-yellow-500 font-bold">
                        _
                    </span>
                </div>
            })
        } else {
            Either::Right(())
        }
    };
    view! { <div class="w-screen lg:w-3/4 flex flex-col ">{views}</div> }
}
