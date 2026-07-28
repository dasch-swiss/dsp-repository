//! Repository interface and filesystem implementation for Records.

use super::record::Record;

/// Repository interface for accessing Records.
pub trait RecordRepository {
    fn get_all(&self) -> &[Record];
    fn get_by_id(&self, id: &str) -> Option<&Record>;
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
}
