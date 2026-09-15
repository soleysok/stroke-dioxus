//! `/draw` — write a character to look it up.
//!
//! For the character in front of you that you cannot type: you know what it looks
//! like, not how it is read. Write it, and the characters it resembles appear
//! below; tapping one opens its stroke order.
//!
//! Candidates are the same tiles every other list of characters uses, so they
//! carry a reading and lead where a character always leads.

use dioxus::prelude::*;

use crate::components::char_list::CharGrid;
use crate::components::shell::Section;
use crate::components::stroke_pad::StrokePad;
use crate::index::{self, use_index};
use crate::recognize;
use crate::storage;

/// How long after the last stroke to wait before matching. Long enough not to
/// interrupt somebody mid-character, short enough to feel like a reaction.
const SETTLE_MS: u32 = 450;

/// Candidates offered. Two rows of the grid on a phone, which is as far as
/// anybody reads before writing it again.
const CANDIDATES: usize = 8;

#[component]
pub fn Draw() -> Element {
    let state = use_index();
    // A fresh pad every time the page is mounted, including on the way back from
    // a candidate: the character that was just looked up has been read, and the
    // next one is written from scratch.
    let strokes = use_signal(Vec::new);

    // Being here is the end of whatever the pad last opened, so the note it left
    // for that character page is spent.
    use_hook(storage::forget_from_draw);

    // Keyed on the strokes, so a new one cancels this and starts it again — and
    // with it the pause it opens with, which is what debounces the matching while
    // somebody is still writing.
    let matched = use_resource(move || async move {
        let drawn = strokes();
        if drawn.is_empty() {
            return Ok(Vec::new());
        }
        gloo_timers::future::TimeoutFuture::new(SETTLE_MS).await;
        let templates = recognize::templates().await?;
        Ok::<_, String>(templates.recognize(&drawn, CANDIDATES))
    });

    // Everything the pad can recognise is in the index; the lookup is what puts a
    // reading under each tile. Candidates from a finished match stay on screen
    // while the next one runs, so the grid does not blink on every stroke.
    let candidates = use_memo(move || {
        let (Some(idx), Some(Ok(found))) = (index::ready(&state), &*matched.read_unchecked())
        else {
            return Vec::new();
        };
        idx.lookup_all(found)
    });

    let failure = use_memo(move || match &*matched.read_unchecked() {
        Some(Err(message)) => Some(message.clone()),
        _ => None,
    });

    let settling = use_memo(move || matched.state()() != UseResourceState::Ready);
    let empty = use_memo(move || strokes.read().is_empty());

    rsx! {
        div { class: "stack", style: "padding-top: 1.5rem",
            header { class: "section",
                h1 { "Draw" }
                p { class: "lede",
                    "Write a character with a finger or a stylus to look it up. Stroke "
                    "order helps, but getting it wrong only costs you a place or two in "
                    "the list."
                }
                p { class: "sr-only",
                    "Writing needs a pointing device. To look a character up from the "
                    "keyboard, use the search field on the home page instead."
                }
            }

            div { class: "draw",
                StrokePad { strokes }

                // Dimmed rather than emptied while the next match runs: the
                // candidates below are a beat out of date, not gone.
                div {
                    class: "draw-results",
                    "data-settling": settling().to_string(),
                    // Candidates arrive without anything being focused, so they
                    // have to announce themselves.
                    aria_live: "polite",
                    if let Some(message) = failure() {
                        p { class: "draw-note", "{message}" }
                    } else if empty() {
                        p { class: "draw-note", "Candidates appear here as you write." }
                    } else if !candidates().is_empty() {
                        Section { title: "Looks Like".to_string(),
                            // Noted as it is tapped, so the character page can
                            // offer the way back to the pad.
                            CharGrid {
                                entries: candidates(),
                                on_pick: storage::mark_from_draw,
                            }
                        }
                    } else if settling() {
                        p { class: "draw-note", "Looking…" }
                    } else {
                        p { class: "draw-note",
                            "Nothing matched that. Clear the pad and try again, a little "
                            "larger and closer to the guides."
                        }
                    }
                }
            }

            p { class: "draw-scope",
                "Matching compares what you drew against Make Me a Hanzi's stroke "
                "medians — the same centerlines the stroke animation is drawn along. "
                "It covers every character this app carries stroke data for, which is "
                "all of HSK 1 to 7-9."
            }
        }
    }
}
