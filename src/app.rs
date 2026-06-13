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
use crate::translation_page::PlaybackState;
use crate::{
    application_types::{Article, Data},
    translation::{get_data, store_article},
    translation_page::ArticlePage,
    BUTTON_CLASS,
};
use cookie::SameSite;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::use_location;

#[cfg(feature = "hydrate")]
use web_sys::HtmlAudioElement;

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
            <body class="h-screen bg-zinc-950 text-gray-400"></body>
        </html>
    }
}

#[cfg(feature = "hydrate")]
#[component]
fn PlaybackPanel(playback: PlaybackState) -> impl IntoView {
    view! {
        <Show
            when=move || playback.article_title.get().is_some()
            fallback=move || view! { <div class="hidden"></div> }
        >
            <div class="fixed bottom-2 right-2 p-3 bg-zinc-900 shadow-md rounded flex gap-2 items-center">
                <div class="text-sm flex flex-col">
                    <div>{move || playback.article_title.get().unwrap_or_default()}</div>
                    <div>{move || playback.paragraph.get().map(|item| format!("#{}", item + 1)).unwrap_or_default()}</div>
                    <div>{move || playback.selected_voice.get().map(|voice| format!("voice: {}", voice)).unwrap_or_default()}</div>
                </div>
                <Show
                    when=move || playback.article_index.get().is_some() && playback.paragraph.get().is_some()
                    fallback=move || view! { <div class="hidden"></div> }
                >
                    <a
                        class=BUTTON_CLASS
                        href=move || format!(
                            "/article/{}#{}",
                            playback.article_index.get().unwrap(),
                            playback.paragraph.get().unwrap() + 1
                        )
                    >
                        "Go to article"
                    </a>
                </Show>
                <div
                    class=BUTTON_CLASS
                    on:click=move |_| {
                        if playback.is_playing.get() {
                            if let Some(audio) = playback.audio.get_value() {
                                audio.pause().ok();
                            }
                            playback.set_is_playing.set(false);
                        } else if let Some(audio) = playback.audio.get_value() {
                            audio.play().ok();
                            playback.set_is_playing.set(true);
                        }
                    }
                >
                    {move || if playback.is_playing.get() { "Pause" } else { "Resume" }}
                </div>
            </div>
        </Show>
    }
}

#[cfg(not(feature = "hydrate"))]
#[component]
fn PlaybackPanel() -> impl IntoView {
    view! { <div class="hidden"></div> }
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
    let (audio_is_playing, set_audio_is_playing) = signal(false);
    let (audio_article_title, set_audio_article_title) = signal(Option::<String>::None);
    let (audio_article_index, set_audio_article_index) = signal(Option::<usize>::None);
    let (audio_paragraph, set_audio_paragraph) = signal(Option::<usize>::None);
    let (audio_selected_voice, set_audio_selected_voice) = signal(Option::<String>::None);
    let (audio_speech_cursor, set_audio_speech_cursor) = signal(Option::<crate::translation_page::SpeechCursor>::None);
    #[cfg(feature = "hydrate")]
    let playback = PlaybackState {
        audio: StoredValue::new_local(None::<HtmlAudioElement>),
        is_playing: audio_is_playing,
        set_is_playing: set_audio_is_playing,
        article_title: audio_article_title,
        set_article_title: set_audio_article_title,
        article_index: audio_article_index,
        set_article_index: set_audio_article_index,
        paragraph: audio_paragraph,
        set_paragraph: set_audio_paragraph,
        selected_voice: audio_selected_voice,
        set_selected_voice: set_audio_selected_voice,
        speech_cursor: audio_speech_cursor,
        set_speech_cursor: set_audio_speech_cursor,
    };
    #[cfg(not(feature = "hydrate"))]
    let playback = PlaybackState;
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
                <div class="fixed inset-0 bg-zinc-950 bg-opacity-75 transition-opacity">
                    <div class="fixed inset-1 z-10 w-screen overflow-y-auto">
                        <div class="flex min-h-full items-end justify-center text-center sm:items-center sm:p-0 lg:p-5">
                            <div class="flex relative transform overflow-hidden shadow-xl transition-all w-full h-full">
                                <div class="flex flex-1 flex-col px-4 pb-4 pt-5 sm:p-6 sm:pb-4">
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
                                            class=BUTTON_CLASS
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

                                            class=BUTTON_CLASS
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
                                    class="p-3 pt-7 text-xl lg:text-3xl font-bold font-mono w-screen justify-center flex snap-start"
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
                                                #[cfg(feature = "hydrate")]
                                                {
                                                    view! {
                                                        <TranslationPage
                                                            data=translation_post
                                                            set_data=set_translation_post
                                                        />
                                                        <PlaybackPanel playback=playback/>
                                                        <div>{input_popup_component(set_translation_post)}</div>
                                                    }
                                                }
                                                #[cfg(not(feature = "hydrate"))]
                                                {
                                                    view! {
                                                        <TranslationPage
                                                            data=translation_post
                                                            set_data=set_translation_post
                                                        />
                                                        <div>{input_popup_component(set_translation_post)}</div>
                                                    }
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
                                                        playback=playback
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
