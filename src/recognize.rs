//! Handwriting lookup: which characters does this drawing look like?
//!
//! There is no Rust port of HanziLookup, so this matches strokes directly
//! against Make Me a Hanzi's medians — the same centreline polylines the stroke
//! animation sweeps a brush along. Every character in `public/data/strokes.bin`
//! is one of those, pre-normalised by `scripts/build-data.py`; a drawing coming
//! off the pad is normalised the same way and then compared.
//!
//! ## Normalising
//!
//! All of a character's points are scaled by the longer side of their bounding
//! box and centred. Absolute size and position drop out, so it does not matter
//! how large the pad is or where on it somebody wrote, but proportion survives:
//! 一 still spreads across one line and 目 still stands in a narrow column.
//! Each stroke is then resampled to a fixed number of evenly spaced points, which
//! is what makes a median's handful of samples comparable to the few hundred a
//! finger drags out.
//!
//! ## Scoring
//!
//! Two strokes are compared by the mean distance between their resampled points,
//! which is sensitive to shape and to the direction the stroke was drawn in — 横
//! written right to left is not the same stroke. Drawing it backwards is a common
//! enough slip that the reverse is tried too, under a penalty.
//!
//! Whole characters are compared by aligning their stroke lists with a banded
//! edit distance, so one stroke too many or too few costs a gap instead of
//! shifting every stroke after it out of position. That matters because
//! miscounting strokes is exactly what a learner does.
//!
//! The normalisation here and the one in `scripts/build-data.py` are two halves of
//! one algorithm. Change either and the other has to follow.

use std::cell::RefCell;
use std::rc::Rc;

use crate::data;

/// File header: `SOSR`, a version, and the points-per-stroke the file was built
/// with. The count is read rather than assumed so the generator can be retuned
/// without a matching code change.
const MAGIC: &[u8; 4] = b"SOSR";
const VERSION: u8 = 1;
const HEADER: usize = 12;

/// Sanity bounds on the header, so a corrupt or misrouted download fails here
/// rather than by allocating something absurd.
const MAX_POINTS: usize = 32;
const MAX_TEMPLATES: usize = 1 << 20;

/// How far a template's stroke count may differ from the drawing's before it is
/// not worth scoring. Three covers the usual miscounts — a 横折 written as two
/// strokes, or a pair run together — without opening the field up to everything.
const COUNT_WINDOW: usize = 3;

/// Half-width of the alignment band, on top of the difference in stroke counts.
/// Strokes can be matched slightly out of order but not reordered wholesale,
/// which is both faster and truer to how the same character gets written.
const BAND: usize = 1;

/// Cost of leaving a stroke unmatched. Roughly what a mediocre match costs, so a
/// stroke too many is forgiven but never free.
const GAP: f32 = 0.30;

/// Added to the distance of a stroke matched against its own reverse.
const REVERSE_PENALTY: f32 = 0.12;

/// Per stroke of disagreement about how many strokes there were, on top of the
/// gaps that absorb them.
///
/// A gap alone under-prices a miscount, because the total is divided by the
/// stroke count and a longer character spreads it thinner. Without this, a plain
/// square box ranks 曰 above 口: the calligraphic 口 in the data narrows towards
/// the bottom, so its outer box fits a drawn square *worse* than 曰's does, and
/// 曰's extra stroke is not enough to make up the difference. How many strokes
/// somebody drew is evidence in its own right, and this is what says so.
const COUNT_PENALTY: f32 = 0.02;

/// One character's strokes, as an offset into [`Templates::points`].
struct Template {
    glyph: char,
    strokes: usize,
    offset: usize,
}

/// The whole template set, parsed once and kept for the session.
pub struct Templates {
    entries: Vec<Template>,
    /// Every template's points, back to back: x then y, each a byte across the
    /// unit square. Kept quantised rather than expanded to floats — it is a third
    /// of a megabyte as bytes and four times that as `f32`.
    points: Vec<u8>,
    points_per_stroke: usize,
}

