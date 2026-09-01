use std::collections::BTreeSet;
use std::time::Duration;

use leptos::prelude::*;
use leptos_meta::Title;

use crate::application_types::Data;
use crate::local_store;
use crate::BUTTON_CLASS;
use crate::BUTTON_PRIMARY_CLASS;

/// A single drivable pair extracted from an article paragraph.
#[derive(Clone, Debug)]
struct MatchItem {
    original: String,
    translation: String,
}

/// Deterministic Fisher–Yates shuffle so a "reset" reorders the same way
/// every time for a given run id, but differs between columns / articles.
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

/// Extract the saved pairs from a single paragraph.
fn extract_paragraph_items(paragraph: &crate::application_types::Paragraph) -> Vec<MatchItem> {
    let mut items: Vec<MatchItem> = Vec::new();
    let original_words: Vec<&str> = paragraph.original.split(' ').collect();
    let translation_words: Vec<&str> = paragraph
        .translation
        .as_deref()
        .unwrap_or("")
        .split(' ')
        .collect();
    if let Some(pairs) = &paragraph.pairs {
        for pair in pairs {
            let original: Vec<String> = pair
                .original
                .iter()
                .filter_map(|i| original_words.get(*i))
                .map(|w| w.to_string())
                .collect();
            let translation: Vec<String> = pair
                .translation
                .iter()
                .filter_map(|i| translation_words.get(*i))
                .map(|w| w.to_string())
                .collect();
            let original = original.join(" ");
            let translation = translation.join(" ");
            if original.is_empty() && translation.is_empty() {
                continue;
            }
            items.push(MatchItem {
                original,
                translation,
            });
        }
    }
    items
}

/// A single exercise: one article (or, when grouping by paragraph, one
/// paragraph of an article) with its saved pairs.
struct MatchGroup {
    article_title: String,
    eyebrow: String,
    items: Vec<MatchItem>,
}

/// Maximum number of pairs a single matching group may contain. Larger
/// collections are split into multiple groups of at most this many pairs.
const MAX_PAIRS_PER_GROUP: usize = 10;

/// Split a list of pairs into consecutive groups of at most
/// `MAX_PAIRS_PER_GROUP` pairs each, preserving order.
fn chunk_pairs(items: Vec<MatchItem>) -> Vec<Vec<MatchItem>> {
    items
        .chunks(MAX_PAIRS_PER_GROUP)
        .map(<[MatchItem]>::to_vec)
        .collect()
}

/// Extract every saved pair, grouped per article or per paragraph, and split
/// so no group holds more than `MAX_PAIRS_PER_GROUP` pairs.
///
/// When `group_by_paragraph` is false, each article becomes one group. When
/// true, each paragraph that has at least one pair becomes its own group so
/// the exercise stays small. Either way the pairs are further chunked into
/// groups of at most `MAX_PAIRS_PER_GROUP` pairs.
fn collect_groups(data: &Data, group_by_paragraph: bool) -> Vec<MatchGroup> {
    let mut out: Vec<MatchGroup> = Vec::new();
    for (ai, article) in data.articles.iter().enumerate() {
        let mut article_items: Vec<MatchItem> = Vec::new();
        for (pi, paragraph) in article.paragraphs.iter().enumerate() {
            let items = extract_paragraph_items(paragraph);
            if group_by_paragraph {
                for (ci, chunk) in chunk_pairs(items).into_iter().enumerate() {
                    out.push(MatchGroup {
                        article_title: article.title.clone(),
                        eyebrow: match (chunk.len() > 1, ci) {
                            (true, part) => format!(
                                "Article {:02} · Paragraph {:02} · Part {}",
                                ai + 1,
                                pi + 1,
                                part + 1,
                            ),
                            (false, _) => format!(
                                "Article {:02} · Paragraph {:02}",
                                ai + 1,
                                pi + 1,
                            ),
                        },
                        items: chunk,
                    });
                }
            } else {
                article_items.extend(items);
            }
        }
        if !group_by_paragraph {
            let chunks = chunk_pairs(article_items);
            let num_chunks = chunks.len();
            for (ci, chunk) in chunks.into_iter().enumerate() {
                if chunk.is_empty() {
                    continue;
                }
                out.push(MatchGroup {
                    article_title: article.title.clone(),
                    eyebrow: match (num_chunks > 1, ci) {
                        (true, part) => {
                            format!("Article {:02} · Part {}", ai + 1, part + 1)
                        }
                        (false, _) => format!("Article {:02}", ai + 1),
                    },
                    items: chunk,
                });
            }
        }
    }
    out
}

