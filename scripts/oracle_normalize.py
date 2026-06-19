#!/usr/bin/env python3
"""Normalize HTML on stdin for a semantic (structure/attribute) diff (DEV-6642).

The pre-migration HTML "oracle" lets us compare what the old (Leptos) server produced
against the new (Maud) server. A raw `curl old | diff curl new` is noisy: Leptos emits
`data-hk` hydration markers, hot-reload comments, and arbitrary whitespace/attribute
order. This re-serializes HTML into a canonical form so the diff shows only *meaningful*
structural differences:

  - drops `data-hk` attributes and ALL comments (hot-reload markers etc.),
  - sorts attribute names (order is not semantic),
  - collapses insignificant whitespace, one node per line.

This is a structural/semantic reference, NOT a byte-equality gate (same spirit as the
insta snapshots). Class-attribute value order is preserved (Tailwind order can matter).

Usage:
    curl -s http://localhost:4100/dpe/projects/0803 | python3 scripts/oracle_normalize.py
"""

import sys
from html.parser import HTMLParser

# Attributes that are pure hydration/runtime noise and never structurally meaningful.
DROP_ATTRS = {"data-hk"}
# Elements whose text content is whitespace-significant — don't collapse inside them.
PRE_ELEMENTS = {"pre", "textarea", "script", "style"}
# Void elements (no closing tag) per the HTML spec.
VOID_ELEMENTS = {
    "area", "base", "br", "col", "embed", "hr", "img", "input",
    "link", "meta", "param", "source", "track", "wbr",
}


def _esc(value: str) -> str:
    return value.replace("&", "&amp;").replace('"', "&quot;")


class Normalizer(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.out: list[str] = []
        self.pre_depth = 0

    def _emit_start(self, tag: str, attrs: list, self_closing: bool) -> None:
        kept = sorted(
            (name, val) for name, val in attrs if name not in DROP_ATTRS
        )
        parts = [tag]
        for name, val in kept:
            parts.append(name if val is None else f'{name}="{_esc(val)}"')
        slash = "/" if self_closing else ""
        self.out.append(f"<{' '.join(parts)}{slash}>")
        if tag in PRE_ELEMENTS:
            self.pre_depth += 1

    def handle_starttag(self, tag, attrs):
        self._emit_start(tag, attrs, tag in VOID_ELEMENTS)

    def handle_startendtag(self, tag, attrs):
        self._emit_start(tag, attrs, True)

    def handle_endtag(self, tag):
        if tag in PRE_ELEMENTS and self.pre_depth > 0:
            self.pre_depth -= 1
        if tag not in VOID_ELEMENTS:
            self.out.append(f"</{tag}>")

    def handle_data(self, data):
        if self.pre_depth > 0:
            if data:
                self.out.append(data)
            return
        text = " ".join(data.split())
        if text:
            self.out.append(text)

    def handle_comment(self, data):
        # Drop all comments (hot-reload markers, template boundaries, etc.).
        pass


def main() -> int:
    parser = Normalizer()
    parser.feed(sys.stdin.read())
    parser.close()
    sys.stdout.write("\n".join(parser.out) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
