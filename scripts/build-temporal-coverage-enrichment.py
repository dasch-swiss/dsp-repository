#!/usr/bin/env python3
"""Build the temporal-coverage enrichment table for DataCite OAI output.

Some project `temporalCoverage` entries cannot be resolved through ChronOntology
(free-text names, or ChronOntology periods that carry no timespan). This tool
collects every distinct temporalCoverage entry in the project data at the current
git HEAD and produces `temporal-coverage-enrichment.json`, a reviewed lookup table
the OAI mapping consults at request time:

    { "<normalized name>": { "date": "<W3CDTF range|null>",
                             "original_name": "<name>",
                             "source": "parsed" | "chronontology" | "llm" } }

Resolution per entry:
  1. ChronOntology URL with a timespan in the slim periods file -> use it.
  2. Otherwise, parse common free-text forms deterministically (numeric ranges,
     centuries, BC/AD/BCE/CE, "today"/"present").
  3. Otherwise, look the name up in CURATED_PERIODS (researched date ranges for
     named historical/archaeological periods), tagged `source: "llm"`.
  4. Otherwise, emit the name with `date: null` (the OAI mapping then emits a
     dateInformation-only date). These are reported as unresolved.

The run is *merge-preserving*: existing rows in the output file are never
overwritten, so hand-corrected or reviewed entries survive a re-run. Only
genuinely new names are added. A coverage report is printed to stderr.

Key normalization MUST match `get_multilingual_value` in the Rust mapping
(`modules/dpe/api-oai/src/metadata/helpers.rs`): prefer the `en` value, else the
value of the lexicographically smallest language code. Reference entries are
keyed by their `text` field.

Usage:
    python3 scripts/build-temporal-coverage-enrichment.py [--data-dir DIR] [--check]

`--check` exits non-zero if the table would change (for CI), without writing.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

DEFAULT_DATA_DIR = "modules/dpe/server/data"
OUTPUT_NAME = "temporal-coverage-enrichment.json"
PERIODS_NAME = "chronontology-periods.json"
PERIOD_URL_MARKER = "/period/"


# --- W3CDTF formatting (mirrors modules/dpe/core/src/w3cdtf.rs) ---------------

def w3cdtf_year(year: int) -> str:
    """Zero-pad a (possibly negative) year to at least four digits, keep BCE sign."""
    return f"-{abs(year):04d}" if year < 0 else f"{year:04d}"


def w3cdtf_range(begin: int | None, end: int | None) -> str | None:
    """Assemble a W3CDTF range; None when both bounds missing or begin > end."""
    if begin is None and end is None:
        return None
    if begin is not None and end is not None:
        if begin > end:
            return None
        return w3cdtf_year(begin) if begin == end else f"{w3cdtf_year(begin)}/{w3cdtf_year(end)}"
    if begin is not None:
        return f"{w3cdtf_year(begin)}/.."
    return f"../{w3cdtf_year(end)}"


# --- Deterministic free-text parsing ------------------------------------------

# The current year is used for open-ended "today"/"present" forms.
CURRENT_YEAR = 2026

_BC = re.compile(r"\b(\d{1,5})\s*(?:BC|BCE)\b", re.IGNORECASE)
_AD = re.compile(r"\b(?:AD|CE)\s*(\d{1,5})\b|\b(\d{1,5})\s*(?:AD|CE)\b", re.IGNORECASE)


def _year_token(tok: str) -> int | None:
    """Parse a single year token like '1850', '300 BC', 'AD 400', or 'today'."""
    tok = tok.strip()
    low = tok.lower()
    if low in ("today", "present", "now"):
        return CURRENT_YEAR
    m = _BC.search(tok)
    if m:
        return -int(m.group(1))
    m = _AD.search(tok)
    if m:
        return int(m.group(1) or m.group(2))
    m = re.fullmatch(r"-?\d{1,5}", tok)
    if m:
        return int(tok)
    return None


# No trailing \b: some forms end in "c." or "Jh." where the final char is a
# period, after which \b would not match.
_CENTURY_WORD = r"(?:cent(?:ury|uries)?|c\.|siècles?|jh\.?|jhd\.?|dynasty)"
_CENTURY = re.compile(rf"\b(\d{{1,2}})(?:st|nd|rd|th|e|er|\.)?\s*{_CENTURY_WORD}", re.IGNORECASE)

# A century range like "15th-16th c.", "16th to 18th centuries".
_CENTURY_RANGE = re.compile(
    rf"\b(\d{{1,2}})(?:st|nd|rd|th|e|er|\.)?\s*(?:-|–|to|and)\s*(\d{{1,2}})(?:st|nd|rd|th|e|er|\.)?\s*{_CENTURY_WORD}",
    re.IGNORECASE,
)

# A decade like "1990s", "2020s".
_DECADE = re.compile(r"\b(\d{3})0s\b")

# Roman-numeral century, e.g. "XIXe siècle", "XVIIIe siècle".
_ROMAN_CENTURY = re.compile(r"\b([IVXLC]+)(?:e|er|ème)?\s*siècles?\b", re.IGNORECASE)

_ROMAN = {"I": 1, "V": 5, "X": 10, "L": 50, "C": 100}


def _roman_to_int(s: str) -> int | None:
    s = s.upper()
    total, prev = 0, 0
    for ch in reversed(s):
        if ch not in _ROMAN:
            return None
        val = _ROMAN[ch]
        total += -val if val < prev else val
        prev = max(prev, val)
    return total or None


def _century_to_range(n: int, bce: bool = False) -> tuple[int, int]:
    """Nth century AD -> (start, end) years, e.g. 19 -> (1800, 1899)."""
    start = (n - 1) * 100 + 1
    end = n * 100
    if bce:
        return (-end, -start)
    return (start, end)


def parse_free_text(name: str) -> str | None:
    """Best-effort deterministic parse of a free-text temporal name to W3CDTF.

    Handles: explicit year ranges with -, –, "to", "and"; single years; BC/AD/
    BCE/CE; "today"/"present"; and simple "Nth century" forms. Returns None when
    no confident parse is possible (caller falls back to the curated table)."""
    text = name.strip()
    bce = bool(re.search(r"\b(BC|BCE)\b", text, re.IGNORECASE))

    # A label may carry an embedded explicit range after a comma or colon, e.g.
    # "Second Empire, 1852-1870", "Photography: 1850-today". Parse the tail.
    tail = re.split(r"[,:]\s*", text, maxsplit=1)
    if len(tail) == 2:
        embedded = parse_free_text(tail[1])
        if embedded:
            return embedded

    # Century range: "15th-16th c.", "16th to 18th centuries".
    m = _CENTURY_RANGE.search(text)
    if m:
        s, _ = _century_to_range(int(m.group(1)), bce)
        _, e = _century_to_range(int(m.group(2)), bce)
        return w3cdtf_range(s, e)

    # Range: "1850-1900", "1850–today", "1700 to 2020", "300 BC", "AD 200"
    parts = re.split(r"\s*(?:-|–|—|to|and|\bbis\b)\s*", text, maxsplit=1, flags=re.IGNORECASE)
    if len(parts) == 2:
        begin = _year_token(parts[0])
        end = _year_token(parts[1])
        if begin is not None and end is not None:
            return w3cdtf_range(begin, end)

    # Single year / BC / AD / today.
    single = _year_token(text)
    if single is not None:
        return w3cdtf_range(single, single)

    # Decade: "1990s" -> 1990/1999.
    m = _DECADE.fullmatch(text)
    if m:
        decade = int(m.group(1)) * 10
        return w3cdtf_range(decade, decade + 9)

    # Roman-numeral century: "XIXe siècle".
    m = _ROMAN_CENTURY.search(text)
    if m:
        n = _roman_to_int(m.group(1))
        if n:
            start, end = _century_to_range(n, bce)
            return w3cdtf_range(start, end)

    # Arabic-numeral century / dynasty / Jahrhundert: "18th century", "15. Jh.".
    m = _CENTURY.search(text)
    if m:
        start, end = _century_to_range(int(m.group(1)), bce)
        return w3cdtf_range(start, end)

    return None


# --- Curated ranges for named periods (researched; tagged source "llm") -------
# Approximate, conventional date spans for named historical/archaeological/art
# periods that appear in the project data. Ranges are intentionally broad; the
# original name is always preserved in dateInformation so consumers see the
# qualitative label alongside the approximation. Review before trusting.
CURATED_PERIODS: dict[str, tuple[int | None, int | None]] = {
    # General Western periodisation
    "Middle Ages": (500, 1500),
    "Medieval": (500, 1500),
    "Late Middle Ages": (1250, 1500),
    "Moyen Âge classique": (1000, 1250),
    "Early Modern": (1500, 1800),
    "Modern period": (1800, 1945),
    "Contemporary period": (1945, CURRENT_YEAR),
    "Classical World": (-800, 500),
    "Long Twentieth Century": (1870, 1991),
    # Enlightenment / early modern French
    "Siècle des Lumières": (1715, 1789),
    "Century of Reformation (Europe)": (1517, 1648),
    "Baroque period, seventeenth and eighteenth centuries": (1600, 1750),
    # Egyptian chronology
    "Middle Kingdom": (-2055, -1650),
    "Second Intermediate Period": (-1650, -1550),
    "New Kingdom": (-1550, -1069),
    "Third Intermediate Period": (-1069, -664),
    "Late Period": (-664, -332),
    "Persian Period": (-525, -332),
    # Near East / Bronze & Iron Age
    "Late Bronze Age": (-1550, -1200),
    "Iron Age": (-1200, -550),
    "Mitanni": (-1500, -1300),
    "Middle Assyrian": (-1365, -1050),
    "Neo-Assyrian": (-911, -609),
    "Hellenistic": (-323, -31),
    # Classical antiquity / art styles
    "Roman": (-27, 476),
    "Ancient Greek (culture or style)": (-800, -146),
    "Etruscan (culture or style)": (-900, -27),
    "South Italian (Greek pottery style)": (-440, -300),
    # Prehistory spans
    "Palaeolithic to Modern": (-3300000, CURRENT_YEAR),
    # Additional named periods present in the data
    "Early Christian": (30, 800),
    "Egyptian (ancient)": (-3100, -332),
    "Near Eastern (Early Western World)": (-3500, -300),
    "From Ancient Greek period to Contemporary period": (-800, CURRENT_YEAR),
    "Ottoman": (1299, 1922),
    # ChronOntology-suffixed Egyptian periods (no timespan in the mirror)
    "New Kingdom (Chronontology)": (-1550, -1069),
    "Late Period (Chronontology)": (-664, -332),
    # NOTE: "English (culture or style)" and "Swiss" are cultural/national styles,
    # not time periods — intentionally left unresolved (emitted as name-only).
}


def normalized_key(entry: dict) -> str | None:
    """Compute the lookup key for a temporalCoverage entry, mirroring the Rust
    `get_multilingual_value` (en, else lexicographically smallest lang) and the
    reference `text` field."""
    if "url" in entry or "type" in entry:
        # AuthorityFileReference: keyed by its text.
        return entry.get("text")
    # Multilingual text map.
    if "en" in entry:
        return entry["en"]
    keys = sorted(k for k in entry.keys())
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
            begin = _year_token(str((ts.get("begin") or {}).get("at"))) if (ts.get("begin") or {}).get("at") else None
            end = _year_token(str((ts.get("end") or {}).get("at"))) if (ts.get("end") or {}).get("at") else None
            rng = w3cdtf_range(begin, end)
            if rng:
                out[pid] = rng
                break
    return out


def period_id(url: str) -> str:
    return url.rsplit(PERIOD_URL_MARKER, 1)[-1].rstrip("/")


def collect_entries(data_dir: Path):
    """Yield (key, original_name, chronontology_url|None) for each distinct
    temporalCoverage entry across all project files."""
    seen: set[str] = set()
    projects = sorted((data_dir / "projects").glob("*.json"))
    for f in projects:
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
            yield key, key, url


def resolve(key: str, url: str | None, periods: dict[str, str]):
    """Return (date|None, source) for one entry."""
    if url:
        rng = periods.get(period_id(url))
        if rng:
            return rng, "chronontology"
    parsed = parse_free_text(key)
    if parsed:
        return parsed, "parsed"
    if key in CURATED_PERIODS:
        begin, end = CURATED_PERIODS[key]
        return w3cdtf_range(begin, end), "llm"
    return None, "unresolved"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--data-dir", default=DEFAULT_DATA_DIR, help=f"data directory (default: {DEFAULT_DATA_DIR})")
    ap.add_argument("--check", action="store_true", help="exit non-zero if the table would change; do not write")
    args = ap.parse_args()

    data_dir = Path(args.data_dir)
    out_path = data_dir / OUTPUT_NAME

    periods = load_periods(data_dir)
    existing: dict[str, dict] = json.loads(out_path.read_text()) if out_path.exists() else {}

    table = dict(existing)  # merge-preserve: keep existing rows verbatim.
    added, resolved_n, unresolved = 0, 0, 0
    unresolved_names: list[str] = []

    for key, name, url in collect_entries(data_dir):
        if key in existing:
            # Preserve reviewed/hand-corrected rows.
            if existing[key].get("date") is not None:
                resolved_n += 1
            else:
                unresolved += 1
                unresolved_names.append(name)
            continue
        date, source = resolve(key, url, periods)
        table[key] = {"date": date, "original_name": name, "source": source}
        added += 1
        if date is not None:
            resolved_n += 1
        else:
            unresolved += 1
            unresolved_names.append(name)

    ordered = {k: table[k] for k in sorted(table)}
    serialized = json.dumps(ordered, ensure_ascii=False, indent=2) + "\n"

    total = len(ordered)
    print(f"temporal-coverage enrichment: {total} entries "
          f"({resolved_n} resolved, {unresolved} unresolved), {added} new this run", file=sys.stderr)
    if unresolved_names:
        print("unresolved (emitted as dateInformation-only):", file=sys.stderr)
        for n in sorted(unresolved_names):
            print(f"  - {n}", file=sys.stderr)

    if args.check:
        current = out_path.read_text() if out_path.exists() else ""
        if current != serialized:
            print("ERROR: enrichment table is out of date; re-run without --check.", file=sys.stderr)
            return 1
        return 0

    out_path.write_text(serialized)
    print(f"wrote {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
