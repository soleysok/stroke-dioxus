# Stroke Order

Learn Chinese characters stroke by stroke. A [Dioxus](https://dioxuslabs.com/) (Rust →
WebAssembly) rewrite of the production app at
[stroke-mssl.vercel.app](https://stroke-mssl.vercel.app), built in parallel and
independently.

Look up any of 9,574 characters by character, pinyin, or English meaning, then watch it
written one stroke at a time in the order a native writer would use.

> The React/TanStack app in `soleysok/mssl-strok` stays in production and is untouched.
> This repository is the parallel rebuild; it was read only as a feature and data
> reference.

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

Current release payload: **787 KB of wasm (297 KB gzipped)** plus a 518 KB search index,
fetched once.

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
and only believes the answer. When that fails it falls back to `cargo install
dioxus-cli`, which costs about 17 minutes of CPU — call it ten on a two-core Basic build
machine, against Vercel's 45-minute limit.

To avoid paying that on every deploy, the compiled binary is parked in
`node_modules/.cache`, the only directory Vercel restores between builds. A warm build
skips to the app itself, which is about a minute.

[`scripts/vercel-build.sh`](scripts/vercel-build.sh) then runs the release build and
copies `target/dx/stroke/release/web/public` to `dist/`, so the Output Directory setting
does not have to track `dx`'s layout across CLI versions. It fails loudly if the wasm or
the search index is missing, which is the difference between a failed deploy and a blank
page in production.

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

Everything under `public/data/` is generated. To rebuild it:

```bash
python3 scripts/build-data.py
```

The script downloads its sources into `scripts/.cache/` (reused on later runs) and emits:

| Output | Contents |
| --- | --- |
| `public/data/index.json` | 518 KB columnar search index over all 9,574 characters — reading, toneless pinyin keys, short gloss, stroke count, HSK band, radical. |
| `public/data/char/<hex>.json` | Per character: stroke outlines, stroke medians, and the full dictionary entry. Named by zero-padded Unicode code point so filenames stay ASCII. |

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

---

## What's Built

- **Home** — search plus the most common characters as an immediate way in, and your
  recently viewed.
- **Search** (`/search?q=`) — three tiers, tried in order so a typed character never gets
  buried: any Han characters in the query (which makes pasting a sentence useful), then
  toneless pinyin with exact syllables ahead of prefixes, then English gloss substring.
  `ü` folds to `v`, because every Chinese IME types it that way.
- **Character detail** (`/character/好`) — the animated stroke player with play/pause,
  per-stroke stepping, a tappable per-stroke scrubber, 0.5×–2× speed and looping; a
  static stroke-order filmstrip; numbered senses; radical, stroke count and decomposition;
  other characters sharing the radical; Mandarin pronunciation via the browser's speech
  synthesiser; and save-to-list.
- **HSK** (`/hsk`, `/hsk/:band`) — all seven bands with character counts, browsable down
  to the characters each band introduces.
- **My Lists** (`/lists`) — saved and recently viewed characters, in `localStorage`.

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

---

## Roadmap: Parity With mssl-strok

The React app is considerably further along. Ordered by what it would take to close the
gap:

### HSK course

The character side is done — every character carries its HSK 3.0 band. The course is not.

- **Word lists and units.** The React app ships 5,369 HSK words across 512 themed units,
  with pinyin, CC-CEDICT meanings, part of speech, traditional forms, classifiers, and
  7,596 Tatoeba example sentences with per-token pinyin. Needs a second generator pass
  over `complete-hsk-vocabulary` plus a Tatoeba sentence pass, emitting per-level files.
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

- Several named lists you can reorder and study as a set, rather than one flat list.
- Import and export as plain text.

### Login

Deliberately deferred; there is no fake login. The React app has an inert Better Auth
scaffold that gates nothing, so there is no behaviour to port yet. Everything is on-device
until accounts exist, at which point lists and progress become the thing worth syncing.

### Also in the React app, not here

Writing practice with stroke scoring (cosine similarity, start/end distance and Fréchet
distance against the median, in the 1024 grid), handwriting input via HanziLookup, photo
OCR through Tesseract, a printable practice worksheet, pinyin and radical browse indexes,
and a parallel Khmer script section.

---

## Layout

```
src/
  main.rs                     route table, app root
  data.rs                     search index, per-character fetch, the search tiers
  index.rs                    the index, loaded once and shared through context
  storage.rs                  saved and recent, in localStorage
  speech.rs                   Mandarin playback via speechSynthesis
  url.rs                      percent-encoding for routes carrying Han characters
  components/
    stroke_player.rs          the animation: timeline, transform, controls
    shell.rs                  top bar, phone tab bar, footer, page furniture
    char_list.rs              character grid and search-result rows
    search_field.rs
    icons.rs
  routes/                     home, search, character, hsk, lists, not_found
assets/main.css               design tokens and base styles
public/data/                  generated; see Data above
scripts/build-data.py         the generator
scripts/vercel-*.sh           the deploy: toolchain, build, shared paths
vercel.json                   Vercel settings, SPA rewrite, cache headers
```

## Licence

Application code is [MIT](LICENSE). The generated character data is not — see
[`NOTICE`](NOTICE).
