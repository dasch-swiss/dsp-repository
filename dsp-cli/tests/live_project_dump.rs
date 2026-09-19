// Live integration test for `dsp vre project dump`.
//
// This entire file is compiled and run ONLY when the `live` feature is active:
//   cargo test --features live --test 'live*'
//
// The `just test-live` recipe runs exactly this command.
//
// dsp-cli/ADR-0009 (testing strategy): live tests are **not** in CI. They require
// real environment variables pointing at a live DSP instance. Missing config
// causes an early-return skip — never a test failure.
//
// Required environment variables:
//   DSP_TEST_SERVER   — server URL or shortcut (e.g. "dev", "https://api.dev.dasch.swiss")
//   DSP_TEST_PROJECT  — shortcode, shortname, or IRI of a small disposable project
//
// Token supply (at least one required):
//   DSP_TOKEN         — system-admin bearer token (wins if present)
//   DSP_TEST_USER     — e-mail address  } used to obtain a token via login if
//   DSP_TEST_PASSWORD — password        } DSP_TOKEN is not set
//
// NEVER log the token value regardless of which path supplies it.
#![cfg(feature = "live")]

use std::io::{Read as _, Seek as _, SeekFrom};
use std::time::{Duration, Instant};

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::config::Config;
use dsp_cli::model::{CreateDumpOutcome, DumpStatus};
use tempfile::tempfile;

mod common;
use common::{optional_env, require_env};

// ---------------------------------------------------------------------------
// Live test
// ---------------------------------------------------------------------------

