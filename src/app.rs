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
use crate::translation_page::PlaybackState;
use crate::translation_page::TranslationPage;
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
                <Title text="Tippen — Fluency Studio"/>
                <Meta name="theme-color" content="#070a10"/>
                <Meta
                    name="description"
                    content="A focused workspace for typing practice and language fluency."
                />
                <link rel="manifest" href="/manifest.webmanifest"/>
                <link rel="apple-touch-icon" href="/icons/icon-192.png"/>
                <Meta name="mobile-web-app-capable" content="yes"/>
                <Meta name="apple-mobile-web-app-capable" content="yes"/>
                <Meta name="apple-mobile-web-app-status-bar-style" content="black-translucent"/>
                <Meta name="apple-mobile-web-app-title" content="Tippen"/>
                <Stylesheet id="leptos" href="/typing.css"/>
                <script src="/app.js"></script>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body class="min-h-screen bg-[#070a10] text-slate-300 antialiased"></body>
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
            <div class="fixed bottom-5 right-5 z-50 flex items-center gap-4 rounded-2xl border border-white/10 bg-slate-950/85 p-3 shadow-2xl shadow-black/50 backdrop-blur-2xl animate-slide-up">
                <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-cyan-300/15 bg-cyan-300/10 text-cyan-200">
                    <span class="text-sm">"▶"</span>
                </div>
                <div class="flex min-w-0 flex-col gap-0.5 text-sm">
                    <div class="max-w-52 truncate font-semibold text-slate-100">
                        {move || playback.article_title.get().unwrap_or_default()}
                    </div>
                    <div class="flex items-center gap-2 font-mono text-[10px] uppercase tracking-wider text-slate-500">
                        <span>
                            {move || {
                                playback
                                    .paragraph
                                    .get()
                                    .map(|item| format!("Paragraph {:02}", item + 1))
                                    .unwrap_or_default()
                            }}
                        </span>
                        <span class="h-1 w-1 rounded-full bg-cyan-300/60"></span>
                        <span>
                            {move || {
                                playback
                                    .selected_voice
                                    .get()
                                    .unwrap_or_else(|| "Default voice".to_string())
                            }}
                        </span>
                    </div>
                </div>
                <Show
                    when=move || {
                        playback.article_index.get().is_some() && playback.paragraph.get().is_some()
                    }
                    fallback=move || view! { <div class="hidden"></div> }
                >
                    <a
                        class=BUTTON_CLASS
                        href=move || {
                            format!(
                                "/article/{}#{}",
                                playback.article_index.get().unwrap(),
                                playback.paragraph.get().unwrap() + 1,
                            )
                        }
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

                    {move || {
                        if playback.is_playing.get() {
                            view! { <span>"❚❚ Pause"</span> }
                        } else {
                            view! { <span>"▶ Resume"</span> }
                        }
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

    // Seed the library from the local cache first; only fall back to the
    // server automatically when there is no cached copy.
    #[cfg(feature = "hydrate")]
    let cached = crate::local_store::cached_data();
    #[cfg(not(feature = "hydrate"))]
    let cached = None;
    let (translation_post, set_translation_post) = signal(cached.clone().unwrap_or_default());
    let (last_sync, set_last_sync) = signal(crate::local_store::cached_last_sync());
    let (from_cache, set_from_cache) = signal(cached.is_some());
    // Becomes true as soon as we have something to render: immediately when the
    // cache is present, otherwise once the server returns.
    let (data_ready, set_data_ready) = signal(cached.is_some());
    let (refreshing, set_refreshing) = signal(false);
    // bump to force a server refresh on demand
    let (refresh_request, set_refresh_request) = signal(0u32);

    // Persist every mutated library to the local cache so we always have the
    // latest offline snapshot.
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        let data = translation_post.get();
        if !data.articles.is_empty() {
            log!("dafault data will not be stored");
            crate::local_store::cache_data(&data);
        };
    });

    let (input_popup, set_input_popup) = signal(false);
    let (audio_is_playing, set_audio_is_playing) = signal(false);
    let (audio_article_title, set_audio_article_title) = signal(Option::<String>::None);
    let (audio_article_index, set_audio_article_index) = signal(Option::<usize>::None);
    let (audio_paragraph, set_audio_paragraph) = signal(Option::<usize>::None);
    #[cfg(feature = "hydrate")]
    let (audio_selected_voice, set_audio_selected_voice) =
        signal(crate::local_store::preferred_voice());
    #[cfg(not(feature = "hydrate"))]
    let (audio_selected_voice, set_audio_selected_voice) = signal(Option::<String>::None);
    let (audio_speech_cursor, set_audio_speech_cursor) =
        signal(Option::<crate::translation_page::SpeechCursor>::None);
    #[cfg(feature = "hydrate")]
    let (audio_current_paragraph_only, set_audio_current_paragraph_only) =
        signal(crate::local_store::current_paragraph_only());
    #[cfg(not(feature = "hydrate"))]
    let (audio_current_paragraph_only, set_audio_current_paragraph_only) = signal(false);
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        crate::local_store::save_current_paragraph_only(audio_current_paragraph_only.get());
    });
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
        move || (session_id.get(), refresh_request.get()),
        move |(session, _refresh)| async move {
            log!("got the session check it data refresh is needed");
            // Auto-fetch only when there is no local cache; once we are sitting on
            // a cached snapshot, data updates require an explicit refresh.
            if let Some(session) = session {
                if from_cache.get_untracked() && refresh_request.get_untracked() == 0 {
                    None
                } else {
                    log!(
                        "refreshing the data {} {} ",
                        from_cache.get_untracked(),
                        refresh_request.get_untracked()
                    );
                    set_refreshing.set(false);
                    get_data(session).await.ok()
                }
            } else {
                None
            }
        },
    );
    Effect::new(move |_| {
        // The resource future only ever resolves to Some(Some(data)) when a
        // fresh server fetch actually happened (no cache, or explicit refresh),
        // so whenever we reach this branch the result should always be applied.
        if let Some(Some(data)) = resource.get() {
            let now = crate::local_store::now_ms();
            set_translation_post.set(data.clone());
            set_last_sync.set(Some(now));
            set_from_cache.set(false);
            set_data_ready.set(true);
            set_refreshing.set(false);
            #[cfg(feature = "hydrate")]
            crate::local_store::cache_data_with_sync(&data, now);
        }
    });
    let input_popup_component = move |set_translation_post: WriteSignal<Data>| {
        if input_popup.get() {
            Either::Left(view! {
                <div class="fixed inset-0 z-50 overflow-y-auto">
                    <div class="fixed inset-0 bg-[#020408]/80 backdrop-blur-md transition-opacity animate-fade-in"></div>
                    <div class="relative flex min-h-full items-center justify-center p-4 sm:p-6">
                        <div class="glass-panel relative w-full max-w-2xl overflow-hidden rounded-[28px] p-6 shadow-2xl shadow-black/60 animate-slide-up sm:p-8">
                            <div class="pointer-events-none absolute -right-20 -top-20 h-48 w-48 rounded-full bg-cyan-300/10 blur-3xl"></div>
                            <div class="relative mb-6 flex items-start justify-between gap-6">
                                <div>
                                    <span class="eyebrow">"New material"</span>
                                    <h2 class="mt-3 text-2xl font-bold tracking-tight text-white">
                                        "Add an article"
                                    </h2>
                                    <p class="mt-1 text-sm text-slate-400">
                                        "Separate paragraphs with a new line. We’ll prepare the rest."
                                    </p>
                                </div>
                                <button
                                    class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-white/10 bg-white/5 text-lg text-slate-400 transition hover:bg-white/10 hover:text-white"
                                    aria-label="Close dialog"
                                    on:click=move |_event| set_input_popup.set(false)
                                >
                                    "×"
                                </button>
                            </div>
                            <textarea
                                class="relative h-80 w-full resize-none rounded-2xl border border-white/[0.08] bg-black/25 p-5 font-mono text-sm leading-relaxed text-slate-200 placeholder-slate-600 outline-none transition focus:border-cyan-300/35 focus:ring-4 focus:ring-cyan-300/[0.06]"
                                placeholder="Paste or type your article here…"
                                prop:value=translation_input
                                on:input=move |event| {
                                    set_translation_input.set(event_target_value(&event));
                                }
                            >
                            </textarea>
                            <div class="mt-5 flex items-center justify-between gap-3">
                                <span class="hidden font-mono text-[10px] uppercase tracking-widest text-slate-600 sm:block">
                                    "Plain text · UTF-8"
                                </span>
                                <div class="ml-auto flex items-center gap-3">
                                    <input
                                        class=BUTTON_CLASS
                                        type="button"
                                        value="Cancel"
                                        on:click=move |_event| {
                                            set_input_popup.set(false);
                                        }
                                    />

                                    <input
                                        class=BUTTON_PRIMARY_CLASS
                                        type="button"
                                        value="Add to library →"
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
            <main class="app-shell flex min-h-screen w-full flex-col items-center">
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
                                <div class="grid min-h-screen w-full place-items-center px-5 py-10">
                                    <div class="glass-panel relative w-full max-w-md overflow-hidden rounded-[32px] p-8 animate-slide-up sm:p-10">
                                        <div class="absolute -right-20 -top-24 h-56 w-56 rounded-full bg-cyan-300/10 blur-3xl"></div>
                                        <div class="relative">
                                            <a href="/" class="inline-flex items-center gap-3">
                                                <span class="flex h-11 w-11 items-center justify-center rounded-2xl border border-cyan-200/20 bg-cyan-300 text-sm font-black text-slate-950 shadow-lg shadow-cyan-300/20">
                                                    "T/"
                                                </span>
                                                <span>
                                                    <span class="block text-lg font-extrabold tracking-tight text-white">
                                                        "Tippen"
                                                    </span>
                                                    <span class="block font-mono text-[9px] uppercase tracking-[0.22em] text-slate-500">
                                                        "Fluency studio"
                                                    </span>
                                                </span>
                                            </a>
                                            <div class="my-10">
                                                <span class="eyebrow">"Welcome back"</span>
                                                <h1 class="mt-4 text-balance text-4xl font-bold tracking-[-0.04em] text-white">
                                                    "Your daily language practice starts here."
                                                </h1>
                                                <p class="mt-4 leading-relaxed text-slate-400">
                                                    "Build rhythm, vocabulary, and confidence with focused bilingual typing sessions."
                                                </p>
                                            </div>
                                            <button
                                                class=format!("{} w-full", BUTTON_PRIMARY_CLASS)
                                                on:click=move |_event| {
                                                    window()
                                                        .location()
                                                        .assign(
                                                            "https://typing.auth.us-east-1.amazoncognito.com/oauth2/authorize?client_id=2n9mqgc2vfhharda4r547sdcpm&response_type=token&scope=email+openid&redirect_uri=https%3A%2F%2Fxfwvamzfhhd4wjc766gmj6qsza0pcjeq.lambda-url.us-east-1.on.aws%2F",
                                                        )
                                                        .unwrap();
                                                }
                                            >
                                                <span>"Continue with your account"</span>
                                                <span>"→"</span>
                                            </button>
                                            <p class="mt-5 text-center font-mono text-[10px] uppercase tracking-widest text-slate-600">
                                                "Focused practice · Measurable progress"
                                            </p>
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
                                <header class="sticky top-0 z-40 w-full border-b border-white/[0.06] bg-[#070a10]/75 backdrop-blur-2xl">
                                    <div class="mx-auto flex h-[72px] w-full max-w-6xl items-center justify-between px-5 lg:px-6">
                                        <a href="/" class="group inline-flex items-center gap-3">
                                            <span class="flex h-10 w-10 items-center justify-center rounded-2xl border border-cyan-200/20 bg-cyan-300 text-xs font-black text-slate-950 shadow-lg shadow-cyan-300/15 transition-transform group-hover:-rotate-3">
                                                "T/"
                                            </span>
                                            <span>
                                                <span class="block text-base font-extrabold leading-none tracking-tight text-white">
                                                    "Tippen"
                                                </span>
                                                <span class="mt-1 block font-mono text-[8px] uppercase tracking-[0.22em] text-slate-500">
                                                    "Fluency studio"
                                                </span>
                                            </span>
                                        </a>
                                        <div class="flex items-center gap-3">
                                            <span class="hidden font-mono text-[10px] uppercase tracking-widest text-slate-600 md:block">
                                                "Your practice library"
                                            </span>
                                            <button
                                                class=BUTTON_CLASS
                                                on:click=move |_event| {
                                                    set_refreshing.set(true);
                                                    set_refresh_request.update(|n| *n += 1);
                                                }
                                            >
                                                <Show
                                                    when=move || last_sync.get().is_some()
                                                    fallback=|| view! { <span>Get data from server</span> }
                                                >
                                                    <span class="flex items-center gap-1.5">
                                                        {move || {
                                                            if refreshing.get() {
                                                                "Refreshing…".to_string()
                                                            } else {
                                                                format!(
                                                                    "Last sync · {}",
                                                                    last_sync
                                                                        .get()
                                                                        .map(crate::local_store::format_sync_time)
                                                                        .unwrap_or_default()
                                                                )
                                                            }
                                                        }}
                                                    </span>
                                                </Show>
                                            </button>
                                            <a
                                                href="/properties"
                                                class=BUTTON_CLASS
                                                aria-label="Properties"
                                                title="Properties"
                                            >
                                                "⚙"
                                            </a>
                                            <button
                                                class=BUTTON_PRIMARY_CLASS
                                                on:click=move |_event| set_input_popup.set(true)
                                            >
                                                <span class="text-lg leading-none">"+"</span>
                                                <span>"New article"</span>
                                            </button>
                                        </div>
                                    </div>
                                </header>

                                <div class="mx-auto flex w-full max-w-6xl flex-col px-5 pb-16 lg:px-6">
                                    <section class="grid gap-10 border-b border-white/[0.06] py-10 md:grid-cols-[1fr_auto] md:items-end md:py-14">
                                        <div class="max-w-3xl animate-slide-up">
                                            <span class="eyebrow">"Practice dashboard"</span>
                                            <h1 class="mt-5 text-balance text-4xl font-bold tracking-[-0.045em] text-white sm:text-5xl lg:text-[3.5rem] lg:leading-[1.05]">
                                                "Read deeply. Type precisely. "
                                                <span class="text-slate-500">"Learn naturally."</span>
                                            </h1>
                                            <p class="mt-5 max-w-2xl text-base leading-relaxed text-slate-400 sm:text-lg">
                                                "Turn the articles you care about into focused bilingual typing sessions, with audio and live performance feedback."
                                            </p>
                                        </div>
                                        <div class="glass-panel grid min-w-[320px] grid-cols-3 divide-x divide-white/[0.07] rounded-2xl px-2 py-4 animate-float-in">
                                            <div class="px-4">
                                                <div class="text-2xl font-bold text-white">
                                                    {move || translation_post.get().articles.len()}
                                                </div>
                                                <div class="mt-1 font-mono text-[9px] uppercase tracking-widest text-slate-500">
                                                    "Articles"
                                                </div>
                                            </div>
                                            <div class="px-4">
                                                <div class="text-2xl font-bold text-white">
                                                    {move || {
                                                        translation_post
                                                            .get()
                                                            .articles
                                                            .iter()
                                                            .map(|article| article.paragraphs.len())
                                                            .sum::<usize>()
                                                    }}
                                                </div>
                                                <div class="mt-1 font-mono text-[9px] uppercase tracking-widest text-slate-500">
                                                    "Paragraphs"
                                                </div>
                                            </div>
                                            <div class="px-4">
                                                <div class="text-2xl font-bold text-cyan-200">
                                                    {move || {
                                                        translation_post
                                                            .get()
                                                            .articles
                                                            .iter()
                                                            .flat_map(|article| article.paragraphs.iter())
                                                            .filter(|paragraph| {
                                                                paragraph
                                                                    .pairs
                                                                    .as_ref()
                                                                    .is_some_and(|pairs| !pairs.is_empty())
                                                            })
                                                            .count()
                                                    }}
                                                </div>
                                                <div class="mt-1 font-mono text-[9px] uppercase tracking-widest text-slate-500">
                                                    "Paired"
                                                </div>
                                            </div>
                                        </div>
                                    </section>

                                    <section class="pt-9">
                                        <div class="mb-6 flex items-end justify-between gap-6">
                                            <div>
                                                <span class="eyebrow">"Your collection"</span>
                                                <h2 class="mt-3 text-2xl font-bold tracking-tight text-white">
                                                    "Continue practicing"
                                                </h2>
                                            </div>
                                            <p class="hidden max-w-xs text-right text-sm leading-relaxed text-slate-500 sm:block">
                                                "Choose an article and pick up where you left off."
                                            </p>
                                        </div>
                                        <Suspense fallback=move || {
                                            view! {
                                                <div class="glass-panel flex items-center justify-center gap-3 rounded-3xl py-20 text-slate-500">
                                                    <div class="h-5 w-5 animate-spin rounded-full border-2 border-slate-700 border-t-cyan-300"></div>
                                                    <span class="font-mono text-xs uppercase tracking-widest">
                                                        "Loading library"
                                                    </span>
                                                </div>
                                            }
                                        }>
                                            {move || {
                                                #[cfg(feature = "hydrate")]
                                                {
                                                    if data_ready.get() {
                                                        Some(view! {
                                                            <TranslationPage
                                                                data=translation_post
                                                                set_data=set_translation_post
                                                            />
                                                            <PlaybackPanel playback=playback/>
                                                            <div>{input_popup_component(set_translation_post)}</div>
                                                        })
                                                    } else {
                                                        None
                                                    }
                                                }
                                                #[cfg(not(feature = "hydrate"))]
                                                {
                                                    if data_ready.get() {
                                                        Some(view! {
                                                            <TranslationPage
                                                                data=translation_post
                                                                set_data=set_translation_post
                                                            />
                                                            <div>{input_popup_component(set_translation_post)}</div>
                                                        })
                                                    } else {
                                                        None
                                                    }
                                                }
                                            }}

                                        </Suspense>
                                    </section>
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
                                        if data_ready.get() {
                                            Some(view! {
                                                <ArticlePage
                                                    data=translation_post
                                                    set_data=set_translation_post
                                                    playback=playback
                                                />
                                            })
                                        } else {
                                            None
                                        }
                                    }}

                                </Suspense>
                            }
                        }
                    />

                    <Route
                        path=path!("/properties")
                        view=move || {
                            view! { <crate::properties_page::PropertiesPage/> }
                        }
                    />

                </Routes>
            </main>
        </Router>
    }
}
