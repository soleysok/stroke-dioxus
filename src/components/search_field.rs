//! The search field.
//!
//! Typing filters in place on whichever page hosts the field; submitting routes to
//! `/search` so the query is shareable and survives a reload.

use dioxus::prelude::*;

use crate::components::icons;
use crate::url;
use crate::Route;

#[component]
pub fn SearchField(
    /// Live query, owned by the host page so it can filter its own content.
    query: Signal<String>,
    #[props(default = false)] autofocus: bool,
) -> Element {
    let navigator = use_navigator();

    let submit = move |_| {
        let text = query().trim().to_string();
        if text.is_empty() {
            return;
        }
        // A single character is unambiguous: go straight to it.
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(only), None) if !only.is_ascii() => {
                navigator.push(Route::CharacterPage {
                    glyph: url::encode(&only.to_string()),
                });
            }
            _ => {
                navigator.push(Route::Search { q: text });
            }
        }
    };

    rsx! {
        form {
            class: "searchfield",
            role: "search",
            onsubmit: move |event| {
                // Dioxus 0.7 allows native form submission by default, which
                // would reload the page.
                event.prevent_default();
                submit(());
            },
            span { icons::Search {} }
            input {
                r#type: "search",
                value: "{query}",
                autofocus,
                autocomplete: "off",
                autocapitalize: "off",
                spellcheck: "false",
                enterkeyhint: "search",
                aria_label: "Search characters",
                placeholder: "字, pinyin, or meaning",
                oninput: move |event| query.set(event.value()),
            }
            if !query().is_empty() {
                button {
                    class: "searchfield-clear",
                    r#type: "button",
                    aria_label: "Clear Search",
                    onclick: move |_| query.set(String::new()),
                    icons::Clear {}
                }
            }
        }
    }
}
