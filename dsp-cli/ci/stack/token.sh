#!/usr/bin/env bash
# Logs in to the dsp-api instance started by this stack and prints two
# `export` lines for the resulting JWTs. Also appends NAME=value lines to
# $GITHUB_ENV when it is set, so a CI step can `source` the exports and a
# later step can pick up the env vars directly.
#
# Admin user: root@example.com / test (system admin fixture user).
# Non-admin user: anything.user01@example.org / test — a member of project
# 0001 (anything) in test_data/project_data/admin-data.ttl who does not
# carry isInSystemAdminGroup true and is not a project admin either, so it
# exercises an ordinary authenticated-but-unprivileged path.
set -euo pipefail

API_HOST="http://localhost:3333"

login() {
  local email="$1"
  local password="$2"
  local response
  response="$(curl -sf -X POST "${API_HOST}/v2/authentication" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"${email}\",\"password\":\"${password}\"}")"

  local token
  token="$(printf '%s' "${response}" | sed -n 's/.*"token"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"

  if [[ -z "${token}" ]]; then
    echo "token.sh: login failed for ${email} (no token in response)" >&2
    exit 1
  fi
  printf '%s' "${token}"
}

DSP_TOKEN="$(login "root@example.com" "test")"
DSP_TEST_NON_ADMIN_TOKEN="$(login "anything.user01@example.org" "test")"

echo "export DSP_TOKEN=${DSP_TOKEN}"
echo "export DSP_TEST_NON_ADMIN_TOKEN=${DSP_TEST_NON_ADMIN_TOKEN}"

if [[ -n "${GITHUB_ENV:-}" ]]; then
  {
    echo "DSP_TOKEN=${DSP_TOKEN}"
    echo "DSP_TEST_NON_ADMIN_TOKEN=${DSP_TEST_NON_ADMIN_TOKEN}"
  } >>"${GITHUB_ENV}"
fi
