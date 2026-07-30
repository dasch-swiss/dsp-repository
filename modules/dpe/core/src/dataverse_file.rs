//! File-level metadata for the Dataverse-compatible API.
//!
//! The EOSC Data Commons crawler harvests our OAI-PMH feed and then, per record,
//! asks a Dataverse Native API for that record's files. That contract needs a
//! numeric id, a filename, a size, a MIME type, a checksum, dates, and a
//! restricted flag per file.
//!
//! Most of it is derived from the record itself ([`DataverseFile::from_record`]).
//! Three fields have no source in DPE or dsp-api today — `filename`, `filesize`,
//! and the checksum — and are filled with placeholders until the dsp-api file
//! endpoint supplies them. They are grouped in [`FilePlaceholders`] so the seam is
//! explicit and the swap is a single call site.

use crate::record::Record;

/// One file's Dataverse-facing metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct DataverseFile {
    /// Stable numeric id. In the Dataverse format the id *is* the download
    /// address: clients build `/api/access/datafile/{id}` from it by string
    /// concatenation and cache the result, so it must keep resolving to the same
    /// file forever. Derived deterministically from the record IRI — see
    /// [`file_id_for_iri`].
    pub id: u64,
    /// MIME type, from the record's `file.mimeType`.
    pub content_type: String,
    /// ISO 8601, from the record's `dateCreated`.
    pub creation_date: String,
    /// Always `false`: all DaSCH data is currently open. See [`DataverseFile::RESTRICTED`].
    pub restricted: bool,
    /// Where the bytes live (dsp-ingest), from the record's `file.url`. Not part
    /// of the API response — the download route redirects here.
    pub download_url: String,
    /// Fields with no upstream source yet.
    pub placeholders: FilePlaceholders,
}

/// The fields the Dataverse contract requires but neither DPE nor dsp-api can
/// supply yet.
///
/// The contract's consumer (`datahugger-ng`) hard-fails a record when any of these
/// is missing, so they cannot simply be omitted; they are filled with obvious
/// placeholders instead. **These values are fiction.** A client verifying a
/// checksum against the downloaded bytes will find a mismatch.
///
/// When the dsp-api file endpoint lands, this struct is what it populates — the
/// wire DTOs and handlers do not change.
#[derive(Clone, Debug, PartialEq)]
pub struct FilePlaceholders {
    /// Original filename. dsp-api has it (`FileValueV2.originalFilename`) but
    /// discards it during export; the asset id in the download URL is opaque and
    /// carries no name or extension, so nothing real can be derived locally.
    pub filename: String,
    /// Size in bytes. Not present in dsp-api at all — must come from dsp-ingest.
    pub filesize: u64,
    /// Checksum algorithm: `MD5`, `SHA-1`, or `SHA-256`. Note the consumer accepts
    /// only MD5 and SHA-1 today, so placeholders use MD5.
    pub checksum_type: String,
    /// Checksum value, hex.
    pub checksum_value: String,
}

impl FilePlaceholders {
    /// The checksum algorithm placeholders are emitted under.
    pub const CHECKSUM_TYPE: &'static str = "MD5";

    /// A syntactically valid MD5 that is recognisable as a placeholder: the
    /// well-known hash of the empty string. Chosen deliberately over random hex so
    /// that anyone inspecting a response can tell the value is not real.
    pub const CHECKSUM_VALUE: &'static str = "d41d8cd98f00b204e9800998ecf8427e";

    /// Placeholder size in bytes. Non-zero because the contract treats a file as
    /// having content, and some consumers reject zero-length entries.
    pub const FILESIZE: u64 = 1;

    /// Builds placeholders for a record whose file has the given MIME type.
    ///
    /// The filename is synthesised from the asset id in the download URL plus an
    /// extension guessed from the MIME type. It is *not* the original filename —
    /// it exists so the field is present and plausible, and is replaced wholesale
    /// once dsp-api exports the real one.
    pub fn for_asset(asset_id: &str, mime_type: &str) -> Self {
        Self {
            filename: format!("{asset_id}{}", extension_for_mime(mime_type)),
            filesize: Self::FILESIZE,
            checksum_type: Self::CHECKSUM_TYPE.to_string(),
            checksum_value: Self::CHECKSUM_VALUE.to_string(),
        }
    }
}

