#!/usr/bin/env python3
"""Build Stroke Order's character data from Make Me a Hanzi + HSK 3.0 vocabulary.

Outputs three things under `public/data/`:

  index.json        A columnar search index over every character. Parallel arrays
                    rather than a list of objects, because the browser parses this
                    on first search and columnar JSON is both smaller and quicker
                    to deserialize.

  char/<hex>.json   Per character: stroke outlines, stroke medians, and the full
                    dictionary entry. Named by zero-padded Unicode code point so
                    the filenames stay ASCII and need no URL escaping.

  strokes.bin       Handwriting templates for the draw-to-look-up pad: every
                    character's medians, normalised and resampled to a fixed
                    number of points per stroke. See `recognition_record`.

By default only the HSK-ranked characters get a per-character file — that is every
character a learner meets in HSK 1 through 7-9, about 3k of the 9.5k total, and it
keeps the committed data near 8 MB. The app falls back to the jsDelivr mirror of
hanzi-writer-data (the same Make Me a Hanzi graphics) for the rare tail. Pass
--all to vendor every character instead, at roughly 32 MB.

Run it from the repo root:

    python3 scripts/build-data.py

Sources are downloaded into scripts/.cache/ and reused on later runs.

  Make Me a Hanzi          https://github.com/skishore/makemeahanzi
                           graphics.txt (stroke outlines + medians),
                           dictionary.txt (pinyin, definitions, radicals).
                           Data is LGPL / ARPHIC Public License; see NOTICE.

  complete-hsk-vocabulary  https://github.com/drkameleon/complete-hsk-vocabulary
                           HSK 3.0 bands and corpus frequency (MIT).
"""

from __future__ import annotations

import argparse
import json
import math
import os
import struct
import sys
import unicodedata
import urllib.request
from datetime import date, timezone, datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / "scripts" / ".cache"
OUT = ROOT / "public" / "data"

SOURCES = {
    "graphics.txt": "https://raw.githubusercontent.com/skishore/makemeahanzi/master/graphics.txt",
    "dictionary.txt": "https://raw.githubusercontent.com/skishore/makemeahanzi/master/dictionary.txt",
    "hsk-complete.json": "https://raw.githubusercontent.com/drkameleon/complete-hsk-vocabulary/main/complete.json",
}

# Longest definition we keep in the per-character file. Make Me a Hanzi glosses
# run to several hundred characters for common words; the tail is rarely useful.
MAX_DEFINITION = 240
# The search index carries a much shorter gloss, since it holds all 9.5k of them.
MAX_GLOSS = 64

# ── Handwriting templates ───────────────────────────────────────────────────
# Points each stroke is resampled to. Six resolves the corner in a 横折 and the
# hook on a 竖钩 while keeping the file to a third of a megabyte; eight measured no
# better against distorted drawings and cost a quarter more. The matcher in
# src/recognize.rs reads this from the header rather than assuming it, so it can
# be retuned without a matching code change.
RECOGNITION_POINTS = 6

# "Stroke Order Stroke Recognition". A four-byte magic makes a truncated or
# misrouted download fail loudly instead of being parsed as garbage.
RECOGNITION_MAGIC = b"SOSR"
RECOGNITION_VERSION = 1

# ü is typed as "v" by every Chinese IME, so index it both ways.
TONE_MAP = str.maketrans(
    {
        **{c: "a" for c in "āáǎà"},
        **{c: "e" for c in "ēéěè"},
        **{c: "i" for c in "īíǐì"},
        **{c: "o" for c in "ōóǒò"},
        **{c: "u" for c in "ūúǔù"},
        **{c: "v" for c in "ǖǘǚǜü"},
    }
)


def log(msg: str) -> None:
    print(msg, file=sys.stderr, flush=True)


def fetch(name: str) -> Path:
    """Download a source file into the cache, or reuse the cached copy."""
    CACHE.mkdir(parents=True, exist_ok=True)
    path = CACHE / name
    if path.exists() and path.stat().st_size > 0:
        log(f"  cached  {name} ({path.stat().st_size:,} bytes)")
        return path
    url = SOURCES[name]
    log(f"  fetch   {name} <- {url}")
    tmp = path.with_suffix(path.suffix + ".part")
    with urllib.request.urlopen(url, timeout=180) as res, tmp.open("wb") as f:
        while chunk := res.read(1 << 20):
            f.write(chunk)
    tmp.replace(path)
    log(f"          {path.stat().st_size:,} bytes")
    return path


