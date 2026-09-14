//! Word rows: the headword, its reading and its meaning, as used by a course unit
//! and by a saved list.
//!
//! A word is not a character, so a row cannot lead to a character page — 你好 has
//! no strokes of its own. It leads to a search for the word instead, which the
//! dictionary answers with the characters it is written with, each of which does
//! have a stroke animation.

use dioxus::prelude::*;

use crate::components::icons;
use crate::Route;

/// One row's content, built from either a course word or a saved one.
#[derive(Clone, PartialEq)]
pub struct WordRow {
    pub word: String,
    pub pinyin: String,
    pub gloss: String,
}

impl From<&crate::hsk::Word> for WordRow {
    fn from(word: &crate::hsk::Word) -> Self {
        Self {
            word: word.word.clone(),
            pinyin: word.pinyin.clone(),
            gloss: word.gloss.clone(),
        }
    }
}

impl From<&crate::lists::ListWord> for WordRow {
    fn from(word: &crate::lists::ListWord) -> Self {
        Self {
            word: word.word.clone(),
            pinyin: word.pinyin.clone(),
            gloss: word.gloss.clone(),
        }
    }
}

/// `on_remove` turns the rows into editable ones. It carries the headword, which
/// is what identifies a word inside its list.
#[component]
pub fn WordRows(
    rows: Vec<WordRow>,
    #[props(default)] on_remove: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        div { class: "rows",
            for row in rows {
                Row { key: "{row.word}", row, on_remove }
            }
        }
    }
}

#[component]
fn Row(row: WordRow, on_remove: Option<EventHandler<String>>) -> Element {
    let headword = row.word.clone();

    rsx! {
        div { class: "row-split",
            Link { class: "row", to: Route::Search { q: row.word.clone() },
                span { class: "row-word han", aria_hidden: "true", "{row.word}" }
                div { class: "row-body",
                    div { class: "row-title",
                        span { class: "row-pinyin pinyin", "{row.pinyin}" }
                    }
                    div { class: "row-gloss", "{row.gloss}" }
                }
                span { class: "row-chevron", icons::Chevron {} }
            }
            if let Some(remove) = on_remove {
                button {
                    class: "row-action",
                    r#type: "button",
                    aria_label: "Remove {row.word}",
                    onclick: move |_| remove.call(headword.clone()),
                    icons::Clear {}
                }
            }
        }
    }
}
