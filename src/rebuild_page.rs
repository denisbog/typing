//! "Rebuild the text" — reconstruct an article's original paragraph text
//! from its saved word pairs.
//!
//! The original paragraph is shown with the translation of every saved pair
//! inserted inline in place of the original words. The user picks the correct
//! original word for each inserted translation from a shuffled word bank.
//! When every insertion has been replaced the paragraph is back to its
//! original form. The page is scoped to a single article (chosen from its
//! library card).

use std::collections::BTreeMap;
#[cfg(feature = "hydrate")]
use std::time::Duration;

use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params;
use leptos_router::params::Params;

use crate::application_types::Data;
use crate::BUTTON_CLASS;

/// Route params: the article id this rebuild session is scoped to. Rebuilding
/// is always triggered per-article from the library card, so `id` is always
/// present.
#[derive(Params, PartialEq)]
pub struct RebuildParams {
    id: Option<usize>,
}

/// A single inline blank where an original word has been replaced by its
/// translation and must be restored.
#[derive(Clone, Debug)]
struct BlankSlot {
    slot_id: usize,
    /// The original word that belongs here (the correct answer).
    expected: String,
    /// The translation currently shown in that position (the hint).
    hint: String,
}

/// One selectable answer in the word bank.
#[derive(Clone, Debug)]
struct BankWord {
    id: usize,
    word: String,
}

/// Pre-computed, immutable description of one paragraph for the exercise.
#[derive(Clone, Debug)]
struct BuildData {
    article_index: usize,
    paragraph_index: usize,
    /// Original words of the paragraph, in order (including attached
    /// punctuation, matching how pairs were saved).
    words: Vec<String>,
    /// For each word position, the slot id of the pair that covers it (or
    /// None).
    word_to_slot: Vec<Option<usize>>,
    /// The blanks, indexed by slot id.
    blanks: Vec<BlankSlot>,
    /// The shuffled word bank (one entry per paired word).
    bank: Vec<BankWord>,
}

/// Deterministic Fisher–Yates shuffle (same as the Matching page) so the
/// server and hydrated client always render the bank in the same order.
fn shuffled(seed: u32, len: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..len).collect();
    let mut state = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345) | 1;
    for i in (1..len).rev() {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let j = (state as usize) % (i + 1);
        idx.swap(i, j);
    }
    idx
}

impl BuildData {
    fn build(
        article_index: usize,
        paragraph_index: usize,
        paragraph: &crate::application_types::Paragraph,
    ) -> Self {
        let words: Vec<String> = paragraph.original.split(' ').map(|s| s.to_string()).collect();
        let translation: Vec<String> = paragraph
            .translation
            .as_deref()
            .unwrap_or("")
            .split(' ')
            .map(|s| s.to_string())
            .collect();

        let mut word_to_slot: Vec<Option<usize>> = vec![None; words.len()];
        let mut blanks: Vec<BlankSlot> = Vec::new();
        if let Some(pairs) = &paragraph.pairs {
            for pair in pairs {
                let hint = pair
                    .translation
                    .iter()
                    .filter_map(|i| translation.get(*i))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" ");
                for &wi in &pair.original {
                    if wi >= words.len() || word_to_slot[wi].is_some() {
                        continue;
                    }
                    let slot_id = blanks.len();
                    blanks.push(BlankSlot {
                        slot_id,
                        expected: words[wi].clone(),
                        hint: hint.clone(),
                    });
                    word_to_slot[wi] = Some(slot_id);
                }
            }
        }

        let mut bank: Vec<BankWord> = blanks
            .iter()
            .map(|b| BankWord {
                id: b.slot_id,
                word: b.expected.clone(),
            })
            .collect();
        let seed =
            (article_index as u32).wrapping_mul(97).wrapping_add((paragraph_index as u32) * 31)
                + 0x9E37;
        let order = shuffled(seed, bank.len());
        bank = order.into_iter().map(|i| bank[i].clone()).collect();

