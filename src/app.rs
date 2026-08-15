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
    BUTTON_CLASS, BUTTON_PRIMARY_CLASS,
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
            <body class="h-screen text-zinc-300 antialiased"></body>
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
            <div class="fixed bottom-4 right-4 z-40 flex items-center gap-4 rounded-2xl border border-white/10 bg-zinc-900/80 p-4 shadow-2xl shadow-black/50 backdrop-blur-xl animate-slide-up">
                <div class="flex flex-col gap-0.5 text-sm">
                    <div class="max-w-48 truncate font-semibold text-zinc-100">
                        {move || playback.article_title.get().unwrap_or_default()}
                    </div>
                    <div class="font-mono text-xs text-zinc-500">
                        {move || playback.paragraph.get().map(|item| format!("paragraph #{}", item + 1)).unwrap_or_default()}
                    </div>
                    <div class="text-xs text-indigo-300/70">
                        {move || playback.selected_voice.get().map(|voice| format!("voice: {}", voice)).unwrap_or_default()}
                    </div>
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
                        "Open article"
                    </a>
                </Show>
                <div
                    class=BUTTON_PRIMARY_CLASS
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
                    {move || if playback.is_playing.get() {
                        view! { <span>"❚❚ Pause"</span> }
                    } else {
                        view! { <span>"▶ Resume"</span> }
                    }}
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
    let (audio_current_paragraph_only, set_audio_current_paragraph_only) = signal(false);
    #[cfg(feature = "hydrate")]
    let playback = PlaybackState {
        audio: StoredValue::new_local(None::<HtmlAudioElement>),
        cue_starts: StoredValue::new_local(Vec::<f64>::new()),
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
        current_paragraph_only: audio_current_paragraph_only,
        set_current_paragraph_only: set_audio_current_paragraph_only,
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
                <div class="fixed inset-0 z-50 overflow-y-auto">
                    <div class="fixed inset-0 bg-black/70 backdrop-blur-sm transition-opacity animate-fade-in"></div>
                    <div class="relative flex min-h-full items-center justify-center p-4 sm:p-6">
                        <div class="relative w-full max-w-2xl rounded-2xl border border-white/10 bg-zinc-900/95 p-6 shadow-2xl shadow-black/60 backdrop-blur-xl animate-slide-up">
                            <div class="mb-4 flex items-center justify-between">
                                <h2 class="text-lg font-semibold text-white">"Add a new article"</h2>
                                <span class="font-mono text-xs text-zinc-500">"paste text · paragraphs split by blank line"</span>
                            </div>
                            <textarea
                                class="h-80 w-full resize-none rounded-xl border border-zinc-700/80 bg-zinc-950/80 p-4 font-mono text-sm leading-relaxed text-zinc-200 placeholder-zinc-600 focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20 transition-colors"
                                placeholder="type here your text"
                                prop:value=translation_input
                                on:input=move |event| {
                                    set_translation_input.set(event_target_value(&event));
                                }
                            >
                            </textarea>
                            <div class="mt-5 flex items-center justify-end gap-3">
                                <input
                                    class=BUTTON_CLASS
                                    type="button"
                                    value="Close"
                                    on:click=move |_event| {
                                        set_input_popup.set(false);
                                    }
                                />

                                <input
                                    class=BUTTON_PRIMARY_CLASS
                                    type="button"
                                    value="+ Add article"
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
                                <div class="flex min-h-screen flex-col items-center justify-center gap-8 p-6">
                                    <div class="flex flex-col items-center gap-4 animate-slide-up">
                                        <div class="flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-br from-indigo-500 to-violet-600 text-3xl shadow-xl shadow-indigo-500/30">
                                            "⌨️"
                                        </div>
                                        <a href="/" class="text-4xl font-black tracking-tight lg:text-5xl">
                                            <span class="bg-gradient-to-r from-indigo-400 via-violet-400 to-fuchsia-400 bg-clip-text text-transparent">
                                                Tippen!
                                            </span>
                                        </a>
                                        <p class="max-w-md text-center text-zinc-400">
                                            "Type faster, translate smarter. Practice with paired word translations and real-time audio."
                                        </p>
                                    </div>
                                    <div class="animate-float-in">
                                        <div
                                            class=BUTTON_PRIMARY_CLASS
                                            on:click=move |_event| {
                                                window()
                                                    .location()
                                                    .assign(
                                                        "https://typing.auth.us-east-1.amazoncognito.com/oauth2/authorize?client_id=2n9mqgc2vfhharda4r547sdcpm&response_type=token&scope=email+openid&redirect_uri=https%3A%2F%2Fxfwvamzfhhd4wjc766gmj6qsza0pcjeq.lambda-url.us-east-1.on.aws%2F",
                                                    )
                                                    .unwrap();
                                            }>
                                            <span>"Sign in to continue"</span>
                                            <span class="ml-2">"→"</span>
                                        </div>
                                    </div>
                                </div>
                            }
                        }
                    />

                    <Route
                        path=path!("")
                        view=move || {
                            view! {
                                <div class="sticky top-0 z-30 w-full border-b border-white/5 bg-[#08080d]/70 backdrop-blur-xl">
                                    <div class="mx-auto flex w-full max-w-5xl items-center justify-between px-4 py-3">
                                        <a href="/" class="text-xl font-bold tracking-tight">
                                            <span class="bg-gradient-to-r from-indigo-400 via-violet-400 to-fuchsia-400 bg-clip-text text-transparent">
                                                Tippen!
                                            </span>
                                        </a>
                                        <div
                                            class=BUTTON_PRIMARY_CLASS
                                            on:click=move |_event| {
                                                set_input_popup.set(true)
                                            }
                                        >
                                            "+ Add Article"
                                        </div>
                                    </div>
                                </div>
                                <div class="mx-auto flex w-full max-w-5xl flex-col px-4 py-8">
                                    <Suspense fallback=move || {
                                        view! {
                                            <div class="flex items-center justify-center gap-3 py-20 text-zinc-500">
                                                <div class="h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-indigo-400"></div>
                                                "Loading…"
                                            </div>
                                        }
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
                                </div>
                            }
                        }
                    />

                    <Route
                        path=path!("/article/:id")
                        view=move || {
                            view! {
                                <Suspense fallback=move || {
                                    view! {
                                        <div class="flex items-center justify-center gap-3 py-20 text-zinc-500">
                                            <div class="h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-indigo-400"></div>
                                            "Loading…"
                                        </div>
                                    }
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
