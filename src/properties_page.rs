use leptos::prelude::*;
use leptos_meta::Title;

use crate::preferences::PreferencesStore;
use crate::BUTTON_CLASS;


#[component]
pub fn PropertiesPage() -> impl IntoView {
    let prefs = use_context::<PreferencesStore>()
        .expect("PreferencesStore not provided")
        .prefs;
    let current_paragraph_only = Memo::new(move |_| prefs.get().current_paragraph_only);
    let group_matching_by_paragraph =
        Memo::new(move |_| prefs.get().group_matching_by_paragraph);
    let (saved, set_saved) = signal(false);

    view! {
        <Title text="Properties — Tippen"/>
        <div class="props-wrap">
            <a href="/" class=BUTTON_CLASS>
                "← Library"
            </a>

            <div class="props-panel glass-panel">
                <div class="relative">
                    <span class="eyebrow">"Properties"</span>
                    <h1 class="props-title">
                        "Sound preferences"
                    </h1>
                    <p class="props-sub">
                        "Control how articles are read aloud while you type. The narration audio
                        comes with each imported article."
                    </p>
                    <div class="props-toggle-row">
                        <input
                            id="current-paragraph-only"
                            type="checkbox"
                            class="props-checkbox"
                            prop:checked=move || current_paragraph_only.get()
                            on:change=move |event| {
                                let checked = event_target_checked(&event);
                                prefs.update(|p| p.current_paragraph_only = checked);
                                set_saved.set(true);
                            }
                        />

                        <label for="current-paragraph-only" class="props-toggle-label">
                            <span class="props-toggle-name">
                                "Play only current paragraph"
                            </span>
                            <span class="props-toggle-desc">
                                "When enabled, audio playback stops at the end of the paragraph
                                you started it in instead of continuing through the article."
                            </span>
                        </label>
                    </div>

                    <div class="props-toggle-row">
                        <input
                            id="group-matching-by-paragraph"
                            type="checkbox"
                            class="props-checkbox"
                            prop:checked=move || group_matching_by_paragraph.get()
                            on:change=move |event| {
                                let checked = event_target_checked(&event);
                                prefs.update(|p| p.group_matching_by_paragraph = checked);
                                set_saved.set(true);
                            }
                        />

                        <label for="group-matching-by-paragraph" class="props-toggle-label">
                            <span class="props-toggle-name">
                                "Group matching pairs by paragraph"
                            </span>
                            <span class="props-toggle-desc">
                                "On the Match the pairs page, group saved words per paragraph
                                instead of per article, so each exercise stays small and
                                easier to complete."
                            </span>
                        </label>
                    </div>

                    <Show when=move || saved.get() fallback=|| ()>
                        <p class="props-saved">"Saved ✓"</p>
                    </Show>
                </div>
            </div>
        </div>
    }
}