        BuildData {
            article_index,
            paragraph_index,
            words,
            word_to_slot,
            blanks,
            bank,
        }
    }
}

/// The exercise for a single paragraph that contains saved pairs.
#[component]
fn ParagraphExercise(data: BuildData) -> impl IntoView {
    let data = RwSignal::new(data);
    let n = data.read().blanks.len();
    let (remaining, set_remaining) = signal(data.read().bank.clone());
    let (placed, set_placed) = signal(BTreeMap::<usize, usize>::new());
    let (sel_slot, set_sel_slot) = signal(None::<usize>);
    let (sel_word, set_sel_word) = signal(None::<usize>);
    let (wrong_slot, set_wrong_slot) = signal(None::<usize>);
    // The pair revealed by the Hint button (persists until placed).
    let (hint, set_hint) = signal(None::<usize>);

    let clear_wrong_after = move |slot: usize| {
        #[cfg(feature = "hydrate")]
        {
            let set_wrong_slot = set_wrong_slot;
            leptos::task::spawn_local(async move {
                let _ = wasm_timer::Delay::new(Duration::from_millis(650)).await;
                if wrong_slot.get() == Some(slot) {
                    set_wrong_slot.set(None);
                }
            });
        }
        #[cfg(not(feature = "hydrate"))]
        let _ = slot;
    };

    // Attempt to fill a blank with a chosen bank word. Only a correct match
    // sticks; a wrong one flashes red and the word returns to the bank.
    let place = move |slot_id: usize, word_id: usize| {
        let Some(word) = remaining
            .get()
            .into_iter()
            .find(|b| b.id == word_id)
        else {
            return;
        };
        let expected = data.read().blanks[slot_id].expected.clone();
        if word.word == expected {
            set_placed.update(|m| {
                m.insert(slot_id, word_id);
            });
            set_remaining.update(|v| v.retain(|b| b.id != word_id));
            set_sel_slot.set(None);
            set_sel_word.set(None);
            set_wrong_slot.set(None);
            if hint.get() == Some(slot_id) {
                set_hint.set(None);
            }
        } else {
            set_wrong_slot.set(Some(slot_id));
            set_sel_slot.set(None);
            set_sel_word.set(None);
            clear_wrong_after(slot_id);
        }
    };

    let on_slot = move |slot_id: usize| {
        if placed.get().contains_key(&slot_id) {
            return; // already restored — locked
        }
        if let Some(word_id) = sel_word.get() {
            place(slot_id, word_id);
        } else {
            set_sel_slot.set(Some(slot_id));
            set_wrong_slot.set(None);
        }
    };

    let on_word = move |word_id: usize| {
        if let Some(slot_id) = sel_slot.get() {
            place(slot_id, word_id);
        } else {
            if sel_word.get() == Some(word_id) {
                set_sel_word.set(None);
            } else {
                set_sel_word.set(Some(word_id));
            }
            set_wrong_slot.set(None);
        }
    };

    // Reveal the selected side's counterpart as a hint. Only reachable while
    // a blank or a bank word is selected (the button only shows then).
    // Persists until that pair is placed. The bank word ids equal the blank
    // slot ids, so one value covers both sides.
    let on_hint = move |_| {
        if let Some(slot_id) = sel_slot.get() {
            set_hint.set(Some(slot_id));
        } else if let Some(word_id) = sel_word.get() {
            set_hint.set(Some(word_id));
        }
    };

    // The paragraph with each paired position either a locked original word
    // or an inline translation blank waiting to be restored. Each paired word
    // is rendered as its own blank; each word not part of a pair gets its own
    // plain span so the paragraph wraps word-by-word to the end of the line.
    let paragraph_view = move || {
        let d = data.read();
        d.words
            .iter()
            .enumerate()
            .map(|(pos, w)| match d.word_to_slot[pos] {
                Some(slot_id) => {
                    if placed.get().contains_key(&slot_id) {
                        let expected = d.blanks[slot_id].expected.clone();
                        view! { <span class="rebuild-locked">{expected}</span> }
                        .into_any()
                    } else {
                        let hint_text = d.blanks[slot_id].hint.clone();
                        let class = move || {
                            if wrong_slot.get() == Some(slot_id) {
                                "rebuild-blank rebuild-blank--wrong"
                            } else if sel_slot.get() == Some(slot_id) {
                                "rebuild-blank rebuild-blank--selected"
                            } else if hint.get() == Some(slot_id) {
                                "rebuild-blank rebuild-blank--hint"
                            } else {
                                "rebuild-blank"
                            }
                        };
                        view! {
                            <button
                                type="button"
                                class=class
                                title="Restore the original word"
                                on:click=move |_| on_slot(slot_id)
                            >
                                {hint_text}
                            </button>
                        }
                        .into_any()
                    }
                }
                None => view! { <span class="rebuild-word-plain">{w.clone()}</span> }
                .into_any(),
            })
            .collect_view()
    };

    let bank_view = move || {
        remaining
            .get()
            .into_iter()
            .map(|b| {
                let id = b.id;
                let word = b.word.clone();
                let class = move || {
                    if sel_word.get() == Some(id) {
                        "rebuild-word rebuild-word--selected"
                    } else if hint.get() == Some(id) {
                        "rebuild-word rebuild-word--hint"
                    } else {
                        "rebuild-word"
                    }
                };
                view! {
                    <button type="button" class=class on:click=move |_| on_word(id)>
                        {word}
                    </button>
                }
            })
            .collect_view()
    };

    let label = move || {
        let ai = data.read().article_index;
        let pi = data.read().paragraph_index;
        format!("Article {:02} · Paragraph {:02}", ai + 1, pi + 1)
    };

    view! {
        <section class="rebuild-section">
            <div class="rebuild-head">
                <div class="rebuild-heading">
                    <span class="eyebrow">{move || label()}</span>
                    <h2 class="rebuild-title">"Restore the original text"</h2>
                    <p class="rebuild-sub">
                        "The translation is inserted below — pick the correct original word for each highlighted spot from the bank."
                    </p>
                </div>
                <div class="rebuild-head-actions">
                    <button
                        type="button"
                        class=move || {
                            if sel_slot.get().is_some() || sel_word.get().is_some() {
                                "rebuild-hint-btn"
                            } else {
                                "rebuild-hint-btn rebuild-hint-btn--hidden"
                            }
                        }

                        title="Reveal this pair as a hint"
                        on:click=on_hint
                    >
                        <span>"💡"</span>
                        <span>"Hint"</span>
                    </button>
                    <div class="rebuild-progress">
                        {move || {
                            let done = placed.get().len();
                            if done == n {
                                "Done ✓".to_string()
                            } else {
                                format!("{} / {} restored", done, n)
                            }
                        }}

                    </div>
                </div>
            </div>

            <div class="rebuild-body">
                <div class="rebuild-paragraph">{paragraph_view}</div>
                <Show when=move || { !remaining.get().is_empty() } fallback=|| ()>
                    <div class="rebuild-bank">
                        <div class="rebuild-bank-head">
                            <span class="rebuild-bank-label">"Original words"</span>
                            <span class="rebuild-bank-count">
                                {move || remaining.get().len()} " left"
                            </span>
                        </div>
                        <div class="rebuild-bank-grid">{bank_view}</div>
                    </div>
                </Show>
            </div>
        </section>
    }
}

