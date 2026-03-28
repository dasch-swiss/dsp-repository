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
#[cfg(not(target_arch = "wasm32"))]
pub struct FsRecordRepository;

#[cfg(not(target_arch = "wasm32"))]
impl Default for FsRecordRepository {
    fn default() -> Self {
        Self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl FsRecordRepository {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl RecordRepository for FsRecordRepository {
    fn get_all(&self) -> &[Record] {
        super::record_cache::all_records()
    }

    fn get_by_id(&self, ark_suffix: &str) -> Option<&Record> {
        super::record_cache::all_records()
            .iter()
            .find(|r| r.pid.ark_suffix() == ark_suffix)
    }
}