impl Templates {
    /// The characters a drawing looks most like, best first.
    ///
    /// `strokes` are the drawn polylines in any consistent, square coordinate
    /// space — the pad passes fractions of its own width — with y pointing down.
    pub fn recognize(&self, strokes: &[Vec<[f32; 2]>], limit: usize) -> Vec<char> {
        self.rank(strokes, limit)
            .into_iter()
            .map(|(glyph, _)| glyph)
            .collect()
    }

    /// [`Self::recognize`], with the score each candidate earned. Lower is a
    /// closer match; the unit is mean distance across the pad, per stroke.
    fn rank(&self, strokes: &[Vec<[f32; 2]>], limit: usize) -> Vec<(char, f32)> {
        let Some(drawn) = Drawing::normalize(strokes, self.points_per_stroke) else {
            return Vec::new();
        };

        let mut scored: Vec<(f32, char)> = Vec::new();
        // One allocation for the whole sweep; the longest character in the data
        // has well under a hundred strokes.
        let widest = self.entries.iter().map(|t| t.strokes).max().unwrap_or(0);
        let mut previous = vec![0.0f32; widest + 1];
        let mut current = vec![0.0f32; widest + 1];

        for template in &self.entries {
            let difference = template.strokes.abs_diff(drawn.strokes);
            if difference > COUNT_WINDOW {
                continue;
            }
            let aligned = self.align(&drawn, template, difference, &mut previous, &mut current);
            // Per stroke, so a fifteen-stroke character is not penalised for
            // having more places to be slightly wrong than a three-stroke one.
            let score = aligned / drawn.strokes.max(template.strokes) as f32
                + COUNT_PENALTY * difference as f32;
            scored.push((score, template.glyph));
        }

        scored.sort_by(|a, b| a.0.total_cmp(&b.0));
        scored.truncate(limit);
        scored
            .into_iter()
            .map(|(score, glyph)| (glyph, score))
            .collect()
    }

    /// Total cost of the cheapest alignment between the drawing's strokes and the
    /// template's: a Levenshtein edit distance where substitution costs the
    /// distance between two strokes and a gap costs [`GAP`], restricted to a band
    /// around the diagonal.
    fn align(
        &self,
        drawn: &Drawing,
        template: &Template,
        difference: usize,
        previous: &mut [f32],
        current: &mut [f32],
    ) -> f32 {
        let band = difference + BAND;
        let columns = template.strokes;
        let unreachable = f32::MAX / 4.0;

        previous[..=columns].fill(unreachable);
        previous[0] = 0.0;
        for (j, cell) in previous
            .iter_mut()
            .enumerate()
            .take(band.min(columns) + 1)
            .skip(1)
        {
            *cell = j as f32 * GAP;
        }

        for i in 1..=drawn.strokes {
            current[..=columns].fill(unreachable);
            if i <= band {
                current[0] = i as f32 * GAP;
            }
            let first = i.saturating_sub(band).max(1);
            let last = (i + band).min(columns);
            for j in first..=last {
                let cost = self.stroke_distance(drawn, i - 1, template, j - 1);
                let mut best = previous[j - 1] + cost;
                best = best.min(previous[j] + GAP);
                best = best.min(current[j - 1] + GAP);
                current[j] = best;
            }
            previous[..=columns].copy_from_slice(&current[..=columns]);
        }

        previous[columns]
    }

    /// Mean distance between two strokes' resampled points, taking the reverse
    /// under a penalty when it fits better.
    fn stroke_distance(
        &self,
        drawn: &Drawing,
        drawn_stroke: usize,
        template: &Template,
        template_stroke: usize,
    ) -> f32 {
        let n = self.points_per_stroke;
        let a = drawn_stroke * n * 2;
        let b = template.offset + template_stroke * n * 2;

        // Plain `sqrt` rather than `hypot`, which is several times slower for the
        // overflow care it takes — care that is wasted on coordinates that cannot
        // leave the unit square. This is the innermost loop of the whole sweep.
        let distance = |dx: f32, dy: f32| (dx * dx + dy * dy).sqrt();

        let mut forward = 0.0;
        let mut reverse = 0.0;
        for i in 0..n {
            let x = drawn.points[a + i * 2];
            let y = drawn.points[a + i * 2 + 1];

            let k = (n - 1 - i) * 2;
            forward += distance(
                x - unit(self.points[b + i * 2]),
                y - unit(self.points[b + i * 2 + 1]),
            );
            reverse += distance(
                x - unit(self.points[b + k]),
                y - unit(self.points[b + k + 1]),
            );
        }

        let n = n as f32;
        (forward / n).min(reverse / n + REVERSE_PENALTY)
    }

