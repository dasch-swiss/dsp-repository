//! Layer-neutral utility helpers.
//!
//! This module is the home for dependency-free helpers that may be imported
//! from any layer (`client`, `render`, …) without creating a cross-layer
//! dependency (dsp-cli/ADR-0008). Unlike `src/client/` or `src/render/`, `src/util/`
//! imports from no other `dsp-cli` layer; all layers may import from it.

use std::io::{self, Write};

pub(crate) mod text;

/// Wraps a stdout writer so a broken pipe (e.g. `dsp ... | head`) exits the
/// process cleanly with status 0 and no stderr output, instead of surfacing
/// through the normal write-error path as `Diagnostic::Internal`.
///
/// Every other write error is passed through unchanged, so it continues to
/// flow through the existing `?` / blanket `From<io::Error> for Diagnostic`
/// path (`src/diagnostic.rs`, dsp-cli/ADR-0012) exactly as before — this
/// wrapper only special-cases `ErrorKind::BrokenPipe`. `Diagnostic::from(io::Error)`
/// itself is deliberately left alone (it is also reached from non-stdout
/// paths, e.g. reading `auth.toml`); this wrapper intercepts the error
/// before it ever reaches that conversion. Used at every stdout write site:
/// the five renderers' sinks, and the ad hoc writers in `docs`, `vre sparql
/// query`, and `auth token`.
pub(crate) struct BrokenPipeWriter<W>(W);

impl<W: Write> BrokenPipeWriter<W> {
    pub(crate) fn new(inner: W) -> Self {
        Self(inner)
    }
}

/// Exits the process immediately (status 0, no stderr) on `BrokenPipe`;
/// otherwise returns the result unchanged.
fn exit_on_broken_pipe<T>(result: io::Result<T>) -> io::Result<T> {
    if let Err(ref e) = result
        && e.kind() == io::ErrorKind::BrokenPipe
    {
        std::process::exit(0);
    }
    result
}

impl<W: Write> Write for BrokenPipeWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        exit_on_broken_pipe(self.0.write(buf))
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        exit_on_broken_pipe(self.0.write_all(buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        exit_on_broken_pipe(self.0.flush())
    }
}

/// The `User-Agent` header value sent on every outgoing HTTP request
/// (DSP-API calls and the crates.io update check alike), so server operators
/// can identify dsp-cli traffic. Plain form `dsp-cli/<version>`; the version is
/// baked in at compile time from Cargo. See plan 033.
pub(crate) const USER_AGENT: &str = concat!("dsp-cli/", env!("CARGO_PKG_VERSION"));

/// The `DSP-Client` header value sent on every outgoing DSP-API request
/// (never the crates.io update check, which only carries `USER_AGENT`), so
/// server operators can identify dsp-cli traffic by a header name that is
/// dsp-cli-specific rather than shared with every other HTTP client's
/// generic `User-Agent`. Same plain form as `USER_AGENT`.
pub(crate) const DSP_CLIENT_HEADER: &str = concat!("dsp-cli/", env!("CARGO_PKG_VERSION"));

/// Logs an auth-cache load failure: the full error at `debug`, a body-free
/// message at `warn`.
///
/// A `toml` parse error quotes the offending source line, which can contain
/// bearer-token bytes; keep it out of the default-verbosity (`warn`) stream
/// and log it only at `debug`. `fallback` is the per-site tail describing
/// what happens next (e.g. "falling back to anonymous for project list")
/// and must itself be a fixed, token-free string.
pub(crate) fn warn_auth_cache_load_failed(e: &impl std::fmt::Display, fallback: &str) {
    tracing::debug!(error = %e, "auth cache load failed");
    tracing::warn!("auth cache load failed; {fallback}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_has_expected_prefix() {
        assert!(USER_AGENT.starts_with("dsp-cli/"));
    }

    #[test]
    fn user_agent_is_plain_form_no_suffix() {
        assert_eq!(USER_AGENT, concat!("dsp-cli/", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn dsp_client_header_has_expected_prefix() {
        assert!(DSP_CLIENT_HEADER.starts_with("dsp-cli/"));
    }

    #[test]
    fn broken_pipe_writer_passes_through_normal_writes() {
        let mut w = BrokenPipeWriter::new(Vec::<u8>::new());
        w.write_all(b"hello").unwrap();
        w.flush().unwrap();
        assert_eq!(w.0, b"hello");
    }

    #[test]
    fn broken_pipe_writer_passes_through_non_broken_pipe_errors_unchanged() {
        struct FailingWriter;
        impl Write for FailingWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::NotFound, "boom"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut w = BrokenPipeWriter::new(FailingWriter);
        let err = w.write(b"x").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
