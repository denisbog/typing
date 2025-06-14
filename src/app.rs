use std::convert::Infallible;

use codee::string::FromToStringCodec;
use leptos::either::Either;
use leptos::logging::log;
use leptos::task::spawn_local;
use leptos_router::path;
use leptos_use::use_cookie_with_options;
use leptos_use::UseCookieOptions;

use crate::get_user_info;
use crate::parse_hash;
use crate::translation_page::TranslationPage;
use crate::{
    application_types::{Article, Data},
    translation::{get_data, store_article},
    translation_page::ArticlePage,
    BUTTON_CLASS,
};
use cookie::SameSite;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::use_location;

use leptos::prelude::*;
use leptos_meta::*;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    provide_meta_context();
    view! {
        <!DOCTYPE html>
        <html lang="en" class="snap-y snap-y-mandatory">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <Title text="Typing app"/>
                <Stylesheet id="leptos" href="/typing.css"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body class="h-screen bg-gray-400 text-gray-900"></body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (translation_input, set_translation_input) = signal("".to_string());
    let (session_id, set_session_id) = use_cookie_with_options::<String, FromToStringCodec>(
        "session_id",
        UseCookieOptions::<String, (), Infallible>::default()
            .default_value(None)
            .same_site(SameSite::None),
    );

    let (translation_post, set_translation_post) = signal(Data::default());
    let (input_popup, set_input_popup) = signal(false);
    let resource = Resource::new(
        move || session_id,
        |session| async move {
            if session.get().is_some() {
                get_data(session.get().unwrap()).await.unwrap()
            } else {
                Data::default()
            }
        },
    );
    Effect::new(move |_| {
        if let Some(data) = resource.get() {
            log!("setting data {:?}", data);
            set_translation_post.set(data);
        } else {
            log!("data not loaded");
        }
    });
    let input_popup_component = move |set_translation_post: WriteSignal<Data>| {
        if input_popup.get() {
            Either::Left(view! {
                <div class="fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity">
                    <div class="fixed inset-1 z-10 w-screen overflow-y-auto">
                        <div class="flex min-h-full items-end justify-center text-center sm:items-center sm:p-0 lg:p-5">
                            <div class="flex relative transform overflow-hidden bg-gray-100 shadow-xl transition-all w-full h-full">
                                <div class="flex flex-1 flex-col bg-white px-4 pb-4 pt-5 sm:p-6 sm:pb-4">
                                    <textarea
                                        class="h-80"
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
                                                log!("passing argument: {}", temp);
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
                                                });
                                            }
                                        />

                                        <input
                                            on:click=move |_event| {
                                                set_input_popup.set(false);
                                            }

                                            class="p-2 m-1 shadow-md rounded bg-gray-100 text-gray-700"
                                            type="button"
                                            value="Close"
                                        />

                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            }.into_view())
        } else {
            Either::Right(())
        }
    };

    view! {
        <Router>
            {move || {
                if session_id.get().is_none() {
                    let location = use_location();
                    let hash = location.hash.get();
                    if !hash.is_empty() {
                        let hash = parse_hash(hash);
                        log!("hash {:?}", hash);
                        spawn_local(async move {
                            let user_info = get_user_info(hash);
                            set_session_id.set(Some(user_info.await.sub));
                        });
                    } else {
                        #[cfg(feature = "hydrate")]
                        {
                            use leptos_router::hooks::use_navigate;
                            let navigate = use_navigate();
                            navigate("/login", Default::default());
                        }
                    }
                }
            }}
            <main class="w-screen flex flex-col items-center">
                <Routes fallback=|| "Not found!">
                    <Route
                        path=path!("/login")
                        view=move || {
                            #[cfg(feature = "hydrate")]
                            if session_id.get().is_some() {
                                {
                                    use leptos_router::hooks::use_navigate;
                                    let navigate = use_navigate();
                                    navigate("/", Default::default());
                                }
                            }
                            view! {
                                <div
                                    id="top"
                                    class="p-3 pt-7 text-xl lg:text-3xl font-bold text-gray-100 font-mono w-screen justify-center flex snap-start"
                                >
                                    <a href="/">
                                        <div>Tippen!</div>
                                    </a>
                                </div>
                                <div class="flex justify-center">
                                    <div class=BUTTON_CLASS>
                                        <div on:click=move |_event| {
                                            window()
                                                .location()
                                                .assign(
                                                    "https://typing.auth.us-east-1.amazoncognito.com/oauth2/authorize?client_id=2n9mqgc2vfhharda4r547sdcpm&response_type=token&scope=email+openid&redirect_uri=https%3A%2F%2Fxfwvamzfhhd4wjc766gmj6qsza0pcjeq.lambda-url.us-east-1.on.aws%2F",
                                                )
                                                .unwrap();
                                        }>Login</div>
                                    </div>
                                </div>
                            }
                        }
                    />

                    <Route
                        path=path!("")
                        view=move || {
                            view! {
                                <div
                                    id="top"
                                    class="p-3 pt-7 text-xl lg:text-3xl font-bold text-gray-100 font-mono w-screen justify-center flex snap-start"
                                >
                                    <a href="/">
                                        <div>Tippen!</div>
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
                                                    <div>{input_popup_component(set_translation_post)}</div>
                                                }
                                            })
                                    }}

                                </Suspense>
                            }
                        }
                    />

                    <Route
                        path=path!("/article/:id")
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
