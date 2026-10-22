//! CPE's two ports onto the Access Area: the archive projection and the bytes port (ADR-0003,
//! ADR-0007, ADR-0008, ADR-0012).
//!
//! [`ArchiveProjection`] is what CPE reads to rebuild its store: one project's facts in the
//! archive's shape, never CPE's presentation model. `sync` implements the port beside its data;
//! CPE's tests use [`FakeArchiveProjection`], and every adapter's output must pass
//! [`contract::violations`].
//!
//! [`DepositedFiles`], the bytes port, is what CPE reads from `media`: the files a project
//! deposited, by shortcode and name, whole. It lists nothing, so CPE learns which names to ask for
//! from the project's configuration. `media` implements it; CPE's tests use
//! [`FakeDepositedFiles`]. Every adapter applies [`is_shortcode`] and [`is_file_name`] before
//! touching storage.
//!
//! The crate depends on `std` alone: a consumer's `ports` crate may reach only `std` and `shared-*`
//! (ADR-0003), and nothing in `shared-*` is needed here. No type is `#[non_exhaustive]`, so CPE's
//! fixtures can use struct literals and a new [`ValueKind`] fails CPE's compile instead of hiding
//! behind a `_ =>` arm.

use std::error::Error;
use std::fmt;

pub mod contract;
mod fake;
mod names;
mod snapshot;

pub use fake::{FakeArchiveProjection, FakeDepositedFiles};
pub use names::{is_file_name, is_shortcode};
pub use snapshot::{
    Annotation, Calendar, ClassIri, DataArk, DateBound, DatePrecision, DateValue, File, LangString, ListNode,
    ListNodeIri, Motivation, ProjectSnapshot, PropertyIri, Resource, ResourceIri, Value, ValueKind, DATA_ARK_PREFIX,
};

/// What CPE reads from the Access Area's archive projection: one project's facts as the archive
/// records them.
///
/// Synchronous and dyn-compatible, so the composition root can hold an `Arc<dyn
/// ArchiveProjection>`; CPE calls it inside `spawn_blocking`.
pub trait ArchiveProjection: Send + Sync {
    /// The project's current facts, whole. Every call is a full snapshot; the port
    /// announces no change. A read that fails midway is [`ProjectionError::Unavailable`], never a
    /// partial snapshot.
    fn snapshot(&self, shortcode: &str) -> Result<ProjectSnapshot, ProjectionError>;
}

/// Why [`ArchiveProjection::snapshot`] returned no snapshot.
#[derive(Debug)]
pub enum ProjectionError {
    /// The projection holds no project under this shortcode.
    UnknownProject { shortcode: String },
    /// The projection could not be read; the source is the adapter's own error.
    Unavailable(Box<dyn Error + Send + Sync>),
}

impl ProjectionError {
    /// Wrap an adapter's error.
    #[must_use]
    pub fn unavailable(source: impl Error + Send + Sync + 'static) -> Self {
        Self::Unavailable(Box::new(source))
    }
}

/// `Unavailable` includes its source's message, because call sites log with `%error`.
impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProject { shortcode } => {
                write!(f, "the archive projection holds no project {shortcode}")
            }
            Self::Unavailable(source) => write!(f, "the archive projection is unavailable: {source}"),
        }
    }
}

impl Error for ProjectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::UnknownProject { .. } => None,
            Self::Unavailable(source) => Some(source.as_ref()),
        }
    }
}

/// What CPE reads from the Access Area's `media`: the files a project deposited, by name, whole.
///
/// It gives no file a meaning and refuses no file for want of a right: in the target each file
/// comes with its rights attached and CPE decides at request time (ADR-0012, ADR-0013). Not yet in
/// code: the rights, and the change the port announces (not scheduled); today it announces none.
/// Synchronous and dyn-compatible like [`ArchiveProjection`]; CPE calls it inside
/// `spawn_blocking`.
pub trait DepositedFiles: Send + Sync {
    /// The file's bytes, whole. A read that fails midway is [`FilesError::Unavailable`], never a
    /// prefix. A `shortcode` that fails [`is_shortcode`] is [`FilesError::UnknownProject`], and
    /// otherwise a `name` that fails [`is_file_name`] is [`FilesError::UnknownFile`].
    fn file(&self, shortcode: &str, name: &str) -> Result<Vec<u8>, FilesError>;
}

/// Why [`DepositedFiles::file`] returned no bytes.
#[derive(Debug)]
pub enum FilesError {
    /// `media` holds no project under this shortcode, or the shortcode fails [`is_shortcode`].
    UnknownProject { shortcode: String },
    /// The project holds no file under this name, or the name fails [`is_file_name`].
    UnknownFile { shortcode: String, name: String },
    /// The file could not be read; the source is the adapter's own error. It may carry an
    /// absolute path, so it is logged, never rendered.
    Unavailable(Box<dyn Error + Send + Sync>),
}

impl FilesError {
    /// Wrap an adapter's error.
    #[must_use]
    pub fn unavailable(source: impl Error + Send + Sync + 'static) -> Self {
        Self::Unavailable(Box::new(source))
    }
}

/// `Unavailable` includes its source's message, because call sites log with `%error`. The
/// shortcode and name are quoted and escaped, since they may be strings the port refused.
impl fmt::Display for FilesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProject { shortcode } => write!(f, "no deposited files for project {shortcode:?}"),
            Self::UnknownFile { shortcode, name } => write!(f, "project {shortcode:?} deposited no file {name:?}"),
            Self::Unavailable(source) => write!(f, "the deposited files are unavailable: {source}"),
        }
    }
}

impl Error for FilesError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::UnknownProject { .. } | Self::UnknownFile { .. } => None,
            Self::Unavailable(source) => Some(source.as_ref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[test]
    fn test_display_unknown_project_names_shortcode() {
        let error = ProjectionError::UnknownProject { shortcode: "0803".to_string() };

        assert!(error.to_string().contains("0803"), "{error}");
    }

    #[test]
    fn test_display_unavailable_includes_source_message() {
        let error = ProjectionError::unavailable(io::Error::other("store file truncated"));

        assert!(error.to_string().contains("store file truncated"), "{error}");
    }

    #[test]
    fn test_files_error_display_unknown_project_names_shortcode() {
        let error = FilesError::UnknownProject { shortcode: "0803".to_string() };

        assert!(error.to_string().contains("0803"), "{error}");
    }

    #[test]
    fn test_files_error_display_unknown_file_names_shortcode_and_file() {
        let error = FilesError::UnknownFile {
            shortcode: "0803".to_string(),
            name: "narrative/home.de.md".to_string(),
        };

        let message = error.to_string();
        assert!(message.contains("0803"), "{message}");
        assert!(message.contains("narrative/home.de.md"), "{message}");
    }

    #[test]
    fn test_files_error_display_refused_name_escapes_line_breaks() {
        let error = FilesError::UnknownFile {
            shortcode: "0803".to_string(),
            name: "a\nforged line".to_string(),
        };

        assert!(!error.to_string().contains('\n'), "{error}");
    }

    #[test]
    fn test_files_error_display_unavailable_includes_source_message() {
        let error = FilesError::unavailable(io::Error::other("permission denied"));

        assert!(error.to_string().contains("permission denied"), "{error}");
        assert!(error.source().is_some());
    }
}
