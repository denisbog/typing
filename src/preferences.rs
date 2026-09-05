//! Client-side reactive store for per-user preferences and favourite articles.
//!
//! The store is the single source of truth for preferences on the client. It is
//! initialised from the local (offline) cache, kept in sync with the local
//! cache on every change, loaded once from the server when a session becomes
//! available, and persisted to the server (the `translation_preferences`
//! DynamoDB table) reactively — but only for genuine user changes, never for
//! the initial server load.

use crate::application_types::UserPreferences;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// A cloneable handle to the preferences store, provided via leptos context so
/// any component (Properties page, Matching page, Translation page, …) can read
/// and mutate the user's preferences reactively.
#[derive(Clone, Copy)]
pub struct PreferencesStore {
    pub prefs: RwSignal<UserPreferences>,
}

/// Initialise the preferences store, provide it as context, and wire up local +
/// server persistence. Must be called once inside `App` (before the router).
pub fn init_preferences(session: Signal<Option<String>>) -> PreferencesStore {
    let prefs = RwSignal::new(crate::local_store::load_preferences());
    let store = PreferencesStore { prefs };
    provide_context(store);
    #[cfg(not(feature = "hydrate"))]
    let _ = session;

    // Keep the local (offline) cache in sync with every change so preferences
    // survive reloads and are available offline.
    Effect::new(move |_| {
        let p = prefs.get();
        crate::local_store::save_preferences(&p);
    });

    // Client-only: load once per session from the server, and save reactively.
    #[cfg(feature = "hydrate")]
    {
        let loaded_for_session = StoredValue::new(None::<String>);
        // Server persistence is reactive, but distinguishes the initial load
        // from a genuine user change: the FIRST time a user id appears it is
        // just the server data being applied (nothing is written back); any
        // later change is a user action and is saved. This keeps page loads
        // from writing to the table while still saving on user edits.
        let saved_for_session = StoredValue::new(None::<String>);
        Effect::new(move |_| {
            let p = prefs.get();
            if p.user_id.is_empty() {
                return;
            }
            if saved_for_session.get_value().as_deref() == Some(p.user_id.as_str()) {
                // Already synchronised for this session → a real user change.
                spawn_local(async move {
                    let _ = crate::translation::save_preferences(p).await;
                });
            } else {
                // First time we see this user id: the initial load is being
                // applied. Don't write to the server, just remember the session.
                saved_for_session.set_value(Some(p.user_id.clone()));
            }
        });
        // Load once per session from the server.
        Effect::new(move |_| {
            let Some(session) = session.get() else {
                return;
            };
            if loaded_for_session.get_value().as_deref() == Some(session.as_str()) {
                return;
            }
            loaded_for_session.set_value(Some(session.clone()));
            spawn_local(async move {
                let mut p = prefs.get_untracked();
                match crate::translation::get_preferences(session.clone()).await {
                    Ok(Some(server_prefs)) => {
                        p.user_id = session;
                        p.voice = server_prefs.voice;
                        p.current_paragraph_only = server_prefs.current_paragraph_only;
                        p.group_matching_by_paragraph = server_prefs.group_matching_by_paragraph;
                        p.favorites = server_prefs.favorites;
                    }
                    Ok(None) => {
                        // No row yet: keep local defaults, just tag the user.
                        p.user_id = session;
                    }
                    Err(_) => return,
                }
                prefs.set(p);
            });
        });
    }

    store
}