    fn parse(bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.len() < HEADER || &bytes[..4] != MAGIC {
            return Err("that is not a stroke template file".to_string());
        }
        if bytes[4] != VERSION {
            return Err(format!(
                "stroke templates are version {}, this build reads version {VERSION}",
                bytes[4]
            ));
        }

        let points_per_stroke = bytes[5] as usize;
        if !(2..=MAX_POINTS).contains(&points_per_stroke) {
            return Err(format!(
                "{points_per_stroke} points per stroke is not usable"
            ));
        }
        let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        if count > MAX_TEMPLATES {
            return Err(format!("{count} templates is more than this can be"));
        }

        let glyphs = HEADER + count * 4;
        if bytes.len() < glyphs {
            return Err("stroke templates are truncated".to_string());
        }

        let mut entries = Vec::with_capacity(count);
        let mut cursor = glyphs;
        for i in 0..count {
            let at = HEADER + i * 4;
            let code = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            let glyph =
                char::from_u32(code).ok_or_else(|| format!("U+{code:04X} is not a character"))?;

            if cursor >= bytes.len() {
                return Err("stroke templates are truncated".to_string());
            }
            let strokes = bytes[cursor] as usize;
            cursor += 1;

            let span = strokes * points_per_stroke * 2;
            if strokes == 0 || cursor + span > bytes.len() {
                return Err(format!("{glyph} has no usable stroke template"));
            }
            entries.push(Template {
                glyph,
                strokes,
                offset: cursor,
            });
            cursor += span;
        }

        Ok(Self {
            entries,
            points: bytes,
            points_per_stroke,
        })
    }
}

/// A drawing, normalised into the same space as the templates.
struct Drawing {
    /// Resampled points, back to back: x then y, over the unit square.
    points: Vec<f32>,
    strokes: usize,
}

impl Drawing {
    /// Scale by the longer side of the bounding box, centre, and resample every
    /// stroke to `points_per_stroke` evenly spaced points.
    ///
    /// `None` when there is nothing to match: no strokes, or every stroke a
    /// single point.
    fn normalize(strokes: &[Vec<[f32; 2]>], points_per_stroke: usize) -> Option<Self> {
        let drawn: Vec<&Vec<[f32; 2]>> = strokes.iter().filter(|s| !s.is_empty()).collect();
        if drawn.is_empty() {
            return None;
        }

        let mut min = [f32::MAX; 2];
        let mut max = [f32::MIN; 2];
        for point in drawn.iter().flat_map(|s| s.iter()) {
            for axis in 0..2 {
                min[axis] = min[axis].min(point[axis]);
                max[axis] = max[axis].max(point[axis]);
            }
        }

        // A single tap has no extent at all, and would otherwise divide by zero.
        let side = match (max[0] - min[0]).max(max[1] - min[1]) {
            side if side > 0.0 => side,
            _ => 1.0,
        };
        let centre = [(min[0] + max[0]) / 2.0, (min[1] + max[1]) / 2.0];

        let mut points = Vec::with_capacity(drawn.len() * points_per_stroke * 2);
        let mut unit_stroke = Vec::new();
        for stroke in &drawn {
            unit_stroke.clear();
            unit_stroke.extend(stroke.iter().map(|p| {
                [
                    (p[0] - centre[0]) / side + 0.5,
                    (p[1] - centre[1]) / side + 0.5,
                ]
            }));
            resample(&unit_stroke, points_per_stroke, &mut points);
        }

        Some(Self {
            points,
            strokes: drawn.len(),
        })
    }
}

