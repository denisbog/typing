//! "Rebuild the text" — pick an article that has saved word pairs, then
//! reconstruct the original paragraph.
//!
//! The original paragraph is shown with the translation of every saved pair
//! inserted inline in place of the original words. The user picks the correct
//! original word for each inserted translation from a shuffled word bank.
//! When every insertion has been replaced the paragraph is back to its
//! original form.

use std::collections::BTreeMap;
#[cfg(feature = "hydrate")]
use std::time::Duration;

use leptos::prelude::*;
use leptos_meta::Title;

use crate::application_types::Data;
use crate::BUTTON_CLASS;

/// A single inline blank where a whole saved pair has been replaced by its
/// translation and must be restored with its original phrase.
#[derive(Clone, Debug)]
struct BlankSlot {
    slot_id: usize,
    /// The original phrase that belongs here (the correct answer).
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
    /// None). A whole pair is one slot, never split into its words.
    word_to_slot: Vec<Option<usize>>,
    /// The blanks, one per saved pair, indexed by slot id.
    blanks: Vec<BlankSlot>,
    /// The shuffled word bank (one entry per pair's original phrase).
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
                let original_phrase: Vec<String> = pair
                    .original
                    .iter()
                    .filter_map(|i| words.get(*i))
                    .cloned()
                    .collect();
                // A pair with no original words has nothing to restore.
                if original_phrase.is_empty() {
                    continue;
                }
                let hint = pair
                    .translation
                    .iter()
                    .filter_map(|i| translation.get(*i))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" ");
                let slot_id = blanks.len();
                blanks.push(BlankSlot {
                    slot_id,
                    expected: original_phrase.join(" "),
                    hint,
                });
                for &wi in &pair.original {
                    if wi < words.len() {
                        word_to_slot[wi] = Some(slot_id);
                    }
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
    let (done, set_done) = signal(false);

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
            set_done.set(placed.get().len() == n);
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

    // The paragraph with each paired position either a locked original word
    // or an inline translation blank waiting to be restored. Each saved pair
    // is rendered once, as a single whole, at the first word that belongs to
    // it; its remaining words are skipped so the pair is never split.
    let paragraph_view = move || {
        let d = data.read();
        let mut emitted: Vec<bool> = vec![false; d.blanks.len()];
        let mut out: Vec<AnyView> = Vec::new();
        for (pos, w) in d.words.iter().enumerate() {
            match d.word_to_slot[pos] {
                Some(slot_id) => {
                    if emitted[slot_id] {
                        continue;
                    }
                    if placed.get().contains_key(&slot_id) {
                        let expected = d.blanks[slot_id].expected.clone();
                        out.push(
                            view! { <span class="rebuild-locked">{expected}</span> }
                            .into_any(),
                        );
                    } else {
                        let hint = d.blanks[slot_id].hint.clone();
                        let class = move || {
                            if wrong_slot.get() == Some(slot_id) {
                                "rebuild-blank rebuild-blank--wrong"
                            } else if sel_slot.get() == Some(slot_id) {
                                "rebuild-blank rebuild-blank--selected"
                            } else {
                                "rebuild-blank"
                            }
                        };
                        out.push(
                            view! {
                                <button
                                    type="button"
                                    class=class
                                    title="Restore the original phrase"
                                    on:click=move |_| on_slot(slot_id)
                                >
                                    {hint}
                                </button>
                            }
                            .into_any(),
                        );
                    }
                    emitted[slot_id] = true;
                }
                // Each word not part of a pair gets its own span so the
                // paragraph wraps word-by-word to the end of the line.
                None => {
                    out.push(
                        view! { <span class="rebuild-word-plain">{w.clone()}</span> }.into_any(),
                    );
                }
            }
        }
        out
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

    let is_done = move || done.get();
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

            <Show when=is_done fallback=|| view! { <div class="hidden"></div> }>
                <div class="rebuild-done">
                    <span class="rebuild-done-icon">"✓"</span>
                    "The paragraph is back to its original text."
                </div>
            </Show>
        </section>
    }
}

