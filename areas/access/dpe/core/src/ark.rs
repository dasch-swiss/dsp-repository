//! ARK-host normalisation, applied as corpus data enters the caches.
//!
//! # Where this belongs, and where it is going
//!
//! Conceptually this is the **`sync` capability's** work. `sync` is the Access
//! Area's single writer of the archive projection, and the root `CONTEXT.md`
//! records that DPE, CPE and the SPARQL endpoint all read through its ports.
//! Normalising an identifier as data enters the projection is data
//! normalisation at ingress, not a concern of any reader. `sync` does not exist
//! yet, and `dpe-core`'s corpus caches are today's stand-in for it — they are
//! the one place every consumer reads through. **When `sync` lands, this module
//! moves there**, and nothing downstream has to change, because nothing
//! downstream knows it exists.
//!
//! That is the test of the placement and it is worth stating: the project
//! sidebar's permalink, the JSON API's `pid`, both graph builders, every
//! `shared-fair` writer and the OAI payloads are all correct without any of
//! them being told about a resolver. `shared-fair` in particular reads the
//! graph it is handed and nothing more — which is the property the plan's
//! *DPE's adapter is the seam for the capability split* paragraph claims, in
//! the words "`shared-fair` does not change".
//!
//! # What it does
//!
//! Unset — the default, and what production, DEV and STAGE run — nothing is
//! normalised and every ARK is the one the corpus records. Set, on
//! [`crate::CorpusSettings::ark_resolver_base_url`], every ARK entering the
//! caches carries the configured host instead, so a deployment that is not the
//! one the recorded ARK resolves to does not publish an identifier that sends
//! a reader somewhere else. Only the host: the ARK path is the identifier.

use shared_metadata::{ProjectRaw, Record};

/// Normalises one project's recorded PID.
///
/// `pub` so a consumer's test can sweep its own rendered output through the
/// real rule rather than a copy of it — `dpe-server`'s `served_bytes` does.
/// Production callers are the two cache loaders in this crate and nothing else.
///
/// A value that is not an ARK — a placeholder — is left alone rather than
/// prefixed with a host it never had, and every other recorded field is
/// untouched. In particular the project's `url` keeps what the corpus records
/// even when that is an ARK (083D records its own): a website is data the
/// corpus states, not an identifier this deployment asserts, and rewriting it
/// would invent a fact (ADR-0005, *Nothing is invented for a score*).
pub fn normalise_project(raw: &mut ProjectRaw, host: Option<&str>) {
    if let Some(host) = host {
        raw.pid = shared_metadata::with_ark_host(&raw.pid, host);
    }
}

