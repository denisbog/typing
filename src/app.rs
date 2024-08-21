use std::convert::Infallible;

use crate::{
    application_types::{Article, Data},
    error_template::{AppError, ErrorTemplate},
    translation::{get_data, store_article},
    translation_page::{ArticlePage, TranslationPage},
};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use leptos_use::{use_cookie_with_options, UseCookieOptions};
use uuid::Uuid;

use codee::string::FromToStringCodec;

use cookie::SameSite;

use crate::BUTTON_CLASS;
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let (translation_input, set_translation_input) = create_signal("".to_string());
    let (session_id, _set_session_id) = use_cookie_with_options::<String, FromToStringCodec>(
        "session_id",
        UseCookieOptions::<String, (), Infallible>::default()
            .same_site(SameSite::None)
            .default_value(Some("session_id".to_string())), //.default_value(Some(Uuid::new_v4().to_string())),
    );

    let (translation_post, set_translation_post) = create_signal(Data::default());
    let (input_popup, set_input_popup) = create_signal(false);
    let (is_trigger, trigger) = create_signal(false);
    let resource = create_resource(
        move || (session_id.get(), is_trigger.get()),
        |(session, _)| async { get_data(session.unwrap()).await.unwrap() },
    );

    #[cfg(feature = "hydrate")]
    spawn_local(async move {
        resource.get().map(|data| {
            set_translation_post.set(data);
        });
    });

    let input_popup_component = move |set_translation_post: WriteSignal<Data>| {
        if input_popup.get() {
            view! {
                <div class="fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity">
                    <div class="fixed inset-1 z-10 w-screen overflow-y-auto">
                        <div class="flex min-h-full items-end justify-center text-center sm:items-center sm:p-0 lg:p-5">
                            <div class="flex relative transform overflow-hidden bg-gray-100 shadow-xl transition-all w-full h-full">
                                <div class="flex flex-1 flex-col bg-white px-4 pb-4 pt-5 sm:p-6 sm:pb-4">
                                    <textarea
                                        class="h-80"
                                        type="textarea"
                                        placeholder="type here your text"
                                        prop:value=translation_input
                                        on:input=move |event| {
                                            set_translation_input.set(event_target_value(&event));
                                        }
                                    >
                                    </textarea>
                                    <div class="p-2">
                                        <input
                                            class="p-2 m-1 shadow-md rounded bg-green-100"
                                            type="button"
                                            value="Add article"
                                            on:click=move |_event| {
                                                let temp = translation_input.get();
                                                logging::log!("passing argument: {}", temp);
                                                set_input_popup.set(false);
                                                spawn_local(async move {
                                                    let paragraphs = temp
                                                        .split("\n")
                                                        .map(str::to_string)
                                                        .collect::<Vec<String>>();
                                                    let article = Article::from_str(
                                                        session_id.get().unwrap(),
                                                        paragraphs,
                                                    );
                                                    set_translation_post
                                                        .update(|data| {
                                                            data.articles.push(article.clone());
                                                        });
                                                    store_article(article).await.unwrap();
                                                    //trigger.update(|current| { *current = !*current });
                                                });
                                            }
                                        />

                                        <input
                                            class="p-2 m-1 shadow-md rounded bg-gray-100 text-gray-700"
                                            type="button"
                                            value="Close"
                                            on:click=move |_event| {
                                                set_input_popup.set(false);
                                            }
                                        />

                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            }.into_view()
        } else {
            view! {}.into_view()
        }
    };

    view! {
        <Html class="snap-y snap-y-mandatory"/>

        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/typing.css"/>

        // sets the document title
        <Title text="Typing app"/>

        <Body class="h-screen bg-gray-400 text-gray-900"/>
        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors/> }.into_view()
        }>
            <main class="w-screen flex flex-col items-center">
                <Routes>
                    <Route
                        path=""
                        view=move || {
                            view! {
                                <div
                                    id="top"
                                    class="p-3 pt-7 text-xl lg:text-3xl font-bold text-gray-100 font-mono w-screen justify-center flex snap-start"
                                >
                                    <a href="/">
                                        <div>Lernen durch Tippen!</div>
                                    </a>
                                </div>
                                <div class="flex justify-center">
                                    <div class=BUTTON_CLASS>
                                        <div on:click=move |_event| {
                                            set_input_popup.set(true)
                                        }>Add Article</div>
                                    </div>
                                </div>
                                <Suspense fallback=move || {
                                    view! { <div>Loading...</div> }
                                }>
                                    {move || {
                                        resource
                                            .get()
                                            .map(|_data| {
                                                view! {
                                                    <TranslationPage
                                                        data=translation_post
                                                        set_data=set_translation_post
                                                    />
                                                    <div>
                                                        {move || input_popup_component(set_translation_post)}
                                                    </div>
                                                }
                                            })
                                    }}

                                </Suspense>
                            }
                        }
                    />

                    <Route
                        path="/article/:id"
                        view=move || {
                            view! {
                                <Suspense fallback=move || {
                                    view! { <div>Loading...</div> }
                                }>
                                    {move || {
                                        resource
                                            .get()
                                            .map(|_data| {
                                                view! {
                                                    <ArticlePage
                                                        data=translation_post
                                                        set_data=set_translation_post
                                                    />
                                                }
                                            })
                                    }}

                                </Suspense>
                            }
                        }
                    />

                </Routes>
            </main>
        </Router>
    }
}
