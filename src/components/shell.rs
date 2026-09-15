//! App chrome: the top bar, the phone tab bar, and the footer.
//!
//! Navigation is duplicated by viewport rather than reflowed. Phones get a fixed
//! bottom tab bar because that is where thumbs are; wider viewports get the same
//! destinations inline in the top bar. CSS decides which is visible.

use dioxus::prelude::*;

use crate::components::icons;
use crate::Route;

#[component]
pub fn Shell() -> Element {
    rsx! {
        div { class: "app",
            TopBar {}
            main { class: "page", Outlet::<Route> {} }
            Footer {}
            TabBar {}
        }
    }
}

/// Destinations shown in both navigation surfaces, in order.
fn destinations() -> [(Route, &'static str); 4] {
    [
        (Route::Home {}, "Browse"),
        (Route::Draw {}, "Draw"),
        (Route::Hsk {}, "HSK"),
        (Route::Lists {}, "My Lists"),
    ]
}

#[component]
fn TopBar() -> Element {
    rsx! {
        header { class: "topbar",
            div { class: "topbar-inner",
                Link { class: "wordmark", to: Route::Home {},
                    // A seal, in the spirit of the red stamp on a piece of
                    // calligraphy.
                    span { class: "wordmark-seal", aria_hidden: "true", "笔" }
                    span { class: "wordmark-text", "Stroke Order" }
                }
                nav { class: "topbar-links", aria_label: "Sections",
                    for (route, label) in destinations() {
                        Link {
                            key: "{label}",
                            class: "topbar-link",
                            to: route.clone(),
                            active_class: "",
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TabBar() -> Element {
    let icon = |label: &str| match label {
        "Draw" => rsx! { icons::TabDraw {} },
        "HSK" => rsx! { icons::TabLevels {} },
        "My Lists" => rsx! { icons::TabLists {} },
        _ => rsx! { icons::TabBrowse {} },
    };

    rsx! {
        nav { class: "tabbar", aria_label: "Sections",
            for (route, label) in destinations() {
                Link { key: "{label}", class: "tab", to: route.clone(),
                    {icon(label)}
                    span { "{label}" }
                }
            }
        }
    }
}

#[component]
fn Footer() -> Element {
    rsx! {
        footer { class: "footer",
            p {
                "Stroke graphics and definitions from "
                a { href: "https://github.com/skishore/makemeahanzi", "Make Me a Hanzi" }
                " (LGPL / ARPHIC Public License), which draws its glosses from CC-CEDICT and Unihan. "
                "HSK 3.0 bands and course vocabulary from "
                a { href: "https://github.com/drkameleon/complete-hsk-vocabulary",
                    "complete-hsk-vocabulary"
                }
                " (MIT). The course units follow the React app's own arrangement of them."
            }
            p { "Built with Dioxus. A parallel rewrite of the React app at stroke-mssl.vercel.app." }
        }
    }
}

// ── Shared page furniture ───────────────────────────────────────────────────

#[component]
pub fn Section(
    title: String,
    #[props(default)] note: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        section { class: "section",
            div { class: "section-head",
                h2 { "{title}" }
                if let Some(note) = note {
                    span { class: "section-note num", "{note}" }
                }
            }
            {children}
        }
    }
}

#[component]
pub fn Empty(glyph: String, title: String, children: Element) -> Element {
    rsx! {
        div { class: "empty",
            span { class: "empty-glyph", aria_hidden: "true", "{glyph}" }
            h3 { "{title}" }
            {children}
        }
    }
}

/// A roadmap entry. `done` marks what already ships, so these placeholder pages
/// are honest about what is and is not built.
#[component]
pub fn RoadmapItem(title: String, detail: String, #[props(default = false)] done: bool) -> Element {
    rsx! {
        li { class: "roadmap-item",
            span { class: "roadmap-marker", "data-done": done.to_string(),
                if done {
                    icons::Check {}
                } else {
                    icons::Dot {}
                }
            }
            div {
                div { class: "roadmap-title", "{title}" }
                div { class: "roadmap-detail", "{detail}" }
            }
        }
    }
}
