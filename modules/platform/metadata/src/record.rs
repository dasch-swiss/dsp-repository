//! Record domain model.
//!
//! Represents an individual data Record within a Research Project,
//! as defined in the DaSCH Metadata 2.0 schema.

/// ARK path prefix for DaSCH resources.
///
/// `72163` is DaSCH's NAAN (Name Assigning Authority Number).
pub const ARK_PATH_PREFIX: &str = "ark:/72163/1/";

use serde::{Deserialize, Serialize};

use crate::utils::Multilingual;

/// A parsed ARK persistent identifier for a DaSCH record.
///
/// Example: `https://ark.dasch.swiss/ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh`
/// - `host`: `https://ark.dasch.swiss`
/// - `shortcode`: `0803`
/// - `record_id`: `lklK7rVuVOmpBZYWrF8o=gh`
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Pid {
    pub host: String,
    pub shortcode: String,
    pub record_id: String,
}

impl Pid {
    /// Returns the full ARK URL, e.g. `https://ark.dasch.swiss/ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh`.
    pub fn as_url(&self) -> String {
        format!("{}/{}{}/{}", self.host, ARK_PATH_PREFIX, self.shortcode, self.record_id)
    }

    /// Returns the ARK path without host, e.g. `ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh`.
    pub fn ark_path(&self) -> String {
        format!("{}{}/{}", ARK_PATH_PREFIX, self.shortcode, self.record_id)
    }

    /// Returns the suffix after the ARK prefix, e.g. `0803/lklK7rVuVOmpBZYWrF8o=gh`.
    pub fn ark_suffix(&self) -> String {
        format!("{}/{}", self.shortcode, self.record_id)
    }
}

impl Pid {
    pub fn new(host: &str, shortcode: &str, record_id: &str) -> Self {
        Self {
            host: host.to_string(),
            shortcode: shortcode.to_string(),
            record_id: record_id.to_string(),
        }
    }
}

impl<'de> Deserialize<'de> for Pid {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        parse_pid(&s).map_err(serde::de::Error::custom)
    }
}

