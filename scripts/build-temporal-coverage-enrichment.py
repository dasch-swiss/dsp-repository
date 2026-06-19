#!/usr/bin/env python3
"""Build the temporal-coverage enrichment table for DataCite OAI output.

Project `temporalCoverage` entries that are not ChronOntology references (free
text such as "Late Bronze Age", "18th century", "1850-1900") have no machine
date range. This tool collects every distinct entry in the project data at the
current git HEAD and produces `temporal-coverage-enrichment.json`, a reviewed
lookup table the OAI mapping consults at request time:

    { "<normalized name>": { "date": "<W3CDTF range|null>",
                             "original_name": "<name>",
                             "source": "chronontology" | "llm" | "unresolved" } }

Resolution per entry:
  1. ChronOntology URL with a timespan in the slim periods file -> use that
     authoritative timespan (`source: "chronontology"`).
  2. Otherwise the range is LLM-generated. This tool does NOT itself call an LLM;
     it emits the name with `date: null`, `source: "unresolved"` as a skeleton row
     for an operator (or an LLM agent) to fill in, then re-tag `source: "llm"`.

The run is *merge-preserving*: existing rows are never overwritten, so reviewed
LLM-filled rows survive a re-run. Only genuinely new names are added (as skeleton
rows). A coverage report is printed to stderr.

Workflow:
  1. Run this tool. New non-ChronOntology names appear with `date: null`.
  2. Fill each null `date` with a W3CDTF range (year or `start/end`, zero-padded
     to 4+ digits, `-` for BCE, `..` for an open bound) and set `source: "llm"`.
     Leave `date: null` only for names that are not time periods at all
     (e.g. "Swiss"); those are emitted as dateInformation-only.
  3. Re-run with `--check` in CI to ensure every distinct dataset name is present.

Key normalization MUST match `get_multilingual_value` in the Rust mapping
(`modules/dpe/api-oai/src/metadata/helpers.rs`): prefer the `en` value, else the
value of the lexicographically smallest language code. Reference entries are
keyed by their `text` field.

Usage:
    python3 scripts/build-temporal-coverage-enrichment.py [--data-dir DIR] [--check]

`--check` exits non-zero if a new (unseen) name appears that is not yet in the
table, without writing — so CI fails when project data adds a coverage name that
has not been enriched.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

DEFAULT_DATA_DIR = "modules/dpe/server/data"
OUTPUT_NAME = "temporal-coverage-enrichment.json"
PERIODS_NAME = "chronontology-periods.json"
PERIOD_URL_MARKER = "/period/"


# --- W3CDTF formatting (mirrors modules/dpe/core/src/w3cdtf.rs) ---------------
# Used only to convert authoritative ChronOntology year bounds. Free-text ranges
# are LLM-generated and are expected to already be valid W3CDTF.

def _year(raw) -> int | None:
    """Parse a ChronOntology bound year; None for null/'not specified'/'present'."""
    if raw is None:
        return None
    s = str(raw).strip()
    if s == "" or s.lower() in ("not specified", "present"):
        return None
    try:
        return int(s)
    except ValueError:
        return None


def _w3cdtf_year(year: int) -> str:
    return f"-{abs(year):04d}" if year < 0 else f"{year:04d}"


def w3cdtf_range(begin: int | None, end: int | None) -> str | None:
    if begin is None and end is None:
        return None
    if begin is not None and end is not None:
        if begin > end:
            return None
        return _w3cdtf_year(begin) if begin == end else f"{_w3cdtf_year(begin)}/{_w3cdtf_year(end)}"
    if begin is not None:
        return f"{_w3cdtf_year(begin)}/.."
    return f"../{_w3cdtf_year(end)}"


def normalized_key(entry: dict) -> str | None:
    """Lookup key for a temporalCoverage entry, mirroring the Rust
    `get_multilingual_value` (en, else lexicographically smallest lang) and the
    reference `text` field."""
    if "url" in entry or "type" in entry:
        return entry.get("text")
    if "en" in entry:
        return entry["en"]
    keys = sorted(entry.keys())
    return entry[keys[0]] if keys else None


def load_periods(data_dir: Path) -> dict[str, str]:
    """Map ChronOntology bare id -> W3CDTF range from the slim periods file."""
    path = data_dir / PERIODS_NAME
    if not path.exists():
        return {}
    raw = json.loads(path.read_text())
    out: dict[str, str] = {}
    for pid, period in raw.items():
        for ts in period.get("hasTimespan", []):
            rng = w3cdtf_range(_year((ts.get("begin") or {}).get("at")), _year((ts.get("end") or {}).get("at")))
            if rng:
                out[pid] = rng
                break
    return out


def period_id(url: str) -> str:
    return url.rsplit(PERIOD_URL_MARKER, 1)[-1].rstrip("/")


def collect_entries(data_dir: Path):
    """Yield (name, chronontology_url|None) for each distinct temporalCoverage
    entry across all project files."""
    seen: set[str] = set()
    for f in sorted((data_dir / "projects").glob("*.json")):
        d = json.loads(f.read_text())
        tc = d.get("temporalCoverage")
        if not tc:
            continue
        if isinstance(tc, dict):
            tc = [tc]
        for entry in tc:
            if not isinstance(entry, dict):
                continue
            key = normalized_key(entry)
            if not key or key in seen:
                continue
            seen.add(key)
            url = entry.get("url") if "chronontology" in str(entry.get("url", "")).lower() else None
            yield key, url


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--data-dir", default=DEFAULT_DATA_DIR, help=f"data directory (default: {DEFAULT_DATA_DIR})")
    ap.add_argument("--check", action="store_true", help="exit non-zero if a new un-enriched name appears; do not write")
    args = ap.parse_args()

    data_dir = Path(args.data_dir)
    out_path = data_dir / OUTPUT_NAME

    periods = load_periods(data_dir)
    existing: dict[str, dict] = json.loads(out_path.read_text()) if out_path.exists() else {}

    table = dict(existing)  # merge-preserve: keep existing (LLM-filled) rows verbatim.
    new_names: list[str] = []

    for name, url in collect_entries(data_dir):
        if name in existing:
            continue
        if url and period_id(url) in periods:
            table[name] = {"date": periods[period_id(url)], "original_name": name, "source": "chronontology"}
        else:
            # Non-ChronOntology (or timespan-less) name: skeleton row for LLM fill.
            table[name] = {"date": None, "original_name": name, "source": "unresolved"}
            new_names.append(name)

    ordered = {k: table[k] for k in sorted(table)}
    serialized = json.dumps(ordered, ensure_ascii=False, indent=2) + "\n"

    filled = sum(1 for v in ordered.values() if v["date"] is not None)
    name_only = sum(1 for v in ordered.values() if v["date"] is None)
    print(f"temporal-coverage enrichment: {len(ordered)} entries "
          f"({filled} with a range, {name_only} name-only)", file=sys.stderr)
    if new_names:
        print("new names this run (added as skeleton rows; fill `date` and set source=llm, "
              "or leave null for non-period labels):", file=sys.stderr)
        for n in sorted(new_names):
            print(f"  - {n}", file=sys.stderr)

    if args.check:
        if new_names:
            print("ERROR: new coverage names are not yet enriched; re-run without --check and fill them.", file=sys.stderr)
            return 1
        return 0

    out_path.write_text(serialized)
    print(f"wrote {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
