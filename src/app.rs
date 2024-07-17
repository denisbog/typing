use std::convert::Infallible;

use crate::{
    application_types::{Article, Data},
    error_template::{AppError, ErrorTemplate},
    translation::{get_data, get_translations, store_data, store_pairs, TranslationRequest},
    translation_page::{ArticlePage, TranslationPage},
    TypePairs,
};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use leptos_use::{use_cookie_with_options, utils::FromToStringCodec, UseCookieOptions};
use uuid::Uuid;

use cookie::SameSite;

use crate::BUTTON_CLASS;
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let (translation_input, set_translation_input) = create_signal("".to_string());
    let (session_id, _set_session_id) = use_cookie_with_options::<String, FromToStringCodec>(
        "session_id",
        UseCookieOptions::<String, Infallible>::default()
            .same_site(SameSite::None)
            //.default_value(Some(Uuid::new_v4().to_string())),
            .default_value(Some("session-id".to_string())),
    );

    let (translation_post, set_translation_post) = create_signal(Data::default());
    let (pairs, set_pairs) = create_signal(TypePairs::new());
    if let Some(session) = session_id.get() {
        spawn_local(async move {
            #[cfg(feature = "hydrate")]
            set_pairs.set(
                crate::translation::get_pairs(session.clone())
                    .await
                    .unwrap(),
            );
            set_translation_post.set(get_data(session).await.unwrap());
        });
    }
    let (input_popup, set_input_popup) = create_signal(false);

    let input_popup_component = move || {
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
                                            value="Translate article"
                                            on:click=move |_event| {
                                                let temp = translation_input.get();
                                                logging::log!("passing argument: {}", temp);
                                                set_input_popup.set(false);
                                                spawn_local(async move {
                                                    let request = TranslationRequest::from_str(&temp);
                                                    let response = get_translations(request.clone())
                                                        .await
                                                        .unwrap();
                                                    set_translation_post
                                                        .update(|data| {
                                                            data.articles
                                                                .push(Article::from_pair(request.src, response.translated));
                                                        });
                                                    let _response = store_data(
                                                            session_id.get().unwrap(),
                                                            translation_post.get_untracked(),
                                                        )
                                                        .await
                                                        .unwrap();
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
            <div class="p-3 pt-7 text-xl lg:text-3xl font-bold text-gray-100 font-mono w-screen justify-center flex snap-start">
                <a href="/">
                    <div>Learn German by typing!</div>
                </a>
            </div>
            <div class="flex justify-center">
                <div class=BUTTON_CLASS>
                    <div on:click=move |_event| set_input_popup.set(true)>Add Article</div>
                </div>
                <div
                    class=BUTTON_CLASS
                    on:click=move |_event| {
                        spawn_local(async move {
                            logging::log!("store pairs {:?}", pairs.get());
                            let _ = store_pairs(session_id.get().unwrap(), pairs.get()).await;
                        });
                    }
                >

                    Save Pairs
                </div>
            </div>
            <main class="w-screen flex flex-col items-center">
                <Routes>
                    <Route
                        path=""
                        view=move || {
                            view! {
                                <TranslationPage
                                    data=translation_post
                                    set_data=set_translation_post
                                    pairs
                                    set_pairs
                                />
                            }
                        }
                    />

                    <Route
                        path="/article/:id"
                        view=move || {
                            view! {
                                <ArticlePage
                                    data=translation_post
                                    set_data=set_translation_post
                                    pairs
                                    set_pairs
                                />
                            }
                        }
                    />

                </Routes>
                <div>{move || input_popup_component}</div>
            </main>
        </Router>
    }
}
