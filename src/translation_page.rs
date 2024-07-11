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
                        <a class="w-full" href=format!("/article/{}", index)>
                            {item.title}
                        </a>
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

#[component]
pub fn ArticlePage(data: Data, delete_article: impl Fn(usize)) -> impl IntoView {
    let params = use_params::<ArticleParams>();
    let article_id = params.with(|param| param.as_ref().unwrap().id);

    let views = move || {
        data.articles
            .get(article_id.unwrap())
            .unwrap()
            .paragraphs
            .clone()
            .into_iter()
            .map(|item| {
                view! { <Sentance text=item.original translation=item.translation/> }
            })
            .collect_view()
    };
    view! { <div class="w-screen lg:w-3/4 flex flex-col relative">{views}</div> }
}
