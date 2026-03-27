//! Repository interface and filesystem implementation for Records.

use super::record::Record;

/// Repository interface for accessing Records.
pub trait RecordRepository {
    fn get_all(&self) -> Vec<Record>;
    fn get_by_id(&self, id: &str) -> Option<Record>;
}

/// Filesystem-backed implementation of [`RecordRepository`].
///
/// Backed by the in-process record cache (loaded once on first access).
#[cfg(not(target_arch = "wasm32"))]
pub struct FsRecordRepository;

#[cfg(not(target_arch = "wasm32"))]
impl FsRecordRepository {
    pub fn new(_data_dir: String) -> Self {
        Self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl RecordRepository for FsRecordRepository {
    fn get_all(&self) -> Vec<Record> {
        super::record_cache::all_records().to_vec()
    }

    fn get_by_id(&self, ark_suffix: &str) -> Option<Record> {
        super::record_cache::all_records()
            .iter()
            .find(|r| r.pid.ark_suffix() == ark_suffix)
            .cloned()
    }
}
