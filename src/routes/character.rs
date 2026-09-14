//! `/character/好` — the stroke animation, the reading, and the meaning.

use dioxus::prelude::*;

use crate::components::char_list::CharGrid;
use crate::components::icons;
use crate::components::shell::{Empty, Section};
use crate::components::stroke_player::{StrokeFrame, StrokePlayer};
use crate::data::{self, Character};
use crate::history;
use crate::index::{self, use_index};
use crate::Route;
use crate::{speech, storage, url};

/// How many other characters to offer under "same radical" and "same reading".
const RELATED: usize = 24;

#[component]
pub fn CharacterPage(glyph: String) -> Element {
    let state = use_index();
    let entry = history::use_entry();
    // The route segment may arrive percent-encoded or literal depending on how it
    // was navigated to; `decode_char` handles both.
    let target = use_memo(use_reactive!(|(glyph,)| url::decode_char(&glyph)));

    // Settled once per character rather than per render, because reading the note
    // the pad left also clears a stale one.
    let back = use_memo(move || match target() {
        Some(glyph) if storage::came_from_draw(glyph) => Back::Draw,
        _ if entry.app_has_pushed() => Back::Previous,
        _ => Back::Browse,
    });

    let character = use_resource(move || async move {
        let Some(glyph) = target() else {
            return Err("That is not a character.".to_string());
        };
        let Some(idx) = index::ready(&state) else {
            // Wait for the index: it decides whether the character is vendored or
            // needs the CDN, and supplies metadata in the latter case.
            return Err("loading".to_string());
        };
        let summary = idx.get(glyph).cloned();
        data::load_character(glyph, &idx, summary.as_ref()).await
    });

    // Record the visit once the character is known to be real.
    use_effect(move || {
        if let Some(Ok(loaded)) = character.read().as_ref() {
            storage::push_recent(loaded.glyph);
        }
    });

    rsx! {
        div { class: "stack", style: "padding-top: 0.75rem",
            BackLink { back: back() }

            match &*character.read_unchecked() {
                Some(Ok(loaded)) => rsx! { Detail { character: loaded.as_ref().clone() } },
                Some(Err(message)) if message == "loading" => rsx! { Loading {} },
                Some(Err(message)) => rsx! {
                    Empty { glyph: "？".to_string(), title: "Not Available".to_string(),
                        p { "{message}" }
                    }
                },
                None => rsx! { Loading {} },
            }
        }
    }
}

#[component]
fn Loading() -> Element {
    rsx! {
        div { class: "empty", p { "Loading stroke data…" } }
    }
}

/// Where the Back control leads. A character can be arrived at from anywhere,
/// including from nowhere — a shared link opened in a new tab — so the control
/// says where it goes rather than promising an undo.
#[derive(Clone, Copy, PartialEq)]
enum Back {
    /// The handwriting pad, for a character that was written to be found.
    Draw,
    /// The page the app came from, whichever it was: the browser knows the entry
    /// behind this one and we do not.
    Previous,
    /// The app has not navigated anywhere yet — a shared link opened cold — so
    /// there is nothing of ours to go back to. Browsing is the way out.
    Browse,
}

/// Back, in the Apple sense: top-leading, a chevron, and the name of where it
/// leads. Rendered above the character so it is the first thing a thumb finds.
#[component]
fn BackLink(back: Back) -> Element {
    let navigator = use_navigator();

    match back {
        Back::Draw => rsx! {
            Link { class: "backlink", to: Route::Draw {}, aria_label: "Back to Draw",
                icons::ChevronBack {}
                span { "Draw" }
            }
        },
        Back::Previous => rsx! {
            button {
                class: "backlink",
                r#type: "button",
                onclick: move |_| navigator.go_back(),
                icons::ChevronBack {}
                span { "Back" }
            }
        },
        Back::Browse => rsx! {
            Link { class: "backlink", to: Route::Home {}, aria_label: "Back to Browse",
                icons::ChevronBack {}
                span { "Browse" }
            }
        },
    }
}

