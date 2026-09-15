# Stroke Order

Learn Chinese characters stroke by stroke. A [Dioxus](https://dioxuslabs.com/) (Rust →
WebAssembly) rewrite of the production app at
[stroke-mssl.vercel.app](https://stroke-mssl.vercel.app), built in parallel and
independently.

Look up any of 9,574 characters by character, pinyin, or English meaning — or draw one
you cannot type — then watch it written one stroke at a time in the order a native
writer would use.

The HSK 3.0 syllabus is here as a course as well as a tag on each character: 5,369 words
split into 512 short themed units, and one tap on a unit turns the lesson into a saved
vocabulary list.

> The React/TanStack app in `soleysok/mssl-strok` stays in production and is untouched.
> This repository is the parallel rebuild; it is read as a feature reference, and its
> committed HSK course files are copied here rather than reinvented — see
> [`NOTICE`](NOTICE) and the Data section.

---

## Running It

You need Rust **1.85 or newer**. Dioxus 0.7.10 declares an MSRV of 1.83, but its
dependency tree resolves to crates that need edition 2024, so 1.83 will not build.

```bash
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli --version 0.7.10   # or grab a prebuilt `dx` from the
                                            # Dioxus GitHub releases, which is faster
```

Then, from the repo root:

```bash
dx serve --web          # dev server with hot reload, on http://127.0.0.1:8080
dx build --web --release
```

The release build lands in `target/dx/stroke/release/web/public/` — a plain static
directory: `index.html`, a hashed CSS/JS/wasm triple under `assets/`, and the character
data under `data/`.

For a fast type-check without the asset pipeline, plain cargo works and is much quicker
than a full `dx` cycle:

```bash
cargo check --target wasm32-unknown-unknown
```

Note that a cargo-only **build** will panic at runtime on any `asset!()`, because `dx`
acts as the linker that resolves asset hashes. Use cargo to check, `dx` to run.

The handwriting matcher is the one part with no browser in it, so it has tests that run on
the host. They read the generated `public/data/strokes.bin` rather than a fixture, because
what they are really guarding is that the generator and the matcher still agree:

```bash
cargo test
```

Current release payload: **1,080 KB of wasm (314 KB brotli, which is what Vercel serves)**
plus a 518 KB search index (137 KB brotli), both fetched once. Everything else is fetched
only when somebody asks for it: the Draw pad's 352 KB of handwriting templates the first
time the pad is opened, and one HSK level's units — 48 KB for HSK 1 up to 107 KB for HSK 6,
13 to 31 KB brotli — the first time that level is opened, then kept for the session.

---

## Deploying

The release output is a plain static directory, so any static host will do. Vercel is
what's configured here, deployed from a `git push`.

`vercel.json` carries the whole setup, so a new Vercel project needs nothing in the
dashboard beyond pointing it at this repository:

| Setting | Value | Comes from |
| --- | --- | --- |
| Framework Preset | Other | `"framework": null` |
| Install Command | `bash scripts/vercel-install.sh` | `"installCommand"` |
| Build Command | `bash scripts/vercel-build.sh` | `"buildCommand"` |
| Output Directory | `dist` | `"outputDirectory"` |
| Root Directory | repository root | — |

Nothing in the build is JavaScript, so the project's Node.js version setting is
irrelevant. Leave the dashboard Override toggles off and the values above apply.

### What the build does

Vercel's build image is Amazon Linux 2023 with no Rust in it, so
[`scripts/vercel-install.sh`](scripts/vercel-install.sh) installs the toolchain itself:
rustup, Rust 1.90, and the `wasm32-unknown-unknown` target, all inside the project
directory — the one place in a build container that is reliably writable.

Then it needs `dx`, which is not optional, since it is the linker that resolves
`asset!()`. The prebuilt release binary is built against glibc 2.39 and Amazon Linux 2023
has 2.34, so on Vercel it will not run: the script downloads it, asks it for its version,
and only believes the answer. When that fails it compiles `dioxus-cli` instead, which
also wants `gcc`, `pkgconf` and `openssl-devel` — none of which a bare Amazon Linux image
has, so they go in with `dnf` first. On that image the compile measures 5m30s across four
cores, or 17 CPU-minutes: roughly ten minutes on the two-core Basic build machine,
against Vercel's 45-minute limit.

To avoid paying that on every deploy, the compiled binary is parked in
`node_modules/.cache`, the only directory Vercel restores between builds. A warm build
reinstalls the toolchain, reuses the binary, and spends under a minute on the app itself
— call it two minutes end to end.

[`scripts/vercel-build.sh`](scripts/vercel-build.sh) then runs the release build and
copies `target/dx/stroke/release/web/public` to `dist/`, so the Output Directory setting
does not have to track `dx`'s layout across CLI versions. It fails loudly if the wasm, the
search index, the handwriting templates or any of the six HSK course levels is missing,
which is the difference between a failed deploy and a blank page — or an HSK level that
will not open — in production.

Both scripts read their versions and paths from
[`scripts/vercel-env.sh`](scripts/vercel-env.sh) and run anywhere, not just on Vercel:

```bash
bash scripts/vercel-install.sh && bash scripts/vercel-build.sh
```

### Routing

Client-side routes like `/character/%E5%A5%BD` have no file on disk, so anything that is
not a real file is rewritten to `index.html`. Vercel checks the filesystem before
rewrites, so the hashed assets still serve themselves.

`/assets/` and `/data/` are excluded from that fallback on purpose. A character with no
vendored file has to come back as a 404 so the runtime fallback to the jsDelivr mirror
takes over; rewriting it would return 200 with a page of HTML that fails to parse as
JSON. Hashed assets are cached for a year as immutable, the generated data for an hour
with a week of stale-while-revalidate.

### If the Vercel build stops working

[`.github/workflows/deploy-vercel.yml`](.github/workflows/deploy-vercel.yml) is the
escape hatch, and is manual-dispatch only so it never fires on its own. GitHub's runners
can use the prebuilt `dx`, so it builds in a few minutes, then deploys through `vercel
build && vercel deploy --prebuilt` — reading the same `vercel.json`, so the two paths
cannot drift. It needs `VERCEL_TOKEN`, `VERCEL_ORG_ID` and `VERCEL_PROJECT_ID` as
repository secrets, and the Vercel project's Git integration turned off, or every push
would build twice.

---

## Data

Everything under `public/data/` is generated, by two scripts: the characters come from
public datasets, the course units come from the React app. To rebuild the characters:

```bash
python3 scripts/build-data.py
```

The script downloads its sources into `scripts/.cache/` (reused on later runs) and emits:

| Output | Contents |
| --- | --- |
| `public/data/index.json` | 518 KB columnar search index over all 9,574 characters — reading, toneless pinyin keys, short gloss, stroke count, HSK band, radical. |
| `public/data/char/<hex>.json` | Per character: stroke outlines, stroke medians, and the full dictionary entry. Named by zero-padded Unicode code point so filenames stay ASCII. |
| `public/data/strokes.bin` | 352 KB of handwriting templates for the Draw pad: the same medians, normalised and resampled. Fetched only when somebody opens the pad. |

Sources are [Make Me a Hanzi](https://github.com/skishore/makemeahanzi) for graphics and
definitions, and
[complete-hsk-vocabulary](https://github.com/drkameleon/complete-hsk-vocabulary) for
HSK 3.0 bands and corpus frequency — the same data philosophy as the React app. See
[`NOTICE`](NOTICE) for licence terms; the graphics are LGPL / ARPHIC and the attribution
must travel with them.

**What ships in git.** Per-character files are vendored only for the 2,970 HSK-ranked
characters — every character a learner meets in HSK 1 through 7-9, about 8 MB. The rare
tail falls back at runtime to the jsDelivr mirror of `hanzi-writer-data`, which is derived
from the same Make Me a Hanzi graphics and uses an identical coordinate space. Pass
`--all` to vendor all 9,574 instead, at roughly 32 MB.

Characters are ranked by HSK band, then corpus frequency. That ordering is why the index
is columnar and rank-ordered: a prefix of it is already the most useful slice, so "most
common characters" and "is this vendored locally" are both just offset comparisons.

### The HSK course

The course data is a second generated tree, from a different source and by a different
script:

```bash
git clone https://github.com/soleysok/mssl-strok ../mssl-strok
python3 scripts/build-hsk-data.py            # or --from PATH
```

| Output | Contents |
| --- | --- |
| `public/data/hsk/index.json` | 736 bytes: unit, word and character counts per level, plus provenance. Compiled into the wasm with `include_str!`, so the HSK hub renders its level cards without a fetch. |
| `public/data/hsk/level-<n>.json` | One level: its units in teaching order (id, title, topic, headwords) and a word map with pinyin and up to three meanings. 48 KB for HSK 1 to 107 KB for HSK 6; fetched when that level is opened. |

Which words a unit teaches, in what order, under what title is editorial work, and it
already exists in the React app — [`soleysok/mssl-strok`](https://github.com/soleysok/mssl-strok)
groups the 5,369 HSK 3.0 words into 512 themed units of at most twelve. Deriving units
here from the raw vocabulary list would invent a second, slightly different course, so
[`scripts/build-hsk-data.py`](scripts/build-hsk-data.py) copies that repository's own
committed `public/data/hsk/level-*.json` and trims each one to the fields this app
renders. See [`NOTICE`](NOTICE): the arrangement is the React app's, the vocabulary and
meanings are still complete-hsk-vocabulary and CC-CEDICT.

The trim is what makes the copy worth committing rather than the originals. It drops the
example sentences with their per-token pinyin (which this app has no page for yet, and
which are 85% of the bytes), part of speech, traditional forms, classifiers, corpus
frequency, and the per-level character detail — that last one because all 9,574 characters
are already in `public/data/index.json`, so a unit's characters are resolved against the
dictionary at runtime instead of shipping twice. Six levels go from 3.2 MB to 505 KB, and
no unit page loses anything. The reader ignores unknown fields, so dropping the untrimmed
files in place of these still works if the sentences are ever wanted.

| Level | Words | Units | Characters | File |
| --- | --- | --- | --- | --- |
| HSK 1 | 509 | 50 | 300 | 48 KB |
| HSK 2 | 753 | 74 | 499 | 70 KB |
| HSK 3 | 953 | 91 | 649 | 89 KB |
| HSK 4 | 972 | 95 | 821 | 92 KB |
| HSK 5 | 1,059 | 99 | 939 | 100 KB |
| HSK 6 | 1,123 | 103 | 975 | 107 KB |

A word is not a character: 你好 is a headword with no strokes of its own. So a word row
leads to a search for the word, which the dictionary answers with the characters it is
written with — each of which does have an animation. Every unit and every list also offers
its characters directly, as the usual grid of tiles.

### How the stroke animation works

There is no Rust equivalent of `hanzi-writer`, so
[`src/components/stroke_player.rs`](src/components/stroke_player.rs) reimplements the
drawing technique directly in SVG. Make Me a Hanzi gives two things per stroke: a closed
**outline** meant to be filled, and a **median** polyline down the stroke's centre.

To draw a stroke, the renderer clips to that stroke's outline and sweeps a thick line
along its median using `stroke-dashoffset`, so the fill appears to grow from the brush
entry point to the exit point. Once a stroke finishes it is swapped for its filled
outline, which is crisper than a clipped sweep and makes the tips resolve exactly. Medians
are extended half a brush-width past both tips so the round line cap starts already
covering the entry point instead of creeping in.

The coordinate space is the tricky part. Make Me a Hanzi paths live in a 1024-unit grid
whose **y axis points up**, with a glyph bounding box of `(0, -124)` to `(1024, 900)`. SVG's
y axis points down, so the glyph is wrapped in a flipping, padding-insetting transform —
see `glyph_transform`. Get it wrong and every character renders upside down.

Playback runs off a 16 ms clock that advances by measured wall-clock deltas rather than a
fixed step, so a dropped frame does not slow the animation down. Per-stroke duration
scales with median length, so long strokes take longer to draw than short ones.

### How handwriting lookup works

The React app hands drawn strokes to HanziLookup, a JavaScript library, with an 808 KB
data file beside it. There is no Rust port, so
[`src/recognize.rs`](src/recognize.rs) matches strokes against the same medians the
animation is drawn along.

`scripts/build-data.py` emits `public/data/strokes.bin`: every vendored character's
medians, **normalised** and **resampled**. Normalising scales all of a character's points
by the longer side of their bounding box and centres them, so absolute size and position
drop out but proportion survives — 一 still spreads across one line and 目 still stands in
a narrow column. Resampling then reduces each stroke to six evenly spaced points, which is
what makes a median's handful of samples comparable to the few hundred points a finger
drags out. Quantised to a byte per coordinate, that is 352 KB for 2,970 characters, and it
is fetched only when somebody opens the pad.

A drawing is put through the same two steps, so the comparison is like for like — which is
why the normalisation in the generator and the one in `recognize.rs` are best read as two
halves of one algorithm. Two strokes are then scored by the mean distance between their
resampled points, which is sensitive to the direction a stroke was drawn in; 横 written
right to left is not the same stroke, though drawing it backwards is a common enough slip
that the reverse is tried too, under a penalty.

Whole characters are compared by aligning their stroke lists with a banded edit distance,
so one stroke too many or too few costs a gap rather than shifting every stroke after it
out of position — which matters, because miscounting strokes is exactly what a learner
does. The stroke count still counts for something on its own: without a small penalty for
disagreeing about it, a plain square box ranks 曰 above 口, since the calligraphic 口 in
the data narrows towards its base and fits a drawn square *worse* than 曰's outer box
does.

A twelve-stroke drawing is matched against every plausible template in about 15 ms, after
a 450 ms pause that keeps the matching from interrupting somebody mid-character.

---

## What's Built

- **Home** — search plus the most common characters as an immediate way in, and your
  recently viewed.
- **Draw** (`/draw`) — write a character on a 米字格 with a finger or a stylus and get the
  characters it looks like, as tiles that lead to the usual character page. Undo, and a
  tinted Clear for starting over; matching runs once you pause. The pad comes back empty
  from a character, ready for the next one. Offered beside the search field as well as in
  the tab bar, because that is where somebody discovers they cannot type what they are
  looking at.
- **Search** (`/search?q=`) — three tiers, tried in order so a typed character never gets
  buried: any Han characters in the query (which makes pasting a sentence useful), then
  toneless pinyin with exact syllables ahead of prefixes, then English gloss substring.
  `ü` folds to `v`, because every Chinese IME types it that way.
- **Character detail** (`/character/好`) — the animated stroke player with play/pause,
  per-stroke stepping, a tappable per-stroke scrubber, 0.5×–2× speed and looping; a
  static stroke-order filmstrip; numbered senses; radical, stroke count and decomposition;
  other characters sharing the radical; Mandarin pronunciation via the browser's speech
  synthesiser; and save-to-list. Back leads where you came from and says so: **Draw** for
  a character the pad opened, which the pad notes per tab in `sessionStorage` because the
  URL is deliberately the same however it was reached; **Back** for anywhere else in the
  app; and **Browse** for a shared link opened cold, since `can_go_back` is always true on
  the web and would otherwise walk out of the app.
- **HSK** (`/hsk`) — the syllabus from both ends. The course is `/hsk/:level` for a level's
  units and `/hsk/:level/:unit` for the lesson itself: its words with pinyin and meaning,
  the characters they are written with, the units either side of it, and one button that
  turns the whole lesson into a saved list. `/hsk/band/:band` is the character side, all
  seven bands including the merged 7-9 the course does not cover.
- **My Lists** (`/lists`, `/lists/:id`) — several named lists in `localStorage`, started
  empty or taken whole from an HSK unit, either from the lesson page or from the unit
  picker on `/lists`. A list can be renamed, deleted, and pruned a word at a time, and one
  imported from a unit links back to the lesson it came from. Saved and recently viewed
  characters live on the same page, because they answer a different question from a list.

Importing a unit is idempotent, which is the whole point of a one-tap button: a list
remembers the unit it came from, so the second tap opens that list instead of making a
second copy of it. A list that has since been pruned is left alone — the words that are
missing were removed on purpose.

### Design

Follows [Apple's Human Interface Guidelines](https://developer.apple.com/design/) as
closely as the web allows: the iOS type scale and system colours, 44 px minimum hit
targets, chrome that defers to content, and depth carried by translucency and hairlines
rather than heavy borders. Mobile-first at ~390 px, with a bottom tab bar on phones and
the same destinations inline in the top bar on wider viewports. Dark mode follows
`prefers-color-scheme`; motion respects `prefers-reduced-motion`.

Cinnabar is the single accent — the colour of seal paste (印泥), so it belongs to the
subject rather than being decoration. It marks the stroke currently being drawn, which
doubles as the animation's primary affordance.

Going back is one control everywhere: top-leading, a chevron, and the name of where it
leads. A page with one parent names it — `HSK 1` from a lesson, `My Lists` from a list —
and only the character page, which can be arrived at from anywhere, has to work out at
runtime whether that is Draw, the previous page, or Browse.

---

## Roadmap: Parity With mssl-strok

The React app is considerably further along. Ordered by what it would take to close the
gap:

### HSK course

Characters, units and words are here: every character carries its band, and all 512 units
of all six levels are browsable and importable. What the React app still has that this does
not:

- **Example sentences.** 7,596 Tatoeba sentences with per-token pinyin, 83% of words
  covered. They are in the source course files and are dropped by the import for want of a
  page to put them on; adding one means keeping them, which roughly doubles a level file.
- **Part of speech, traditional forms, classifiers.** Also in the source and also dropped,
  for the same reason. Cheap to restore — a field each in `scripts/build-hsk-data.py` and
  in `src/hsk.rs`.
- **Practice and review.** Eight exercise kinds (meaning, recall, pinyin, listening,
  fill-the-gap, sentence building, speaking, writing), a tap-to-pair warm-up, and
  distractors ranked by similarity. Needs a seeded PRNG so a session does not reshuffle
  on re-render.
- **Spaced repetition.** Leitner boxes over `[0, 1, 2, 4, 8, 16, 32]` day intervals, with
  cross-level review, per-unit progress and a day streak.
- **Level guides.** Hand-written teaching pages per band: grammar points, pronunciation
  focus, pitfalls, a study plan.

### Voice

Mandarin playback works. What's missing:

- A global voice-speed preference (0.6–1.4) read at play time rather than baked into
  components, plus a slow-playback variant for copying rhythm.
- Sequenced utterances chained on `end` rather than queued, for tone ladders. Queueing
  immediately after `cancel()` makes several engines drop everything after the first
  utterance — worth copying the React app's workaround verbatim.
- Speech **recognition** for the speaking exercise, scored by longest-common-subsequence
  ratio against the target, degrading to self-grading where unsupported.

### My Lists

Named lists and course imports are here. Still missing:

- **Studying a list.** Hiding the pinyin, the translation, or both, per list, which is what
  turns a list into a drill rather than a reference.
- **Reordering**, and adding a word straight from the dictionary or the handwriting pad
  rather than only from a unit.
- **Import and export** as plain text. The stored document already matches the React app's
  `stroke:lists:v1` shape field for field, so the two could exchange lists as JSON.

### Login

Deliberately deferred; there is no fake login. The React app has an inert Better Auth
scaffold that gates nothing, so there is no behaviour to port yet. Everything is on-device
until accounts exist, at which point lists and progress become the thing worth syncing.

### Also in the React app, not here

Writing practice with stroke scoring (cosine similarity, start/end distance and Fréchet
distance against the median, in the 1024 grid), photo OCR through Tesseract, a printable
practice worksheet, pinyin and radical browse indexes, and a parallel Khmer script
section.

Handwriting input has landed here — see Draw above — though the React app also offers its
pad when adding words to a list, which this does not yet.

---

## Layout

```
src/
  main.rs                     route table, app root
  data.rs                     search index, per-character fetch, the search tiers
  index.rs                    the index, loaded once and shared through context
  hsk.rs                      the course: levels, units, words, per-level fetch
  lists.rs                    named lists: the document, its mutations, localStorage
  recognize.rs                handwriting lookup: normalising, resampling, matching
  storage.rs                  saved and recent, and the raw localStorage keys the lists
                              use; where a character was opened from, in sessionStorage
  history.rs                  how long the session history was when the document loaded
  speech.rs                   Mandarin playback via speechSynthesis
  text.rs                     grouped numbers and counted nouns
  url.rs                      percent-encoding for routes carrying Han characters
  components/
    stroke_player.rs          the animation: timeline, transform, controls
    stroke_pad.rs             the writing surface: ink, undo, clear
    shell.rs                  top bar, phone tab bar, footer, page furniture
    char_list.rs              character grid and search-result rows
    word_list.rs              word rows, for a unit and for a list
    search_field.rs
    icons.rs
  routes/                     home, draw, search, character, hsk, lists, not_found
assets/main.css               design tokens and base styles
public/data/                  generated; see Data above
scripts/build-data.py         the character generator
scripts/build-hsk-data.py     the course import
scripts/vercel-*.sh           the deploy: toolchain, build, shared paths
vercel.json                   Vercel settings, SPA rewrite, cache headers
```

## Licence

Application code is [MIT](LICENSE). The generated character data is not — see
[`NOTICE`](NOTICE).
