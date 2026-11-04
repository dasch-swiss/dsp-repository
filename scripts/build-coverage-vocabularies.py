#!/usr/bin/env python3
"""Build the bundled place and period lists for the metadata editor.

Inputs: the committed corpus (`projects/*.json`, `temporal-coverage-enrichment.json` and the slim
`chronontology-periods.json` under `--data-dir`) and the full ChronOntology mirror that
`scripts/fetch-chronontology-periods.py` writes (`--chronontology-full`). Outputs: `places.json` and
`periods.json` under `areas/deposit/editor/core/data/`, loaded by `editor_core::vocabulary`. The rules
are in `docs/src/editor/project-form.md`.

A mirror period is offered only when the slim file holds its id, since submit resolves against the
slim file. Without the mirror only `places.json` is rewritten and `periods.json` is left as it is.

Usage: python3 scripts/build-coverage-vocabularies.py [--data-dir DIR] [--chronontology-full PATH]
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_DATA_DIR = "areas/access/dpe/server/data"
DEFAULT_MIRROR = ".claude/tmp/chronontology-periods.json"
OUTPUT_DIR = ROOT / "areas/deposit/editor/core/data"
PLACE_TYPES = {"Geonames", "Pleiades", "Gazetteer", "URL"}
PERIOD_URL = "https://chronontology.dainst.org/period/{}"
PERIOD_ID = re.compile(r"[A-Za-z0-9_-]+")
UNSAFE_URL = re.compile(r"[\s\"'<>\\]")
UNSAFE_NAME = re.compile(r"[<>\x00-\x1f\x7f]")
# Place detail by host: the source's name and the pattern of the id in its url. GeoNames is
# `geonames_id`'s.
PLACE_HOSTS = {
    "pleiades.stoa.org": ("Pleiades", re.compile(r"/places/(\d+)")),
    "gazetteer.dainst.org": ("DAI Gazetteer", re.compile(r"/show/(\d+)")),
    "vocab.getty.edu": ("Getty TGN", re.compile(r"/tgn/(\d+)")),
}
SOURCE_NAMES = {
    "periodo": "PeriodO",
    "Periodo": "PeriodO",
    "aat": "Getty AAT",
    "Chronontology": "ChronOntology",
    "chronontology": "ChronOntology",
}


def committed_references(projects: list[tuple[Path, dict]], field: str) -> dict[str, tuple[str, Counter]]:
    """`url -> (type, Counter of texts)` over the `http` references of `field`."""
    found: dict[str, tuple[str, Counter]] = {}
    for path, project in projects:
        for entry in project.get(field, []):
            url = entry.get("url") if isinstance(entry, dict) else None
            if not isinstance(url, str) or not url.startswith("http"):
                continue
            if UNSAFE_URL.search(url):
                sys.exit(f"{path}: {field} url `{url}` holds whitespace or a quote, angle bracket or backslash")
            type_, text = entry.get("type"), entry.get("text")
            if not isinstance(type_, str) or not isinstance(text, str) or not text:
                sys.exit(f"{path}: {field} entry `{url}` lacks a type or text")
            if url in found and found[url][0] != type_:
                sys.exit(f"{path}: `{url}` carries the types `{found[url][0]}` and `{type_}`")
            found.setdefault(url, (type_, Counter()))[1][text] += 1
    return found


def geonames_id(url: str) -> str | None:
    """The numeric id in a GeoNames url (`https://www.geonames.org/2658434/switzerland.html`)."""
    path = url.split("geonames.org", 1)[-1]
    match = re.search(r"/(\d+)(?:/|$)", path)
    return match.group(1) if match else None


def place_detail(url: str) -> str:
    """What tells a place apart from another of the same name: its source and id, else the url's host."""
    host = (urlparse(url).hostname or "").removeprefix("www.")
    if host == "geonames.org":
        place = geonames_id(url)
        return f"GeoNames {place}" if place else "GeoNames"
    if host in PLACE_HOSTS:
        name, pattern = PLACE_HOSTS[host]
        found = pattern.search(url)
        return f"{name} {found.group(1)}" if found else name
    return host


def place_entries_of(references: dict[str, tuple[str, Counter]]) -> list[dict]:
    """One entry per place and label, the place being a GeoNames id or a url."""
    groups: dict[tuple[str, str, str], Counter] = {}
    for url, (type_, texts) in references.items():
        place = geonames_id(url) if type_ == "Geonames" else None
        for text, count in texts.items():
            groups.setdefault((type_, place or url, text), Counter())[url] += count
    entries = []
    for (type_, _, text), urls in groups.items():
        url = min(urls, key=lambda url: (-urls[url], not url.startswith("https://"), url))
        entries.append({"type": type_, "url": url, "text": text, "detail": place_detail(url)})
    return entries


def mirror_name(resource: dict) -> str | None:
    """The English name, else the first name of the alphabetically first language, stripped (a
    pick must not store padding). A name holding `<`, `>` or a control character is dropped."""
    names = resource.get("names") or {}
    candidates = names.get("en") or next((names[language] for language in sorted(names) if names[language]), [])
    name = candidates[0].strip() if candidates else ""
    return name if name and not UNSAFE_NAME.search(name) else None


