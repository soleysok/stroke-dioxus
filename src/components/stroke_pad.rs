//! The handwriting pad: a 米字格 to write on, and undo and clear.
//!
//! It only collects strokes. What they mean is the host page's business, the way
//! [`crate::components::search_field`] collects a query and leaves searching to
//! whoever mounted it.
//!
//! Ink is SVG rather than a `<canvas>`, which is what the React app uses. Over a
//! 米字格 the ink is one more layer on the same square, so it shares the grid's
//! geometry, scales with it, and picks up `--ink` in either appearance without a
//! device-pixel-ratio dance or a redraw loop. Points are recorded as fractions of
//! the pad, which is both what the SVG wants as a view box and what the matcher
//! wants as coordinates.

use dioxus::prelude::*;
use wasm_bindgen::JsCast;

use crate::components::icons;
use crate::components::stroke_player::RiceGrid;

/// One stroke: the path a single contact with the pad traced, in fractions of the
/// pad's width and height, y pointing down.
pub type Stroke = Vec<[f32; 2]>;

/// Points closer together than this add nothing but work. A fraction of the pad,
/// so it means the same thing whatever size the pad is drawn at.
const MIN_STEP: f32 = 0.004;

/// Ink width, as a fraction of the pad. Close to what the stroke player's brush
/// works out to, so writing and watching feel like the same medium.
const INK: f32 = 0.022;

#[component]
pub fn StrokePad(
    /// Finished strokes, in the order they were drawn. Owned by the host page,
    /// which decides what to do with them.
    strokes: Signal<Vec<Stroke>>,
) -> Element {
    // The stroke in progress is kept apart from the finished ones, so a moving
    // finger does not look like new input to whatever is watching `strokes`.
    let mut current = use_signal(Stroke::new);
    let mut holding = use_signal(|| None as Option<i32>);

    let mut strokes = strokes;
    let inked = !strokes.read().is_empty() || !current.read().is_empty();

    rsx! {
        div { class: "pad-stack",
            div {
                class: "pad",
                // Without this the browser reads a stroke as a pan and scrolls
                // the page out from under the writing.
                style: "touch-action: none",
                onpointerdown: move |event| {
                    event.prevent_default();
                    let Some(raw) = event.downcast::<web_sys::PointerEvent>() else {
                        return;
                    };
                    let Some(point) = pad_point(raw) else { return };
                    // Capture, so a stroke that runs off the edge still ends here
                    // instead of leaving the pad waiting on a finger that left.
                    if let Some(pad) = pad_element(raw) {
                        let _ = pad.set_pointer_capture(raw.pointer_id());
                    }
                    holding.set(Some(raw.pointer_id()));
                    current.set(vec![point]);
                },
                onpointermove: move |event| {
                    let Some(raw) = event.downcast::<web_sys::PointerEvent>() else {
                        return;
                    };
                    if holding() != Some(raw.pointer_id()) {
                        return;
                    }
                    let Some(point) = pad_point(raw) else { return };
                    let mut trace = current.write();
                    if trace.last().is_none_or(|last| far_enough(*last, point)) {
                        trace.push(point);
                    }
                },
                onpointerup: move |event| {
                    let Some(raw) = event.downcast::<web_sys::PointerEvent>() else {
                        return;
                    };
                    if holding() != Some(raw.pointer_id()) {
                        return;
                    }
                    holding.set(None);
                    if let Some(point) = pad_point(raw) {
                        current.write().push(point);
                    }
                    let stroke = std::mem::take(&mut *current.write());
                    // A tap is not a stroke; ignoring it keeps a stray touch from
                    // throwing the match off.
                    if stroke.len() > 1 {
                        strokes.write().push(stroke);
                    }
                },
                onpointercancel: move |_| {
                    holding.set(None);
                    current.write().clear();
                },
                RiceGrid {}
                Ink { strokes, current }
                if !inked {
                    span { class: "pad-hint han", aria_hidden: "true", "写" }
                }
            }

            div { class: "pad-controls",
                button {
                    class: "btn",
                    r#type: "button",
                    disabled: !inked,
                    onclick: move |_| {
                        // Whatever was mid-stroke is already abandoned by the tap
                        // that landed on this button.
                        current.write().clear();
                        strokes.write().pop();
                    },
                    icons::Undo {}
                    "Undo"
                }
                button {
                    class: "btn",
                    r#type: "button",
                    disabled: !inked,
                    onclick: move |_| {
                        current.write().clear();
                        strokes.write().clear();
                    },
                    icons::Erase {}
                    "Clear"
                }
            }
        }
    }
}

/// The ink. Split out so a moving finger repaints the strokes and nothing else.
#[component]
fn Ink(strokes: Signal<Vec<Stroke>>, current: Signal<Stroke>) -> Element {
    rsx! {
        svg {
            class: "pad-ink",
            view_box: "0 0 1 1",
            preserve_aspect_ratio: "none",
            "aria-hidden": "true",
            g {
                fill: "none",
                stroke: "var(--ink)",
                stroke_width: "{INK}",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                for (i, stroke) in strokes.read().iter().enumerate() {
                    path { key: "{i}", d: "{trace(stroke)}" }
                }
                if current.read().len() > 1 {
                    path { d: "{trace(&current.read())}" }
                }
            }
        }
    }
}

fn trace(stroke: &[[f32; 2]]) -> String {
    let mut d = String::with_capacity(stroke.len() * 14);
    for (i, point) in stroke.iter().enumerate() {
        d.push_str(if i == 0 { "M" } else { "L" });
        d.push_str(&format!("{:.4} {:.4}", point[0], point[1]));
    }
    d
}

fn far_enough(last: [f32; 2], next: [f32; 2]) -> bool {
    (next[0] - last[0]).hypot(next[1] - last[1]) >= MIN_STEP
}

/// The pad itself. Everything inside it is `pointer-events: none`, so a pointer
/// event's target is always the pad — and stays the pad once it has capture.
fn pad_element(event: &web_sys::PointerEvent) -> Option<web_sys::Element> {
    event.target()?.dyn_into::<web_sys::Element>().ok()
}

/// Where on the pad a pointer is, as a fraction of its width and height. Clamped,
/// so a stroke that runs off the edge reads as one that grazed it.
fn pad_point(event: &web_sys::PointerEvent) -> Option<[f32; 2]> {
    let bounds = pad_element(event)?.get_bounding_client_rect();
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return None;
    }
    Some([
        (((event.client_x() as f64 - bounds.left()) / bounds.width()) as f32).clamp(0.0, 1.0),
        (((event.client_y() as f64 - bounds.top()) / bounds.height()) as f32).clamp(0.0, 1.0),
    ])
}
