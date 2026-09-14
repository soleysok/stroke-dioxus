#!/usr/bin/env python3
"""Import the HSK course units from the React app's committed level files.

The curriculum — which words a unit teaches, in what order, under what title — is
editorial work that already exists in `soleysok/mssl-strok`, built there by
`scripts/build-hsk-data.mjs` from complete-hsk-vocabulary plus a Tatoeba sentence
pass. Regenerating it here would invent a second, slightly different course, so
this script copies that one instead and trims it to what this app renders.

Outputs under `public/data/hsk/`:

  index.json        Per-level unit, word and character counts, plus provenance.
                    Small enough to be compiled into the wasm (see src/hsk.rs),
                    so the HSK hub renders without a fetch.

  level-<n>.json    One level: its units in teaching order, and a word map with
                    the pinyin and meanings each unit row shows.

What is dropped, and why: example sentences with per-token pinyin (85% of the
source bytes, and this app has no sentence UI yet), part of speech, traditional
forms, classifiers, corpus frequency, and the per-level character detail — this
app already has all 9,574 characters in `public/data/index.json`, so a unit's
characters are resolved against that at runtime rather than shipped twice. The
trim takes the six levels from 3.2 MB to 505 KB; nothing a unit page shows is
lost. Because the reader ignores unknown fields, dropping an untrimmed copy of
the source files in place of these still works.

Run it from the repo root, with a checkout of the React app beside this one:

    git clone https://github.com/soleysok/mssl-strok ../mssl-strok
    python3 scripts/build-hsk-data.py

Vocabulary and meanings are ultimately complete-hsk-vocabulary (MIT) and CC-CEDICT
(CC BY-SA 4.0); see NOTICE.
"""

from __future__ import annotations

import argparse
import json
import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "public" / "data" / "hsk"

DEFAULT_SOURCE = ROOT.parent / "mssl-strok" / "public" / "data" / "hsk"

LEVELS = range(1, 7)

# The course file format. Bumped only when the reader in src/hsk.rs has to change.
VERSION = 1

# Meanings a row can show before it stops being a gloss and starts being an
# article. The React app's list rows keep three; so does this.
MAX_MEANINGS = 3

# Provenance, recorded in index.json so the data can be traced without this
# script in hand.
SOURCE_REPO = "https://github.com/soleysok/mssl-strok"
SOURCES = {
    "course": "soleysok/mssl-strok public/data/hsk/level-*.json — units, titles and word order",
    "vocabulary": "drkameleon/complete-hsk-vocabulary (MIT) — HSK 3.0 bands 1-6",
    "meanings": "CC-CEDICT (CC BY-SA 4.0) via the same dataset",
}

HAN = ((0x3400, 0x9FFF), (0x3005, 0x3005), (0xF900, 0xFAFF))


def log(msg: str) -> None:
    print(msg, file=sys.stderr, flush=True)


def is_han(ch: str) -> bool:
    point = ord(ch)
    return any(low <= point <= high for low, high in HAN)


def clean(text: object, limit: int) -> str:
    if not isinstance(text, str):
        return ""
    return " ".join(text.split())[:limit]


def read_level(source: Path, level: int) -> dict:
    path = source / f"level-{level}.json"
    if not path.exists():
        raise SystemExit(
            f"No {path}.\n"
            f"Clone the course app beside this one, or pass --from:\n"
            f"    git clone {SOURCE_REPO} ../mssl-strok\n"
            f"    python3 scripts/build-hsk-data.py --from PATH"
        )
    with path.open(encoding="utf-8") as f:
        raw = json.load(f)
    if raw.get("level") != level:
        raise SystemExit(f"{path} declares level {raw.get('level')!r}, expected {level}")
    return raw


def trim_level(raw: dict, level: int) -> tuple[dict, int]:
    """One source level, reduced to units and the words they teach."""
    words: dict[str, dict] = {}
    for word, entry in raw.get("words", {}).items():
        meanings = [
            clean(m, 160) for m in entry.get("meanings", [])[:MAX_MEANINGS] if clean(m, 160)
        ]
        words[word] = {"pinyin": clean(entry.get("pinyin"), 60), "meanings": meanings}

    units = []
    seen_ids: set[str] = set()
    for unit in raw.get("units", []):
        unit_id = clean(unit.get("id"), 16)
        title = clean(unit.get("title"), 80)
        headwords = [w for w in unit.get("words", []) if isinstance(w, str) and w in words]
        if not unit_id or not title or not headwords:
            raise SystemExit(f"HSK {level} has an incomplete unit: {unit!r}")
        if unit_id in seen_ids:
            raise SystemExit(f"HSK {level} repeats unit id {unit_id}")
        seen_ids.add(unit_id)
        units.append(
            {
                "id": unit_id,
                "title": title,
                "topic": clean(unit.get("topic"), 32),
                "words": headwords,
            }
        )

    if not units:
        raise SystemExit(f"HSK {level} has no units")

    # Only words a unit actually teaches: the source's word map is the whole
    # level list, and a word no unit references would be dead weight.
    taught = {w for unit in units for w in unit["words"]}
    dropped = len(words) - len(taught)
    words = {w: entry for w, entry in words.items() if w in taught}

    return {"version": VERSION, "level": level, "units": units, "words": words}, dropped


def write_json(path: Path, payload: object) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    # Compact separators, since these are fetched rather than read: the whitespace
    # in level-6 alone is 20 KB.
    text = json.dumps(payload, ensure_ascii=False, separators=(",", ":"), sort_keys=False)
    path.write_text(text + "\n", encoding="utf-8")
    return path.stat().st_size


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--from",
        dest="source",
        type=Path,
        default=DEFAULT_SOURCE,
        help=f"directory holding the source level-*.json (default: {DEFAULT_SOURCE})",
    )
    args = parser.parse_args()

    log(f"Importing HSK course units from {args.source}")

    summaries = []
    total = 0
    for level in LEVELS:
        trimmed, dropped = trim_level(read_level(args.source, level), level)
        chars = {ch for word in trimmed["words"] for ch in word if is_han(ch)}
        size = write_json(OUT / f"level-{level}.json", trimmed)
        total += size
        summaries.append(
            {
                "level": level,
                "unitCount": len(trimmed["units"]),
                "wordCount": len(trimmed["words"]),
                "charCount": len(chars),
                "bytes": size,
            }
        )
        note = f", {dropped} unused words dropped" if dropped else ""
        log(
            f"  HSK {level}  {len(trimmed['units']):>3} units  "
            f"{len(trimmed['words']):>4} words  {len(chars):>4} characters  "
            f"{size / 1024:6.1f} KB{note}"
        )

    index = {
        "version": VERSION,
        "generatedAt": date.today().isoformat(),
        "sources": SOURCES,
        "levels": summaries,
    }
    index_size = write_json(OUT / "index.json", index)

    log(
        f"  index    {index_size:,} bytes\n"
        f"  total    {total / 1024:.0f} KB across {len(summaries)} levels, "
        f"{sum(s['wordCount'] for s in summaries):,} words in "
        f"{sum(s['unitCount'] for s in summaries):,} units"
    )


if __name__ == "__main__":
    main()
