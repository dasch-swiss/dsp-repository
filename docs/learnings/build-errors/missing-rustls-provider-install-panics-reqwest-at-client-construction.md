---
title: "Without a default rustls provider, reqwest panics at client construction and pyroscope silently stops profiling"
date: 2026-11-18
category: build-errors
component: rust_crate
module: dsp-cli/src/client/tls.rs + areas/access/server/src/observability.rs
problem_type: test-setup
severity: high
symptoms:
  - "reqwest panics at client construction when no rustls crypto provider is installed"
  - "In the servers, pyroscope builds its client on its own session thread: the panic ends profiling while the server keeps running"
  - "pyroscope's stop() never returns once its session thread is dead"
root_cause: "With reqwest on rustls-no-provider, no default provider exists until the program calls install_default. A missing install fails at the first client construction, and where that happens on a library-owned thread nothing reports it."
tags: [rustls, ring, reqwest, pyroscope, crypto-provider, install_default, tests, silent-failure]
related:
  - dasch-specs/learnings/best-practices/sipi-nix-to-bazel-migration-lessons.md
issue: "DEV-7699"
---

# Without a default rustls provider, reqwest panics at client construction and pyroscope silently stops profiling

## Problem

The migration moved TLS to a single `ring` provider with `rustls-no-provider`. A binary that forgets `install_default` panics when it builds its first client. In dsp-cli that is loud. In the servers pyroscope builds its client on its own thread, so the panic ends profiling and the server stays up.

## Root Cause

No provider is the default. The failure surfaces at construction, on whichever thread constructs. A pyroscope session thread that has died also leaves `stop()` waiting forever.

## Solution

Each binary installs the provider at startup (`areas/access/server/src/observability.rs`; dsp-cli goes through `client::tls::client_builder`). The tests start the agent against a local listener, call `stop()` (which pushes one profile) and wait for `POST /push.v1.PusherService/Push`. The agent runs on a helper thread and both waits are bounded, since a dead session thread would hang `stop()`. With the install line removed the test fails at its 10 s timeout.

## Prevention

- One test binary per check: a provider installed by one test masks a missing install in another test in the same process. `dsp-cli/tests/tls_provider.rs` is its own binary.
- Verify such a test by removing the install line and watching it fail.
- Start new dsp-cli HTTP clients from `client::tls::client_builder`, not `reqwest::blocking::Client::builder()`.

## References

- https://github.com/dasch-swiss/dsp-repository/pull/591
- `dsp-cli/src/client/tls.rs`, `dsp-cli/tests/tls_provider.rs`
- `areas/access/server/src/observability.rs`
