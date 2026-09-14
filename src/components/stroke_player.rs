//! Animated stroke order.
//!
//! There is no Rust equivalent of hanzi-writer, so this reimplements the drawing
//! technique directly in SVG. For each stroke, Make Me a Hanzi gives us two
//! things: a closed **outline** that gets filled, and a **median** polyline down
//! the stroke's centre. To draw a stroke we clip to its outline and sweep a thick
//! line along the median using `stroke-dashoffset`, so the fill appears to grow
//! from the brush entry point to the exit point. A finished stroke is swapped for
//! its filled outline, which is crisper than a clipped sweep and guarantees the
//! tips resolve exactly.
//!
//! ## Coordinate space
//!
//! Make Me a Hanzi paths live in a 1024-unit grid whose y axis points **up**,
//! with a glyph bounding box of `(0, -124)` to `(1024, 900)`. SVG's y axis points
//! down, so the whole glyph is wrapped in a flipping transform. See
//! [`glyph_transform`].

use dioxus::prelude::*;

use crate::components::icons;
use crate::data::Graphics;

/// Make Me a Hanzi's glyph bounding box: 1024 wide, and y running from -124 to
/// 900. Both dimensions span 1024 units.
const GRID: f64 = 1024.0;
const GLYPH_MIN_Y: f64 = -124.0;

/// Breathing room between the glyph and the edge of the writing square, in grid
/// units.
const PADDING: f64 = 44.0;

/// Width of the line swept along the median, in grid units. It must be wide
/// enough to cover the thickest part of a stroke; overshoot is invisible because
/// the sweep is clipped to the stroke outline.
const BRUSH: f64 = 132.0;

/// Milliseconds per grid unit of median length, so long strokes take longer to
/// draw than short ones. Tuned so a full-width horizontal reads as a deliberate
/// brush movement rather than a flick.
const MS_PER_UNIT: f64 = 0.55;
const MIN_STROKE_MS: f64 = 220.0;
const MAX_STROKE_MS: f64 = 900.0;

/// Beat between strokes, and the longer rest before a loop restarts.
const GAP_MS: f64 = 110.0;
const LOOP_REST_MS: f64 = 900.0;

/// Animation clock interval. 16ms is one frame at 60Hz; elapsed time is measured
/// from the wall clock rather than accumulated, so a slow frame does not slow the
/// animation down.
const TICK_MS: u32 = 16;

// ── Timeline ────────────────────────────────────────────────────────────────

/// One stroke, pre-processed into everything the renderer needs per frame.
#[derive(PartialEq)]
struct Stroke {
    /// The filled outline, straight from the data.
    outline: String,
    /// The median as an SVG path, extended past both tips (see [`extend`]).
    median: String,
    /// Length of `median`, which is also its `stroke-dasharray`.
    length: f64,
    /// When this stroke starts drawing, in milliseconds from the start.
    start: f64,
    duration: f64,
}

#[derive(PartialEq)]
struct Timeline {
    strokes: Vec<Stroke>,
    /// Total run time including the rest before looping.
    total: f64,
}

impl Timeline {
    fn build(graphics: &Graphics) -> Self {
        let mut strokes = Vec::with_capacity(graphics.strokes.len());
        let mut cursor = 0.0;

        for (outline, median) in graphics.strokes.iter().zip(&graphics.medians) {
            // Extending by half the brush width means the round line cap starts
            // already covering the stroke's entry tip, instead of creeping in.
            let points = extend(median, BRUSH / 2.0);
            let length = polyline_length(&points);
            let duration = (MIN_STROKE_MS.max(length * MS_PER_UNIT)).min(MAX_STROKE_MS);

            strokes.push(Stroke {
                outline: outline.clone(),
                median: polyline_path(&points),
                length,
                start: cursor,
                duration,
            });
            cursor += duration + GAP_MS;
        }

        Self {
            total: cursor + LOOP_REST_MS,
            strokes,
        }
    }