/// Append `n` points spaced evenly along a polyline's arc length, ends included.
fn resample(points: &[[f32; 2]], n: usize, out: &mut Vec<f32>) {
    let repeat = |p: [f32; 2], out: &mut Vec<f32>| {
        for _ in 0..n {
            out.push(p[0]);
            out.push(p[1]);
        }
    };

    if points.len() < 2 {
        repeat(points[0], out);
        return;
    }

    let lengths: Vec<f32> = points
        .windows(2)
        .map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]))
        .collect();
    let total: f32 = lengths.iter().sum();
    // A stroke that never moved: a tap, or a run of identical samples.
    if total <= 0.0 {
        repeat(points[0], out);
        return;
    }

    let mut segment = 0;
    let mut walked = 0.0;
    for k in 0..n {
        let target = total * k as f32 / (n - 1) as f32;
        while segment < lengths.len() - 1 && walked + lengths[segment] < target {
            walked += lengths[segment];
            segment += 1;
        }
        let span = lengths[segment];
        let t = if span > 0.0 {
            (target - walked) / span
        } else {
            0.0
        };
        let (from, to) = (points[segment], points[segment + 1]);
        out.push(from[0] + (to[0] - from[0]) * t);
        out.push(from[1] + (to[1] - from[1]) * t);
    }
}

/// A quantised coordinate, back over 0..1.
fn unit(byte: u8) -> f32 {
    byte as f32 / 255.0
}

thread_local! {
    /// Parsed once per session. A third of a megabyte is worth fetching only the
    /// first time somebody opens the pad, and parsing only once after that.
    static TEMPLATES: RefCell<Option<Rc<Templates>>> = const { RefCell::new(None) };
}

