#! /usr/bin/env bash

# Downloads and patches the JSON schema the DataCite JSON writer's output is
# shape-checked against.
#
# Run this script from within the schemas/ directory to refresh the local copy:
#
#   cd shared/fair/testdata/schemas/
#   bash download-schemas.sh
#
# Source:
#   DataCite kernel-4.3 JSON: https://github.com/datacite/schema
#                             (source/json/kernel-4.3/datacite_4.3_schema.json)
#                             Pinned to commit 2ade77951cf2, the same commit the
#                             OAI XSD copies use (see
#                             modules/dpe/api-oai/src/handlers/testdata/schemas/download-schemas.sh).
#
# Why 4.3 when the writer emits 4.6: DataCite publishes JSON schemas for kernel
# 4.2 and 4.3 only — the official kernel is XSD-only, and the JSON schemas are
# Invenio-derived. 4.6's additions over 4.3 are optional *properties* plus new
# *enum values*, so the 4.3 document still checks the shape. F-UJI's own parse
# in `just fair-check` is the acceptance for the 4.6 output.
#
# Patches applied:
#   resourceTypeGeneral: widened to the kernel-4.6 value set. The writer emits
#                        "Project", which 4.3 does not know, so every project
#                        would fail an unpatched schema.
#   dateType:            widened the same way, for "Coverage", which 4.6 added
#                        and the writer emits for every resolved
#                        temporalCoverage entry.
#   uniqueItems:         removed everywhere. The DataCite kernel XSD imposes no
#                        uniqueness on any repeatable property; the constraint
#                        is the Invenio-derived JSON schema's own addition. The
#                        writer serialises what the record holds, and the record
#                        holds what a curator entered — four committed projects
#                        repeat a keyword, and one person file repeats an ORCID
#                        and an affiliation. Dropping a duplicate here would
#                        make the JSON and the XML disagree about content, which
#                        is the one thing the two representations may not do.
#
# Both enum widenings read their values out of the XSDs already committed under
# modules/dpe/api-oai/src/handlers/testdata/schemas/include/, so the two copies
# of the kernel's vocabulary cannot drift.
#
# Not patched, and not needed: the schema's `format` assertions. They are
# annotations in draft-07, and the validator is built with format checking off
# (see `datacite_json_validator` in modules/dpe/api-oai/src/metadata/corpus.rs).

set -euo pipefail

DATACITE_COMMIT="2ade77951cf2"
SCHEMA_URL="https://raw.githubusercontent.com/datacite/schema/${DATACITE_COMMIT}/source/json/kernel-4.3/datacite_4.3_schema.json"
XSD_INCLUDE="../../../../modules/dpe/api-oai/src/handlers/testdata/schemas/include"

curl -Ls "${SCHEMA_URL}" -o datacite-4.3-schema.json

# Both enums are replaced wholesale with the committed 4.6 XSD's value set, in
# the XSD's own order, so a future kernel bump is one `download-schemas.sh` run
# in both directories rather than a hand-edited list here.
python3 - "$XSD_INCLUDE" <<'PATCH'
import json
import re
import sys

include = sys.argv[1]


def xsd_values(filename):
    with open(f"{include}/{filename}", encoding="utf-8") as handle:
        return re.findall(r'value="([^"]*)"', handle.read())


def drop_unique_items(node):
    if isinstance(node, dict):
        node.pop("uniqueItems", None)
        for value in node.values():
            drop_unique_items(value)
    elif isinstance(node, list):
        for value in node:
            drop_unique_items(value)


with open("datacite-4.3-schema.json", encoding="utf-8") as handle:
    schema = json.load(handle)

for definition, filename in (
    ("resourceTypeGeneral", "datacite-resourceType-v4.xsd"),
    ("dateType", "datacite-dateType-v4.xsd"),
):
    values = xsd_values(filename)
    if not values:
        raise SystemExit(f"no enum values found in {filename}")
    schema["definitions"][definition]["enum"] = values

drop_unique_items(schema)
if "uniqueItems" in json.dumps(schema):
    raise SystemExit("uniqueItems survived the patch")

with open("datacite-4.3-schema.json", "w", encoding="utf-8") as handle:
    json.dump(schema, handle, indent=2, ensure_ascii=False)
    handle.write("\n")
PATCH

echo "patched datacite-4.3-schema.json: kernel-4.6 resourceTypeGeneral and dateType value sets, uniqueItems removed"