    /// How far through stroke `i` the animation is at time `at`, from 0 to 1.
    fn progress(&self, i: usize, at: f64) -> f64 {
        let stroke = &self.strokes[i];
        if at <= stroke.start {
            0.0
        } else if at >= stroke.start + stroke.duration {
            1.0
        } else {
            (at - stroke.start) / stroke.duration
        }
    }

    /// Index of the stroke being drawn at time `at`, if any.
    fn active(&self, at: f64) -> Option<usize> {
        self.strokes
            .iter()
            .position(|s| at > s.start && at < s.start + s.duration)
    }

    /// How many strokes are fully drawn at time `at`.
    fn completed(&self, at: f64) -> usize {
        self.strokes
            .iter()
            .filter(|s| at >= s.start + s.duration)
            .count()
    }

    /// Time at which stroke `i` has just finished, for stepping and scrubbing.
    fn seek_past(&self, i: usize) -> f64 {
        self.strokes
            .get(i)
            .map_or(self.total, |s| s.start + s.duration)
    }
}

/// Extend a median past both of its tips, along the direction of its terminal
/// segments. Without this, a round line cap of radius `by` would leave the entry
/// tip unpainted until the sweep had already moved on.
fn extend(points: &[[f64; 2]], by: f64) -> Vec<[f64; 2]> {
    if points.len() < 2 {
        return points.to_vec();
    }
    let mut out = Vec::with_capacity(points.len() + 2);

    let (first, second) = (points[0], points[1]);
    out.push(step_from(first, second, by));
    out.extend_from_slice(points);
    let (last, penultimate) = (points[points.len() - 1], points[points.len() - 2]);
    out.push(step_from(last, penultimate, by));
    out
}

/// A point `by` units from `from`, heading directly away from `toward`.
fn step_from(from: [f64; 2], toward: [f64; 2], by: f64) -> [f64; 2] {
    let (dx, dy) = (from[0] - toward[0], from[1] - toward[1]);
    let len = (dx * dx + dy * dy).sqrt();
    if len < f64::EPSILON {
        return from;
    }
    [from[0] + dx / len * by, from[1] + dy / len * by]
}

fn polyline_length(points: &[[f64; 2]]) -> f64 {
    points
        .windows(2)
        .map(|w| {
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            (dx * dx + dy * dy).sqrt()
        })
        .sum()
}

fn polyline_path(points: &[[f64; 2]]) -> String {
    let mut d = String::with_capacity(points.len() * 14);
    for (i, p) in points.iter().enumerate() {
        d.push_str(if i == 0 { "M" } else { "L" });
        d.push_str(&format!("{:.1} {:.1}", p[0], p[1]));
    }
    d
}

/// Maps Make Me a Hanzi's y-up 1024 grid into a `0 0 1024 1024` viewBox, flipping
/// the y axis and insetting by [`PADDING`].
///
/// Scaling down by the padding on both sides, then translating so the glyph's
/// bottom edge (`y = -124` before the flip) lands on the padded baseline.
fn glyph_transform() -> String {
    let scale = (GRID - 2.0 * PADDING) / GRID;
    let y = GRID - PADDING - (-GLYPH_MIN_Y * scale);
    format!("translate({PADDING} {y:.4}) scale({scale:.7} -{scale:.7})")
}

// ── Components ──────────────────────────────────────────────────────────────

/// The 米字格 practice square: a border plus dashed centre lines and diagonals.
/// Traditional Chinese copybooks use it to show where a stroke sits relative to
/// the character's centre.
#[component]
pub fn RiceGrid() -> Element {
    rsx! {
        svg {
            view_box: "0 0 1024 1024",
            preserve_aspect_ratio: "none",
            "aria-hidden": "true",
            g {
                stroke: "var(--grid)",
                fill: "none",
                stroke_width: "6",
                rect { x: "3", y: "3", width: "1018", height: "1018", rx: "10" }
                g { stroke_dasharray: "14 22", stroke_width: "4",
                    line { x1: "512", y1: "0", x2: "512", y2: "1024" }
                    line { x1: "0", y1: "512", x2: "1024", y2: "512" }
                    line { x1: "0", y1: "0", x2: "1024", y2: "1024" }
                    line { x1: "1024", y1: "0", x2: "0", y2: "1024" }
                }
            }
        }
    }
}

