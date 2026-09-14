//! `/hsk` and `/hsk/:band` — HSK bands.
//!
//! The character side is real: every character carries its HSK 3.0 band, so a
//! band can be browsed today. The course itself — word lists, units, exercises,
//! spaced review — is not built yet, and this page says so rather than
//! pretending otherwise.

use dioxus::prelude::*;

use crate::components::char_list::CharGrid;
use crate::components::shell::{Empty, RoadmapItem, Section};
use crate::index::{self, use_index};
use crate::Route;

/// HSK 3.0 has nine levels; 7, 8 and 9 share one vocabulary list and are treated
/// as a single band throughout.
const BANDS: [(u8, &str); 7] = [
    (1, "HSK 1"),
    (2, "HSK 2"),
    (3, "HSK 3"),
    (4, "HSK 4"),
    (5, "HSK 5"),
    (6, "HSK 6"),
    (7, "HSK 7-9"),
];

#[component]
pub fn Hsk() -> Element {
    let state = use_index();

    let counts = use_memo(move || {
        index::ready(&state).map(|idx| {
            BANDS
                .iter()
                .map(|&(band, label)| (band, label, idx.by_hsk_band(band).len()))
                .collect::<Vec<_>>()
        })
    });

    rsx! {
        div { class: "stack", style: "padding-top: 1.5rem",
            header { class: "section",
                h1 { "HSK" }
                p { class: "lede",
                    "Characters grouped by the band at which HSK 3.0 first introduces them. "
                    "Pick a band to browse and practise its characters."
                }
            }

            match counts() {
                Some(bands) => rsx! {
                    Section { title: "Bands".to_string(),
                        div { class: "levels",
                            for (band, label, count) in bands {
                                Link { key: "{band}", class: "level", to: Route::HskBand { band },
                                    span { class: "level-name", "{label}" }
                                    span { class: "level-count num", "{count} characters" }
                                }
                            }
                        }
                    }
                },
                None => rsx! {
                    div { class: "empty", p { "Loading bands…" } }
                },
            }

            Section { title: "Coming To This Section".to_string(),
                div { class: "card card-pad",
                    ul { class: "roadmap",
                        RoadmapItem {
                            done: true,
                            title: "Characters By Band".to_string(),
                            detail: "Every character tagged with the HSK 3.0 band that introduces it.".to_string(),
                        }
                        RoadmapItem {
                            title: "Word Lists And Units".to_string(),
                            detail: "The 5,369 HSK words split into themed units, with example sentences.".to_string(),
                        }
                        RoadmapItem {
                            title: "Practice And Review".to_string(),
                            detail: "Recognition, listening and writing drills, scheduled by Leitner box.".to_string(),
                        }
                        RoadmapItem {
                            title: "Progress Tracking".to_string(),
                            detail: "Per-unit progress and a study streak, stored on your device.".to_string(),
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn HskBand(band: u8) -> Element {
    let state = use_index();

    let label = BANDS
        .iter()
        .find(|&&(value, _)| value == band)
        .map(|&(_, label)| label);

    let entries = use_memo(use_reactive!(|(band,)| {
        index::ready(&state)
            .map(|idx| idx.by_hsk_band(band))
            .unwrap_or_default()
    }));

    let Some(label) = label else {
        return rsx! {
            Empty { glyph: "？".to_string(), title: "No Such Band".to_string(),
                p { "HSK 3.0 has bands 1 to 6 plus a combined 7-9 band." }
                Link { class: "btn", to: Route::Hsk {}, "All Bands" }
            }
        };
    };

    rsx! {
        div { class: "stack", style: "padding-top: 1.5rem",
            header { class: "section",
                h1 { "{label}" }
                p { class: "lede",
                    "Characters introduced at this band, in the order HSK teaches them. "
                    "Tap any one to watch it written."
                }
            }

            if entries().is_empty() {
                div { class: "empty", p { "Loading characters…" } }
            } else {
                Section { title: "Characters".to_string(), note: entries().len().to_string(),
                    CharGrid { entries: entries() }
                }
            }
        }
    }
}
