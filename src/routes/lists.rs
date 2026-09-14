//! `/lists` — saved and recently viewed characters.
//!
//! Named "My Lists" for parity with the React app's roadmap, but there is only one
//! list so far: what you saved. Both lists live in `localStorage`, so there is no
//! account and nothing to sign in to.

use dioxus::prelude::*;

use crate::components::char_list::CharGrid;
use crate::components::shell::{Empty, RoadmapItem, Section};
use crate::index::{self, use_index};
use crate::storage;
use crate::Route;

#[component]
pub fn Lists() -> Element {
    let state = use_index();

    // Re-read on mount rather than caching: the lists change on other pages.
    let saved = use_memo(move || {
        index::ready(&state)
            .map(|idx| idx.lookup_all(&storage::saved()))
            .unwrap_or_default()
    });
    let recent = use_memo(move || {
        index::ready(&state)
            .map(|idx| idx.lookup_all(&storage::recent()))
            .unwrap_or_default()
    });

    rsx! {
        div { class: "stack", style: "padding-top: 1.5rem",
            header { class: "section",
                h1 { "My Lists" }
                p { class: "lede",
                    "Characters you saved, and where you have been. Kept on this device — "
                    "there is no account to create."
                }
            }

            if saved().is_empty() {
                Empty { glyph: "空".to_string(), title: "Nothing Saved Yet".to_string(),
                    p { "Tap Save on any character and it will appear here." }
                    Link { class: "btn btn-accent", to: Route::Home {}, "Browse Characters" }
                }
            } else {
                Section { title: "Saved".to_string(), note: saved().len().to_string(),
                    CharGrid { entries: saved() }
                }
            }

            if !recent().is_empty() {
                Section { title: "Recently Viewed".to_string(), note: recent().len().to_string(),
                    CharGrid { entries: recent() }
                }
            }

            Section { title: "Coming To This Section".to_string(),
                div { class: "card card-pad",
                    ul { class: "roadmap",
                        RoadmapItem {
                            done: true,
                            title: "Saved Characters".to_string(),
                            detail: "One list, stored in this browser.".to_string(),
                        }
                        RoadmapItem {
                            title: "Named Lists".to_string(),
                            detail: "Several lists you can name, reorder and study as a set.".to_string(),
                        }
                        RoadmapItem {
                            title: "Accounts And Sync".to_string(),
                            detail: "Sign in to carry lists and progress between devices.".to_string(),
                        }
                        RoadmapItem {
                            title: "Import And Export".to_string(),
                            detail: "Move a list in or out as plain text.".to_string(),
                        }
                    }
                }
            }
        }
    }
}