#[component]
fn ArticleMatchSection(
    group_index: usize,
    eyebrow: String,
    title: String,
    items: Vec<MatchItem>,
    run_id: ReadSignal<u32>,
) -> impl IntoView {
    let n = items.len();
    let (left_order, set_left_order) = signal(
        shuffled(run_id.get().wrapping_add(101 + group_index as u32 * 7), n),
    );
    let (right_order, set_right_order) = signal(
        shuffled(run_id.get().wrapping_add(599 + group_index as u32 * 11), n),
    );
    let (matched, set_matched) = signal(BTreeSet::<usize>::new());
    let (sel_left, set_sel_left) = signal(None::<usize>);
    let (sel_right, set_sel_right) = signal(None::<usize>);
    let (wrong, set_wrong) = signal(None::<(usize, usize)>);
    // The pair revealed by the Hint button (persists until matched/reset).
    let (hint, set_hint) = signal(None::<usize>);

    // Every time the run id changes (global "Reset" button) we re-shuffle
    // and clear all matching state for this section.
    Effect::new(move |_| {
        let run = run_id.get();
        set_left_order.set(shuffled(run.wrapping_add(101 + group_index as u32 * 7), n));
        set_right_order.set(shuffled(run.wrapping_add(599 + group_index as u32 * 11), n));
        set_matched.set(BTreeSet::new());
        set_sel_left.set(None);
        set_sel_right.set(None);
        set_wrong.set(None);
        set_hint.set(None);
    });

    let clear_wrong_after = move |run: u32| {
        #[cfg(feature = "hydrate")]
        {
            let set_wrong = set_wrong;
            leptos::task::spawn_local(async move {
                let _ = wasm_timer::Delay::new(Duration::from_millis(650)).await;
                if run_id.get() == run {
                    set_wrong.set(None);
                }
            });
        }
        #[cfg(not(feature = "hydrate"))]
        let _ = run;
    };

    let on_left = move |lp: usize| {
        let left_pair = left_order.get()[lp];
        if matched.get().contains(&left_pair) {
            return;
        }
        if let Some(rp) = sel_right.get() {
            let right_pair = right_order.get()[rp];
            if left_pair == right_pair {
                set_matched.update(|m| {
                    m.insert(left_pair);
                });
                if hint.get() == Some(left_pair) {
                    set_hint.set(None);
                }
                set_sel_left.set(None);
                set_sel_right.set(None);
                set_wrong.set(None);
            } else {
                set_wrong.set(Some((lp, rp)));
                set_sel_left.set(None);
                set_sel_right.set(None);
                let run = run_id.get();
                clear_wrong_after(run);
            }
        } else {
            set_sel_left.set(Some(lp));
            set_wrong.set(None);
        }
    };

    let on_right = move |rp: usize| {
        let right_pair = right_order.get()[rp];
        if matched.get().contains(&right_pair) {
            return;
        }
        if let Some(lp) = sel_left.get() {
            let left_pair = left_order.get()[lp];
            if left_pair == right_pair {
                set_matched.update(|m| {
                    m.insert(right_pair);
                });
                if hint.get() == Some(right_pair) {
                    set_hint.set(None);
                }
                set_sel_left.set(None);
                set_sel_right.set(None);
                set_wrong.set(None);
            } else {
                set_wrong.set(Some((lp, rp)));
                set_sel_left.set(None);
                set_sel_right.set(None);
                let run = run_id.get();
                clear_wrong_after(run);
            }
        } else {
            set_sel_right.set(Some(rp));
            set_wrong.set(None);
        }
    };

    let items_left = items.clone();
    let left_cards = move || {
        left_order
            .get()
            .into_iter()
            .enumerate()
            .map(|(pos, pair)| {
                let item = items_left[pair].clone();
                let text = item.original;
                let class = move || {
                    if wrong.get().is_some_and(|(l, _)| l == pos) {
                        "match-card match-card--wrong"
                    } else if matched.get().contains(&pair) {
                        "match-card match-card--matched"
                    } else if hint.get() == Some(pair) {
                        "match-card match-card--hint"
                    } else if sel_left.get() == Some(pos) {
                        "match-card match-card--selected"
                    } else {
                        "match-card"
                    }
                };
                let is_disabled = move || {
                    matched.get().contains(&pair) || wrong.get().is_some_and(|(l, _)| l == pos)
                };
                view! {
                    <button
                        type="button"
                        class=class
                        disabled=is_disabled
                        on:click=move |_| on_left(pos)
                    >
                        {text}
                    </button>
                }
            })
            .collect_view()
    };

    let items_right = items.clone();
    let right_cards = move || {
        right_order
            .get()
            .into_iter()
            .enumerate()
            .map(|(pos, pair)| {
                let item = items_right[pair].clone();
                let text = item.translation;
                let class = move || {
                    if wrong.get().is_some_and(|(_, r)| r == pos) {
                        "match-card match-card--wrong"
                    } else if matched.get().contains(&pair) {
                        "match-card match-card--matched"
                    } else if hint.get() == Some(pair) {
                        "match-card match-card--hint"
                    } else if sel_right.get() == Some(pos) {
                        "match-card match-card--selected"
                    } else {
                        "match-card"
                    }
                };
                let is_disabled = move || {
                    matched.get().contains(&pair) || wrong.get().is_some_and(|(_, r)| r == pos)
                };
                view! {
                    <button
                        type="button"
                        class=class
                        disabled=is_disabled
                        on:click=move |_| on_right(pos)
                    >
                        {text}
                    </button>
                }
            })
            .collect_view()
    };

    // Reveal the selected card's counterpart as a hint. Only reachable while a
    // card is selected (the button only shows then). Persists until the pair
    // is matched or the run is reset (no timeout).
    let on_hint = move |_| {
        let pair = if let Some(lp) = sel_left.get() {
            Some(left_order.get()[lp])
        } else if let Some(rp) = sel_right.get() {
            Some(right_order.get()[rp])
        } else {
            None
        };
        if let Some(p) = pair {
            set_hint.set(Some(p));
        }
    };

    let is_done = move || matched.get().len() == n && n > 0;

    view! {
        <section class="match-section">
            <div class="match-section-head">
                <div class="match-section-heading">
                    <span class="eyebrow">{eyebrow.clone()}</span>
                    <h2 class="match-section-title">{title.clone()}</h2>
                </div>
                <div class="match-section-actions">
                    <Show
                        when=move || sel_left.get().is_some() || sel_right.get().is_some()
                        fallback=|| ()
                    >
                        <button
                            type="button"
                            class="match-hint-btn"
                            title="Reveal this pair as a hint"
                            on:click=on_hint
                        >
                            <span>"💡"</span>
                            <span>"Hint"</span>
                        </button>
                    </Show>
                    <div class="match-section-progress">
                        {move || {
                            let done = matched.get().len();
                            if done == n {
                                "Done ✓".to_string()
                            } else {
                                format!("{} / {} matched", done, n)
                            }
                        }}
                    </div>
                </div>
            </div>

            <div class="match-row">
                <div class="match-col">
                    <div class="match-col-head">"Original"</div>
                    <div class="match-col-grid">{left_cards}</div>
                </div>
                <div class="match-col">
                    <div class="match-col-head">"Translation"</div>
                    <div class="match-col-grid">{right_cards}</div>
                </div>
            </div>

            <Show when=is_done fallback=|| view! { <div class="hidden"></div> }>
                <div class="match-done">
                    <span class="match-done-icon">"✓"</span>
                    "Every pair in this article is matched."
                </div>
            </Show>
        </section>
    }
}