/// Fetch and parse the template set, or hand back the copy already in memory.
pub async fn templates() -> Result<Rc<Templates>, String> {
    if let Some(loaded) = TEMPLATES.with(|t| t.borrow().clone()) {
        return Ok(loaded);
    }

    let url = format!("{}/strokes.bin", data::DATA_ROOT);
    let bytes = data::fetch_bytes(&url)
        .await
        .map_err(|e| format!("could not load the handwriting data: {e}"))?;
    let loaded = Rc::new(Templates::parse(bytes)?);
    TEMPLATES.with(|t| *t.borrow_mut() = Some(loaded.clone()));
    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The file the app will actually serve. Reading the shipped artefact rather
    /// than a fixture is the point: it is what catches the generator and the
    /// matcher drifting apart, which no amount of testing either half alone can.
    fn shipped() -> Templates {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/public/data/strokes.bin");
        Templates::parse(std::fs::read(path).expect("generated by scripts/build-data.py"))
            .expect("the shipped templates parse")
    }

    /// A polyline through the given points on the pad, sampled the way dragging a
    /// finger along it would be.
    fn drag(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
        let mut out = Vec::new();
        for pair in points.windows(2) {
            for step in 0..12 {
                let t = step as f32 / 12.0;
                out.push([
                    pair[0][0] + (pair[1][0] - pair[0][0]) * t,
                    pair[0][1] + (pair[1][1] - pair[0][1]) * t,
                ]);
            }
        }
        out.push(*points.last().unwrap());
        out
    }

    #[test]
    fn the_shipped_templates_parse() {
        let templates = shipped();
        assert!(templates.entries.len() > 2_000);
        assert!(templates.points_per_stroke >= 4);
        assert!(templates.entries.iter().any(|t| t.glyph == '好'));
    }

    #[test]
    fn a_truncated_file_is_rejected() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/public/data/strokes.bin");
        let whole = std::fs::read(path).unwrap();
        assert!(Templates::parse(whole[..whole.len() / 2].to_vec()).is_err());
        assert!(Templates::parse(b"not a template file at all".to_vec()).is_err());
    }

    /// Characters somebody would actually try first, written roughly rather than
    /// precisely — which is the interesting case.
    #[test]
    fn simple_characters_are_recognised() {
        let templates = shipped();

        // 一: one 横, drifting up the way a hand does.
        let yi = vec![drag(&[[0.13, 0.53], [0.87, 0.47]])];
        // 十: 横 then 竖.
        let shi = vec![
            drag(&[[0.12, 0.5], [0.88, 0.48]]),
            drag(&[[0.5, 0.12], [0.51, 0.9]]),
        ];
        // 人: 撇 then 捺.
        let ren = vec![
            drag(&[[0.53, 0.16], [0.38, 0.52], [0.19, 0.86]]),
            drag(&[[0.5, 0.44], [0.82, 0.86]]),
        ];
        // 口: 竖, then 横折, then the 横 that closes it.
        let kou = vec![
            drag(&[[0.24, 0.21], [0.23, 0.8]]),
            drag(&[[0.24, 0.21], [0.79, 0.22], [0.77, 0.8]]),
            drag(&[[0.23, 0.8], [0.77, 0.79]]),
        ];

        for (glyph, strokes) in [('一', yi), ('十', shi), ('人', ren), ('口', kou)] {
            let found = templates.recognize(&strokes, 5);
            assert_eq!(
                found.first(),
                Some(&glyph),
                "{glyph} drawn plainly came back as {found:?}"
            );
        }
    }

    /// Size and position are normalised away, so the same character written small
    /// in a corner matches as well as one that fills the pad.
    #[test]
    fn where_on_the_pad_it_was_written_does_not_matter() {
        let templates = shipped();
        let large = vec![
            drag(&[[0.1, 0.5], [0.9, 0.5]]),
            drag(&[[0.5, 0.1], [0.5, 0.9]]),
        ];
        let small = vec![
            drag(&[[0.62, 0.2], [0.86, 0.2]]),
            drag(&[[0.74, 0.08], [0.74, 0.32]]),
        ];
        assert_eq!(
            templates.recognize(&large, 3),
            templates.recognize(&small, 3)
        );
    }

    /// Every template, fed back in as though it had been drawn, must find itself.
    ///
    /// Circular on its face, but it is the one test that exercises the whole
    /// pipeline end to end — the generator's byte layout, the offsets this reads
    /// them back at, the resampling, and the alignment — over real characters
    /// rather than four hand-written ones.
    #[test]
    fn every_template_recognises_itself() {
        let templates = shipped();
        let n = templates.points_per_stroke;

        let mut missed = Vec::new();
        for template in &templates.entries {
            let strokes: Vec<Vec<[f32; 2]>> = (0..template.strokes)
                .map(|s| {
                    (0..n)
                        .map(|i| {
                            let at = template.offset + (s * n + i) * 2;
                            [unit(templates.points[at]), unit(templates.points[at + 1])]
                        })
                        .collect()
                })
                .collect();
            if templates.recognize(&strokes, 1).first() != Some(&template.glyph) {
                missed.push(template.glyph);
            }
        }

        assert!(
            missed.is_empty(),
            "{} of {} templates did not match themselves: {missed:?}",
            missed.len(),
            templates.entries.len()
        );
    }

    #[test]
    fn nothing_drawn_matches_nothing() {
        let templates = shipped();
        assert!(templates.recognize(&[], 5).is_empty());
        assert!(templates.recognize(&[vec![]], 5).is_empty());
        // A tap has no extent, and must not divide by zero on the way to saying so.
        assert_eq!(templates.recognize(&[vec![[0.5, 0.5]]], 5).len(), 5);
    }

    #[test]
    fn resampling_spreads_points_along_the_length() {
        let mut out = Vec::new();
        // Two segments, the second three times the first, so an evenly spaced
        // sample must land inside the longer one more often.
        resample(&[[0.0, 0.0], [1.0, 0.0], [4.0, 0.0]], 5, &mut out);
        assert_eq!(out, vec![0.0, 0.0, 1.0, 0.0, 2.0, 0.0, 3.0, 0.0, 4.0, 0.0]);

        out.clear();
        resample(&[[2.0, 3.0]], 3, &mut out);
        assert_eq!(out, vec![2.0, 3.0, 2.0, 3.0, 2.0, 3.0]);

        out.clear();
        resample(&[[2.0, 3.0], [2.0, 3.0]], 2, &mut out);
        assert_eq!(out, vec![2.0, 3.0, 2.0, 3.0]);
    }
}