#[component]
fn Detail(character: Character) -> Element {
    let state = use_index();
    let glyph = character.glyph;

    let mut saved = use_signal(move || storage::is_saved(glyph));
    use_effect(use_reactive!(|(glyph,)| saved.set(storage::is_saved(glyph))));

    let reading = character.pinyin.join(" · ");
    let senses = character.senses();
    let components = character.components();

    // Other characters built on the same radical, in frequency order. Restricted
    // to the ranked slice so the suggestions stay characters worth learning.
    let radical = character.radical;
    let same_radical = use_memo(use_reactive!(|(radical, glyph)| {
        index::ready(&state)
            .map(|idx| {
                idx.most_common(idx.ranked())
                    .into_iter()
                    .filter(|e| e.radical == radical && e.glyph != glyph)
                    .take(RELATED)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }));

    rsx! {
        div { class: "detail",
            // The animation leads, because it is the reason to be here.
            div { class: "stack",
                StrokePlayer { glyph, graphics: character.graphics.clone() }
            }

            div { class: "stack",
                header { class: "section",
                    div { class: "detail-headline",
                        span { class: "detail-glyph", aria_hidden: "true", "{glyph}" }
                        div {
                            if reading.is_empty() {
                                span { class: "detail-pinyin", "—" }
                            } else {
                                h1 { class: "detail-pinyin pinyin", "{reading}" }
                            }
                            div { class: "badges", style: "margin-top: 0.375rem",
                                if let Some(band) = character.hsk_label() {
                                    span { class: "badge badge-accent", "{band}" }
                                }
                                span { class: "badge num", "{character.stroke_count()} strokes" }
                            }
                        }
                    }

                    div { style: "display: flex; gap: 0.5rem; flex-wrap: wrap",
                        if speech::is_supported() {
                            button {
                                class: "btn",
                                r#type: "button",
                                onclick: move |_| speech::speak(&glyph.to_string()),
                                icons::Speaker {}
                                "Pronounce"
                            }
                        }
                        button {
                            class: if saved() { "btn btn-accent" } else { "btn" },
                            r#type: "button",
                            aria_pressed: saved().to_string(),
                            onclick: move |_| saved.set(storage::toggle_saved(glyph)),
                            icons::Bookmark { filled: saved() }
                            if saved() { "Saved" } else { "Save" }
                        }
                    }
                }

                if !senses.is_empty() {
                    Section { title: "Meaning".to_string(),
                        ol { class: "senses",
                            for (i, sense) in senses.iter().enumerate() {
                                li { key: "{i}", class: "sense", "{sense}" }
                            }
                        }
                    }
                }

                Section { title: "Details".to_string(),
                    div { class: "card card-pad facts",
                        div { class: "fact",
                            span { class: "fact-label", "Radical" }
                            span { class: "fact-value",
                                span { class: "han", "{character.radical}" }
                            }
                        }
                        div { class: "fact",
                            span { class: "fact-label", "Strokes" }
                            span { class: "fact-value num", "{character.stroke_count()}" }
                        }
                        if !components.is_empty() {
                            div { class: "fact",
                                span { class: "fact-label", "Built From" }
                                span { class: "fact-value",
                                    for (i, part) in components.iter().enumerate() {
                                        Fragment { key: "{i}",
                                            if i > 0 {
                                                span { class: "fact-join", aria_hidden: "true", "+" }
                                            }
                                            span { class: "han", "{part}" }
                                        }
                                    }
                                }
                                if let Some(arrangement) = character.arrangement() {
                                    span { class: "fact-note", "{arrangement}" }
                                }
                            }
                        }
                    }
                }

                Section {
                    title: "Stroke Order".to_string(),
                    note: format!("{} steps", character.stroke_count()),
                    div { class: "filmstrip",
                        for step in 1..=character.stroke_count() {
                            StrokeFrame {
                                key: "{step}",
                                graphics: character.graphics.clone(),
                                scope: format!("frame-{:x}-{step}", glyph as u32),
                                upto: step,
                            }
                        }
                    }
                }

                if !same_radical().is_empty() {
                    Section {
                        title: format!("Shares The Radical {}", character.radical),
                        CharGrid { entries: same_radical() }
                    }
                }
            }
        }
    }
}
