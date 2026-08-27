use leptos::prelude::*;
use leptos_meta::Title;

use crate::local_store;
use crate::BUTTON_CLASS;

/// Known voice options. Add new voices here as they become available.
const KNOWN_VOICES: &[&str] = &["merz"];

#[component]
pub fn PropertiesPage() -> impl IntoView {
    let (voice, set_voice) = signal(local_store::preferred_voice().unwrap_or_default());
    let (saved, set_saved) = signal(false);

    view! {
        <Title text="Properties — Tippen"/>
        <div class="mx-auto w-full max-w-2xl px-5 py-10 lg:px-6">
            <a href="/" class=BUTTON_CLASS>"← Library"</a>

            <div class="glass-panel relative mt-8 overflow-hidden rounded-3xl p-6 animate-slide-up sm:p-8">
                <div class="pointer-events-none absolute -right-16 -top-16 h-40 w-40 rounded-full bg-cyan-300/10 blur-3xl"></div>
                <div class="relative">
                    <span class="eyebrow">"Properties"</span>
                    <h1 class="mt-3 text-2xl font-bold tracking-tight text-white">
                        "Sound preferences"
                    </h1>
                    <p class="mt-2 text-sm leading-relaxed text-slate-400">
                        "Choose the voice used when reading articles aloud. If the preferred
                        voice is not available for an article, it falls back to "default".
                        If the article has no audio, the voice selector is disabled."
                    </p>

                    <label
                        for="preferred-voice"
                        class="mt-7 block font-mono text-[10px] uppercase tracking-widest text-slate-500"
                    >
                        "Preferred voice"
                    </label>
                    <select
                        id="preferred-voice"
                        class="mt-2 w-full rounded-xl border border-white/[0.08] bg-slate-950/80 px-4 py-3 text-sm text-slate-200 outline-none transition focus:border-cyan-300/35 disabled:cursor-not-allowed disabled:opacity-50"
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
                    <p class="mt-1 text-xs text-slate-600">
                        "Available: default, "
                        {KNOWN_VOICES.join(", ")}
                    </p>

                    <Show when=move || saved.get() fallback=|| ()>
                        <p class="mt-3 text-sm text-emerald-300">"Saved ✓"</p>
                    </Show>
                </div>
            </div>
        </div>
    }
}
