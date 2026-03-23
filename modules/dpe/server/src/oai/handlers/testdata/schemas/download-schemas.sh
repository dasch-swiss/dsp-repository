#! /usr/bin/env bash

# Downloads and patches all XML schemas used for OAI-PMH response validation.
#
# Run this script from within the schemas/ directory to refresh the local copies:
#
#   cd modules/leptos-dpe/server/src/oai/handlers/testdata/schemas/
#   bash download-schemas.sh
#
# Sources:
#   OAI-PMH 2.0:         https://www.openarchives.org/OAI/2.0/
#   Dublin Core:         https://www.dublincore.org/schemas/xmls/
#   DataCite OAI format: https://github.com/datacite/schema (source/oai/oai-1.1/oai.xsd, master)
#   DataCite kernel-4:   https://github.com/datacite/schema (source/meta/kernel-4/)
#                        Pinned to commit 2ade77951cf2 (DataCite v4.6, 2024-12-31).
#                        Upstream has since released v4.7. To upgrade, find the commit
#                        that introduces the desired version and update DATACITE_COMMIT.
#   W3C XML namespace:   http://www.w3.org/2001/03/xml.xsd (dated permalink, unchanged)
#   DataCite xml.xsd:    same GitHub commit as kernel-4
#
# Patches applied:
#   oai_dc.xsd:           remote schemaLocation -> local simpledc20021212.xsd
#   simpledc20021212.xsd: remote schemaLocation -> local xml.xsd
#   (All other files are used as-is; validate.xsd is handcrafted and not downloaded.)

set -euo pipefail

OAI_BASE="https://www.openarchives.org/OAI/2.0"
DC_BASE="https://www.dublincore.org/schemas/xmls"
DATACITE_COMMIT="2ade77951cf2"
DATACITE_BASE="https://raw.githubusercontent.com/datacite/schema/${DATACITE_COMMIT}/source/meta/kernel-4"
DATACITE_OAI="https://raw.githubusercontent.com/datacite/schema/master/source/oai/oai-1.1/oai.xsd"
W3C_XML="http://www.w3.org/2001/03/xml.xsd"

curl -Ls "${OAI_BASE}/OAI-PMH.xsd" -o OAI-PMH.xsd

# Patch: use local simpledc file instead of remote schemaLocation
curl -Ls "${OAI_BASE}/oai_dc.xsd" | \
  sed 's|schemaLocation="http://dublincore.org/schemas/xmls/simpledc20021212.xsd"|schemaLocation="simpledc20021212.xsd"|' > \
    oai_dc.xsd

# Patch: use local xml.xsd instead of remote schemaLocation
curl -Ls "${DC_BASE}/simpledc20021212.xsd" | \
  sed 's|schemaLocation="http://www.w3.org/2001/03/xml.xsd"|schemaLocation="xml.xsd"|' > \
    simpledc20021212.xsd

curl -Ls "${DATACITE_OAI}" -o oai_datacite.xsd
curl -Ls "${W3C_XML}" -o xml.xsd
curl -Ls "${DATACITE_BASE}/metadata.xsd" -o datacite-kernel-4.xsd

mkdir -p include
curl -Ls "${DATACITE_BASE}/include/datacite-contributorType-v4.xsd"       -o include/datacite-contributorType-v4.xsd
curl -Ls "${DATACITE_BASE}/include/datacite-dateType-v4.xsd"              -o include/datacite-dateType-v4.xsd
curl -Ls "${DATACITE_BASE}/include/datacite-descriptionType-v4.xsd"       -o include/datacite-descriptionType-v4.xsd
curl -Ls "${DATACITE_BASE}/include/datacite-funderIdentifierType-v4.xsd"  -o include/datacite-funderIdentifierType-v4.xsd
curl -Ls "${DATACITE_BASE}/include/datacite-nameType-v4.xsd"              -o include/datacite-nameType-v4.xsd
curl -Ls "${DATACITE_BASE}/include/datacite-numberType-v4.xsd"            -o include/datacite-numberType-v4.xsd
curl -Ls "${DATACITE_BASE}/include/datacite-relatedIdentifierType-v4.xsd" -o include/datacite-relatedIdentifierType-v4.xsd
curl -Ls "${DATACITE_BASE}/include/datacite-relationType-v4.xsd"          -o include/datacite-relationType-v4.xsd
curl -Ls "${DATACITE_BASE}/include/datacite-resourceType-v4.xsd"          -o include/datacite-resourceType-v4.xsd
curl -Ls "${DATACITE_BASE}/include/datacite-titleType-v4.xsd"             -o include/datacite-titleType-v4.xsd
curl -Ls "${DATACITE_BASE}/include/xml.xsd"                               -o include/xml.xsd