/// End-to-end live test: trigger → poll → download → assert ZIP magic bytes.
///
/// Skips cleanly (with an `eprintln!`) if any required environment variable
/// is absent. Never fails due to missing config — only due to real errors.
#[test]
#[ignore = "needs a DSP stack; run with just dsp-cli-test-live"]
fn live_project_dump_end_to_end() {
    // ── 1. Collect config ─────────────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };
    let project = match require_env("DSP_TEST_PROJECT") {
        Some(v) => v,
        None => return,
    };

    // Resolve server shortcut via Config::resolve (mirrors production path).
    let cfg = Config::resolve(Some(server_raw.trim())).expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

    // ── 2. Resolve token ──────────────────────────────────────────────────────
    // Prefer DSP_TOKEN; fall back to DSP_TEST_USER + DSP_TEST_PASSWORD via login.
    // Never log the token value.
    let token: String = if let Some(t) = optional_env("DSP_TOKEN") {
        // DSP_TOKEN is set — use it directly. Do not echo it.
        t
    } else {
        let user = match require_env("DSP_TEST_USER") {
            Some(v) => v,
            None => return,
        };
        let password = match require_env("DSP_TEST_PASSWORD") {
            Some(v) => v,
            None => return,
        };
        let client = HttpDspClient::new().expect("failed to build HTTP client for live login");
        let resp = client
            .login(&cfg.server, &user, &password)
            .expect("live login failed — check DSP_TEST_USER / DSP_TEST_PASSWORD");
        resp.token
    };

    // ── 3. Build client and resolve the project ───────────────────────────────
    let client = HttpDspClient::new().expect("failed to build HTTP client");

    let proj = client
        .resolve_project(&cfg.server, &project)
        .expect("resolve_project failed — check DSP_TEST_PROJECT");

    eprintln!(
        "live test: resolved project {} (shortcode {}, shortname {})",
        proj.iri, proj.shortcode, proj.shortname
    );

    // ── 4. Trigger dump (skip assets to keep it small) ────────────────────────
    // create_project_dump returns CreateDumpOutcome — handle both the fresh case
    // and the "already exists" case (idempotent: adopt the existing dump).
    let outcome = client
        .create_project_dump(&cfg.server, &proj.iri, true, &token)
        .expect("create_project_dump failed");

    let dump_id = match outcome {
        CreateDumpOutcome::Created(ref task) => {
            eprintln!("live test: triggered new dump {}, initial status {:?}", task.id, task.status);
            task.id.clone()
        }
        CreateDumpOutcome::Exists { ref id } => {
            eprintln!("live test: dump already exists ({id}); adopting it");
            id.clone()
        }
        CreateDumpOutcome::ExistsForOtherProject { ref id, ref project_iri } => {
            eprintln!(
                "live test: dump slot held by another project ({project_iri}, id {id}); \
skipping live test (manual --discard-other-project required)"
            );
            return;
        }
    };

    // ── 5. Poll to completion ─────────────────────────────────────────────────
    // Use a generous wall-clock timeout so a slow server doesn't hang the test
    // suite forever. Cap at ~5 minutes with exponential backoff (same constants
    // as the action layer: BASE=1s, CAP=30s).
    const POLL_TIMEOUT: Duration = Duration::from_secs(300);
    const BASE: Duration = Duration::from_secs(1);
    const CAP: Duration = Duration::from_secs(30);

    let start = Instant::now();
    let mut delay = BASE;

    loop {
        let t = client
            .get_project_dump_status(&cfg.server, &proj.iri, &dump_id, &token)
            .expect("get_project_dump_status failed");

        eprintln!("live test: poll status {:?} (elapsed {}s)", t.status, start.elapsed().as_secs());

        match t.status {
            DumpStatus::Completed => break,
            DumpStatus::Failed => {
                panic!("server-side dump failed: {}", t.error_message.unwrap_or_default());
            }
            DumpStatus::InProgress => {
                if start.elapsed() + delay >= POLL_TIMEOUT {
                    panic!(
                        "dump did not complete within {}s; the server-side dump may still be running",
                        POLL_TIMEOUT.as_secs()
                    );
                }
                std::thread::sleep(delay);
                delay = (delay * 2).min(CAP);
            }
        }
    }

    // ── 6. Download into a tempfile and assert ZIP magic bytes ────────────────
    eprintln!("live test: downloading dump {dump_id}");

    let mut tmp = tempfile().expect("failed to create tempfile for download");

    let bytes = client
        .download_project_dump(&cfg.server, &proj.iri, &dump_id, &token, &mut tmp)
        .expect("download_project_dump failed");

    eprintln!("live test: downloaded {bytes} bytes");

    assert!(bytes > 0, "downloaded file must not be empty");

    // Seek back to the beginning to read the magic bytes.
    tmp.seek(SeekFrom::Start(0)).expect("failed to seek tempfile to start");

    let mut magic = [0u8; 2];
    tmp.read_exact(&mut magic).expect("failed to read magic bytes from tempfile");

    assert_eq!(
        magic,
        [0x50, 0x4B], // "PK" — ZIP local file header magic
        "downloaded dump must start with ZIP magic bytes 'PK' (0x50 0x4B); got {:?}",
        magic
    );

    // ── 7. Idempotent-adopt extension (best-effort, skip-safe) ───────────────
    // A second create on the same project should return Exists { id } — the
    // core of Amendment 1's "adopt" behaviour. This assertion exercises the
    // real server path; it is best-effort: if the server state changed between
    // the download and this call, we log and skip rather than hard-fail.
    match client.create_project_dump(&cfg.server, &proj.iri, true, &token) {
        Ok(CreateDumpOutcome::Exists { id: ref existing_id }) => {
            eprintln!("live test: idempotent re-run correctly returned Exists ({existing_id})");
            assert_eq!(existing_id, &dump_id, "second create should return the same dump id");
        }
        Ok(CreateDumpOutcome::Created(ref t)) => {
            // Unexpected but not a hard failure: the server may have cleaned
            // up the dump between step 6 and now (or the server differs in
            // behaviour). Log and continue to cleanup.
            eprintln!(
                "live test: second create returned Created ({}), not Exists — \
                 server may have cleaned up; this is non-fatal",
                t.id
            );
        }
        Ok(CreateDumpOutcome::ExistsForOtherProject { ref id, ref project_iri }) => {
            // Another project's dump appeared between our download and this call.
            // Non-fatal for the live test — just log it.
            eprintln!(
                "live test: second create returned ExistsForOtherProject \
(id={id}, project_iri={project_iri}) — non-fatal"
            );
        }
        Err(e) => {
            eprintln!("live test: second create returned an error (non-fatal): {e}");
        }
    }

    // ── 8. Cleanup via delete — exercises the --delete path ──────────────────
    // Best-effort: if cleanup fails, log a warning but do not fail the test.
    match client.delete_project_dump(&cfg.server, &proj.iri, &dump_id, &token) {
        Ok(()) => eprintln!("live test: cleaned up server-side dump {dump_id}"),
        Err(e) => eprintln!("live test: cleanup warning (non-fatal): {e}"),
    }

    eprintln!("live test: PASSED");
}
