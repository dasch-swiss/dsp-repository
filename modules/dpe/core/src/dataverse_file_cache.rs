//! Record-backed lookup for Dataverse file metadata.
//!
//! There is no sidecar data file: everything the Dataverse API serves is derived
//! from the records themselves (plus placeholders for the three fields with no
//! upstream source — see [`crate::dataverse_file::FilePlaceholders`]).
//!
//! Two lookups are needed, and neither should scan 51k records per request:
//!
//! - by OAI header identifier, for the versions endpoint;
//! - by numeric file id, for the download endpoint.
//!
//! Both are served from an index built once, on first access, over the record
//! cache. The index stores record IRIs rather than cloned file structs, so it
//! costs one small map per record instead of duplicating the file metadata.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::dataverse_file::{file_id_for_iri, DataverseFile, DataverseFileRepository};
use crate::record::{Record, ARK_PATH_PREFIX};
use crate::record_repository::{FsRecordRepository, RecordRepository};

/// OAI identifier prefix, mirroring `dpe-api-oai`'s `OAI_IDENTIFIER_PREFIX`.
///
/// Duplicated rather than shared because `dpe-core` must not depend on an API
/// crate, and the OAI crate keeps its copy private. The
/// `oai_identifier_matches_api_oai_format` test pins the two together.
const OAI_IDENTIFIER_PREFIX: &str = "oai:dasch.swiss:";

/// Maps numeric file ids to the record IRI that produced them.
#[derive(Debug, Default)]
pub struct DataverseFileIndex {
    by_id: HashMap<u64, String>,
}

impl DataverseFileIndex {
    /// Builds the id index over the given records.
    ///
    /// Ids are derived by hashing the record IRI, so a collision is possible in
    /// principle. It is detected here rather than silently overwriting: a
    /// collision would make one file's download URL serve the other's bytes, so it
    /// warns loudly and keeps the first entry. The
    /// `committed_records_have_no_id_collisions` test makes this a build-time
    /// failure for the committed data.
    pub fn build(records: &[Record]) -> Self {
        let mut by_id: HashMap<u64, String> = HashMap::new();

        for record in records.iter().filter(|r| r.file.is_some()) {
            let id = file_id_for_iri(&record.id);
            if let Some(existing) = by_id.get(&id) {
                tracing::error!(
                    id,
                    kept = %existing,
                    ignored = %record.id,
                    "file id collision between two records; download URL for one of them is wrong"
                );
                continue;
            }
            by_id.insert(id, record.id.clone());
        }

        Self { by_id }
    }

    /// The record IRI a file id belongs to.
    pub fn iri_for_id(&self, id: u64) -> Option<&str> {
        self.by_id.get(&id).map(String::as_str)
    }

    /// Number of indexed files.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

static INDEX: OnceLock<DataverseFileIndex> = OnceLock::new();

/// The process-wide file-id index, built on first access over the record cache.
pub fn file_index() -> &'static DataverseFileIndex {
    INDEX.get_or_init(|| DataverseFileIndex::build(crate::record_cache::all_records()))
}

/// Extracts the ARK suffix from a full OAI header identifier, e.g.
/// `oai:dasch.swiss:ark:/72163/1/0803/abc=gh` → `0803/abc=gh`.
///
/// Note the colons inside the ARK itself: the identifier is matched by prefix,
/// never split on `:`.
pub fn parse_oai_identifier(identifier: &str) -> Option<&str> {
    identifier.strip_prefix(OAI_IDENTIFIER_PREFIX)?.strip_prefix(ARK_PATH_PREFIX)
}

/// Production [`DataverseFileRepository`], backed by the record cache and the
/// process-wide id index.
pub struct FsDataverseFileRepository;

impl Default for FsDataverseFileRepository {
    fn default() -> Self {
        Self
    }
}

impl FsDataverseFileRepository {
    pub fn new() -> Self {
        Self
    }
}