/// The glyph itself, drawn to a given point in the timeline.
///
/// Split out from [`StrokePlayer`] so the static filmstrip can reuse it without
/// the animation clock.
#[component]
fn Glyph(
    graphics: Graphics,
    /// Unique per instance: SVG clip paths live in one global id namespace, so
    /// two players on the same page would otherwise clip each other.
    scope: String,
    /// Strokes drawn in full.
    completed: usize,
    /// Stroke currently being drawn, with its progress from 0 to 1.
    drawing: Option<(usize, f64)>,
    /// Show un-drawn strokes as a faint outline, so the target is visible.
    ghost: bool,
) -> Element {
    let timeline = use_memo(use_reactive!(|(graphics,)| Timeline::build(&graphics)));
    let transform = use_memo(glyph_transform);

    rsx! {
        svg { view_box: "0 0 1024 1024", "aria-hidden": "true",
            defs {
                for (i, stroke) in timeline.read().strokes.iter().enumerate() {
                    clipPath { key: "{i}", id: "{scope}-{i}",
                        path { d: "{stroke.outline}" }
                    }
                }
            }
            g { transform: "{transform}",
                if ghost {
                    for (i, stroke) in timeline.read().strokes.iter().enumerate() {
                        path { key: "g{i}", d: "{stroke.outline}", fill: "var(--ink-ghost)" }
                    }
                }
                for (i, stroke) in timeline.read().strokes.iter().enumerate().take(completed) {
                    path { key: "d{i}", d: "{stroke.outline}", fill: "var(--ink)" }
                }
                if let Some((i, progress)) = drawing {
                    if let Some(stroke) = timeline.read().strokes.get(i) {
                        g { clip_path: "url(#{scope}-{i})",
                            path {
                                d: "{stroke.median}",
                                fill: "none",
                                // The stroke being drawn is accented, so the eye
                                // knows where to look; it settles to ink when done.
                                stroke: "var(--accent)",
                                stroke_width: "{BRUSH}",
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_dasharray: "{stroke.length}",
                                stroke_dashoffset: "{stroke.length * (1.0 - progress)}",
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A single still frame: strokes 1..=`upto` drawn, with the last one accented.
/// Used by the stroke-order filmstrip.
#[component]
pub fn StrokeFrame(graphics: Graphics, scope: String, upto: usize) -> Element {
    rsx! {
        div { class: "frame",
            RiceGrid {}
            Glyph {
                graphics,
                scope,
                completed: upto.saturating_sub(1),
                // Progress 1.0 renders the final stroke fully, in the accent
                // colour, which is what marks it as "this frame's stroke".
                drawing: Some((upto.saturating_sub(1), 1.0)),
                ghost: false,
            }
            span { class: "frame-index num", "{upto}" }
        }
    }
}

/// Playback speed options, as multipliers.
const SPEEDS: [(f64, &str); 4] = [(0.5, "0.5×"), (1.0, "1×"), (1.5, "1.5×"), (2.0, "2×")];

/// Watch a character being written, with play/pause, per-stroke stepping,
/// scrubbing and speed control.
#[component]
pub fn StrokePlayer(glyph: char, graphics: Graphics) -> Element {
    let art = graphics.clone();
    let timeline = use_memo(use_reactive!(|(art,)| Timeline::build(&art)));
    let count = use_memo(move || timeline.read().strokes.len());

    let mut at = use_signal(|| 0.0f64);
    let mut playing = use_signal(|| true);
    let mut looping = use_signal(|| true);
    let mut speed = use_signal(|| 1.0f64);

    // Start over when the character changes; the component itself is reused as
    // the reader navigates between characters.
    use_effect(use_reactive!(|(glyph,)| {
        let _ = glyph;
        at.set(0.0);
        playing.set(true);
    }));

    // The clock. Advancing by measured wall-clock deltas rather than by a fixed
    // 16ms keeps playback honest when frames are dropped.
    use_future(move || async move {
        let mut last = now();
        loop {
            gloo_timers::future::TimeoutFuture::new(TICK_MS).await;
            let current = now();
            let delta = (current - last).clamp(0.0, 250.0);
            last = current;

            if !playing() {
                continue;
            }
            let total = timeline.read().total;
            let next = at() + delta * speed();
            if next >= total {
                if looping() {
                    at.set(0.0);
                } else {
                    at.set(total);
                    playing.set(false);
                }
            } else {
                at.set(next);
            }
        }
    });

    let position = at();
    let completed = timeline.read().completed(position);
    let active = timeline.read().active(position);
    let drawing = active.map(|i| (i, timeline.read().progress(i, position)));
    // While a stroke is mid-draw, report it as the current one; between strokes,
    // report how many are finished.
    let shown = active.map_or(completed, |i| i + 1);

    let mut seek = move |index: usize| {
        playing.set(false);
        at.set(timeline.read().seek_past(index));
    };

    rsx! {
        div { class: "player",
            div { class: "canvas",
                RiceGrid {}
                Glyph {
                    graphics: graphics.clone(),
                    scope: format!("stroke-{:x}", glyph as u32),
                    completed,
                    drawing,
                    ghost: true,
                }
            }

            div { class: "player-controls",
                button {
                    class: "btn btn-icon",
                    r#type: "button",
                    disabled: shown == 0,
                    aria_label: "Previous Stroke",
                    onclick: move |_| {
                        let target = shown.saturating_sub(1);
                        playing.set(false);
                        at.set(if target == 0 { 0.0 } else { timeline.read().seek_past(target - 1) });
                    },
                    icons::StepBack {}
                }
                button {
                    class: "btn btn-accent btn-icon",
                    r#type: "button",
                    aria_label: if playing() { "Pause" } else { "Play" },
                    onclick: move |_| {
                        // Restarting from the end is friendlier than a dead button.
                        if !playing() && at() >= timeline.read().total {
                            at.set(0.0);
                        }
                        let next = !playing();
                        playing.set(next);
                    },
                    if playing() { icons::Pause {} } else { icons::Play {} }
                }
                button {
                    class: "btn btn-icon",
                    r#type: "button",
                    disabled: shown >= count(),
                    aria_label: "Next Stroke",
                    onclick: move |_| seek(shown),
                    icons::StepForward {}
                }
                button {
                    class: "btn btn-icon",
                    r#type: "button",
                    aria_label: "Replay From The First Stroke",
                    onclick: move |_| {
                        at.set(0.0);
                        playing.set(true);
                    },
                    icons::Replay {}
                }
                button {
                    class: "btn btn-icon",
                    r#type: "button",
                    aria_label: "Loop",
                    aria_pressed: looping().to_string(),
                    style: if looping() { "color: var(--accent)" } else { "" },
                    onclick: move |_| {
                        let next = !looping();
                        looping.set(next);
                    },
                    icons::Loop {}
                }
            }

            div { class: "player-dots", role: "group", aria_label: "Jump To A Stroke",
                for i in 0..count() {
                    button {
                        key: "{i}",
                        class: "player-dot",
                        r#type: "button",
                        aria_label: "Stroke {i + 1}",
                        "data-state": if active == Some(i) {
                            "active"
                        } else if i < completed {
                            "done"
                        } else {
                            "todo"
                        },
                        onclick: move |_| seek(i),
                    }
                }
            }

            div { class: "player-meta",
                span { class: "num", "Stroke {shown} of {count()}" }
                span { "aria-hidden": "true", "·" }
                div { class: "segmented", role: "group", aria_label: "Speed",
                    for (value, label) in SPEEDS {
                        button {
                            key: "{label}",
                            class: "segment num",
                            r#type: "button",
                            aria_pressed: (speed() == value).to_string(),
                            onclick: move |_| speed.set(value),
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}

/// Milliseconds from a monotonic-ish clock. `performance.now()` where available,
/// falling back to the wall clock.
fn now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or_else(js_sys::Date::now)
}
