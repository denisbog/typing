use std::collections::BTreeSet;
use std::time::Duration;

use leptos::prelude::*;
use leptos_meta::Title;

use crate::application_types::Data;
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

/// Extract every saved pair across all articles, grouped per article.
///
/// Returns `(article_index, title, items)` for each article that has at
/// least one pair.
fn collect_groups(data: &Data) -> Vec<(usize, String, Vec<MatchItem>)> {
    data.articles
        .iter()
        .enumerate()
        .filter_map(|(ai, article)| {
            let mut items: Vec<MatchItem> = Vec::new();
            for paragraph in &article.paragraphs {
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
            }
            if items.is_empty() {
                None
            } else {
                Some((ai, article.title.clone(), items))
            }
        })
        .collect()
}

#[component]
fn ArticleMatchSection(
    article_index: usize,
    title: String,
    items: Vec<MatchItem>,
    run_id: ReadSignal<u32>,
) -> impl IntoView {
    let n = items.len();
    let (left_order, set_left_order) = signal(
        shuffled(run_id.get().wrapping_add(101 + article_index as u32 * 7), n),
    );
    let (right_order, set_right_order) = signal(
        shuffled(run_id.get().wrapping_add(599 + article_index as u32 * 11), n),
    );
    let (matched, set_matched) = signal(BTreeSet::<usize>::new());
    let (sel_left, set_sel_left) = signal(None::<usize>);
    let (sel_right, set_sel_right) = signal(None::<usize>);
    let (wrong, set_wrong) = signal(None::<(usize, usize)>);

    // Every time the run id changes (global "Reset" button) we re-shuffle
    // and clear all matching state for this article's section.
    Effect::new(move |_| {
        let run = run_id.get();
        set_left_order.set(shuffled(run.wrapping_add(101 + article_index as u32 * 7), n));
        set_right_order.set(shuffled(run.wrapping_add(599 + article_index as u32 * 11), n));
        set_matched.set(BTreeSet::new());
        set_sel_left.set(None);
        set_sel_right.set(None);
        set_wrong.set(None);
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

    let is_done = move || matched.get().len() == n && n > 0;

    view! {
        <section class="match-section">
            <div class="match-section-head">
                <div class="match-section-heading">
                    <span class="eyebrow">{format!("Article {:02}", article_index + 1)}</span>
                    <h2 class="match-section-title">{title.clone()}</h2>
                </div>
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

    let groups = move || collect_groups(&data.get());

    let total_pairs = move || {
        groups()
            .iter()
            .map(|(_, _, items)| items.len())
            .sum::<usize>()
    };
    let total_articles = move || groups().len();
    let has_any = move || total_articles() > 0;

    let sections = move || {
        groups()
            .into_iter()
            .map(move |(ai, title, items)| {
                view! {
                    <ArticleMatchSection article_index=ai title items run_id/>
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
                        article — press Reset to shuffle everything and start over."
                    </p>
                </div>
                <div class="match-stats glass-panel">
                    <div class="stat">
                        <div class="stat-num">{move || total_articles()}</div>
                        <div class="stat-label">"Articles"</div>
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
