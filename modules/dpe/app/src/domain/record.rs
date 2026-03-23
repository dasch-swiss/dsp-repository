//! Record domain model.
//!
//! Represents an individual data Record within a Research Project,
//! as defined in the DaSCH Metadata 2.0 schema.

/// ARK path prefix for DaSCH resources.
///
/// `72163` is DaSCH's NAAN (Name Assigning Authority Number).
pub const ARK_PATH_PREFIX: &str = "ark:/72163/1/";

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub pid: String,
    pub label: HashMap<String, String>,
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
    pub description: HashMap<String, String>,
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
    pub keywords: Vec<HashMap<String, String>>,
}

impl Record {
    // `https://ark.dasch.swiss/ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh` → `https://ark.dasch.swiss/ark:/72163/1/0803`
    // Returns `None` if the PID does not contain a two-segment ARK path.
    pub fn project_ark(&self) -> Option<String> {
        let ark_start = self.pid.find(ARK_PATH_PREFIX)?;
        let after_prefix = &self.pid[ark_start + ARK_PATH_PREFIX.len()..];
        // after_prefix is "{shortcode}/{record_id}" — need at least one slash
        let slash = after_prefix.find('/')?;
        let shortcode = &after_prefix[..slash];
        if shortcode.is_empty() {
            return None;
        }
        Some(format!("https://ark.dasch.swiss/{}{}", ARK_PATH_PREFIX, shortcode))
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
    use std::collections::HashMap;

    use super::*;

    fn first_0803_record() -> Record {
        let json = include_str!("../../../server/data/records/0803-records.json");
        let [record]: [Record; 1] = serde_json::from_str(json).expect("parse 0803-records.json");
        record
    }

    #[test]
    fn deserialise_0803_record() {
        let record = first_0803_record();

        assert_eq!(record.id, "http://rdfh.ch/0803/lklK7rVuVOmpBZYWrF8o-g");
        assert_eq!(record.pid, "https://ark.dasch.swiss/ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh");
        assert_eq!(record.label, HashMap::from([("en".to_string(), "Seitenbezeichnung : o2r".to_string())]));
        assert_eq!(record.access_rights, "Full Open Access");
        assert_eq!(record.legal_info.license.license_identifier, "public domain");
        assert_eq!(record.legal_info.license.license_date, "2023-01-01");
        assert_eq!(record.legal_info.copyright_holder, "DaSCH");
        assert_eq!(record.legal_info.authorship, vec!["DaSCH"]);
        assert_eq!(record.publisher, "DaSCH");
        assert_eq!(record.type_of_data, "Text");
        assert_eq!(record.keywords, Vec::<HashMap<String, String>>::new());
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
    fn project_ark_extracted_from_two_segment_pid() {
        let record = first_0803_record();
        assert_eq!(
            record.project_ark(),
            Some("https://ark.dasch.swiss/ark:/72163/1/0803".to_string())
        );
    }

    #[test]
    fn project_ark_is_none_for_single_segment_pid() {
        let mut record = first_0803_record();
        record.pid = "https://ark.dasch.swiss/ark:/72163/1/record-0001".to_string();
        assert_eq!(record.project_ark(), None);
    }

    #[test]
    fn datestamp_falls_back_to_hardcoded_default() {
        let mut record = first_0803_record();
        record.date_modified = String::new();
        record.date_published = String::new();
        record.date_created = String::new();
        assert_eq!(record_datestamp(&record), "2015-01-01");
    }
}
