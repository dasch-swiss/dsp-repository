#!/usr/bin/env bash
# Polls dsp-api's /health endpoint until it reports ready or a timeout is
# reached. Compose's `--wait` already blocks on the container healthcheck;
# this script exists as an explicit, debuggable wait for CI logs.
set -euo pipefail

API_HOST="http://localhost:3333"
TIMEOUT_SECONDS=300

echo "==> Waiting up to ${TIMEOUT_SECONDS}s for ${API_HOST}/health"

elapsed=0
while [[ "${elapsed}" -lt "${TIMEOUT_SECONDS}" ]]; do
  if curl -sf -o /dev/null "${API_HOST}/health"; then
    echo "==> dsp-api is healthy after ${elapsed}s"
    exit 0
  fi
  sleep 5
  elapsed=$((elapsed + 5))
done

echo "wait-for-api.sh: dsp-api did not become healthy at ${API_HOST}/health within ${TIMEOUT_SECONDS}s" >&2
exit 1
