#!/usr/bin/env bash
# Bazel's workspace status command (`.bazelrc`). Only stamped targets read its output, under
# `--stamp`, which the image builds pass. Only `STABLE_` keys: a volatile key changes on every build.
set -euo pipefail

# The editor's version for `deposit-shell`'s footer. The editor workflows compute it (`git
# describe` on `main`, the tag on a release, `pr-<N>-<sha7>` on a preview); unset reads `dev`.
echo "STABLE_DEPOSIT_VERSION ${DEPOSIT_VERSION:-dev}"
