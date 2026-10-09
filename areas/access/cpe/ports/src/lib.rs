//! CPE's port onto the Access Area's archive projection (ADR-0003, ADR-0007, ADR-0008).
//!
//! [`ArchiveProjection`] is what CPE reads to rebuild its store: one project's facts in the
//! archive's shape, never CPE's presentation model. The snapshot also carries the project's
//! curation ([`CuratedValue`]), which is not an archive fact. `sync` implements the port beside its
//! data; CPE's tests use [`FakeArchiveProjection`], and every adapter's output must pass
//! [`contract::violations`].
//!
//! The crate depends on `std` alone: a consumer's `ports` crate may reach only `std` and `shared-*`
//! (ADR-0003), and nothing in `shared-*` is needed here. No type is `#[non_exhaustive]`, so CPE's
//! fixtures can use struct literals and a new [`ValueKind`] fails CPE's compile instead of hiding
//! behind a `_ =>` arm.

use std::error::Error;
use std::fmt;

pub mod contract;
mod fake;
mod snapshot;

pub use fake::FakeArchiveProjection;
pub use snapshot::{
    Annotation, Calendar, ClassIri, CuratedValue, DataArk, DateBound, DatePrecision, DateValue, File, LangString,
    ListNode, ListNodeIri, Motivation, ProjectSnapshot, PropertyIri, Resource, ResourceIri, Value, ValueKind,
    DATA_ARK_PREFIX,
};

/// What CPE reads from the Access Area's archive projection: one project's facts as the archive
/// records them, and with them the project's curation, which is not an archive fact.
///
/// Synchronous and dyn-compatible, so the composition root can hold an `Arc<dyn
/// ArchiveProjection>`; CPE calls it inside `spawn_blocking`.
pub trait ArchiveProjection: Send + Sync {
    /// The project's current facts and its curation, whole. Every call is a full snapshot; the port
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
}
