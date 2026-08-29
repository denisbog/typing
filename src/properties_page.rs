use leptos::prelude::*;
use leptos_meta::Title;

use crate::local_store;
use crate::BUTTON_CLASS;

/// Known voice options. Add new voices here as they become available.
const KNOWN_VOICES: &[&str] = &["merz"];

#[component]
pub fn PropertiesPage() -> impl IntoView {
    let (voice, set_voice) = signal(local_store::preferred_voice().unwrap_or_default());
    let (current_paragraph_only, set_current_paragraph_only) =
        signal(local_store::current_paragraph_only());
    let (group_matching_by_paragraph, set_group_matching_by_paragraph) =
        signal(local_store::group_matching_by_paragraph());
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
                        "Choose the voice used when reading articles aloud. If the preferred
                        voice is not available for an article, it falls back to "default ".
                        If the article has no audio, the voice selector is disabled."
                    </p>

                    <label
                        for="preferred-voice"
                        class="props-label"
                    >
                        "Preferred voice"
                    </label>
                    <select
                        id="preferred-voice"
                        class="props-select"
                        prop:value=move || {
                            if voice.get().is_empty() { "default".to_string() } else { voice.get() }
                        }

                        on:change=move |event| {
                            let value = event_target_value(&event);
                            let v = if value == "default" { String::new() } else { value };
                            set_voice.set(v.clone());
                            local_store::save_preferred_voice(&v);
                            set_saved.set(true);
                        }
                    >

                        <option value="default">"default"</option>
                        {KNOWN_VOICES
                            .iter()
                            .map(|v| {
                                let value = v.to_string();
                                view! { <option value=value>{*v}</option> }
                            })
                            .collect_view()}
                    </select>
                    <p class="props-hint">
                        "Available: default, " {KNOWN_VOICES.join(", ")}
                    </p>

                    <div class="props-toggle-row">
                        <input
                            id="current-paragraph-only"
                            type="checkbox"
                            class="props-checkbox"
                            prop:checked=move || current_paragraph_only.get()
                            on:change=move |event| {
                                let checked = event_target_checked(&event);
                                set_current_paragraph_only.set(checked);
                                local_store::save_current_paragraph_only(checked);
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
                                set_group_matching_by_paragraph.set(checked);
                                local_store::save_group_matching_by_paragraph(checked);
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