#[component]
pub fn RebuildPage(data: ReadSignal<Data>) -> impl IntoView {
    let params = use_params::<RebuildParams>();
    // The route param never changes while this page is mounted, so this is a
    // one-time untracked read.
    let article_id = params
        .with_untracked(|param| param.as_ref().ok().and_then(|p| p.id))
        .unwrap();

    // The article being rebuilt, cloned out for rendering.
    let chosen = move || data.get().articles.get(article_id).cloned();

    // The article has at least one paragraph with saved pairs.
    let has_any = move || {
        chosen().is_some_and(|a| {
            a.paragraphs
                .iter()
                .any(|p| p.pairs.as_ref().is_some_and(|x| !x.is_empty()))
        })
    };

    // (paragraphs_with_pairs, total_pairs) for the stats panel.
    let pairs_summary = move || {
        let Some(article) = chosen() else { return (0, 0) };
        let mut paragraphs = 0;
        let mut pairs = 0;
        for paragraph in &article.paragraphs {
            if let Some(list) = paragraph.pairs.as_ref() {
                if !list.is_empty() {
                    paragraphs += 1;
                    pairs += list.len();
                }
            }
        }
        (paragraphs, pairs)
    };

    let exercise_views = move || {
        let Some(article) = chosen() else { return None };
        let mut views: Vec<AnyView> = Vec::new();
        for (pi, paragraph) in article.paragraphs.iter().enumerate() {
            let has_pairs = paragraph.pairs.as_ref().is_some_and(|x| !x.is_empty());
            if has_pairs {
                let build = BuildData::build(article_id, pi, paragraph);
                views.push(view! { <ParagraphExercise data=build/> }.into_any());
            }
        }
        Some(views)
    };

    view! {
        <Title text="Rebuild the text — Tippen"/>
        <header class="page-header">
            <div class="page-header-inner">
                <a href="/" class="back-link group">
                    <span class="back-icon">"←"</span>
                    <span class="back-link-label">"Library"</span>
                </a>
                <div class="page-title-wrap">
                    <div class="page-title-eyebrow">
                        {move || format!("Reconstruction drill · Article {:02}", article_id + 1)}
                    </div>
                    <div class="page-title">
                        {move || {
                            data.get()
                                .articles
                                .get(article_id)
                                .map(|a| a.title.clone())
                                .unwrap_or_else(|| "Rebuild the text".to_string())
                        }}

                    </div>
                </div>
            </div>
        </header>

        <div class="dash-container">
            <section class="match-intro">
                <div class="match-intro-text">
                    <span class="eyebrow">"Reconstruct the original text"</span>
                    <h1 class="match-title">
                        "Reconnect every pair of this article."
                    </h1>
                    <p class="match-sub">
                        "Each saved pair hides its original word behind its translation.
                        Rebuild the original paragraphs by picking the correct word from the
                        bank for every highlighted spot — until the whole text reads naturally
                        again. Correct choices lock in place; the article is reconstructed one
                        word at a time."
                    </p>
                </div>
                <div class="match-stats glass-panel">
                    <div class="stat">
                        <div class="stat-num">{move || pairs_summary().0}</div>
                        <div class="stat-label">"Paragraphs"</div>
                    </div>
                    <div class="stat">
                        <div class="stat-num">{move || pairs_summary().1}</div>
                        <div class="stat-label">"Pairs"</div>
                    </div>
                </div>
            </section>

            <Show
                when=has_any
                fallback=move || {
                    view! {
                        <div class="library-empty glass-panel">
                            <span class="library-empty-icon">"⧉"</span>
                            <h3 class="library-empty-title">"No saved pairs in this article"</h3>
                            <p class="library-empty-sub">
                                "This article has no saved word pairs yet. Open it,
                                switch to Pair words mode, and save at least one pair,
                                then come back here to reconstruct the originals."
                            </p>
                            <a href="/" class=BUTTON_CLASS>"← Back to library"</a>
                        </div>
                    }
                }
            >

                <section class="rebuild-exercises">
                    {move || {
                        match exercise_views() {
                            Some(v) => view! { {v} }.into_any(),
                            None => ().into_any(),
                        }
                    }}

                </section>
            </Show>
        </div>
    }
}