#[component]
pub fn MatchingPage(data: ReadSignal<Data>) -> impl IntoView {
    let (run_id, set_run_id) = signal(0u32);
    let (flash_reset, set_flash_reset) = signal(false);
    // Grouping preference comes from the Properties page (localStorage). On
    // the server it is always false, so SSR always renders the article view;
    // the client hydrate step picks up the real preference reactively.
    #[cfg(feature = "hydrate")]
    let (group_by_paragraph, _set_group_by_paragraph) =
        signal(local_store::group_matching_by_paragraph());
    #[cfg(not(feature = "hydrate"))]
    let (group_by_paragraph, _set_group_by_paragraph) = signal(false);

    let groups = move || collect_groups(&data.get(), group_by_paragraph.get());

    let total_pairs = move || {
        groups()
            .iter()
            .map(|g| g.items.len())
            .sum::<usize>()
    };
    let total_articles = move || groups().len();
    let has_any = move || total_articles() > 0;
    let section_label = move || {
        if group_by_paragraph.get() {
            "Paragraphs"
        } else {
            "Articles"
        }
    };

    let sections = move || {
        groups()
            .into_iter()
            .enumerate()
            .map(move |(gi, group)| {
                let eyebrow = group.eyebrow.clone();
                let title = group.article_title.clone();
                let items = group.items.clone();
                view! {
                    <ArticleMatchSection
                        group_index=gi
                        eyebrow
                        title
                        items
                        run_id
                    />
                }
            })
            .collect_view()
    };

    let on_reset = move |_| {
        set_run_id.update(|n| *n = n.wrapping_add(1));
        set_flash_reset.set(true);
        #[cfg(feature = "hydrate")]
        leptos::task::spawn_local(async move {
            let _ = wasm_timer::Delay::new(Duration::from_millis(900)).await;
            set_flash_reset.set(false);
        });
        #[cfg(not(feature = "hydrate"))]
        let _ = ();
    };

    view! {
        <Title text="Match the pairs — Tippen"/>
        <header class="page-header">
            <div class="page-header-inner">
                <a href="/" class="back-link group">
                    <span class="back-icon">"←"</span>
                    <span class="back-link-label">"Library"</span>
                </a>
                <div class="page-title-wrap">
                    <div class="page-title-eyebrow">"Vocabulary drill"</div>
                    <div class="page-title">"Match the pairs"</div>
                </div>
                <button
                    class=BUTTON_PRIMARY_CLASS
                    title="Reset and start matching from the beginning"
                    on:click=on_reset
                >
                    <span class="btn-icon">"↻"</span>
                    <span class="btn-label">"Reset"</span>
                </button>
            </div>
        </header>

        <div class="dash-container">
            <section class="match-intro">
                <div class="match-intro-text">
                    <span class="eyebrow">"Match each word to its translation"</span>
                    <h1 class="match-title">
                        "Reconnect every pair across your library."
                    </h1>
                    <p class="match-sub">
                        "Pick a card on the left and the matching card on the right. Correct
                        pairs lock in green; wrong guesses shake red. Pairs are grouped by
                        article (or by paragraph, if you switched it on in Properties), in
                        groups of at most 10 — press Reset to shuffle everything and start over."
                    </p>
                </div>
                <div class="match-stats glass-panel">
                    <div class="stat">
                        <div class="stat-num">{move || total_articles()}</div>
                        <div class="stat-label">{section_label}</div>
                    </div>
                    <div class="stat">
                        <div class="stat-num">{move || total_pairs()}</div>
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
                            <h3 class="library-empty-title">"No saved pairs yet"</h3>
                            <p class="library-empty-sub">
                                "Open an article, switch to Pair words mode, and save at least
                                one pair. Then come back here to match them."
                            </p>
                            <a href="/" class=BUTTON_CLASS>"← Back to library"</a>
                        </div>
                    }
                }
            >

                <div class="match-sections">{sections}</div>
            </Show>

            <Show when=move || flash_reset.get() fallback=|| ()>
                <div class="match-reset-flash">
                    "↻ Shuffled — start matching again"
                </div>
            </Show>
        </div>
    }
}
