//! Repository interface and filesystem implementation for Records.

use super::record::Record;

/// Repository interface for accessing Records.
pub trait RecordRepository {
    fn get_all(&self) -> Vec<Record>;
    fn get_by_id(&self, id: &str) -> Option<Record>;
}

/// Filesystem-backed implementation of [`RecordRepository`].
///
/// Reads record JSON files from the configured data directory.
#[cfg(feature = "ssr")]
pub struct FsRecordRepository {
    data_dir: String,
}

#[cfg(feature = "ssr")]
impl FsRecordRepository {
    pub fn new(data_dir: String) -> Self {
        Self { data_dir }
    }

    fn read_all_records(&self) -> Vec<Record> {
        use std::fs;
        use std::path::PathBuf;

        let records_dir = PathBuf::from(&self.data_dir).join("records");

        if !records_dir.exists() {
            return Vec::new();
        }

        let Ok(entries) = fs::read_dir(&records_dir) else {
            return Vec::new();
        };

        entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "json") {
                    fs::read_to_string(&path)
                        .ok()
                        .and_then(|content| serde_json::from_str::<Record>(&content).ok())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(feature = "ssr")]
impl RecordRepository for FsRecordRepository {
    fn get_all(&self) -> Vec<Record> {
        self.read_all_records()
    }

    fn get_by_id(&self, ark_suffix: &str) -> Option<Record> {
        const ARK_PATH_PREFIX: &str = "ark:/72163/1/";
        self.read_all_records().into_iter().find(|r| {
            r.pid
                .find(ARK_PATH_PREFIX)
                .map(|pos| &r.pid[pos + ARK_PATH_PREFIX.len()..])
                == Some(ark_suffix)
        })
    }
}