def jsonl(path: Path):
    with path.open(encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                yield json.loads(line)


def toneless(pinyin: str) -> str:
    """"hǎo" -> "hao". Also strips combining tone marks, which some rows use."""
    plain = unicodedata.normalize("NFD", pinyin.lower()).translate(TONE_MAP)
    plain = "".join(c for c in plain if not unicodedata.combining(c))
    return "".join(c for c in plain if c.isalpha())


def truncate(text: str, limit: int) -> str:
    text = " ".join(text.split())
    if len(text) <= limit:
        return text
    return text[: limit - 1].rstrip(" ,;/") + "…"


def short_gloss(definition: str) -> str:
    """First sense or two of a definition, for the search results list."""
    if not definition:
        return ""
    senses = [s.strip() for s in definition.split(";") if s.strip()]
    out = ""
    for sense in senses:
        candidate = sense if not out else f"{out}; {sense}"
        if len(candidate) > MAX_GLOSS:
            break
        out = candidate
    return out or truncate(senses[0], MAX_GLOSS)


def hsk_bands(path: Path) -> tuple[dict[str, int], dict[str, int]]:
    """Per-character HSK band and corpus frequency, derived from the word lists.

    HSK 3.0 publishes word lists, not character lists, so a character's band is
    taken to be the lowest band of any word that uses it — the point at which a
    learner first meets it. Frequency is likewise the best (lowest) rank of any
    word containing it.
    """
    bands: dict[str, int] = {}
    freqs: dict[str, int] = {}
    for word in json.load(path.open(encoding="utf-8")):
        simplified = word.get("simplified") or ""
        if not simplified:
            continue

        # Levels look like ["newest-4", "new-3", "old-5"]. "new-*" is HSK 3.0,
        # which is what the app teaches; band 7 covers the merged HSK 7-9 range.
        band = None
        for tag in word.get("level") or []:
            prefix, _, num = tag.rpartition("-")
            if prefix != "new" or not num.isdigit():
                continue
            value = min(int(num), 7)
            band = value if band is None else min(band, value)
        freq = word.get("frequency") or 0

        for ch in simplified:
            if band is not None and (ch not in bands or band < bands[ch]):
                bands[ch] = band
            if freq and (ch not in freqs or freq < freqs[ch]):
                freqs[ch] = freq
    return bands, freqs


def resample(points: list[tuple[float, float]], n: int) -> list[tuple[float, float]]:
    """`n` points spaced evenly along a polyline's arc length, ends included.

    Resampling by length rather than by index is what makes a stroke's shape
    comparable no matter how many samples it was captured with — a median from
    the data has a handful of points, a finger drags out hundreds.
    """
    if len(points) == 1:
        return [points[0]] * n
    lengths = [math.dist(points[i], points[i + 1]) for i in range(len(points) - 1)]
    total = sum(lengths)
    if total <= 0:
        return [points[0]] * n

    out: list[tuple[float, float]] = []
    seg = 0
    walked = 0.0
    for k in range(n):
        target = total * k / (n - 1)
        while seg < len(lengths) - 1 and walked + lengths[seg] < target:
            walked += lengths[seg]
            seg += 1
        span = lengths[seg]
        t = 0.0 if span <= 0 else (target - walked) / span
        (x0, y0), (x1, y1) = points[seg], points[seg + 1]
        out.append((x0 + (x1 - x0) * t, y0 + (y1 - y0) * t))
    return out


def recognition_record(medians: list[list[list[float]]]) -> bytes | None:
    """One character's handwriting template.

    Medians are mapped into the unit square the same way the pad maps a finger
    drawing: the whole character is scaled by its longer side, so absolute size
    and position drop out but proportion survives. What stays is the shape and
    order of the strokes, which is all the matcher compares.

        u8                        stroke count
        strokes × pts × u8 pair   x then y, each 0..255 across the unit square

    Returns None for a character with no usable medians. This must stay in step
    with `Drawing::normalize` in src/recognize.rs — the two halves of one
    algorithm, one run here and one in the browser.
    """
    if not medians or len(medians) > 255:
        return None

    # Make Me a Hanzi's y axis points up and the app's points down. The flip has
    # to happen before anything is measured; the offset does not, because
    # centring on the bounding box removes it.
    flipped = [[(float(x), -float(y)) for x, y in stroke] for stroke in medians]
    points = [p for stroke in flipped for p in stroke]
    if not points:
        return None

    xs = [p[0] for p in points]
    ys = [p[1] for p in points]
    min_x, max_x, min_y, max_y = min(xs), max(xs), min(ys), max(ys)
    # A character with no extent at all — never seen in the data, but a stray
    # single-point median would otherwise divide by zero.
    side = max(max_x - min_x, max_y - min_y) or 1.0
    centre_x, centre_y = (min_x + max_x) / 2, (min_y + max_y) / 2

    def quantise(value: float) -> int:
        return min(255, max(0, round(value * 255)))

    out = bytearray()
    out.append(len(medians))
    for stroke in flipped:
        if not stroke:
            return None
        unit = [
            ((x - centre_x) / side + 0.5, (y - centre_y) / side + 0.5) for x, y in stroke
        ]
        for x, y in resample(unit, RECOGNITION_POINTS):
            out.append(quantise(x))
            out.append(quantise(y))
    return bytes(out)


def write_recognition(path: Path, graphics: dict[str, dict], order: list[str]) -> int:
    """Emit the handwriting template file for `order`, in that order."""
    glyphs: list[str] = []
    records: list[bytes] = []
    for ch in order:
        record = recognition_record(graphics[ch]["medians"])
        if record is not None:
            glyphs.append(ch)
            records.append(record)

    header = bytearray(RECOGNITION_MAGIC)
    header.append(RECOGNITION_VERSION)
    header.append(RECOGNITION_POINTS)
    header += struct.pack("<HI", 0, len(glyphs))
    header += struct.pack(f"<{len(glyphs)}I", *(ord(c) for c in glyphs))

    with path.open("wb") as f:
        f.write(header)
        for record in records:
            f.write(record)
    return len(glyphs)


def build() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        "--all",
        action="store_true",
        help="emit a per-character file for all ~9.5k characters (~32 MB) "
        "instead of just the HSK-ranked ones",
    )
    group.add_argument(
        "--limit",
        type=int,
        default=0,
        help="emit per-character files for the top N characters by rank",
    )
    args = parser.parse_args()

    log("Sources")
    graphics_path = fetch("graphics.txt")
    dictionary_path = fetch("dictionary.txt")
    hsk_path = fetch("hsk-complete.json")

    log("Reading HSK 3.0 word lists")
    bands, freqs = hsk_bands(hsk_path)
    log(f"  {len(bands):,} characters carry an HSK band")

    log("Reading stroke graphics")
    graphics: dict[str, dict] = {}
    for row in jsonl(graphics_path):
        ch = row.get("character")
        strokes = row.get("strokes") or []
        medians = row.get("medians") or []
        if not ch or not strokes or len(strokes) != len(medians):
            continue
        if len(ch) != 1:
            # The index addresses characters by position in a single string, so a
            # multi-code-point entry would desynchronise it.
            log(f"  skip    non-scalar entry {ch!r}")
            continue
        graphics[ch] = {"strokes": strokes, "medians": medians}
    log(f"  {len(graphics):,} characters with stroke data")

    log("Reading dictionary")
    entries: dict[str, dict] = {}
    for row in jsonl(dictionary_path):
        ch = row.get("character")
        if not ch or ch not in graphics:
            continue
        pinyin = [p for p in (row.get("pinyin") or []) if p]
        keys: list[str] = []
        for reading in pinyin:
            key = toneless(reading)
            if key and key not in keys:
                keys.append(key)
        decomposition = row.get("decomposition") or ""
        entries[ch] = {
            "char": ch,
            "pinyin": pinyin,
            "keys": keys,
            "definition": truncate(row.get("definition") or "", MAX_DEFINITION),
            "radical": row.get("radical") or ch,
            # "？" is Make Me a Hanzi's placeholder for "not decomposed".
            "decomposition": "" if "？" in decomposition else decomposition,
            "strokeCount": len(graphics[ch]["strokes"]),
            "hsk": bands.get(ch, 0),
        }

    # Characters with graphics but no dictionary row still deserve a page.
    for ch, art in graphics.items():
        entries.setdefault(
            ch,
            {
                "char": ch,
                "pinyin": [],
                "keys": [],
                "definition": "",
                "radical": ch,
                "decomposition": "",
                "strokeCount": len(art["strokes"]),
                "hsk": bands.get(ch, 0),
            },
        )
    log(f"  {len(entries):,} entries")

    # Rank: HSK characters first by band, then by corpus frequency, then by the
    # simpler character. This is what orders search results and the home page.
    def rank_key(ch: str) -> tuple:
        band = entries[ch]["hsk"] or 99
        return (band, freqs.get(ch, 10**9), entries[ch]["strokeCount"], ch)

    ranked = sorted((ch for ch in entries if entries[ch]["hsk"]), key=rank_key)
    rank = {ch: i + 1 for i, ch in enumerate(ranked)}
    log(f"  {len(rank):,} ranked characters")

    # Emit the search index. Order is rank first, then everything else by stroke
    # count, so a prefix of the index is already the "most useful" slice.
    order = ranked + sorted(
        (ch for ch in entries if ch not in rank),
        key=lambda c: (entries[c]["strokeCount"], c),
    )

    # Which characters get a vendored per-character file. `order` is rank-first,
    # so a prefix of it is always the most-used slice.
    if args.all:
        subset = order
    elif args.limit:
        subset = order[: args.limit]
    else:
        subset = ranked

    OUT.mkdir(parents=True, exist_ok=True)
    index = {
        "version": 1,
        "generated": datetime.now(timezone.utc).date().isoformat(),
        "count": len(order),
        "ranked": len(rank),
        # The app treats the first `bundled` characters of `chars` as locally
        # available and falls back to the CDN mirror for the rest.
        "bundled": len(subset),
        "sources": {
            "graphics": "Make Me a Hanzi (LGPL / ARPHIC Public License)",
            "definitions": "CC-CEDICT and Unihan, via Make Me a Hanzi",
            "hsk": "complete-hsk-vocabulary (MIT), HSK 3.0 bands",
        },
        # Columnar: one entry per character, at the same offset in every array.
        "chars": "".join(order),
        "pinyin": [(entries[c]["pinyin"] or [""])[0] for c in order],
        "keys": [" ".join(entries[c]["keys"]) for c in order],
        "gloss": [short_gloss(entries[c]["definition"]) for c in order],
        "strokes": [entries[c]["strokeCount"] for c in order],
        "hsk": [entries[c]["hsk"] for c in order],
        "radical": "".join(entries[c]["radical"][:1] or c for c in order),
    }
    index_path = OUT / "index.json"
    with index_path.open("w", encoding="utf-8") as f:
        json.dump(index, f, ensure_ascii=False, separators=(",", ":"))
    log(f"Wrote {index_path.relative_to(ROOT)} ({index_path.stat().st_size:,} bytes)")

    # Emit per-character files.
    char_dir = OUT / "char"
    char_dir.mkdir(parents=True, exist_ok=True)
    for stale in char_dir.glob("*.json"):
        stale.unlink()

    total = 0
    for ch in subset:
        entry = dict(entries[ch])
        entry["rank"] = rank.get(ch, 0)
        entry["strokes"] = graphics[ch]["strokes"]
        entry["medians"] = graphics[ch]["medians"]
        path = char_dir / f"{ord(ch):05x}.json"
        with path.open("w", encoding="utf-8") as f:
            json.dump(entry, f, ensure_ascii=False, separators=(",", ":"))
        total += path.stat().st_size
    log(
        f"Wrote {len(subset):,} files to {char_dir.relative_to(ROOT)}/ "
        f"({total:,} bytes, {total // max(len(subset), 1):,} avg)"
    )

    # Handwriting templates, for the same subset. Restricting them to what is
    # vendored is deliberate: every character the pad can offer then has stroke
    # data on the same origin, so tapping a candidate never depends on the CDN.
    strokes_path = OUT / "strokes.bin"
    written = write_recognition(strokes_path, graphics, subset)
    log(
        f"Wrote {strokes_path.relative_to(ROOT)} "
        f"({written:,} templates, {strokes_path.stat().st_size:,} bytes)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(build())
