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
//! touching storage, answering a refused shortcode with [`FilesError::UnknownProject`] and a
//! refused name with [`FilesError::UnknownFile`]. The two rules come from `shared-names` and are
//! re-exported here.
//!
//! Both ports serve a project's [`Provenance`], and [`ArchiveProjection::subscribe`] tells a
//! [`ChangeListener`] of a project whose data changed. Each has a default, so an adapter that
//! records no provenance or announces nothing need not implement it.
//!
//! The crate depends on `std` and `shared-names` alone: a consumer's `ports` crate may reach only
//! `std` and `shared-*` (ADR-0003). No type is `#[non_exhaustive]`, so CPE's fixtures can use
//! struct literals and a new [`ValueKind`] fails CPE's compile instead of hiding behind a `_ =>`
//! arm.

use std::error::Error;
use std::fmt;
use std::sync::Arc;

pub mod contract;
mod fake;
mod snapshot;

pub use fake::{FakeArchiveProjection, FakeDepositedFiles};
pub use shared_names::{is_file_name, is_shortcode};
pub use snapshot::{
    Annotation, Calendar, ClassIri, DataArk, DateBound, DatePrecision, DateValue, File, FileLegal, LangString,
    ListNode, ListNodeIri, Motivation, ProjectSnapshot, PropertyIri, Resource, ResourceIri, Value, ValueKind,
    DATA_ARK_PREFIX,
};

/// What CPE reads from the Access Area's archive projection: one project's facts as the archive
/// records them.
///
/// Synchronous and dyn-compatible, so the composition root can hold an `Arc<dyn
/// ArchiveProjection>`; CPE calls it inside `spawn_blocking`.
pub trait ArchiveProjection: Send + Sync {
    /// The project's current facts, whole. Every call is a full snapshot. A read that fails midway
    /// is [`ProjectionError::Unavailable`], never a partial snapshot.
    fn snapshot(&self, shortcode: &str) -> Result<ProjectSnapshot, ProjectionError>;

    /// What the project's current facts were made from. `Ok(None)`: the adapter records no
    /// provenance for it (fakes, hand-fed data).
    fn provenance(&self, _shortcode: &str) -> Result<Option<Provenance>, ProjectionError> {
        Ok(None)
    }

    /// Calls `listener` for every project whose data appeared, changed or went, from now on: a
    /// change made before `subscribe` returns is not announced. The default announces nothing.
    fn subscribe(&self, _listener: Arc<dyn ChangeListener>) {}

    /// Whether the location the projection keeps `shortcode` in is there to read, as a whole: a
    /// mount that lost its target is not. It tells a removed project from a lost location, which
    /// [`ProjectionError::UnknownProject`] alone does not. The default, `false`, never takes a lost
    /// location for a removal, so an adapter that does not override it can announce no removal a
    /// reader accepts.
    fn location_present(&self, _shortcode: &str) -> bool {
        false
    }
}

/// What a project's current data was made from, as its Ready token records it. A reader compares
/// and logs it, never interprets it: equal values name the same data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub generation: String,
    /// The time the source's data is as of (RFC 3339, UTC).
    pub as_of: String,
    /// The id of the server the data came from.
    pub server: String,
    pub interim_format_version: u32,
    pub transformation_version: u32,
    pub inputs_digest: String,
}

/// Told the shortcode of a project whose data changed. Called on the announcing adapter's thread,
/// so it hands the shortcode off and returns, reading the port on its own thread. It holds no
/// handle on the adapter it subscribed to: the adapter owns its listeners, so such a handle keeps
/// the adapter, and its poll, alive for good.
pub trait ChangeListener: Send + Sync {
    fn changed(&self, shortcode: &str);
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
/// code: the rights, and a subscription to the change `media` announces (DEV-7938).
/// Synchronous and dyn-compatible like [`ArchiveProjection`]; CPE calls it inside
/// `spawn_blocking`.
pub trait DepositedFiles: Send + Sync {
    /// The file's bytes, whole. A read that fails midway is [`FilesError::Unavailable`], never a
    /// prefix. A `shortcode` that fails [`is_shortcode`] is [`FilesError::UnknownProject`], and
    /// otherwise a `name` that fails [`is_file_name`] is [`FilesError::UnknownFile`].
    fn file(&self, shortcode: &str, name: &str) -> Result<Vec<u8>, FilesError>;

    /// What the project's current files were made from: the archive projection's provenance of the
    /// same project when both serve the same data. `Ok(None)`: the adapter records no provenance
    /// for it.
    fn provenance(&self, _shortcode: &str) -> Result<Option<Provenance>, FilesError> {
        Ok(None)
    }
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

    struct Bare;

    impl ArchiveProjection for Bare {
        fn snapshot(&self, shortcode: &str) -> Result<ProjectSnapshot, ProjectionError> {
            Err(ProjectionError::UnknownProject { shortcode: shortcode.to_string() })
        }
    }

    #[test]
    fn test_location_present_by_default_is_false() {
        assert!(!Bare.location_present("0803"));
    }

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