/// Normalises one record's PID.
///
/// `Pid` is already parsed into host, shortcode and record id, so this replaces
/// a field rather than rewriting a string — the shortcode and record id, which
/// are the identifier, cannot be touched by it. `Record::project_ark` and
/// `Pid::as_url` both read `host`, so every ARK derived from this record
/// follows.
pub fn normalise_record(record: &mut Record, host: Option<&str>) {
    if let Some(host) = host {
        record.pid.host = host.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREVIEW: &str = "https://preview.example.test";

    /// A real committed project, because `ProjectRaw` has required fields and a
    /// hand-built fixture only proves what the fixture happens to carry.
    /// Project-level normalisation over the whole corpus is
    /// `project_cache::ingress_tests`; these cover the rules that corpus cannot
    /// show, starting with a placeholder PID, which no committed project has.
    fn committed_project() -> ProjectRaw {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../server/data/projects/0803_incunabula.json");
        serde_json::from_str(&std::fs::read_to_string(path).expect("readable")).expect("parses")
    }

    fn record(host: &str, record_id: &str) -> Record {
        serde_json::from_value(serde_json::json!({
            "id": record_id,
            "pid": format!("{host}/ark:/72163/1/0803/{record_id}"),
            "label": { "en": "A Record" },
            "accessRights": "Full Open Access",
            "legalInfo": {
                "license": { "licenseIdentifier": "", "licenseDate": "", "licenseURI": "" },
                "copyrightHolder": "",
                "authorship": [],
            },
        }))
        .expect("the fixture should parse")
    }

    #[test]
    fn no_configured_host_changes_anything() {
        let mut raw = committed_project();
        let recorded = raw.pid.clone();
        normalise_project(&mut raw, None);
        assert_eq!(raw.pid, recorded);

        let mut rec = record("https://ark.dasch.swiss", "rec-1");
        normalise_record(&mut rec, None);
        assert_eq!(rec.pid.as_url(), "https://ark.dasch.swiss/ark:/72163/1/0803/rec-1");
    }

    #[test]
    fn a_configured_host_replaces_a_record_host_and_nothing_else() {
        let mut rec = record("https://ark.dasch.swiss", "lklK7rVuVOmpBZYWrF8o=gh");
        normalise_record(&mut rec, Some(PREVIEW));
        // The shortcode and record id are the identifier and are separate
        // fields, so replacing the host cannot touch them.
        assert_eq!(
            rec.pid.as_url(),
            "https://preview.example.test/ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"
        );
        assert_eq!(rec.pid.shortcode, "0803");
        assert_eq!(rec.pid.record_id, "lklK7rVuVOmpBZYWrF8o=gh");
        // Every ARK a record derives reads the same field, so the project ARK
        // follows without a second rule.
        assert_eq!(rec.project_ark(), "https://preview.example.test/ark:/72163/1/0803");
    }

    #[test]
    fn a_placeholder_pid_is_not_turned_into_something_resolvable() {
        // No committed project has one, so this is the case the corpus test
        // cannot reach: a placeholder is not an ARK and must not be given a
        // host it never had.
        for pid in ["MISSING", "CALCULATED", ""] {
            let mut raw = ProjectRaw { pid: pid.to_string(), ..committed_project() };
            normalise_project(&mut raw, Some(PREVIEW));
            assert_eq!(raw.pid, pid, "{pid:?}");
        }
    }

    /// The normalisation rule proven above wired end to end: a corpus built
    /// with `ark_resolver_base_url: Some(host)` normalises both a project and
    /// a record on load, and one built with `None` leaves both as committed.
    /// Two corpora over the same fixture directory, differing only in that
    /// setting, both leaked in this test — impossible against the setter this
    /// replaces, which was first-call-wins for the whole process.
    #[test]
    fn a_corpus_settings_host_controls_normalisation_on_load() {
        use crate::corpus::{Corpus, CorpusSettings};

        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("projects")).unwrap();
        std::fs::create_dir_all(dir.path().join("records")).unwrap();
        std::fs::write(
            dir.path().join("projects/0803.json"),
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../server/data/projects/0803_incunabula.json"
            ))
            .expect("committed fixture is readable"),
        )
        .unwrap();
        // Written as the wire format directly, not round-tripped through
        // `Record`: its `pid` deserializes from a string but serializes as a
        // struct, so a `Record` built in memory cannot be written back out
        // and reread.
        std::fs::write(
            dir.path().join("records/0803-records.json"),
            serde_json::to_string(&serde_json::json!([{
                "id": "rec-1",
                "pid": "https://ark.dasch.swiss/ark:/72163/1/0803/rec-1",
                "label": { "en": "A Record" },
                "accessRights": "Full Open Access",
                "legalInfo": {
                    "license": { "licenseIdentifier": "", "licenseDate": "", "licenseURI": "" },
                    "copyrightHolder": "",
                    "authorship": [],
                },
            }]))
            .unwrap(),
        )
        .unwrap();
        // The loader also tries to warm three hardcoded shortcodes over the
        // network when they are not already on disk; pre-seeding them empty
        // keeps this test offline.
        for shortcode in ["0868", "081C"] {
            std::fs::write(dir.path().join(format!("records/{shortcode}-records.json")), "[]").unwrap();
        }

        let configured: &'static Corpus = Box::leak(Box::new(Corpus::new(CorpusSettings {
            data_dir: dir.path().to_string_lossy().into_owned(),
            public_dir: dir.path().to_string_lossy().into_owned(),
            ark_resolver_base_url: Some(PREVIEW.to_string()),
        })));
        let unset: &'static Corpus = Box::leak(Box::new(Corpus::new(CorpusSettings {
            data_dir: dir.path().to_string_lossy().into_owned(),
            public_dir: dir.path().to_string_lossy().into_owned(),
            ark_resolver_base_url: None,
        })));

        assert!(configured.all_projects_raw()[0].pid.starts_with(PREVIEW));
        assert!(unset.all_projects_raw()[0].pid.starts_with("https://ark.dasch.swiss/"));

        assert!(configured.all_records()[0].pid.as_url().starts_with(PREVIEW));
        assert!(unset.all_records()[0].pid.as_url().starts_with("https://ark.dasch.swiss/"));
    }
}