#[component]
pub fn RebuildPage(data: ReadSignal<Data>) -> impl IntoView {
    let (selected, set_selected) = signal(None::<usize>);

    // Articles that have at least one saved pair.
    let pickable = move || {
        data.get()
            .articles
            .iter()
            .enumerate()
            .filter_map(|(ai, a)| {
                let paragraphs_with_pairs = a
                    .paragraphs
                    .iter()
                    .filter(|p| p.pairs.as_ref().is_some_and(|x| !x.is_empty()))
                    .count();
                let pair_count: usize = a
                    .paragraphs
                    .iter()
                    .filter_map(|p| p.pairs.as_ref())
                    .map(Vec::len)
                    .sum();
                if paragraphs_with_pairs == 0 {
                    None
                } else {
                    Some((ai, a.title.clone(), paragraphs_with_pairs, pair_count))
                }
            })
            .collect::<Vec<_>>()
    };
    let total_pairs = move || pickable().iter().map(|(_, _, _, c)| c).sum::<usize>();

    // The currently selected article, cloned out for rendering.
    let chosen = move || {
        let idx = selected.get()?;
        data.get().articles.get(idx).cloned()
    };

    let picker_views = move || {
        pickable()
            .into_iter()
            .map(move |(ai, title, paras, pairs)| {
                let (headline, summary) = title
                    .split_once("||")
                    .map(|(h, s)| (h.trim().to_string(), Some(s.trim().to_string())))
                    .unwrap_or_else(|| (title, None));
                let (headline, summary) = (headline, summary);
                let is_sel = move || selected.get() == Some(ai);
                let class = move || {
                    if is_sel() {
                        "rebuild-pick rebuild-pick--selected"
                    } else {
                        "rebuild-pick"
                    }
                };
                view! {
                    <button type="button" class=class on:click=move |_| set_selected.set(Some(ai))>
                        <span class="rebuild-pick-top">
                            <span class="rebuild-pick-index">
                                {format!("Article {:02}", ai + 1)}
                            </span>
                            <span class="rebuild-pick-check">
                                {move || if is_sel() { "✓" } else { "" }}
                            </span>
                        </span>
                        <span class="rebuild-pick-body">
                            <span class="rebuild-pick-title">{headline}</span>
                            {summary
                                .map(|s| view! { <span class="rebuild-pick-summary">{s}</span> })}
                        </span>
                        <span class="rebuild-pick-meta">
                            {format!("{} paragraphs · {} pairs", paras, pairs)}
                        </span>
                    </button>
                }
            })
            .collect_view()
    };

    let exercise_views = move || {
        let article = chosen()?;
        let mut views: Vec<AnyView> = Vec::new();
        for (pi, paragraph) in article.paragraphs.iter().enumerate() {
            let has_pairs = paragraph.pairs.as_ref().is_some_and(|x| !x.is_empty());
            if has_pairs {
                let build = BuildData::build(selected.get().unwrap_or(0) as usize, pi, paragraph);
                views.push(
                    view! { <ParagraphExercise data=build/> }
                    .into_any(),
                );
            }
        }
        Some(views)
    };

    let has_any = move || !pickable().is_empty();

    view! {
        <Title text="Rebuild the text — Tippen"/>
        <header class="page-header">
            <div class="page-header-inner">
                <a href="/" class="back-link group">
                    <span class="back-icon">"←"</span>
                    <span class="back-link-label">"Library"</span>
                </a>
                <div class="page-title-wrap">
                    <div class="page-title-eyebrow">"Reconstruction drill"</div>
                    <div class="page-title">"Rebuild the text"</div>
                </div>
                <Show when=move || selected.get().is_some() fallback=|| ()>
                    <button
                        class=BUTTON_CLASS
                        title="Go back and choose another article"
                        on:click=move |_| set_selected.set(None)
                    >
                        <span class="btn-icon">"←"</span>
                        <span class="btn-label">"All articles"</span>
                    </button>
                </Show>
            </div>
        </header>

        <div class="dash-container">
            <Show
                when=has_any
                fallback=move || {
                    view! {
                        <div class="library-empty glass-panel">
                            <span class="library-empty-icon">"⧉"</span>
                            <h3 class="library-empty-title">"No articles with saved pairs"</h3>
                            <p class="library-empty-sub">
                                "Open an article, switch to Pair words mode, and save at least
                                one pair. Then come back here to reconstruct the originals."
                            </p>
                            <a href="/" class=BUTTON_CLASS>
                                "← Back to library"
                            </a>
                        </div>
                    }
                }
            >

                <Show
                    when=move || chosen().is_some()
                    fallback=move || {
                        view! {
                            <section class="rebuild-intro">
                                <div class="rebuild-intro-text">
                                    <span class="eyebrow">"Pick an article with saved pairs"</span>
                                    <h1 class="rebuild-title">
                                        "Choose which text to reconstruct."
                                    </h1>
                                    <p class="match-sub">
                                        "Each saved pair hides its original word behind the
                                        translation. Pick the right word from the bank to put the
                                        original paragraph back together, word by word."
                                    </p>
                                </div>
                                <div class="match-stats glass-panel">
                                    <div class="stat">
                                        <div class="stat-num">{move || pickable().len()}</div>
                                        <div class="stat-label">"Articles"</div>
                                    </div>
                                    <div class="stat">
                                        <div class="stat-num">{move || total_pairs()}</div>
                                        <div class="stat-label">"Pairs"</div>
                                    </div>
                                </div>
                            </section>
                            <div class="rebuild-picks">{picker_views}</div>
                        }
                    }
                >

                    {move || {
                        match exercise_views() {
                            Some(v) => {
                                view! {
                                    <section class="rebuild-exercises">
                                        <div class="rebuild-article-head">
                                            <span class="eyebrow">"Selected article"</span>
                                            <h2 class="rebuild-article-title">
                                                {move || {
                                                    chosen()
                                                        .map(|a| {
                                                            a.title
                                                                .split_once("||")
                                                                .map(|(h, _)| h.trim().to_string())
                                                                .unwrap_or(a.title)
                                                        })
                                                        .unwrap_or_default()
                                                }}

                                            </h2>
                                        </div>
                                        {v}
                                    </section>
                                }
                                    .into_any()
                            }
                            None => ().into_any(),
                        }
                    }}

                </Show>
            </Show>
        </div>
    }
}
