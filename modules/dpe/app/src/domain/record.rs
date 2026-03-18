//! Record domain model.
//!
//! Represents an individual data Record within a Research Project,
//! as defined in the DaSCH Metadata 2.0 schema.

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

    fn a_json() -> &'static str {
        r#"{
            "id": "record-0001",
            "pid": "https://ark.dasch.swiss/ark:/72163/1/record-0001",
            "label": { "en": "Survey Responses on Rural Land Use, 1920–1950", "de": "Umfrageantworten zur ländlichen Landnutzung, 1920–1950" },
            "accessRights": "Full Open Access",
            "legalInfo": {
                "license": {
                    "licenseIdentifier": "CC-BY-4.0",
                    "licenseDate": "2024-01-15",
                    "licenseURI": "https://creativecommons.org/licenses/by/4.0/"
                },
                "copyrightHolder": "University of Basel",
                "authorship": ["Dr. Anna Müller", "Prof. Hans Bauer"]
            },
            "howToCite": "Müller, A. & Bauer, H. (2024). Survey Responses on Rural Land Use, 1920–1950 [Data record]. DaSCH. https://ark.dasch.swiss/ark:/72163/1/record-0001",
            "publisher": "DaSCH",
            "source": "Swiss Federal Archives, Fond E7350, 1920–1950",
            "description": {
                "en": "A collection of survey responses documenting rural land use patterns across Swiss cantons between 1920 and 1950.",
                "de": "Eine Sammlung von Umfrageantworten zur Dokumentation ländlicher Landnutzungsmuster in Schweizer Kantonen zwischen 1920 und 1950."
            },
            "dateCreated": "2024-01-15",
            "dateModified": "2024-06-30",
            "datePublished": "2024-02-01",
            "typeOfData": "Text",
            "size": "2.3 GB",
            "keywords": [{ "en": "land use" }, { "en": "rural history" }, { "de": "Landwirtschaft" }, { "en": "Switzerland" }]
        }"#
    }

    #[test]
    fn deserialise_a_json() {
        let actual: Record = serde_json::from_str(a_json()).expect("deserialise");

        let expected = Record {
            id: "record-0001".to_string(),
            pid: "https://ark.dasch.swiss/ark:/72163/1/record-0001".to_string(),
            label: HashMap::from([
                ("en".to_string(), "Survey Responses on Rural Land Use, 1920–1950".to_string()),
                ("de".to_string(), "Umfrageantworten zur ländlichen Landnutzung, 1920–1950".to_string()),
            ]),
            access_rights: "Full Open Access".to_string(),
            legal_info: RecordLegalInfo {
                license: RecordLicense {
                    license_identifier: "CC-BY-4.0".to_string(),
                    license_date: "2024-01-15".to_string(),
                    license_uri: "https://creativecommons.org/licenses/by/4.0/".to_string(),
                },
                copyright_holder: "University of Basel".to_string(),
                authorship: vec!["Dr. Anna Müller".to_string(), "Prof. Hans Bauer".to_string()],
            },
            how_to_cite: "Müller, A. & Bauer, H. (2024). Survey Responses on Rural Land Use, 1920–1950 [Data record]. DaSCH. https://ark.dasch.swiss/ark:/72163/1/record-0001".to_string(),
            publisher: "DaSCH".to_string(),
            source: "Swiss Federal Archives, Fond E7350, 1920–1950".to_string(),
            description: HashMap::from([
                ("en".to_string(), "A collection of survey responses documenting rural land use patterns across Swiss cantons between 1920 and 1950.".to_string()),
                ("de".to_string(), "Eine Sammlung von Umfrageantworten zur Dokumentation ländlicher Landnutzungsmuster in Schweizer Kantonen zwischen 1920 und 1950.".to_string()),
            ]),
            date_created: "2024-01-15".to_string(),
            date_modified: "2024-06-30".to_string(),
            date_published: "2024-02-01".to_string(),
            type_of_data: "Text".to_string(),
            size: "2.3 GB".to_string(),
            keywords: vec![
                HashMap::from([("en".to_string(), "land use".to_string())]),
                HashMap::from([("en".to_string(), "rural history".to_string())]),
                HashMap::from([("de".to_string(), "Landwirtschaft".to_string())]),
                HashMap::from([("en".to_string(), "Switzerland".to_string())]),
            ],
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn datestamp_prefers_date_modified() {
        let mut record: Record = serde_json::from_str(a_json()).unwrap();
        record.date_modified = "2024-06-30".to_string();
        assert_eq!(record_datestamp(&record), "2024-06-30");
    }

    #[test]
    fn datestamp_falls_back_to_date_published() {
        let mut record: Record = serde_json::from_str(a_json()).unwrap();
        record.date_modified = String::new();
        assert_eq!(record_datestamp(&record), "2024-02-01");
    }

    #[test]
    fn datestamp_falls_back_to_date_created() {
        let mut record: Record = serde_json::from_str(a_json()).unwrap();
        record.date_modified = String::new();
        record.date_published = String::new();
        assert_eq!(record_datestamp(&record), "2024-01-15");
    }

    #[test]
    fn datestamp_falls_back_to_hardcoded_default() {
        let mut record: Record = serde_json::from_str(a_json()).unwrap();
        record.date_modified = String::new();
        record.date_published = String::new();
        record.date_created = String::new();
        assert_eq!(record_datestamp(&record), "2015-01-01");
    }
}
