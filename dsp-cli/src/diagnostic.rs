//! Diagnostics — errors, exit codes, and logging setup. See dsp-cli/ADR-0012.
//!
//! `Diagnostic` is the library-level error type. Each variant carries a stable
//! `kind` mapped via `exit_category()` to one of the four `ExitCategory`
//! values that the binary turns into a process exit code.

use thiserror::Error;
use tracing_subscriber::EnvFilter;

/// Process exit categories. See the table in dsp-cli/ADR-0012.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCategory {
    /// Command ran successfully.
    Success = 0,

    /// Runtime error: HTTP failure, deserialisation, I/O, server 5xx, …
    Runtime = 1,

    /// Usage error: bad flag, missing argument, unknown identifier.
    Usage = 2,

    /// Authentication required: endpoint demands auth or token rejected.
    AuthRequired = 3,
}

/// Library error type. Stable enum variants per dsp-cli/ADR-0012.
///
/// `Clone` is derived so that mock test helpers can hand out
/// `Result<_, Diagnostic>` values repeatedly without consuming them.
/// All current variants hold `String`, which is `Clone`.
#[derive(Debug, Clone, Error)]
pub enum Diagnostic {
    /// Bad input from the caller — bad flag, missing argument, unparseable id.
    #[error("usage error: {0}")]
    Usage(String),

    /// The server demanded auth and none was supplied (or the token was rejected).
    #[error("authentication required: {0}")]
    AuthRequired(String),

    /// The named project / data-model / resource-type does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// The server responded with a 5xx or an unparseable response.
    ///
    /// Also carries a **relayed store rejection** from `dsp vre sparql query`:
    /// the triplestore's own `4xx` is not dsp-cli's failure to classify, so it
    /// is reported here with the store's status and its (sanitised, capped)
    /// message. Exit category `Runtime` (1) — see dsp-cli/ADR-0016 / plan 035 D7.
    #[error("server error: {0}")]
    ServerError(String),

    /// Could not reach the server.
    #[error("network error: {0}")]
    Network(String),

    /// A server-side resource is busy or already exists.
    ///
    /// Examples: a dump for this project is already in progress or present;
    /// a `DELETE` was attempted while the dump was still being produced.
    ///
    /// Maps to `ExitCategory::Runtime` (exit code 1).
    #[error("conflict: {0}")]
    Conflict(String),

    /// A local filesystem operation that the user explicitly requested failed.
    ///
    /// Example: writing or renaming the dump file the user asked for.
    ///
    /// **Distinction from `Internal`**: `Internal` wraps unexpected internal
    /// I/O (renderer writes, plumbing) and is reached via the blanket
    /// `From<std::io::Error>` impl. `Io` is for user-visible filesystem work
    /// (e.g. writing the dump output file) and must be constructed *explicitly*
    /// with a message that includes the target path.
    ///
    /// **Warning**: the dump file-write path must never use bare `?` on a
    /// `std::io::Error`. Bare `?` will hit `From<io::Error>` and mis-classify
    /// the error as `Internal` instead of `Io`. Always map explicitly:
    /// `.map_err(|e| Diagnostic::Io(format!("…{path}…: {e}")))`.
    /// See the scoped helper `stream_dump_to_path` in the action (Step 8).
    ///
    /// Maps to `ExitCategory::Runtime` (exit code 1).
    #[error("io error: {0}")]
    Io(String),

    /// Unexpected state in dsp-cli itself; should be reported as a bug.
    #[error("internal error: {0}")]
    Internal(String),

    /// Placeholder for work-in-progress dispatch paths during Phase 1.
    #[error("not implemented: {0}")]
    NotImplemented(String),
}

/// Bridge from `std::io::Error` so that `?` works inside renderer methods
/// (and any other function returning `Result<_, Diagnostic>`) without
/// spelling out `.map_err(|e| Diagnostic::Internal(...))` at every call site.
impl From<std::io::Error> for Diagnostic {
    fn from(e: std::io::Error) -> Self {
        Self::Internal(format!("io error: {e}"))
    }
}

impl Diagnostic {
    pub fn not_implemented(msg: impl Into<String>) -> Self {
        Self::NotImplemented(msg.into())
    }

    /// Maps a diagnostic to its exit category (per dsp-cli/ADR-0012).
    pub fn exit_category(&self) -> ExitCategory {
        match self {
            Self::Usage(_) => ExitCategory::Usage,
            Self::AuthRequired(_) => ExitCategory::AuthRequired,
            Self::NotFound(_)
            | Self::ServerError(_)
            | Self::Network(_)
            | Self::Conflict(_)
            | Self::Io(_)
            | Self::Internal(_)
            | Self::NotImplemented(_) => ExitCategory::Runtime,
        }
    }
}

/// Initialise the global tracing subscriber.
///
/// Per dsp-cli/ADR-0012: default level WARN; `-v` raises to INFO, `-vv` to DEBUG,
/// `-vvv` to TRACE; `RUST_LOG` overrides if set. Output goes to stderr.
pub fn init_tracing(verbose: u8) {
    let filter = if let Ok(env) = std::env::var("RUST_LOG") {
        EnvFilter::new(env)
    } else {
        let level = match verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        };
        EnvFilter::new(level)
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .compact()
        .try_init()
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_maps_to_exit_code_2() {
        let d = Diagnostic::Usage("missing --server".into());
        assert_eq!(d.exit_category(), ExitCategory::Usage);
    }

    #[test]
    fn auth_required_maps_to_exit_code_3() {
        let d = Diagnostic::AuthRequired("login first".into());
        assert_eq!(d.exit_category(), ExitCategory::AuthRequired);
    }

    #[test]
    fn runtime_errors_map_to_exit_code_1() {
        // This list must cover every Diagnostic variant that maps to Runtime so
        // that adding a new Runtime variant without updating this test fails CI.
        for d in [
            Diagnostic::NotFound("x".into()),
            Diagnostic::ServerError("x".into()),
            Diagnostic::Network("x".into()),
            Diagnostic::Internal("x".into()),
            Diagnostic::Conflict("x".into()),
            Diagnostic::Io("x".into()),
            Diagnostic::NotImplemented("x".into()),
        ] {
            assert_eq!(d.exit_category(), ExitCategory::Runtime, "{d:?} must map to Runtime");
        }
    }

    #[test]
    fn not_implemented_maps_to_exit_code_1() {
        let d = Diagnostic::not_implemented("vre project list");
        assert_eq!(d.exit_category(), ExitCategory::Runtime);
    }

    #[test]
    fn diagnostic_is_clone() {
        // `Clone` is derived so mock test helpers can hand out Result<_, Diagnostic>
        // values repeatedly. If any variant loses Clone this test catches it.
        let original = Diagnostic::AuthRequired("test".into());
        let cloned = original.clone();
        assert_eq!(format!("{original}"), format!("{cloned}"));
    }

    #[test]
    fn conflict_maps_to_runtime() {
        let d = Diagnostic::Conflict("dump already in progress".into());
        assert_eq!(d.exit_category(), ExitCategory::Runtime);
    }

    #[test]
    fn io_maps_to_runtime() {
        let d = Diagnostic::Io("failed to write /tmp/0001.zip: permission denied".into());
        assert_eq!(d.exit_category(), ExitCategory::Runtime);
    }

    #[test]
    fn from_io_error_yields_internal() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let diag = Diagnostic::from(io_err);
        assert!(matches!(diag, Diagnostic::Internal(_)));
        assert!(diag.to_string().contains("io error"));
    }
}
