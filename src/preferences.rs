//! Client-side reactive store for per-user preferences and favourite articles.
//!
//! The store is the single source of truth for preferences on the client. On
//! page load it is initialised entirely from the local (offline) cache — no
//! server request is made. The server is only contacted on two occasions:
//!   * a user-triggered sync pulls preferences down from the server
//!     (`PreferencesStore::pull()`), and
//!   * a genuine user edit (favourite toggle, property change, voice change)
//!     pushes the new preferences up to the `translation_preferences` table.
//! Values that were merely loaded from the cache or applied from a pull are
//! never written back to the server.

use crate::application_types::UserPreferences;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// A cloneable handle to the preferences store, provided via leptos context so
/// any component (Properties page, Matching page, Translation page, …) can read
/// and mutate the user's preferences reactively.
#[derive(Clone, Copy)]
pub struct PreferencesStore {
    pub prefs: RwSignal<UserPreferences>,
    /// Bumped by the user-triggered sync to request a pull from the server.
    pub sync: RwSignal<u32>,
}

impl PreferencesStore {
    /// Request a server pull (user-triggered sync). The fetch itself happens
    /// reactively and is never triggered automatically on page load.
    pub fn pull(self) {
        #[cfg(feature = "hydrate")]
        self.sync.update(|n| *n += 1);
        #[cfg(not(feature = "hydrate"))]
        let _ = self;
    }
}

/// Initialise the preferences store, provide it as context, and wire up local +
/// server persistence. Must be called once inside `App` (before the router).
pub fn init_preferences(session: Signal<Option<String>>) -> PreferencesStore {
    // On load we only ever read the local cache — no server round-trip.
    let prefs = RwSignal::new(crate::local_store::load_preferences());
    let sync = RwSignal::new(0u32);
    let store = PreferencesStore { prefs, sync };
    provide_context(store);
    #[cfg(not(feature = "hydrate"))]
    let _ = session;

    // Keep the local (offline) cache in sync with every change so preferences
    // survive reloads and are available offline.
    Effect::new(move |_| {
        let p = prefs.get();
        crate::local_store::save_preferences(&p);
    });

    // Client-only: save reactively on user edits, and pull on user sync.
    #[cfg(feature = "hydrate")]
    {
        // Remembers which session the cached state belongs to, so the very
        // first observation of a user id (the cached/loaded value, or a value
        // just applied from a pull) is never written back to the server.
        let saved_for_session = StoredValue::new(None::<String>);
        // Snapshot of the last value applied from a server pull — used to skip
        // writing pulled data straight back to the server.
        let last_synced = StoredValue::new(None::<UserPreferences>);

        // Tag the store with the active session so user changes are attributed
        // to the right user. This only touches the local state; it does not
        // fetch anything from the server.
        Effect::new(move |_| {
            let Some(session) = session.get() else {
                return;
            };
            let current = prefs.get_untracked();
            if current.user_id != session {
                prefs.update(|p| p.user_id = session);
            }
        });

        // Reactive server save, but only for genuine user edits: skip values
        // that were merely loaded from the cache or applied from a pull.
        Effect::new(move |_| {
            let p = prefs.get();
            if p.user_id.is_empty() {
                return;
            }
            if last_synced.get_value().as_ref() == Some(&p) {
                return; // just applied from a server pull — nothing to write
            }
            if saved_for_session.get_value().as_deref() == Some(p.user_id.as_str()) {
                // Already synchronised for this session → a real user change.
                spawn_local(async move {
                    let _ = crate::translation::save_preferences(p).await;
                });
            } else {
                // First time we see this user id: this is the cached state being
                // established. Don't write to the server, just remember it.
                saved_for_session.set_value(Some(p.user_id.clone()));
            }
        });

        // User-triggered pull: fetch the server preferences and apply them.
        // `sync` starts at 0 and is only bumped by `PreferencesStore::pull()`,
        // so this effect does nothing on page load.
        Effect::new(move |_| {
            if sync.get() == 0 {
                return;
            }
            let Some(session) = session.get() else {
                return;
            };
            spawn_local(async move {
                match crate::translation::get_preferences(session.clone()).await {
                    Ok(Some(server_prefs)) => {
                        let mut p = prefs.get_untracked();
                        p.user_id = session;
                        p.voice = server_prefs.voice;
                        p.current_paragraph_only = server_prefs.current_paragraph_only;
                        p.group_matching_by_paragraph = server_prefs.group_matching_by_paragraph;
                        p.favorites = server_prefs.favorites;
                        last_synced.set_value(Some(p.clone()));
                        prefs.set(p);
                    }
                    Ok(None) => {
                        // Nothing on the server: just make sure the user is
                        // tagged so later edits are saved to the right user.
                        let mut p = prefs.get_untracked();
                        p.user_id = session;
                        prefs.set(p);
                    }
                    Err(_) => {}
                }
            });
        });
    }

    store
}
