use dioxus::prelude::*;

use crate::components::shell::Empty;
use crate::Route;

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = segments.join("/");

    rsx! {
        div { style: "padding-top: 1.5rem",
            Empty { glyph: "無".to_string(), title: "Page Not Found".to_string(),
                p {
                    if path.is_empty() {
                        "That address does not exist."
                    } else {
                        "There is nothing at /{path}."
                    }
                }
                Link { class: "btn btn-accent", to: Route::Home {}, "Go Home" }
            }
        }
    }
}
