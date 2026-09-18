//! Repository interface and filesystem implementation for Records.

use shared_metadata::Record;

/// Repository interface for accessing Records.
pub trait RecordRepository {
    fn get_all(&self) -> &[Record];
    fn get_by_id(&self, id: &str) -> Option<&Record>;

    /// The records of one project, matched case-insensitively, in `get_all`
    /// order.
    ///
    /// The default is the scan it replaces, which is the right implementation
    /// for an in-memory double holding a handful of records.
    /// [`FsRecordRepository`] overrides it with the cache's shortcode index,
    /// because behind it sit every project's records at once.
    fn records_for_shortcode(&self, shortcode: &str) -> Vec<&Record> {
        self.get_all()
            .iter()
            .filter(|r| r.pid.shortcode.eq_ignore_ascii_case(shortcode))
            .collect()
    }
}

/// Filesystem-backed implementation of [`RecordRepository`].
///
/// Backed by the in-process record cache (loaded once on first access).
pub struct FsRecordRepository;

impl Default for FsRecordRepository {
    fn default() -> Self {
        Self
    }
}

impl FsRecordRepository {
    pub fn new() -> Self {
        Self
    }
}

impl RecordRepository for FsRecordRepository {
    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn get_all(&self) -> &[Record] {
        super::record_cache::all_records()
    }

    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn get_by_id(&self, ark_suffix: &str) -> Option<&Record> {
        super::record_cache::all_records()
            .iter()
            .find(|r| r.pid.ark_suffix() == ark_suffix)
    }

    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn records_for_shortcode(&self, shortcode: &str) -> Vec<&Record> {
        super::record_cache::records_for_shortcode(shortcode).to_vec()
    }
}
