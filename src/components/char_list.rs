//! The two ways characters are listed: a compact grid of tiles for browsing, and
//! full-width rows for search results where the gloss matters.

use dioxus::prelude::*;

use crate::components::icons;
use crate::data::Summary;
use crate::url;
use crate::Route;

fn route_for(glyph: char) -> Route {
    Route::CharacterPage {
        glyph: url::encode(&glyph.to_string()),
    }
}

#[component]
pub fn CharGrid(
    entries: Vec<Summary>,
    /// Called with the character a tile leads to, as it is tapped. The Draw page
    /// uses it to note where the character page is about to be arrived from.
    #[props(default)]
    on_pick: Option<EventHandler<char>>,
) -> Element {
    rsx! {
        div { class: "char-grid",
            for entry in entries {
                Link {
                    key: "{entry.glyph}",
                    class: "char-tile",
                    to: route_for(entry.glyph),
                    aria_label: "{entry.glyph}, {entry.pinyin}",
                    onclick: {
                        let glyph = entry.glyph;
                        move |_| {
                            if let Some(pick) = on_pick {
                                pick.call(glyph);
                            }
                        }
                    },
                    span { class: "char-tile-glyph", aria_hidden: "true", "{entry.glyph}" }
                    span { class: "char-tile-pinyin pinyin", "{entry.pinyin}" }
                }
            }
        }
    }
}

#[component]
pub fn CharRows(entries: Vec<Summary>) -> Element {
    rsx! {
        div { class: "rows",
            for entry in entries {
                Link { key: "{entry.glyph}", class: "row", to: route_for(entry.glyph),
                    span { class: "row-glyph", aria_hidden: "true", "{entry.glyph}" }
                    div { class: "row-body",
                        div { class: "row-title",
                            span { class: "row-pinyin pinyin", "{entry.pinyin}" }
                            if let Some(band) = entry.hsk_label() {
                                span { class: "badge badge-accent", "{band}" }
                            }
                            span { class: "badge num", "{entry.stroke_count}" }
                        }
                        div { class: "row-gloss", "{entry.gloss}" }
                    }
                    span { class: "row-chevron", icons::Chevron {} }
                }
            }
        }
    }
}