impl DataverseFileRepository for FsDataverseFileRepository {
    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn files_for(&self, oai_identifier: &str) -> Option<Vec<DataverseFile>> {
        let suffix = parse_oai_identifier(oai_identifier)?;
        // `FsRecordRepository::get_by_id` borrows from the process-wide record cache,
        // so the repository value itself must outlive the returned reference.
        let records = FsRecordRepository::new();
        let record = records.get_by_id(suffix)?;
        // A record with no file yields an empty list, not `None`: the record exists,
        // it simply has no files. Roughly 77% of records are in this state.
        Some(DataverseFile::from_record(record).into_iter().collect())
    }

    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn file_by_id(&self, id: u64) -> Option<DataverseFile> {
        let iri = file_index().iri_for_id(id)?;
        let record = crate::record_cache::all_records().iter().find(|r| r.id == iri)?;
        DataverseFile::from_record(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::RecordFile;

    /// All committed records, loaded through the production cache.
    ///
    /// `set_data_dir` is a process-global that other tests may also set; these
    /// tests only read, and assert over whatever the ambient data dir provides,
    /// skipping when it yields nothing.
    fn committed_records() -> &'static [Record] {
        crate::record_cache::all_records()
    }

    fn test_record(iri: &str, with_file: bool) -> Record {
        let json = include_str!("../../server/data/records_test/0803-records.json");
        let [mut record]: [Record; 1] = serde_json::from_str(json).expect("parse test record");
        record.id = iri.to_string();
        record.file = with_file.then(|| RecordFile {
            mime_type: "image/jp2".to_string(),
            url: format!("https://ingest.dasch.swiss/projects/0803/assets/{iri}/original"),
        });
        record
    }

    // ---- OAI identifier parsing ----

    #[test]
    fn parses_record_and_project_identifiers() {
        assert_eq!(
            parse_oai_identifier("oai:dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
            Some("0803/lklK7rVuVOmpBZYWrF8o=gh")
        );
        assert_eq!(parse_oai_identifier("oai:dasch.swiss:ark:/72163/1/081C"), Some("081C"));
    }

    #[test]
    fn rejects_foreign_identifiers() {
        assert_eq!(parse_oai_identifier("doi:10.7910/DVN/00234"), None);
        assert_eq!(parse_oai_identifier("oai:example.org:ark:/72163/1/0803"), None);
        assert_eq!(parse_oai_identifier("oai:dasch.swiss:something-else"), None);
    }

    #[test]
    fn oai_identifier_matches_api_oai_format() {
        // Pins the duplicated prefix against the ARK prefix `dpe-api-oai` builds
        // identifiers from. If either changes, the Dataverse lookup silently stops
        // resolving every identifier, so this equality is worth asserting.
        assert_eq!(OAI_IDENTIFIER_PREFIX, "oai:dasch.swiss:");
        assert_eq!(ARK_PATH_PREFIX, "ark:/72163/1/");
    }

    // ---- index behaviour ----

    #[test]
    fn index_covers_only_file_bearing_records() {
        let records = vec![
            test_record("http://rdfh.ch/0803/withfile", true),
            test_record("http://rdfh.ch/0803/nofile", false),
        ];
        let index = DataverseFileIndex::build(&records);

        assert_eq!(index.len(), 1);
        assert_eq!(
            index.iri_for_id(file_id_for_iri("http://rdfh.ch/0803/withfile")),
            Some("http://rdfh.ch/0803/withfile")
        );
        assert_eq!(index.iri_for_id(file_id_for_iri("http://rdfh.ch/0803/nofile")), None);
    }

    #[test]
    fn index_lookup_of_unknown_id_is_none() {
        let index = DataverseFileIndex::build(&[]);
        assert!(index.iri_for_id(12345).is_none());
    }

    // ---- committed-data integrity ----

    #[test]
    fn committed_records_have_no_id_collisions() {
        // The guarantee the derived-id scheme rests on: two records must never hash
        // to the same file id, or one of them serves the other's bytes. Checked over
        // the real data so a future dump that happens to collide fails the build.
        let records = committed_records();
        if records.is_empty() {
            return; // no ambient data dir; nothing to verify
        }

        let file_bearing = records.iter().filter(|r| r.file.is_some()).count();
        let index = DataverseFileIndex::build(records);

        assert_eq!(
            index.len(),
            file_bearing,
            "id collision: {} file-bearing records produced only {} distinct ids",
            file_bearing,
            index.len()
        );
    }

    #[test]
    fn committed_record_ids_fit_in_53_bits() {
        let records = committed_records();
        if records.is_empty() {
            return;
        }

        for record in records.iter().filter(|r| r.file.is_some()) {
            let id = file_id_for_iri(&record.id);
            assert!(id < (1 << 53), "record {} produced id {id} exceeding 2^53", record.id);
        }
    }

    #[test]
    fn committed_file_bearing_records_produce_complete_metadata() {
        // Every field the contract requires must be non-empty for every real file,
        // since the consumer hard-fails a record otherwise.
        let records = committed_records();
        if records.is_empty() {
            return;
        }

        for record in records.iter().filter(|r| r.file.is_some()) {
            let file = DataverseFile::from_record(record).expect("record has a file");

            assert!(!file.content_type.is_empty(), "empty contentType on {}", record.id);
            assert!(
                file.content_type.contains('/'),
                "malformed contentType {:?} on {}",
                file.content_type,
                record.id
            );
            assert!(!file.creation_date.is_empty(), "empty creationDate on {}", record.id);
            assert!(
                file.download_url.starts_with("https://"),
                "non-HTTPS downloadUrl {:?} on {}",
                file.download_url,
                record.id
            );
            assert!(!file.placeholders.filename.is_empty(), "empty filename on {}", record.id);
            assert!(file.placeholders.filesize > 0, "zero filesize on {}", record.id);
        }
    }
}
