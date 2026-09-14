//! Home: search, plus the most common characters as an immediate way in.

use dioxus::prelude::*;

use crate::components::char_list::{CharGrid, CharRows};
use crate::components::search_field::SearchField;
use crate::components::shell::{Empty, Section};
use crate::data::Summary;
use crate::index::{self, use_index};
use crate::storage;

/// How many characters the home grid offers. Enough to browse, few enough to
/// scan.
const FEATURED: usize = 48;

#[component]
pub fn Home() -> Element {
    let state = use_index();
    let query = use_signal(String::new);

    // Filtering happens as you type, so the home page doubles as search.
    let results = use_memo(move || {
        let text = query();
        if text.trim().is_empty() {
            return None;
        }
        index::ready(&state).map(|idx| idx.search(&text))
    });

    let featured =
        use_memo(move || index::ready(&state).map(|idx| (idx.most_common(FEATURED), idx.len())));

    let recent = use_memo(move || {
        index::ready(&state)
            .map(|idx| idx.lookup_all(&storage::recent()))
            .unwrap_or_default()
    });

    rsx! {
        div { class: "stack", style: "padding-top: 1.5rem",
            header { class: "section",
                h1 { "Learn Every Stroke" }
                p { class: "lede",
                    "Look up a Chinese character and watch it written one stroke at a time, "
                    "in the order a native writer would use."
                }
            }

            SearchField { query }

            match results() {
                Some(hits) if hits.is_empty() => rsx! {
                    Empty { glyph: "？".to_string(), title: "No Matches".to_string(),
                        p { "Try a character like 好, pinyin like “hao”, or an English word like “good”." }
                    }
                },
                Some(hits) => rsx! {
                    Section { title: "Results".to_string(), note: hits.len().to_string(),
                        CharRows { entries: hits }
                    }
                },
                None => rsx! {
                    Browse { recent: recent(), featured: featured() }
                },
            }
        }
    }
}

/// What the page shows when nothing is being searched.
#[component]
fn Browse(recent: Vec<Summary>, featured: Option<(Vec<Summary>, usize)>) -> Element {
    rsx! {
        if !recent.is_empty() {
            Section { title: "Recently Viewed".to_string(),
                CharGrid { entries: recent }
            }
        }

        match featured {
            Some((entries, total)) => rsx! {
                Section {
                    title: "Most Common Characters".to_string(),
                    note: format!("{total} in the dictionary"),
                    CharGrid { entries }
                }
            },
            None => rsx! {
                div { class: "empty", p { "Loading the dictionary…" } }
            },
        }
    }
}
