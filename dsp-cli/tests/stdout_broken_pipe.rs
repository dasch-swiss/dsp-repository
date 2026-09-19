//! A broken pipe on `dsp`'s stdout (e.g. `dsp ... | head`) must exit 0 with
//! empty stderr, not surface as an `Internal` diagnostic.
//!
//! Uses `dsp vre sparql query`: its relay path (`src/actions/vre/sparql.rs`)
//! writes the store's response body to stdout in a single `write_all` call,
//! which makes it easy to produce a deterministic, comfortably-larger-than-
//! one-pipe-buffer body via a `wiremock` fixture — no live DSP-API server
//! needed. `dsp docs <topic>`'s embedded bodies max out around 8 KiB, too
//! small to guarantee dsp is still writing when the reader closes its end
//! (the whole output could land in the pipe buffer before the reader even
//! runs, making the test vacuous — see the module doc on `BrokenPipeWriter`
//! in `src/util/mod.rs`).
//!
//! The binary path is resolved once, via `CARGO_BIN_EXE_dsp` below.

use std::io::Read;
use std::process::{Command, Stdio};

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "test-token";

/// Comfortably larger than a pipe's kernel buffer (64 KiB on Linux, smaller
/// on macOS) — guarantees `dsp` is still mid-write when the reader below
/// closes its end after one byte, so the write actually hits `BrokenPipe`.
const LARGE_BODY_LEN: usize = 512 * 1024;

#[tokio::test]
async fn piping_into_a_closed_reader_exits_zero_with_empty_stderr() {
    let server = MockServer::start().await;

    let body = "x".repeat(LARGE_BODY_LEN);
    Mock::given(method("POST"))
        .and(path("/admin/sparql/query"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/sparql-results+json")
                .set_body_string(body),
        )
        .mount(&server)
        .await;

    let mut child = Command::new(env!("CARGO_BIN_EXE_dsp"))
        .args([
            "vre",
            "sparql",
            "query",
            "--server",
            &server.uri(),
            "--query",
            "SELECT * WHERE { ?s ?p ?o } LIMIT 1",
        ])
        .env("DSP_TOKEN", TOKEN)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn dsp");

    // Read a single byte, then drop our end of the stdout pipe while `dsp`
    // is still writing the (much larger) remaining body — this is what
    // forces the next write to fail with `ErrorKind::BrokenPipe`.
    let mut stdout = child.stdout.take().expect("child stdout must be piped");
    let mut first_byte = [0u8; 1];
    stdout
        .read_exact(&mut first_byte)
        .expect("expected at least one byte before the pipe closes");
    drop(stdout);

    let mut stderr_buf = Vec::new();
    child
        .stderr
        .take()
        .expect("child stderr must be piped")
        .read_to_end(&mut stderr_buf)
        .expect("failed to read child stderr");

    let status = child.wait().expect("failed to wait on dsp");

    assert!(
        status.success(),
        "expected exit 0 on a broken pipe, got {status:?}; stderr:\n{}",
        String::from_utf8_lossy(&stderr_buf)
    );
    assert!(
        stderr_buf.is_empty(),
        "expected empty stderr on a broken pipe; got:\n{}",
        String::from_utf8_lossy(&stderr_buf)
    );
}
