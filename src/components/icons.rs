//! Line icons, drawn on a 24-unit grid with a 1.75 stroke to sit comfortably
//! beside SF-style system text. Kept inline rather than as an icon font so they
//! inherit `currentColor` and add nothing to the network.

use dioxus::prelude::*;

/// Shared wrapper so every icon has the same box, weight and joins. Glyphs that
/// read too light at the default weight override `weight`.
#[component]
fn Line(#[props(default = 1.75)] weight: f64, children: Element) -> Element {
    rsx! {
        svg {
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "{weight}",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            {children}
        }
    }
}

#[component]
pub fn Search() -> Element {
    rsx! {
        Line {
            circle { cx: "11", cy: "11", r: "6.5" }
            path { d: "M16 16l4.5 4.5" }
        }
    }
}

#[component]
pub fn Clear() -> Element {
    rsx! {
        Line { weight: 2.5, path { d: "M6 6l12 12M18 6L6 18" } }
    }
}

#[component]
pub fn Play() -> Element {
    rsx! {
        Line { path { d: "M8 5.5l11 6.5-11 6.5z", fill: "currentColor" } }
    }
}

#[component]
pub fn Pause() -> Element {
    rsx! {
        Line { path { d: "M9 5.5v13M15 5.5v13", stroke_width: "2.5" } }
    }
}

#[component]
pub fn StepBack() -> Element {
    rsx! {
        Line {
            path { d: "M17 6.5l-8 5.5 8 5.5z", fill: "currentColor" }
            path { d: "M6.5 6v12" }
        }
    }
}

#[component]
pub fn StepForward() -> Element {
    rsx! {
        Line {
            path { d: "M7 6.5l8 5.5-8 5.5z", fill: "currentColor" }
            path { d: "M17.5 6v12" }
        }
    }
}

#[component]
pub fn Replay() -> Element {
    rsx! {
        Line {
            path { d: "M4.5 12a7.5 7.5 0 1 0 2.6-5.7" }
            path { d: "M4 4v3.2h3.2" }
        }
    }
}

#[component]
pub fn Loop() -> Element {
    rsx! {
        Line {
            path { d: "M6.5 7.5h9a3.5 3.5 0 0 1 0 7h-9a3.5 3.5 0 0 1 0-7z" }
            path { d: "M9 5l-2.5 2.5L9 10" }
        }
    }
}

#[component]
pub fn Speaker() -> Element {
    rsx! {
        Line {
            path { d: "M4 9.5h3L11 6v12l-4-3.5H4z", fill: "currentColor" }
            path { d: "M15 9.5a3.5 3.5 0 0 1 0 5" }
            path { d: "M17.5 7a7 7 0 0 1 0 10" }
        }
    }
}

#[component]
pub fn Bookmark(#[props(default = false)] filled: bool) -> Element {
    rsx! {
        Line {
            path {
                d: "M6 4.5h12v15l-6-4-6 4z",
                fill: if filled { "currentColor" } else { "none" },
            }
        }
    }
}

#[component]
pub fn Undo() -> Element {
    rsx! {
        Line {
            path { d: "M4 9h9.5a5 5 0 0 1 0 10H8" }
            path { d: "M7.5 4.5 3 9l4.5 4.5" }
        }
    }
}

/// An eraser, tilted as if held.
#[component]
pub fn Erase() -> Element {
    rsx! {
        Line {
            path { d: "M9.5 19.5 4 14a1.5 1.5 0 0 1 0-2.1l8-8a1.5 1.5 0 0 1 2.1 0l5.4 5.4a1.5 1.5 0 0 1 0 2.1l-6.6 6.6a1.5 1.5 0 0 1-1 .5z" }
            path { d: "M8.5 8.5 15 15" }
            path { d: "M9 19.5h11" }
        }
    }
}

#[component]
pub fn Plus() -> Element {
    rsx! {
        Line { weight: 2.25, path { d: "M12 5.5v13M5.5 12h13" } }
    }
}

/// A bin, for deleting a list. Deliberately quiet: it sits at the end of a row
/// and needs a second tap to do anything.
#[component]
pub fn Trash() -> Element {
    rsx! {
        Line {
            path { d: "M4.5 7.5h15" }
            path { d: "M9.5 7.5V5.5h5v2" }
            path { d: "M6.5 7.5l.9 12h9.2l.9-12" }
            path { d: "M10.5 11v5M13.5 11v5" }
        }
    }
}

#[component]
pub fn Chevron() -> Element {
    rsx! {
        Line { weight: 2.25, path { d: "M9 5l7 7-7 7" } }
    }
}

/// The back chevron. Lighter than the disclosure one, the way iOS draws it in a
/// navigation bar.
#[component]
pub fn ChevronBack() -> Element {
    rsx! {
        Line { weight: 2.0, path { d: "M15 5l-7 7 7 7" } }
    }
}

#[component]
pub fn Check() -> Element {
    rsx! {
        Line { weight: 2.75, path { d: "M5 12.5l4.5 4.5L19 7" } }
    }
}

#[component]
pub fn Dot() -> Element {
    rsx! {
        Line { circle { cx: "12", cy: "12", r: "3.5", fill: "currentColor" } }
    }
}

// ── Tab bar ─────────────────────────────────────────────────────────────────

#[component]
pub fn TabBrowse() -> Element {
    rsx! {
        Line {
            rect { x: "3.5", y: "3.5", width: "17", height: "17", rx: "4.5" }
            path { d: "M12 3.5v17M3.5 12h17", stroke_dasharray: "2.5 3" }
        }
    }
}

/// A brush on the diagonal, over the stroke it has just laid down.
#[component]
pub fn TabDraw() -> Element {
    rsx! {
        Line {
            path { d: "M20.5 3.5 10 14l-1.2 3.2 3.2-1.2L22.5 5.5z" }
            path { d: "M7 14.5c-1.8.6-2.4 2-2.6 3.4-.1.9-.6 1.5-1.4 1.8 1.6 1.2 4.6 1.2 5.8-.6" }
        }
    }
}

#[component]
pub fn TabLevels() -> Element {
    rsx! {
        Line {
            path { d: "M4.5 19.5v-6M9.833 19.5v-11M15.167 19.5v-7.5M20.5 19.5V4.5" }
        }
    }
}

#[component]
pub fn TabLists() -> Element {
    rsx! {
        Line {
            path { d: "M9 6.5h11M9 12h11M9 17.5h11" }
            path { d: "M4.5 6.5h.01M4.5 12h.01M4.5 17.5h.01", stroke_width: "2.5" }
        }
    }
}
