//! Wire types for the Dataverse `versions` response.
//!
//! These mirror the Dataverse Native API shape, because the consuming parser
//! (`datahugger-ng`'s Dataverse backend) hard-fails a record when a field is
//! missing or malformed. `notes/dataverse-api-handoff.md` §3.1 is the
//! authoritative field list — the parser is the client, not the specification, so
//! a field it happens to tolerate may still be required by the format.
//!
//! Note what the format does *not* have: a download URL. Clients derive it from
//! `dataFile.id` by string concatenation (`/api/access/datafile/{id}`), which is
//! why that id must keep resolving to the same file. Verified against a live
//! Harvard response — see `notes/examples/harvard-dataverse-payload.json`, where
//! the only storage-facing field is an internal `storageIdentifier`.

use dpe_core::DataverseFile;
use serde::Serialize;

/// The response envelope: `{"status": "OK", "data": {"files": [...]}}`.
#[derive(Debug, Serialize)]
pub struct VersionsResponse {
    pub status: &'static str,
    pub data: VersionsData,
}

impl VersionsResponse {
    /// Wraps the given files in the `status: OK` envelope.
    pub fn ok(files: Vec<DataverseFile>) -> Self {
        Self {
            status: "OK",
            data: VersionsData { files: files.into_iter().map(FileEntry::from).collect() },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct VersionsData {
    pub files: Vec<FileEntry>,
}

/// One entry in `data.files`. The per-file metadata sits in the nested `dataFile`;
/// `restricted` and `version` are siblings of it, not members — that nesting is
/// part of the contract.
///
/// `directoryLabel` is not emitted at all: it is optional in the contract, DPE has
/// no directory concept at any layer, and the live Harvard response omits it on
/// every file.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    /// Always `false` — all DaSCH data is currently open.
    pub restricted: bool,
    /// DPE has no per-file version concept, and the contract requires the field,
    /// so every file is version 1. Revisit if record versioning is introduced.
    pub version: u64,
    pub data_file: DataFileEntry,
}

/// The `dataFile` object: the file's own metadata.
///
/// Three of these fields are placeholders today (`filename`, `filesize`,
/// `checksum`) and come from [`dpe_core::FilePlaceholders`]. They are laid out here
/// exactly as the real values will be, so wiring up the dsp-api file endpoint is a
/// change to how `DataverseFile` is built, not to this type or to the handlers.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFileEntry {
    /// Stable numeric id; the client builds `/api/access/datafile/{id}` from it.
    pub id: u64,
    /// PLACEHOLDER — synthesised from the asset id, not the original filename.
    pub filename: String,
    pub content_type: String,
    /// PLACEHOLDER — no upstream source for file size yet.
    pub filesize: u64,
    pub creation_date: String,
    /// Omitted rather than serialized as `null` when absent: the parser expects a
    /// string here, and a `null` is a different shape than a missing key. DPE has
    /// no per-file modification date, so this is currently always absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_update_time: Option<String>,
    /// PLACEHOLDER — not computed from the bytes.
    pub checksum: Checksum,
}

/// The `checksum` object. Dataverse also emits a bare `md5` field alongside it;
/// that variant is only read by the `file.xhtml`-style lookup this pipeline does
/// not exercise, so it is not emitted here.
#[derive(Debug, Serialize)]
pub struct Checksum {
    #[serde(rename = "type")]
    pub checksum_type: String,
    pub value: String,
}

impl From<DataverseFile> for FileEntry {
    fn from(file: DataverseFile) -> Self {
        Self {
            restricted: file.restricted,
            version: 1,
            data_file: DataFileEntry {
                id: file.id,
                filename: file.placeholders.filename,
                content_type: file.content_type,
                filesize: file.placeholders.filesize,
                creation_date: file.creation_date,
                // No per-file modification date exists in DPE; records carry only
                // `dateCreated`. Absent rather than mirroring the creation date,
                // which would assert something untrue.
                last_update_time: None,
                checksum: Checksum {
                    checksum_type: file.placeholders.checksum_type,
                    value: file.placeholders.checksum_value,
                },
            },
        }
    }
}
