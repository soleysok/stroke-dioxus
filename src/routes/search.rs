//! `/search?q=…` — a shareable, reloadable search result.

use dioxus::prelude::*;

use crate::components::char_list::CharRows;
use crate::components::search_field::SearchField;
use crate::components::shell::{Empty, Section};
use crate::index::{self, use_index};

#[component]
pub fn Search(q: String) -> Element {
    let state = use_index();

    // Seeded from the URL, then owned by the field so typing feels immediate.
    // Re-seeded whenever the query string changes underneath us, which happens
    // when a search is submitted from another page.
    let mut query = use_signal(|| q.clone());
    use_effect(use_reactive!(|(q,)| query.set(q)));

    let results = use_memo(move || {
        let text = query();
        if text.trim().is_empty() {
            return None;
        }
        index::ready(&state).map(|idx| idx.search(&text))
    });

    rsx! {
        div { class: "stack", style: "padding-top: 1.5rem",
            h1 { "Search" }
            SearchField { query, autofocus: true }

            match results() {
                None => rsx! {
                    Empty { glyph: "字".to_string(), title: "Search The Dictionary".to_string(),
                        p {
                            "Search by character, by pinyin with or without tones, "
                            "or by an English meaning."
                        }
                    }
                },
                Some(hits) if hits.is_empty() => rsx! {
                    Empty { glyph: "？".to_string(), title: "No Matches".to_string(),
                        p { "Nothing matched “{query}”. Try a shorter query or a single character." }
                    }
                },
                Some(hits) => rsx! {
                    Section { title: "Results".to_string(), note: hits.len().to_string(),
                        CharRows { entries: hits }
                    }
                },
            }
        }
    }
}
