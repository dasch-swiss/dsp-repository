#!/usr/bin/env bash
# Loads the dsp-api test fixtures (ontologies + project data) into the
# Fuseki instance started by this stack's docker-compose.yml.
#
# Takes no arguments. Reads the API tag from stack.env next to this script,
# sparse-checks-out that tag of dasch-swiss/dsp-api into a temp directory,
# and runs its own fuseki-init-knora-test.sh against Fuseki on
# localhost:3030.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/stack.env"

# The fixtures come from the dsp-api repository, so the checkout is keyed on API,
# not DB. The two pins are independent: the Fuseki image is versioned on its own
# line and its tag is not a dsp-api ref.
if [[ -z "${API:-}" ]]; then
  echo "load-fixtures.sh: API tag not set in ${SCRIPT_DIR}/stack.env" >&2
  exit 1
fi

# Note: this re-sources stack.env, so a job that overrides API in its environment
# (the nightly `latest` run) still loads the pinned tag's fixtures. That is
# deliberate — `latest` has no corresponding dsp-api ref to check out.
TAG="${API}"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "${WORKDIR}"' EXIT

echo "==> Sparse-checking out dasch-swiss/dsp-api@${TAG} into ${WORKDIR}"
git clone --quiet --no-checkout --depth 1 --branch "${TAG}" \
  https://github.com/dasch-swiss/dsp-api.git "${WORKDIR}/dsp-api"
git -C "${WORKDIR}/dsp-api" sparse-checkout init --cone
git -C "${WORKDIR}/dsp-api" sparse-checkout set \
  test_data \
  modules/webapi/scripts \
  modules/webapi/src/main/resources/knora-ontologies
git -C "${WORKDIR}/dsp-api" checkout --quiet "${TAG}"

# Fuseki's container health can report ready before the dataset is actually
# reachable, so this explicit poll is the second guard.
echo "==> Waiting for Fuseki at localhost:3030 to answer"
FUSEKI_READY=0
for _ in $(seq 1 60); do
  if curl -sf -o /dev/null "http://localhost:3030/$/ping"; then
    FUSEKI_READY=1
    break
  fi
  sleep 1
done
if [[ "${FUSEKI_READY}" -ne 1 ]]; then
  echo "load-fixtures.sh: Fuseki did not answer at localhost:3030 within 60s" >&2
  exit 1
fi
echo "==> Fuseki is up"

SCRIPTS_DIR="${WORKDIR}/dsp-api/modules/webapi/scripts"

echo "==> Loading fixtures into repository dsp-repo"
OUTPUT="$(cd "${SCRIPTS_DIR}" && bash ./fuseki-init-knora-test.sh 2>&1)" || {
  echo "${OUTPUT}"
  echo "load-fixtures.sh: fuseki-init-knora-test.sh exited non-zero" >&2
  exit 1
}
echo "${OUTPUT}"

if echo "${OUTPUT}" | grep -q "failed with status code"; then
  echo "load-fixtures.sh: one or more graph uploads returned a non-2xx status" >&2
  exit 1
fi

echo "==> Fixtures loaded"
