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
use serde::{Deserialize, Serialize};
use crate::translation_page::PlaybackState;
use crate::translation_page::TranslationPage;
use crate::ORIGIN;
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
        <html lang="en" class="app-html">
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
                <link rel="icon" type="image/x-icon" href="/favicon.ico"/>
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
            <body class="app-body"></body>
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
            <div class="playback-panel">
                <div class="playback-icon">
                    <span class="text-sm">"▶"</span>
                </div>
                <div class="playback-info">
                    <div class="playback-title">
                        {move || playback.article_title.get().unwrap_or_default()}
                    </div>
                    <div class="playback-meta">
                        <span>
                            {move || {
                                playback
                                    .paragraph
                                    .get()
                                    .map(|item| format!("Paragraph {:02}", item + 1))
                                    .unwrap_or_default()
                            }}

                        </span>
                        <span class="playback-dot"></span>
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

/// Whether to skip a load attempt because we already have a cached snapshot
/// and the user hasn't explicitly asked for a refresh ("seed from cache; only
/// fall back to the server when there is no cache").
fn initial_skip(has_cache: bool, refresh_requested: u32) -> bool {
    has_cache && refresh_requested == 0
}

/// Outcome of a server data-loading attempt. The resource returns one of these
/// and a single consumer `Effect` owns all the side effects (applying data,
/// clearing the session, navigation), keeping the fetcher a pure data producer.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum LoadOutcome {
    /// Nothing to do: showing the cached snapshot, or an OAuth callback in flight.
    Noop,
    /// Fresh server data arrived and should be rendered + persisted.
    Applied(Data),
    /// A session is required before anything can be loaded.
    AuthRequired,
    /// The server rejected the stored session; it must be dropped and re-logged-in.
    AuthFailed,
}