def dates(resource: dict) -> tuple[str, str] | None:
    """The first timespan with both a begin and an end, as the mirror spells them (`-5500`, `1200`,
    `present`). A span ending before it begins is skipped: `shared_metadata::w3cdtf::to_w3cdtf_range`
    refuses it, so the period would not resolve."""
    for span in resource.get("hasTimespan") or []:
        begin = (span.get("begin") or {}).get("at")
        end = (span.get("end") or {}).get("at")
        if not begin or not end or begin == "not specified" or end == "not specified":
            continue
        try:
            if int(begin) > int(end):
                continue
        except ValueError:
            pass
        return begin, end
    return None


def year(at: str) -> str:
    """A year as a depositor reads it: `-5500` is `5500 BC`, `0901` is `901`, anything else as given."""
    try:
        value = int(at)
    except ValueError:
        return at
    return f"{-value} BC" if value < 0 else str(value)


def span(begin: str, end: str) -> str:
    return f"{year(begin)} to {year(end)}"


def mirror_detail(resource: dict) -> str:
    begin, end = dates(resource)
    provenance = resource.get("provenance") or "ChronOntology"
    return f"{span(begin, end)} · {SOURCE_NAMES.get(provenance, provenance)}"


def committed_period_detail(entry: dict, mirror: dict, enrichment: dict) -> str:
    """The mirror's detail for a ChronOntology period it dates, else the enrichment's range
    (`begin/end`) with the type's source, all there is for an undated row."""
    if entry["type"] == "Chronontology":
        resource = mirror.get(entry["url"].rsplit("/period/", 1)[-1].rstrip("/"))
        if resource and dates(resource):
            return mirror_detail(resource)
    source = SOURCE_NAMES.get(entry["type"], entry["type"])
    date = (enrichment.get(entry["text"]) or {}).get("date")
    if not date:
        return source
    begin, _, end = date.partition("/")
    return f"{span(begin, end) if end else year(begin)} · {source}"


def period_entries(
    periods: dict[str, tuple[str, Counter]], mirror: dict, enrichment: dict, resolvable: set[str]
) -> list[dict]:
    """Every committed period (its most frequent text), then every other mirror period with a
    begin and an end date, a usable name and an id in `resolvable`."""
    entries = []
    committed_ids = {url.rsplit("/period/", 1)[-1].rstrip("/") for url in periods if "/period/" in url}
    for url, (type_, texts) in periods.items():
        entry = {"type": type_, "url": url, "text": min(texts, key=lambda text: (-texts[text], text))}
        entry["detail"] = committed_period_detail(entry, mirror, enrichment)
        entries.append(entry)
    for period_id, resource in mirror.items():
        url = PERIOD_URL.format(period_id)
        if period_id in committed_ids or period_id not in resolvable or not PERIOD_ID.fullmatch(period_id) or not dates(resource):
            continue
        name = mirror_name(resource)
        if name is not None:
            entries.append({"type": "Chronontology", "url": url, "text": name, "detail": mirror_detail(resource)})
    return entries


def write(path: Path, entries: list[dict]) -> None:
    entries.sort(key=lambda entry: (entry["text"].lower(), entry["url"]))
    path.write_text(json.dumps(entries, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--data-dir", default=DEFAULT_DATA_DIR)
    parser.add_argument("--chronontology-full", default=DEFAULT_MIRROR)
    args = parser.parse_args()

    data_dir = Path(args.data_dir)
    if not data_dir.is_absolute():
        data_dir = ROOT / data_dir
    mirror_path = Path(args.chronontology_full)
    if not mirror_path.is_absolute():
        mirror_path = ROOT / mirror_path
    project_paths = sorted((data_dir / "projects").glob("*.json"))
    if not project_paths:
        sys.exit(f"{data_dir / 'projects'}: no project files")
    projects = [(path, json.loads(path.read_text(encoding="utf-8"))) for path in project_paths]

    places = committed_references(projects, "spatialCoverage")
    for url, (type_, _) in places.items():
        if type_ not in PLACE_TYPES:
            sys.exit(
                f"`{url}` has the place type `{type_}`, expected one of {sorted(PLACE_TYPES)}; "
                "the list is PLACE_SOURCES in areas/deposit/editor/core/src/form/registry.rs"
            )
    place_entries = place_entries_of(places)

    write(OUTPUT_DIR / "places.json", place_entries)
    if not mirror_path.is_file():
        print(
            f"wrote {len(place_entries)} places to {OUTPUT_DIR}; periods.json left as it is, since "
            f"{mirror_path} is missing (run python3 scripts/fetch-chronontology-periods.py for a new period)",
            file=sys.stderr,
        )
        return

    periods = committed_references(projects, "temporalCoverage")
    mirror = json.loads(mirror_path.read_text(encoding="utf-8"))
    resolvable = set(json.loads((data_dir / "chronontology-periods.json").read_text(encoding="utf-8")))
    enrichment = json.loads((data_dir / "temporal-coverage-enrichment.json").read_text(encoding="utf-8"))
    period_list = period_entries(periods, mirror, enrichment, resolvable)

    write(OUTPUT_DIR / "periods.json", period_list)
    print(
        f"wrote {len(place_entries)} places over {len(projects)} projects and {len(period_list)} periods "
        f"({len(periods)} committed, {len(period_list) - len(periods)} from the mirror's {len(mirror)} records) "
        f"to {OUTPUT_DIR}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
