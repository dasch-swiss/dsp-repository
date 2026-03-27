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
#[cfg(not(target_arch = "wasm32"))]
pub struct FsRecordRepository {
    data_dir: String,
}

#[cfg(not(target_arch = "wasm32"))]
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
                    let content = match fs::read_to_string(&path) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("Failed to read record file {:?}: {}", path, e);
                            return None;
                        }
                    };
                    match serde_json::from_str::<Vec<Record>>(&content) {
                        Ok(records) => Some(records),
                        Err(e) => {
                            eprintln!("Failed to parse record file {:?}: {}", path, e);
                            None
                        }
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl RecordRepository for FsRecordRepository {
    fn get_all(&self) -> Vec<Record> {
        self.read_all_records()
    }

    fn get_by_id(&self, ark_suffix: &str) -> Option<Record> {
        self.read_all_records().into_iter().find(|r| r.pid.ark_suffix() == ark_suffix)
    }
}