fn parse_pid(s: &str) -> Result<Pid, String> {
    let ark_pos = s.find(ARK_PATH_PREFIX).ok_or_else(|| format!("pid missing ARK prefix: {s}"))?;
    let host = s[..ark_pos].trim_end_matches('/').to_string();
    let after_prefix = &s[ark_pos + ARK_PATH_PREFIX.len()..];
    let slash = after_prefix
        .find('/')
        .ok_or_else(|| format!("pid missing record_id segment: {s}"))?;
    let shortcode = after_prefix[..slash].to_string();
    let record_id = after_prefix[slash + 1..].to_string();
    if shortcode.is_empty() {
        return Err(format!("pid has empty shortcode: {s}"));
    }
    if record_id.is_empty() {
        return Err(format!("pid has empty record_id: {s}"));
    }
    Ok(Pid { host, shortcode, record_id })
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RecordLicense {
    #[serde(rename = "licenseIdentifier")]
    pub license_identifier: String,
    #[serde(rename = "licenseDate")]
    pub license_date: String,
    #[serde(rename = "licenseURI", alias = "licenseUri")]
    pub license_uri: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RecordLegalInfo {
    pub license: RecordLicense,
    #[serde(rename = "copyrightHolder")]
    pub copyright_holder: String,
    pub authorship: Vec<String>,
}

/// `url` is dsp-ingest's own address — see `docs/src/dpe/oai-pmh.md` for why it is
/// never published. `url` is the only required field: every other value is absent
/// from some production export, and a required field that the data does not carry
/// makes `serde_json` reject the entire dump rather than the one record.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RecordFile {
    #[serde(rename = "mimeType", default)]
    pub mime_type: Option<String>,
    pub url: String,
    #[serde(default)]
    pub checksum: Option<String>,
    #[serde(rename = "checksumAlgorithm", default)]
    pub checksum_algorithm: Option<String>,
    #[serde(rename = "fileName", default)]
    pub file_name: Option<String>,
    #[serde(rename = "fileSize", default)]
    pub file_size: Option<u64>,
    #[serde(rename = "dateCreated", default)]
    pub date_created: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub pid: Pid,
    pub label: Multilingual,
    #[serde(rename = "accessRights")]
    pub access_rights: String,
    #[serde(rename = "legalInfo")]
    pub legal_info: RecordLegalInfo,
    #[serde(rename = "howToCite", default)]
    pub how_to_cite: String,
    #[serde(default)]
    pub publisher: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub description: Multilingual,
    #[serde(rename = "dateCreated", default)]
    pub date_created: String,
    #[serde(rename = "dateModified", default)]
    pub date_modified: String,
    #[serde(rename = "datePublished", default)]
    pub date_published: String,
    #[serde(rename = "typeOfData", default)]
    pub type_of_data: String,
    #[serde(default)]
    pub size: String,
    #[serde(default)]
    pub keywords: Vec<Multilingual>,
    /// The record's file (MIME type + direct download link), present only for bitstream records.
    #[serde(default)]
    pub file: Option<RecordFile>,
}

impl Record {
    // Returns the project-level ARK URL, e.g. `https://ark.dasch.swiss/ark:/72163/1/0803`.
    pub fn project_ark(&self) -> String {
        format!("{}/{}{}", self.pid.host, ARK_PATH_PREFIX, self.pid.shortcode)
    }
}

/// Returns the OAI-PMH datestamp for a record.
///
/// Priority: dateModified > datePublished > dateCreated > fallback "2015-01-01".
pub fn record_datestamp(record: &Record) -> String {
    for date in [&record.date_modified, &record.date_published, &record.date_created] {
        if !date.is_empty() {
            return date.clone();
        }
    }
    "2015-01-01".to_string()
}

#[cfg(test)]
mod tests {

    use super::*;

    fn first_0803_record() -> Record {
        let json = include_str!("../../../dpe/server/data/records_test/0803-records.json");
        let [record]: [Record; 1] = serde_json::from_str(json).expect("parse 0803-records.json");
        record
    }

    /// The one committed record carrying the full technical file metadata. The
    /// production exports still predate those fields, so this fixture is the only
    /// place the populated shape is exercised.
    fn first_0862_record() -> Record {
        let json = include_str!("../../../dpe/server/data/records_test/0862-records.json");
        let [record]: [Record; 1] = serde_json::from_str(json).expect("parse 0862-records.json");
        record
    }

    #[test]
    fn deserialise_0803_record() {
        let record = first_0803_record();

        assert_eq!(record.id, "http://rdfh.ch/0803/lklK7rVuVOmpBZYWrF8o-g");
        assert_eq!(
            record.pid,
            Pid {
                host: "https://ark.dasch.swiss".to_string(),
                shortcode: "0803".to_string(),
                record_id: "lklK7rVuVOmpBZYWrF8o=gh".to_string(),
            }
        );
        assert_eq!(
            record.label,
            Multilingual::from([("en".to_string(), "Seitenbezeichnung : o2r".to_string())])
        );
        assert_eq!(record.access_rights, "Full Open Access");
        assert_eq!(record.legal_info.license.license_identifier, "public domain");
        assert_eq!(record.legal_info.license.license_date, "2023-01-01");
        assert_eq!(record.legal_info.copyright_holder, "DaSCH");
        assert_eq!(record.legal_info.authorship, vec!["DaSCH"]);
        assert_eq!(record.publisher, "DaSCH");
        assert_eq!(record.type_of_data, "Text");
        assert_eq!(record.keywords, Vec::<Multilingual>::new());
        let file = record.file.expect("0803 record should carry a file");
        assert_eq!(file.mime_type.as_deref(), Some("application/pdf"));
        assert_eq!(
            file.url,
            "https://ingest.dasch.swiss/projects/0803/assets/lklK7rVuVOmpBZYWrF8o-g/original"
        );
    }

    #[test]
    fn datestamp_prefers_date_modified() {
        let mut record = first_0803_record();
        record.date_modified = "2024-06-30".to_string();
        assert_eq!(record_datestamp(&record), "2024-06-30");
    }

    #[test]
    fn datestamp_falls_back_to_date_published() {
        let mut record = first_0803_record();
        record.date_modified = String::new();
        record.date_published = "2024-02-01".to_string();
        assert_eq!(record_datestamp(&record), "2024-02-01");
    }

    #[test]
    fn datestamp_falls_back_to_date_created() {
        let mut record = first_0803_record();
        record.date_modified = String::new();
        record.date_published = String::new();
        record.date_created = "2024-01-15".to_string();
        assert_eq!(record_datestamp(&record), "2024-01-15");
    }

    #[test]
    fn project_ark_extracted_from_pid() {
        let record = first_0803_record();
        assert_eq!(record.project_ark(), "https://ark.dasch.swiss/ark:/72163/1/0803");
    }

    #[test]
    fn pid_parse_fails_without_record_id_segment() {
        assert!(parse_pid("https://ark.dasch.swiss/ark:/72163/1/record-0001").is_err());
    }

    #[test]
    fn datestamp_falls_back_to_hardcoded_default() {
        let mut record = first_0803_record();
        record.date_modified = String::new();
        record.date_published = String::new();
        record.date_created = String::new();
        assert_eq!(record_datestamp(&record), "2015-01-01");
    }

    #[test]
    fn file_with_full_technical_metadata_deserialises() {
        let record = first_0862_record();
        let file = record.file.expect("the 0862 fixture carries a file");

        assert_eq!(file.mime_type.as_deref(), Some("image/png"));
        assert_eq!(
            file.checksum.as_deref(),
            Some("9ab438922efe5c31f0a862e10891789d6934685bb6d146afc8a3c67c54e622c9")
        );
        assert_eq!(file.checksum_algorithm.as_deref(), Some("SHA-256"));
        assert_eq!(file.file_name.as_deref(), Some("Screenshot 2026-08-19 at 16.40.02.png"));
        assert_eq!(file.file_size, Some(377685));
        assert_eq!(file.date_created.as_deref(), Some("2026-08-25T10:25:33.455394630Z"));
    }

    /// Delete this once the exports are regenerated and the fields are required.
    #[test]
    fn file_without_technical_metadata_still_deserialises() {
        let json = r#"{
            "mimeType": "application/pdf",
            "url": "https://ingest.dasch.swiss/projects/0803/assets/lklK7rVuVOmpBZYWrF8o-g/original"
        }"#;
        let file: RecordFile = serde_json::from_str(json).expect("parse minimal file object");

        assert_eq!(file.mime_type.as_deref(), Some("application/pdf"));
        assert_eq!(file.checksum, None);
        assert_eq!(file.checksum_algorithm, None);
        assert_eq!(file.file_name, None);
        assert_eq!(file.file_size, None);
        assert_eq!(file.date_created, None);
    }

    /// The production 0803 export shape: every technical field but `mimeType`.
    /// A missing `mimeType` must not fail the record — before it was optional a
    /// single such file made `serde_json` reject the whole dump, and the loader
    /// silently served zero records for the project.
    #[test]
    fn file_without_a_mime_type_deserialises() {
        let json = r#"{
            "url": "https://ingest.dasch.swiss/projects/0803/assets/2vbIabBOEvq-EU9jwmgEe9j/original",
            "checksum": "91781797997988c3859ea6c9e756587482c19b3b7ea1da10da5ffa9853eb82c2",
            "checksumAlgorithm": "SHA-256",
            "fileName": "ad+s167_druck1=0001.tif",
            "fileSize": 28947082,
            "dateCreated": "2011-04-14T07:15:28Z"
        }"#;
        let file: RecordFile = serde_json::from_str(json).expect("parse file object without mimeType");

        assert_eq!(file.mime_type, None);
        assert_eq!(file.file_name.as_deref(), Some("ad+s167_druck1=0001.tif"));
        assert_eq!(file.file_size, Some(28947082));
    }

    /// A whole dump survives one file lacking `mimeType` — the regression that
    /// made project 0803 invisible.
    #[test]
    fn a_record_list_survives_a_file_without_a_mime_type() {
        let json = r#"[{
            "id": "http://rdfh.ch/0803/_qXHiyf6WsOfLFEXr7Zdng",
            "pid": "https://ark.dasch.swiss/ark:/72163/1/0803/_qXHiyf6WsOfLFEXr7Zdng7",
            "label": { "en": "a1r, Titelblatt" },
            "accessRights": "Full Open Access",
            "legalInfo": {
                "license": { "licenseIdentifier": "public domain", "licenseDate": "2023-01-01",
                             "licenseURI": "https://creativecommons.org/publicdomain/zero/1.0/" },
                "copyrightHolder": "DaSCH",
                "authorship": ["DaSCH"]
            },
            "howToCite": "a1r, Titelblatt",
            "publisher": "DaSCH",
            "dateCreated": "2011-04-14T07:15:28Z",
            "datePublished": "2011-04-14T07:15:28Z",
            "typeOfData": "Text",
            "keywords": [],
            "file": {
                "url": "https://ingest.dasch.swiss/projects/0803/assets/2vbIabBOEvq-EU9jwmgEe9j/original",
                "fileName": "ad+s167_druck1=0001.tif"
            }
        }]"#;
        let records: Vec<Record> = serde_json::from_str(json).expect("parse dump with a mimeType-less file");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].file.as_ref().unwrap().mime_type, None);
    }
}
