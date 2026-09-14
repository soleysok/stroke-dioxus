//! `/hsk` and below — the HSK 3.0 syllabus, two ways.
//!
//! The course (`/hsk/:level`, `/hsk/:level/:unit`) is about words: 5,369 of them
//! in 512 themed units, and a unit can be taken into My Lists in one tap. The
//! character side (`/hsk/band/:band`) is about characters: every character in the
//! dictionary carries the band at which HSK introduces it, including the merged
//! 7-9 band the course does not cover.

use dioxus::prelude::*;

use crate::components::char_list::CharGrid;
use crate::components::icons;
use crate::components::shell::{Empty, RoadmapItem, Section};
use crate::components::word_list::{WordRow, WordRows};
use crate::index::{self, use_index};
use crate::lists;
use crate::{hsk, Route};

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

/// Headwords shown on a unit row before it would start wrapping on a phone.
const PREVIEW_WORDS: usize = 6;

#[component]
pub fn Hsk() -> Element {
    let state = use_index();
    let levels = hsk::summaries();

    let bands = use_memo(move || {
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
                    "The HSK 3.0 syllabus, from both ends: the course splits "
                    "{hsk::total_words()} words into {hsk::total_units()} short units, "
                    "and the dictionary tags every character with the band that introduces it."
                }
            }

            Section { title: "Course".to_string(), note: format!("{} units", hsk::total_units()),
                div { class: "levels",
                    for summary in levels.iter() {
                        Link {
                            key: "{summary.level}",
                            class: "level",
                            to: Route::HskLevel { level: summary.level },
                            span { class: "level-name", "HSK {summary.level}" }
                            span { class: "level-count num",
                                "{summary.unit_count} units · {summary.word_count} words"
                            }
                        }
                    }
                }
            }

            match bands() {
                Some(bands) => rsx! {
                    Section { title: "Characters By Band".to_string(),
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
                            done: true,
                            title: "Word Lists And Units".to_string(),
                            detail: format!(
                                "{} words in {} themed units, any of which becomes a saved list in one tap.",
                                hsk::total_words(),
                                hsk::total_units(),
                            ),
                        }
                        RoadmapItem {
                            title: "Example Sentences".to_string(),
                            detail: "A sentence or two per word, with the pinyin of every token.".to_string(),
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

/// `/hsk/:level` — one level's units, in teaching order.
#[component]
pub fn HskLevel(level: u8) -> Element {
    let lists = lists::use_lists();
    let course = use_resource(use_reactive!(|(level,)| async move {
        hsk::load_level(level).await
    }));

    // Which units are already saved, so a level can be scanned for what is left.
    let imported = use_memo(use_reactive!(|(level,)| lists
        .read()
        .imported_units(level)));

    let Some(summary) = hsk::summary(level) else {
        return rsx! { NoSuchLevel {} };
    };
    let imported = imported();

    rsx! {
        div { class: "stack", style: "padding-top: 1.5rem",
            nav { class: "crumbs", aria_label: "Breadcrumb",
                Link { to: Route::Hsk {}, "HSK" }
            }

            header { class: "section",
                h1 { "HSK {level}" }
                p { class: "lede",
                    "{summary.word_count} words in {summary.unit_count} units, grouped by theme. "
                    "Open a unit to read its words, or add the whole lesson to My Lists."
                }
            }

            match &*course.read_unchecked() {
                Some(Ok(loaded)) => rsx! {
                    Section { title: "Units".to_string(), note: loaded.units.len().to_string(),
                        div { class: "rows",
                            for unit in loaded.units.iter() {
                                Link {
                                    key: "{unit.id}",
                                    class: "row",
                                    to: Route::HskUnitPage {
                                        level,
                                        unit: unit.number().to_string(),
                                    },
                                    span { class: "row-index num", aria_hidden: "true", "{unit.number()}" }
                                    div { class: "row-body",
                                        div { class: "row-title",
                                            span { class: "row-pinyin", "{unit.title}" }
                                            if imported.iter().any(|id| id == &unit.id) {
                                                span { class: "badge badge-accent", "Added" }
                                            }
                                        }
                                        div { class: "row-preview han",
                                            "{loaded.preview(unit, PREVIEW_WORDS)}"
                                        }
                                    }
                                    span { class: "row-note num", "{unit.words.len()}" }
                                    span { class: "row-chevron", icons::Chevron {} }
                                }
                            }
                        }
                    }
                },
                Some(Err(message)) => rsx! {
                    Empty { glyph: "！".to_string(), title: format!("Could Not Load HSK {level}"),
                        p { "{message}" }
                        Link { class: "btn", to: Route::Hsk {}, "Back To HSK" }
                    }
                },
                None => rsx! {
                    div { class: "empty", p { "Loading HSK {level}…" } }
                },
            }
        }
    }
}

/// `/hsk/:level/:unit` — one lesson: its words, its characters, and the one tap
/// that turns it into a saved list.
#[component]
pub fn HskUnitPage(level: u8, unit: String) -> Element {
    let course = use_resource(use_reactive!(|(level,)| async move {
        hsk::load_level(level).await
    }));

    if hsk::summary(level).is_none() {
        return rsx! { NoSuchLevel {} };
    }

    rsx! {
        div { class: "stack", style: "padding-top: 1.5rem",
            nav { class: "crumbs", aria_label: "Breadcrumb",
                Link { to: Route::Hsk {}, "HSK" }
                span { aria_hidden: "true", "›" }
                Link { to: Route::HskLevel { level }, "HSK {level}" }
            }

            match &*course.read_unchecked() {
                Some(Ok(loaded)) => match loaded.unit(&unit) {
                    Some(found) => {
                        let (previous, next) = loaded.neighbours(found);
                        rsx! {
                            Lesson {
                                level,
                                unit: found.clone(),
                                words: loaded.unit_words(found),
                                previous: previous.cloned(),
                                next: next.cloned(),
                            }
                        }
                    },
                    None => rsx! {
                        Empty { glyph: "？".to_string(), title: "No Such Unit".to_string(),
                            p { "HSK {level} has no unit {unit}." }
                            Link { class: "btn", to: Route::HskLevel { level }, "All HSK {level} Units" }
                        }
                    },
                },
                Some(Err(message)) => rsx! {
                    Empty { glyph: "！".to_string(), title: format!("Could Not Load HSK {level}"),
                        p { "{message}" }
                        Link { class: "btn", to: Route::Hsk {}, "Back To HSK" }
                    }
                },
                None => rsx! {
                    div { class: "empty", p { "Loading HSK {level}…" } }
                },
            }
        }
    }
}

#[component]
fn Lesson(
    level: u8,
    unit: hsk::Unit,
    words: Vec<hsk::Word>,
    previous: Option<hsk::Unit>,
    next: Option<hsk::Unit>,
) -> Element {
    let state = use_index();
    let lists = lists::use_lists();
    let navigator = use_navigator();

    // The list this unit was imported into, if it has been. Recomputed from the
    // signal, so the button flips the moment the import lands.
    let unit_id = unit.id.clone();
    let saved_as = use_memo(use_reactive!(|(level, unit_id)| lists
        .read()
        .list_for_unit(level, &unit_id)
        .map(|list| (list.id.clone(), list.title.clone()))));

    let headwords: Vec<String> = words.iter().map(|w| w.word.clone()).collect();
    let entries = use_memo(use_reactive!(|(headwords,)| {
        let glyphs = hsk::characters(headwords.iter().map(String::as_str));
        index::ready(&state)
            .map(|idx| idx.lookup_all(&glyphs))
            .unwrap_or_default()
    }));

    let rows: Vec<WordRow> = words.iter().map(WordRow::from).collect();
    let count = rows.len();

    let add = {
        let unit = unit.clone();
        let words = words.clone();
        move |_| {
            let Some(id) = lists::import_unit(lists, level, &unit, &words) else {
                return;
            };
            navigator.push(Route::ListDetail { id });
        }
    };

    rsx! {
        header { class: "section",
            h1 { "{unit.title}" }
            div { class: "badges",
                span { class: "badge badge-accent", "HSK {level}" }
                span { class: "badge num", "Unit {unit.number()}" }
                span { class: "badge num", "{count} words" }
            }
            p { class: "lede",
                "The words this lesson teaches, in the order it teaches them. "
                "Tap a word to look up the characters it is written with."
            }

            match saved_as() {
                Some((id, title)) => rsx! {
                    div { class: "actions",
                        Link { class: "btn btn-accent", to: Route::ListDetail { id },
                            icons::Bookmark { filled: true }
                            "Open In My Lists"
                        }
                    }
                    p { class: "action-note", "Already saved as “{title}”." }
                },
                None => rsx! {
                    div { class: "actions",
                        button { class: "btn btn-accent", r#type: "button", onclick: add,
                            icons::Bookmark {}
                            "Add This Lesson To My Lists"
                        }
                    }
                    p { class: "action-note",
                        "Makes a list of these {count} words on this device. Adding it twice opens the same list."
                    }
                },
            }
        }

        Section { title: "Words".to_string(), note: count.to_string(),
            WordRows { rows }
        }

        if !entries().is_empty() {
            Section { title: "Characters".to_string(), note: entries().len().to_string(),
                CharGrid { entries: entries() }
            }
        }

        div { class: "pager",
            if let Some(previous) = previous {
                Link {
                    class: "btn pager-link",
                    to: Route::HskUnitPage { level, unit: previous.number().to_string() },
                    span { class: "pager-label", "Previous" }
                    span { class: "pager-title", "{previous.title}" }
                }
            }
            if let Some(next) = next {
                Link {
                    class: "btn pager-link",
                    to: Route::HskUnitPage { level, unit: next.number().to_string() },
                    span { class: "pager-label", "Next" }
                    span { class: "pager-title", "{next.title}" }
                }
            }
        }
    }
}

#[component]
fn NoSuchLevel() -> Element {
    rsx! {
        Empty { glyph: "？".to_string(), title: "No Such Level".to_string(),
            p { "The course covers HSK 1 to 6. Characters go further — the merged 7-9 band has its own page." }
            Link { class: "btn", to: Route::Hsk {}, "All Levels" }
        }
    }
}

/// `/hsk/band/:band` — every character introduced at a band, course or no course.
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
            nav { class: "crumbs", aria_label: "Breadcrumb",
                Link { to: Route::Hsk {}, "HSK" }
            }

            header { class: "section",
                h1 { "{label}" }
                p { class: "lede",
                    "Characters introduced at this band, in the order HSK teaches them. "
                    "Tap any one to watch it written."
                }
                if hsk::is_level(band) {
                    div { class: "actions",
                        Link { class: "btn", to: Route::HskLevel { level: band },
                            "The {label} Course"
                        }
                    }
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