/// A file extension for the given MIME type, including the leading dot, or an
/// empty string when the type is unknown.
///
/// Covers only the types actually present in the record data; anything else gets
/// no extension rather than a wrong one.
fn extension_for_mime(mime_type: &str) -> &'static str {
    // Strip any parameters (`text/csv; charset=UTF-8`) before matching.
    match mime_type.split(';').next().unwrap_or("").trim() {
        "image/jp2" | "image/jpx" => ".jp2",
        "image/jpeg" => ".jpg",
        "image/tiff" => ".tif",
        "image/png" => ".png",
        "text/plain" => ".txt",
        "text/csv" => ".csv",
        "application/pdf" => ".pdf",
        "application/zip" => ".zip",
        "application/xml" | "text/xml" => ".xml",
        _ => "",
    }
}

/// Derives the stable numeric file id for a record IRI.
///
/// Deterministic, stateless, and stable across regeneration of the record dumps —
/// which matters because those dumps are machine-generated, so any hand-assigned
/// id would be lost on the next refresh.
///
/// Masked to 53 bits, not 64: JSON numbers are IEEE-754 doubles in many consumers,
/// which lose integer precision above 2^53. A 53-bit space still makes a collision
/// vanishingly unlikely at DaSCH's scale (~5×10^4 records → ~10^-9), and a
/// collision is *detected*, not silent — the id index rejects duplicates, so it
/// surfaces as a failing test rather than a download URL serving the wrong bytes.
pub fn file_id_for_iri(iri: &str) -> u64 {
    /// FNV-1a, 64-bit. Chosen over a cryptographic hash because the property
    /// needed is determinism and spread, not preimage resistance, and this keeps
    /// `dpe-core` dependency-free.
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    const MASK_53_BITS: u64 = (1 << 53) - 1;

    let mut hash = FNV_OFFSET;
    for byte in iri.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    // Fold the high bits down rather than truncating, so entropy above bit 53 is
    // not simply discarded.
    ((hash >> 53) ^ hash) & MASK_53_BITS
}

impl DataverseFile {
    /// All DaSCH data is currently openly accessible: every record carries
    /// `accessRights: "Full Open Access"`, and dsp-api hardcodes it on export, so
    /// there is no per-file restriction signal to represent. The contract requires
    /// the field, so it is emitted as a constant.
    ///
    /// If DaSCH ever holds restricted assets this must become a real per-file
    /// value; a per-record access-rights string cannot express it.
    pub const RESTRICTED: bool = false;

    /// Builds the Dataverse view of a record's file, or `None` when the record has
    /// no file (the majority — roughly 77% of records are metadata-only).
    pub fn from_record(record: &Record) -> Option<Self> {
        let file = record.file.as_ref()?;

        Some(Self {
            id: file_id_for_iri(&record.id),
            content_type: file.mime_type.clone(),
            creation_date: record.date_created.clone(),
            restricted: Self::RESTRICTED,
            download_url: file.url.clone(),
            placeholders: FilePlaceholders::for_asset(asset_id_of(&file.url), &file.mime_type),
        })
    }
}

/// Extracts the dsp-ingest asset id from a download URL, e.g.
/// `https://ingest.dasch.swiss/projects/0803/assets/{asset_id}/original`.
///
/// Falls back to the whole URL's last non-empty segment if the shape differs, so a
/// changed URL layout degrades instead of panicking.
fn asset_id_of(url: &str) -> &str {
    let trimmed = url.trim_end_matches('/');
    match trimmed.strip_suffix("/original") {
        Some(rest) => rest.rsplit('/').next().unwrap_or(trimmed),
        None => trimmed.rsplit('/').find(|s| !s.is_empty()).unwrap_or(trimmed),
    }
}

/// Read access to record file metadata, so handlers can be tested against
/// in-memory data instead of the process caches.
pub trait DataverseFileRepository {
    /// Files belonging to the given full OAI header identifier. `Some(vec![])`
    /// means "record exists, has no files"; `None` means "no such record".
    fn files_for(&self, oai_identifier: &str) -> Option<Vec<DataverseFile>>;

    /// A single file by its stable numeric id.
    fn file_by_id(&self, id: u64) -> Option<DataverseFile>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- id derivation ----

    #[test]
    fn id_is_deterministic() {
        let iri = "http://rdfh.ch/0803/lklK7rVuVOmpBZYWrF8o-g";
        assert_eq!(file_id_for_iri(iri), file_id_for_iri(iri));
    }