/// Renders `children` only once library data is available, sharing the same
/// loading fallback across every route that depends on the article library.
#[component]
fn DataGate(data_ready: Signal<bool>, children: ChildrenFn) -> impl IntoView {
    view! {
        <Suspense
            fallback=move || {
                view! {
                    <div class="loading-inline">
                        <div class="spinner-zinc"></div>
                        "Loading…"
                    </div>
                }
            }
        >
            {move || if data_ready.get() { Some(children().into_any()) } else { None }}
        </Suspense>
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

    // Seed the library from the local cache first; only fall back to the
    // server automatically when there is no cached copy.
    #[cfg(feature = "hydrate")]
    let cached = crate::local_store::cached_data();
    #[cfg(not(feature = "hydrate"))]
    let cached = None;
    let (translation_post, set_translation_post) = signal(cached.clone().unwrap_or_default());
    // Mirror of the library as last known to the server (a fresh fetch or a
    // completed "Save pairs" round-trip). It is used to tell which articles
    // have pairs created/edited locally but not yet persisted to the server.
    let (saved_data, set_saved_data) = signal(cached.clone().unwrap_or_default());
    let (last_sync, set_last_sync) = signal(crate::local_store::cached_last_sync());
    let (from_cache, set_from_cache) = signal(cached.is_some());
    // Becomes true as soon as we have something to render: immediately when the
    // cache is present, otherwise once the server returns.
    let (data_ready, set_data_ready) = signal(cached.is_some());
    let (refreshing, set_refreshing) = signal(false);
    // bump to force a server refresh on demand
    let (refresh_request, set_refresh_request) = signal(0u32);
    // Set by the data loader when server data can't be obtained without a
    // (valid) session; the <Router> watches it and sends the user to /login.
    let (redirect_to_login, set_redirect_to_login) = signal(false);
    // True while an incoming OAuth callback is being processed (the session is
    // set asynchronously after a userInfo round-trip). Suppresses the loader's
    // "no session → /login" redirect so it doesn't clobber the access token in
    // the URL hash before we've had a chance to exchange it for a session.
    //
    // IMPORTANT: this MUST be initialized from the URL hash here, BEFORE the
    // Resource below is created. The resource's fetcher is polled synchronously
    // (via `ArcAsyncDerived`'s `now_or_never`) inside `Resource::new()`, so its
    // no-session branch runs before the <Router> render block can set this.
    #[cfg(feature = "hydrate")]
    let (auth_pending, set_auth_pending) = signal(
        web_sys::window()
            .and_then(|w| w.location().hash().ok())
            .is_some_and(|h| !h.is_empty()),
    );
    #[cfg(not(feature = "hydrate"))]
    let (auth_pending, set_auth_pending) = signal(false);

    // Persist every mutated library to the local cache so we always have the
    // latest offline snapshot.
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        let data = translation_post.get();
        if !data.articles.is_empty() {
            crate::local_store::cache_data(&data);
        } else {
            log!("dafault data will not be stored");
        };
    });

    let (input_popup, set_input_popup) = signal(false);
    let (adding, set_adding) = signal(false);
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
        move |(session, refresh)| async move {
            log!("trigger resource");
            // Seed from the cache; only fetch when there is no cached copy, or
            // when the user explicitly asked for a refresh.
            let skip_initial = initial_skip(from_cache.get_untracked(), refresh);
            let Some(session) = session else {
                log!("session is missing");
                // Initial offline load straight from the local cache → stay put.
                if skip_initial {
                    log!("initial load skipping");
                    return LoadOutcome::Noop;
                }
                // An OAuth callback is still in flight — wait for it to finish
                // instead of redirecting and dropping the access token.
                if auth_pending.get_untracked() {
                    log!("is auth flow");
                    return LoadOutcome::Noop;
                }
                // A server refresh was requested (or there is no cache) but we
                // have no way to authenticate → go log in.
                log!("session required");
                return LoadOutcome::AuthRequired;
            };
            log!("session {}", session);
            // Sitting on a cached snapshot; only fetch on an explicit refresh.
            if skip_initial {
                log!("initial load skipping");
                return LoadOutcome::Noop;
            }
            log!("reading data from the server");
            set_refreshing.set(true);
            match get_data(session).await {
                Ok(data) => {
                    log!("got results");
                    LoadOutcome::Applied(data)
                }
                Err(_) => {
                    // The server rejected the token (expired/revoked) → the
                    // consumer Effect drops the session and requires a re-login.
                    log!("error fetching data from the server, resetting to login");
                    LoadOutcome::AuthFailed
                }
            }
        },
    );
    // Single place where load outcomes translate into side effects (applying
    // data, clearing the session, navigation). Keeps the resource above a pure
    // data producer.
    Effect::new(move |_| {
        use LoadOutcome::*;
        match resource.get() {
            // A fresh server fetch actually happened → apply and persist it.
            Some(Applied(data)) => {
                let now = crate::local_store::now_ms();
                set_translation_post.set(data.clone());
                set_saved_data.set(data.clone());
                set_last_sync.set(Some(now));
                set_from_cache.set(false);
                set_data_ready.set(true);
                #[cfg(feature = "hydrate")]
                crate::local_store::cache_data_with_sync(&data, now);
                set_refreshing.set(false);
            }
            // The server rejected the session → drop it and re-login.
            Some(AuthFailed) => {
                set_refreshing.set(false);
                set_session_id.set(None);
                #[cfg(feature = "hydrate")]
                set_redirect_to_login.set(true);
            }
            // No usable session → go log in. Only decided on the client: the
            // OAuth token never appears in the URL during SSR, so a redirect set
            // here would be serialized into the hydration payload and fire
            // before the client reads it.
            Some(AuthRequired) => {
                #[cfg(feature = "hydrate")]
                set_redirect_to_login.set(true);
            }
            _ => {}
        }
    });
    let input_popup_component = move |set_translation_post: WriteSignal<Data>| {
        if input_popup.get() {
            Either::Left(view! {
                <div class="modal-overlay">
                    <div class="modal-backdrop"></div>
                    <div class="modal-center">
                        <div class="modal-panel glass-panel">
                            <div class="modal-head">
                                <div>
                                    <span class="eyebrow">"New material"</span>
                                    <h2 class="modal-title">
                                        "Add an article"
                                    </h2>
                                    <p class="modal-sub">
                                        "Separate paragraphs with a new line. We’ll prepare the rest."
                                    </p>
                                </div>
                                <button
                                    class="modal-close"
                                    aria-label="Close dialog"
                                    on:click=move |_event| set_input_popup.set(false)
                                >
                                    "×"
                                </button>
                            </div>
                            <textarea
                                class="modal-textarea"
                                placeholder="Paste or type your article here…"
                                prop:value=translation_input
                                on:input=move |event| {
                                    set_translation_input.set(event_target_value(&event));
                                }
                            >
                            </textarea>
                            <div class="modal-foot">
                                <span class="modal-hint">
                                    "Plain text · UTF-8"
                                </span>
                                <div class="modal-actions">
                                    <input
                                        class=BUTTON_CLASS
                                        type="button"
                                        value="Cancel"
                                        on:click=move |_event| {
                                            set_input_popup.set(false);
                                        }
                                    />

                                    <button
                                        class=BUTTON_PRIMARY_CLASS
                                        disabled=move || adding.get()
                                        on:click=move |_event| {
                                            if adding.get() {
                                                return;
                                            }
                                            let temp = translation_input.get();
                                            log!("passing argument: {}", temp);
                                            set_adding.set(true);
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
                                                let _ = store_article(article).await;
                                                set_adding.set(false);
                                                set_input_popup.set(false);
                                            });
                                        }
                                    >

                                        {move || {
                                            if adding.get() {
                                                view! {
                                                    <span class="btn-spinner btn-spinner--dark"></span>
                                                    "Adding…"
                                                }
                                                    .into_any()
                                            } else {
                                                view! { "Add to library →" }.into_any()
                                            }
                                        }}

                                    </button>

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
                // The data loader owns all the auth decisions; when it determines
                // we need a (re)login it flips `redirect_to_login`, which we turn
                // into an actual navigation here (inside the <Router>, where
                // use_navigate has access to RouterContext).
                if redirect_to_login.get() {
                    set_redirect_to_login.set(false);
                    #[cfg(feature = "hydrate")]
                    {
                        log!("navigate to login page");
                        use leptos_router::hooks::use_navigate;
                        let navigate = use_navigate();
                        navigate("/login", Default::default());
                    }
                }
                if session_id.get().is_none() {
                    let location = use_location();
                    let hash = location.hash.get();
                    if !hash.is_empty() {
                        // Incoming OAuth redirect carrying a session token.
                        let hash = parse_hash(hash);
                        log!("hash {:?}", hash);
                        set_auth_pending.set(true);
                        spawn_local(async move {
                            let user_info = get_user_info(hash);
                            set_session_id.set(Some(user_info.await.sub));
                            set_auth_pending.set(false);
                        });
                    }
                    // The "no session, nothing to show → login" redirect is now
                    // decided inside the data loader (the Resource above), not here.
                }
            }}
            <main class="app-shell">
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
                                <div class="login-wrap">
                                    <div class="login-panel glass-panel">
                                        <div class="relative">
                                            <a href="/" class="login-logo">
                                                <span class="brand-mark brand-mark--lg">
                                                    "T/"
                                                </span>
                                                <span>
                                                    <span class="login-logo-name">
                                                        "Tippen"
                                                    </span>
                                                    <span class="login-logo-tag">
                                                        "Fluency studio"
                                                    </span>
                                                </span>
                                            </a>
                                            <div class="login-hero">
                                                <span class="eyebrow">"Welcome back"</span>
                                                <h1 class="login-title">
                                                    "Your daily language practice starts here."
                                                </h1>
                                                <p class="login-sub">
                                                    "Build rhythm, vocabulary, and confidence with focused bilingual typing sessions."
                                                </p>
                                            </div>
                                            <button
                                                class=format!("{} btn-block", BUTTON_PRIMARY_CLASS)
                                                on:click=move |_event| {
                                                    window()
                                                        .location()
                                                        .assign(&format!(
                                                            "https://typing.auth.us-east-1.amazoncognito.com/oauth2/authorize?client_id=2n9mqgc2vfhharda4r547sdcpm&response_type=token&scope=email+openid&redirect_uri=https%3A%2F%2F{}%2F", ORIGIN),
                                                        )
                                                        .unwrap();
                                                }
                                            >

                                                <span>"Continue with your account"</span>
                                                <span>"→"</span>
                                            </button>
                                            <p class="login-footer">
                                                "Focused practice · Measurable progress"
                                            </p>
                                            <p class="login-build">
                                                "build "
                                                {crate::BUILD_NUMBER}
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
                                <header class="site-header">
                                    <div class="site-header-inner">
                                        <a href="/" class="brand group">
                                            <span class="brand-mark brand-mark--sm">
                                                "T/"
                                            </span>
                                            <span>
                                                <span class="brand-name">
                                                    "Tippen"
                                                </span>
                                                <span class="brand-tag">
                                                    "Fluency studio"
                                                </span>
                                            </span>
                                        </a>
                                        <div class="header-actions">
                                            <span class="header-hint">
                                                "Your practice library"
                                            </span>
                                            <button
                                                class=BUTTON_CLASS
                                                on:click=move |_event| {
                                                    set_refresh_request.update(|n| *n += 1);
                                                }
                                            >

                                                <Show
                                                    when=move || last_sync.get().is_some()
                                                    fallback=|| view! { <span>Get data from server</span> }
                                                >
                                                    // Compact label on phones, full timestamp on larger screens.
                                                    <span class="sync-label-compact">
                                                        {move || {
                                                            if refreshing.get() {
                                                                Either::Left(
                                                                    view! {
                                                                        <span class="btn-spinner"></span>
                                                                        "Refreshing…"
                                                                    },
                                                                )
                                                            } else {
                                                                Either::Right(view! { "✓" })
                                                            }
                                                        }}

                                                    </span>
                                                    <span class="sync-label-full">
                                                        {move || {
                                                            if refreshing.get() {
                                                                Either::Left(
                                                                    view! {
                                                                        <span class="btn-spinner"></span>
                                                                        "Refreshing…"
                                                                    },
                                                                )
                                                            } else {
                                                                Either::Right(
                                                                    format!(
                                                                        "Last sync · {}",
                                                                        last_sync
                                                                            .get()
                                                                            .map(crate::local_store::format_sync_time)
                                                                            .unwrap_or_default(),
                                                                    ),
                                                                )
                                                            }
                                                        }}

                                                    </span>
                                                </Show>
                                            </button>
                                            <a
                                                href="/rebuild"
                                                class=BUTTON_CLASS
                                                title="Restore original text from saved pairs"
                                            >
                                                <span class="btn-icon">"↶"</span>
                                                <span class="btn-label">"Rebuild text"</span>
                                            </a>
                                            <a
                                                href="/match"
                                                class=BUTTON_CLASS
                                                title="Match saved pairs"
                                            >
                                                <span class="btn-icon">"⧉"</span>
                                                <span class="btn-label">"Match pairs"</span>
                                            </a>
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
                                                <span class="btn-icon">"+"</span>
                                                <span class="btn-label">"New article"</span>
                                            </button>
                                        </div>
                                    </div>
                                </header>

                                <div class="dash-container">
                                    <section class="hero">
                                        <div class="hero-text">
                                            <span class="eyebrow">"Practice dashboard"</span>
                                            <h1 class="hero-title">
                                                "Read deeply. Type precisely. "
                                                <span class="hero-title-accent">"Learn naturally."</span>
                                            </h1>
                                            <p class="hero-sub">
                                                "Turn the articles you care about into focused bilingual typing sessions, with audio and live performance feedback."
                                            </p>
                                        </div>
                                        <div class="stats glass-panel">
                                            <div class="stat">
                                                <div class="stat-num">
                                                    {move || translation_post.get().articles.len()}
                                                </div>
                                                <div class="stat-label">
                                                    "Articles"
                                                </div>
                                            </div>
                                            <div class="stat">
                                                <div class="stat-num">
                                                    {move || {
                                                        translation_post
                                                            .get()
                                                            .articles
                                                            .iter()
                                                            .map(|article| article.paragraphs.len())
                                                            .sum::<usize>()
                                                    }}

                                                </div>
                                                <div class="stat-label">
                                                    "Paragraphs"
                                                </div>
                                            </div>
                                            <div class="stat">
                                                <div class="stat-num stat-num--muted">
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
                                                <div class="stat-label">
                                                    "Paired"
                                                </div>
                                            </div>
                                        </div>
                                    </section>

                                    <section class="collection-section">
                                        <div class="collection-head">
                                            <div>
                                                <span class="eyebrow">"Your collection"</span>
                                                <h2 class="collection-title">
                                                    "Continue practicing"
                                                </h2>
                                            </div>
                                            <p class="collection-hint">
                                                "Choose an article and pick up where you left off."
                                            </p>
                                        </div>
                                        <Suspense fallback=move || {
                                            view! {
                                                <div class="loading-panel glass-panel">
                                                    <div class="spinner"></div>
                                                    <span class="loading-label">
                                                        "Loading library"
                                                    </span>
                                                </div>
                                            }
                                        }>
                                            {move || {
                                                #[cfg(feature = "hydrate")]
                                                {
                                                    if data_ready.get() {
                                                        Some(
                                                            view! {
                                                                <TranslationPage
                                                                    data=translation_post
                                                                    set_data=set_translation_post
                                                                    saved=saved_data
                                                                    set_saved=set_saved_data
                                                                />
                                                                <PlaybackPanel playback=playback/>
                                                                <div>{input_popup_component(set_translation_post)}</div>
                                                            },
                                                        )
                                                    } else {
                                                        None
                                                    }
                                                }
                                                #[cfg(not(feature = "hydrate"))]
                                                {
                                                    if data_ready.get() {
                                                        Some(
                                                            view! {
                                                                <TranslationPage
                                                                    data=translation_post
                                                                    set_data=set_translation_post
                                                                    saved=saved_data
                                                                    set_saved=set_saved_data
                                                                />
                                                                <div>{input_popup_component(set_translation_post)}</div>
                                                            },
                                                        )
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
                                <DataGate data_ready=data_ready.into()>
                                    <ArticlePage
                                        data=translation_post
                                        set_data=set_translation_post
                                        playback=playback
                                    />
                                </DataGate>
                            }
                        }
                    />

                    <Route
                        path=path!("/properties")
                        view=move || {
                            view! {
                                <crate::properties_page::PropertiesPage></crate::properties_page::PropertiesPage>
                            }
                        }
                    />

                    <Route
                        path=path!("/rebuild")
                        view=move || {
                            view! {
                                <DataGate data_ready=data_ready.into()>
                                    <crate::rebuild_page::RebuildPage data=translation_post/>
                                </DataGate>
                            }
                        }
                    />

                    <Route
                        path=path!("/match")
                        view=move || {
                            view! {
                                <DataGate data_ready=data_ready.into()>
                                    <crate::matching_page::MatchingPage data=translation_post/>
                                </DataGate>
                            }
                        }
                    />

                </Routes>
            </main>
        </Router>
    }
}
