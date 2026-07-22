#!/usr/bin/env python3
"""Mirror all ChronOntology period records into a local cache.

ChronOntology (https://chronontology.dainst.org) is an Elasticsearch-backed
gazetteer of historical periods. DaSCH records reference periods by URI
(e.g. https://chronontology.dainst.org/period/IOGdcZ2CU1tz); the cached data
resolves those references into a label + timespan for DataCite temporal
coverage mapping.

Why this is not a simple paginated loop
---------------------------------------
The list API caps `from + size` at 10000 (deep pagination returns HTTP 500)
and exposes no scroll/search_after cursor and no usable facet enumeration.
With ~13.9k periods, no single `q=*` scan can reach them all.

Strategy: partition the corpus by `fq=resource.provenance:<value>`. Each
provenance slice is small enough to page fully under the 10k ceiling, and the
slices are disjoint by construction, so their union (deduped by id) is the
whole corpus. We discover the provenance vocabulary automatically by scanning
the first 10k of `q=*` -- no value needs to be hard-coded. The run asserts the
deduped union equals the API's reported total and fails loudly otherwise (e.g.
if a future slice grows past 10k, or a new provenance value hides beyond the
first 10k of the `q=*` scan).

Output
------
Writes <out>/chronontology-periods.json: a single JSON object mapping
period id -> the full `resource` object from the API. Re-running overwrites
it; the run is idempotent.

Usage
-----
    python3 scripts/fetch-chronontology-periods.py [--out DIR] [--sleep SECS]

Defaults to writing under .claude/tmp/ (gitignored).
"""

from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

BASE = "https://chronontology.dainst.org/data/period"
PAGE_SIZE = 1000  # well under the 10k from+size ceiling
CEILING = 10000  # API's hard from+size limit


def fetch(params: dict) -> dict:
    """GET the period API with the given query params, return parsed JSON."""
    url = f"{BASE}?{urllib.parse.urlencode(params)}"
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as resp:
        return json.load(resp)


def corpus_total() -> int:
    """The API's own count of all period records."""
    return int(fetch({"q": "*", "size": 0})["total"])


def discover_provenances() -> set[str]:
    """Provenance values seen while scanning the first 10k of q=*.

    The corpus is partitioned on `resource.provenance`; this finds the
    partition keys without hard-coding them. If a provenance value existed
    only beyond the first 10k records it would be missed -- the final
    coverage assertion in main() guards against that.
    """
    provs: set[str] = set()
    fetched = 0
    while fetched < CEILING:
        page = fetch({"q": "*", "size": PAGE_SIZE, "from": fetched}).get("results", [])
        if not page:
            break
        for r in page:
            provs.add(r["resource"].get("provenance", ""))
        fetched += len(page)
    provs.discard("")
    return provs


def fetch_provenance(prov: str, sleep: float, periods: dict) -> None:
    """Page every record in one provenance slice, adding each to `periods`."""
    fq = f"resource.provenance:{prov}"
    total = int(fetch({"q": "*", "size": 0, "fq": fq})["total"])
    if total > CEILING:
        # A single provenance slice outgrew the pagination ceiling; it now
        # needs a finer sub-partition to stay complete.
        raise RuntimeError(
            f"provenance slice {prov!r} has {total} records, exceeding the "
            f"{CEILING} pagination ceiling -- add a finer sub-partition for it."
        )
    fetched = 0
    while fetched < total:
        page = fetch({"q": "*", "size": PAGE_SIZE, "from": fetched, "fq": fq}).get("results", [])
        if not page:
            break
        for r in page:
            res = r["resource"]
            periods[res["id"]] = res
        fetched += len(page)
        if sleep:
            time.sleep(sleep)
    print(f"  provenance {prov!r}: {total} matched, {fetched} paged", file=sys.stderr)


def main() -> int:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--out", default=".claude/tmp", help="output directory (default: .claude/tmp)")
    ap.add_argument("--sleep", type=float, default=0.0, help="seconds to sleep between page requests")
    args = ap.parse_args()

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    out_file = out_dir / "chronontology-periods.json"

    expected = corpus_total()
    print(f"ChronOntology reports {expected} period records", file=sys.stderr)

    provenances = sorted(discover_provenances())
    print(f"Discovered provenance partitions: {provenances}", file=sys.stderr)

    periods: dict = {}
    for prov in provenances:
        fetch_provenance(prov, args.sleep, periods)

    got = len(periods)
    print(f"Collected {got} unique periods (expected {expected})", file=sys.stderr)
    if got != expected:
        print(
            f"ERROR: coverage mismatch -- got {got}, expected {expected}. "
            "A provenance value likely hides beyond the first 10k of q=* "
            "(widen discovery) or a slice was truncated.",
            file=sys.stderr,
        )
        return 1

    out_file.write_text(json.dumps(periods, ensure_ascii=False, indent=2))
    print(f"Wrote {got} periods to {out_file}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