    #[test]
    fn id_differs_between_records() {
        assert_ne!(
            file_id_for_iri("http://rdfh.ch/0803/lklK7rVuVOmpBZYWrF8o-g"),
            file_id_for_iri("http://rdfh.ch/0803/SLtrNyQ3WtyhHKIVLAL9Jg")
        );
    }

    #[test]
    fn id_fits_in_53_bits() {
        // Above 2^53 a JSON consumer using doubles would silently round the id,
        // producing a download URL that resolves to nothing.
        for iri in [
            "http://rdfh.ch/0803/lklK7rVuVOmpBZYWrF8o-g",
            "http://rdfh.ch/081C/KfnRJvxJQ1WyIP59EbDbrw",
            "http://rdfh.ch/0868/0sKCU-ILTt-rl0IpTYQ0mw",
            "",
        ] {
            let id = file_id_for_iri(iri);
            assert!(id < (1 << 53), "id {id} for {iri:?} exceeds 2^53");
        }
    }

    #[test]
    fn id_is_nonzero_for_real_iris() {
        // A zero id would look like "unset" in logs and downstream stores.
        assert_ne!(file_id_for_iri("http://rdfh.ch/0803/lklK7rVuVOmpBZYWrF8o-g"), 0);
    }

    // ---- asset id extraction ----

    #[test]
    fn extracts_asset_id_from_ingest_url() {
        assert_eq!(
            asset_id_of("https://ingest.dasch.swiss/projects/0803/assets/2vbIabBOEvq-EU9jwmgEe9j/original"),
            "2vbIabBOEvq-EU9jwmgEe9j"
        );
    }

    #[test]
    fn asset_id_falls_back_on_unexpected_shape() {
        assert_eq!(asset_id_of("https://example.invalid/some/path"), "path");
        assert_eq!(asset_id_of("https://example.invalid/trailing/"), "trailing");
    }

    // ---- placeholder construction ----

    #[test]
    fn placeholder_filename_uses_asset_id_and_mime_extension() {
        let p = FilePlaceholders::for_asset("2vbIabBOEvq-EU9jwmgEe9j", "image/jp2");
        assert_eq!(p.filename, "2vbIabBOEvq-EU9jwmgEe9j.jp2");
    }

    #[test]
    fn placeholder_filename_handles_mime_parameters() {
        let p = FilePlaceholders::for_asset("abc", "text/csv; charset=UTF-8");
        assert_eq!(p.filename, "abc.csv");
    }

    #[test]
    fn placeholder_filename_omits_extension_for_unknown_mime() {
        // Better a name with no extension than a name with a wrong one.
        let p = FilePlaceholders::for_asset("abc", "application/x-rlang-transport");
        assert_eq!(p.filename, "abc");
    }

    #[test]
    fn placeholder_checksum_is_valid_hex_md5() {
        let p = FilePlaceholders::for_asset("abc", "text/csv");
        assert_eq!(p.checksum_type, "MD5");
        assert_eq!(p.checksum_value.len(), 32);
        assert!(p.checksum_value.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn placeholder_filesize_is_nonzero() {
        assert!(FilePlaceholders::for_asset("abc", "text/csv").filesize > 0);
    }

    // ---- record mapping ----

    fn record_with_file(mime: &str, url: &str) -> Record {
        let json = include_str!("../../server/data/records_test/0803-records.json");
        let [mut record]: [Record; 1] = serde_json::from_str(json).expect("parse test record");
        record.file = Some(crate::record::RecordFile { mime_type: mime.to_string(), url: url.to_string() });
        record
    }

    #[test]
    fn maps_record_fields_from_real_data() {
        let record = record_with_file(
            "image/jp2",
            "https://ingest.dasch.swiss/projects/0803/assets/2vbIabBOEvq-EU9jwmgEe9j/original",
        );
        let file = DataverseFile::from_record(&record).expect("record has a file");

        assert_eq!(file.content_type, "image/jp2");
        assert_eq!(file.creation_date, record.date_created);
        assert_eq!(file.download_url, record.file.as_ref().unwrap().url);
        assert_eq!(file.id, file_id_for_iri(&record.id));
        assert!(!file.restricted, "all DaSCH data is currently open");
    }

    #[test]
    fn record_without_file_maps_to_none() {
        // ~77% of records are metadata-only.
        let mut record = record_with_file("image/jp2", "https://example.invalid/a/original");
        record.file = None;
        assert!(DataverseFile::from_record(&record).is_none());
    }
}
