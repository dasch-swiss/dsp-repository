//! The `dpe-server validate` subcommand.

pub(crate) fn validate(data_dir: std::path::PathBuf) -> std::process::ExitCode {
    let report = collect_validation_errors(&data_dir);

    println!(
        "Validated: {} projects, {} records, {} persons, {} organizations",
        report.project_count, report.record_count, report.person_count, report.org_count
    );

    if report.errors.is_empty() {
        println!("All data files are valid.");
        std::process::ExitCode::SUCCESS
    } else {
        println!("\n{} error(s) found:", report.errors.len());
        for err in &report.errors {
            println!("  - {err}");
        }
        std::process::ExitCode::FAILURE
    }
}

/// Counts and errors from [`collect_validation_errors`], separate from `validate` so the logic
/// is testable without an exit code.
struct ValidationReport {
    project_count: usize,
    record_count: usize,
    person_count: usize,
    org_count: usize,
    errors: Vec<String>,
}

fn collect_validation_errors(data_dir: &std::path::Path) -> ValidationReport {
    use std::fs;

    let mut errors: Vec<String> = Vec::new();
    let mut project_count = 0;
    let mut record_count = 0;
    let mut person_count = 0;
    let mut org_count = 0;

    let projects_dir = data_dir.join("projects");
    // Every contributor id every project references, cross-referenced against
    // `persons/` and `organizations/` once the whole corpus has been read.
    let mut contributor_refs: Vec<shared_metadata::ContributorRef> = Vec::new();
    // The same tables the OAI-PMH `every_committed_temporal_coverage_resolves` test loads, so
    // the two agree on what counts as resolved.
    let temporal_periods = shared_metadata::chronontology::load_from(data_dir);
    let temporal_enrichment = shared_metadata::temporal_enrichment::load_from(data_dir);
    // Each offending value is reported once per member for the whole corpus, against the first
    // file carrying it. A finding with no value is never folded away.
    let mut reported_values: std::collections::HashSet<(&str, String)> = std::collections::HashSet::new();
    if projects_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let filename = path.display().to_string();
                match fs::read_to_string(&path) {
                    Ok(json) => {
                        // Parsing is the one project rule the shared checker cannot
                        // hold: it takes a `&ProjectRaw`, so it is downstream of this.
                        match serde_json::from_str::<shared_metadata::ProjectRaw>(&json) {
                            Ok(raw) => {
                                project_count += 1;
                                contributor_refs.extend(shared_metadata::contributor_refs(&raw));

                                for finding in
                                    shared_metadata::check_project(&raw, &temporal_periods, &temporal_enrichment)
                                {
                                    if let Some(value) = &finding.value {
                                        if !reported_values.insert((finding.field, value.clone())) {
                                            continue;
                                        }
                                    }
                                    // The checker's message has no file prefix; this consumer adds
                                    // one.
                                    errors.push(format!("{filename}: {}", finding.message));
                                }
                            }
                            Err(e) => errors.push(format!("{filename}: {e}")),
                        }
                    }
                    Err(e) => errors.push(format!("{filename}: {e}")),
                }
            }
        }
    } else {
        errors.push(format!("projects directory not found: {}", projects_dir.display()));
    }

    let records_dir = data_dir.join("records");
    if records_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&records_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let filename = path.display().to_string();
                match fs::read_to_string(&path) {
                    Ok(json) => match serde_json::from_str::<Vec<shared_metadata::Record>>(&json) {
                        Ok(recs) => record_count += recs.len(),
                        Err(e) => errors.push(format!("{filename}: {e}")),
                    },
                    Err(e) => errors.push(format!("{filename}: {e}")),
                }
            }
        }
    }

    let persons_dir = data_dir.join("persons");
    let mut known_person_ids = std::collections::HashSet::new();
    if persons_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&persons_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let filename = path.display().to_string();
                match fs::read_to_string(&path) {
                    Ok(json) => match serde_json::from_str::<shared_metadata::Person>(&json) {
                        Ok(p) => {
                            // A role belongs in the project's attributions, not in jobTitles,
                            // or the OAI-PMH creator logic cannot see it.
                            for title in &p.job_titles {
                                if shared_metadata::is_role_job_title(title) {
                                    errors.push(format!(
                                        "{filename}: jobTitle '{title}' on {} is a project role; \
                                         move it to the project's attributions (contributorType)",
                                        p.id
                                    ));
                                }
                            }
                            known_person_ids.insert(p.id.clone());
                            person_count += 1;
                        }
                        Err(e) => errors.push(format!("{filename}: {e}")),
                    },
                    Err(e) => errors.push(format!("{filename}: {e}")),
                }
            }
        }
    }

    let orgs_dir = data_dir.join("organizations");
    let mut known_org_ids = std::collections::HashSet::new();
    if orgs_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&orgs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let filename = path.display().to_string();
                match fs::read_to_string(&path) {
                    Ok(json) => match serde_json::from_str::<shared_metadata::Organization>(&json) {
                        Ok(o) => {
                            known_org_ids.insert(o.id.clone());
                            org_count += 1;
                        }
                        Err(e) => errors.push(format!("{filename}: {e}")),
                    },
                    Err(e) => errors.push(format!("{filename}: {e}")),
                }
            }
        }
    }

    // Which ids a project references is the shared checker's question; what counts as known is
    // this corpus's (the editor answers it differently).
    for reference in &contributor_refs {
        let id = &reference.id;
        if !known_person_ids.contains(id) && !known_org_ids.contains(id) {
            errors.push(format!(
                "broken reference: contributor '{id}' not found in persons/ or organizations/"
            ));
        }
    }

    ValidationReport { project_count, record_count, person_count, org_count, errors }
}

#[cfg(test)]
mod validate_tests {
    use super::collect_validation_errors;

    /// A minimal `ProjectRaw`, valid except for its `temporalCoverage` and
    /// `attributions`, which the caller supplies as raw JSON array literals.
    fn project_json(temporal_coverage: &str, attributions: &str) -> String {
        format!(
            r#"{{
                "id": "0000", "pid": "MISSING", "name": "Test Project", "shortcode": "0000",
                "officialName": "Test Project", "status": "Finished", "shortDescription": "test",
                "description": {{}}, "startDate": "MISSING", "endDate": "MISSING",
                "howToCite": "test", "accessRights": {{ "accessRights": "Full Open Access" }},
                "legalInfo": [], "keywords": [], "disciplines": [],
                "temporalCoverage": {temporal_coverage}, "spatialCoverage": [], "attributions": {attributions},
                "funding": "No funding"
            }}"#
        )
    }

    /// Writes one project file (plus an optional enrichment table) to a fresh data dir and
    /// returns the validation errors.
    fn validate_with(temporal_coverage: &str, enrichment_json: Option<&str>) -> Vec<String> {
        // A per-call counter keeps the dirs distinct: tests run concurrently.
        static CALL_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let call_id = CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("dpe_validate_temporal_{}_{call_id}", std::process::id()));
        let projects_dir = dir.join("projects");
        std::fs::create_dir_all(&projects_dir).unwrap();
        std::fs::write(projects_dir.join("0000_test.json"), project_json(temporal_coverage, "[]")).unwrap();
        if let Some(enrichment) = enrichment_json {
            std::fs::write(dir.join("temporal-coverage-enrichment.json"), enrichment).unwrap();
        }

        let report = collect_validation_errors(&dir);
        std::fs::remove_dir_all(&dir).ok();
        report.errors
    }

    #[test]
    fn flags_temporal_coverage_with_no_resolved_date() {
        let errors = validate_with(r#"[{"en": "Mysterious Era"}]"#, None);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("Mysterious Era") && e.contains("no resolved date")),
            "expected an unresolved temporalCoverage error, got: {errors:?}"
        );
    }

    #[test]
    fn accepts_temporal_coverage_resolved_via_enrichment() {
        let errors = validate_with(
            r#"[{"en": "Early Christianity"}]"#,
            Some(
                r#"{"Early Christianity": {"date": "0030/0451", "original_name": "Early Christianity", "source": "llm"}}"#,
            ),
        );
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn accepts_temporal_coverage_explicitly_marked_unresolved() {
        let errors = validate_with(
            r#"[{"en": "Swiss"}]"#,
            Some(r#"{"Swiss": {"date": null, "original_name": "Swiss", "source": "unresolved"}}"#),
        );
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    /// A data directory assembled for one test.
    ///
    /// The tests below pin the exact wording of every error: `just validate-data` is read by a
    /// human, and `modules/dpe/CLAUDE.md` quotes the temporal-coverage message. The committed
    /// corpus is valid, so nothing else exercises these branches.
    struct Fixture {
        dir: std::path::PathBuf,
    }

    impl Fixture {
        /// A data dir holding an empty `projects/`, so a missing-directory error does not join
        /// every test's list.
        fn new() -> Self {
            let fixture = Self::bare();
            std::fs::create_dir_all(fixture.dir.join("projects")).unwrap();
            fixture
        }

        /// A data dir with nothing in it.
        fn bare() -> Self {
            // Same reasoning as `validate_with`.
            static CALL_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let call_id = CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("dpe_validate_wording_{}_{call_id}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            Self { dir }
        }

        /// Writes `contents` at `relative`, creating parent directories.
        fn with(self, relative: &str, contents: &str) -> Self {
            let path = self.dir.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
            self
        }

        /// The full path `collect_validation_errors` names in an error.
        fn path_of(&self, relative: &str) -> String {
            self.dir.join(relative).display().to_string()
        }

        fn report(&self) -> super::ValidationReport {
            collect_validation_errors(&self.dir)
        }

        fn errors(&self) -> Vec<String> {
            self.report().errors
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.dir).ok();
        }
    }

    /// A minimal `Record`. `pid` deserializes from the ARK URL string, not an
    /// object, so it cannot be assembled field by field.
    fn record_json(record_id: &str) -> String {
        format!(
            r#"{{
                "id": "http://rdfh.ch/0000/{record_id}",
                "pid": "https://ark.dasch.swiss/ark:/72163/1/0000/{record_id}",
                "label": {{ "en": "Record {record_id}" }},
                "accessRights": "Full Open Access",
                "legalInfo": {{
                    "license": {{ "licenseIdentifier": "public domain", "licenseDate": "2023-01-01",
                                  "licenseURI": "https://creativecommons.org/publicdomain/zero/1.0/" }},
                    "copyrightHolder": "DaSCH",
                    "authorship": ["DaSCH"]
                }}
            }}"#
        )
    }

    /// A person, valid except for the `jobTitles` the caller supplies.
    fn person_json(id: &str, job_titles: &str) -> String {
        format!(
            r#"{{
                "id": "{id}", "givenNames": ["Ada"], "familyNames": ["Lovelace"],
                "jobTitles": {job_titles}
            }}"#
        )
    }

    #[test]
    fn unresolved_temporal_coverage_message_is_unchanged() {
        let fixture =
            Fixture::new().with("projects/0000_test.json", &project_json(r#"[{"en": "Mysterious Era"}]"#, "[]"));
        assert_eq!(
            fixture.errors(),
            vec![format!(
                "{}: temporalCoverage 'Mysterious Era' has no resolved date \
                 (add a W3CDTF range to temporal-coverage-enrichment.json, \
                 or mark source=\"unresolved\" if not a time period)",
                fixture.path_of("projects/0000_test.json")
            )]
        );
    }

    #[test]
    fn role_job_title_message_is_unchanged() {
        let fixture = Fixture::new().with("persons/ada.json", &person_json("ada", r#"["Project Leader"]"#));
        assert_eq!(
            fixture.errors(),
            vec![format!(
                "{}: jobTitle 'Project Leader' on ada is a project role; \
                 move it to the project's attributions (contributorType)",
                fixture.path_of("persons/ada.json")
            )]
        );
    }

    #[test]
    fn broken_contributor_reference_message_is_unchanged() {
        let fixture = Fixture::new().with(
            "projects/0000_test.json",
            &project_json("[]", r#"[{"contributor": "ghost", "contributorType": ["Project Leader"]}]"#),
        );
        assert_eq!(
            fixture.errors(),
            vec!["broken reference: contributor 'ghost' not found in persons/ or organizations/".to_string()]
        );
    }

    #[test]
    fn a_contributor_resolves_against_organizations_as_well_as_persons() {
        let fixture = Fixture::new()
            .with(
                "projects/0000_test.json",
                &project_json("[]", r#"[{"contributor": "unibas", "contributorType": ["Project Leader"]}]"#),
            )
            .with(
                "organizations/unibas.json",
                r#"{"id": "unibas", "name": "University of Basel", "url": "https://unibas.ch"}"#,
            );
        assert!(fixture.errors().is_empty(), "expected no errors, got: {:?}", fixture.errors());
    }

    #[test]
    fn reports_a_missing_projects_directory() {
        let fixture = Fixture::bare();
        assert_eq!(
            fixture.errors(),
            vec![format!("projects directory not found: {}", fixture.path_of("projects"))]
        );
    }

    #[test]
    fn a_malformed_file_is_reported_against_its_own_path() {
        let fixture = Fixture::new()
            .with("projects/0000_test.json", "{ not json")
            .with("records/0000.json", "{ not json")
            .with("persons/ada.json", "{ not json")
            .with("organizations/unibas.json", "{ not json");
        let errors = fixture.errors();
        assert_eq!(errors.len(), 4, "expected one error per malformed file, got: {errors:?}");
        // The `{path}: ` prefix is ours and asserted; serde's wording is not.
        for relative in [
            "projects/0000_test.json",
            "records/0000.json",
            "persons/ada.json",
            "organizations/unibas.json",
        ] {
            let prefix = format!("{}: ", fixture.path_of(relative));
            assert!(
                errors.iter().any(|e| e.starts_with(&prefix)),
                "expected an error prefixed {prefix:?}, got: {errors:?}"
            );
        }
    }

    #[test]
    fn a_name_resolved_in_one_project_no_longer_masks_a_gap_in_another() {
        // "Trajanic" resolves in one project and is a real gap in the other: the gap is
        // reported once, whichever file is read first.
        let fixture = Fixture::new()
            .with(
                "projects/0000_resolves.json",
                &project_json(
                    r#"[{"type": "Chronontology",
                         "url": "https://chronontology.dainst.org/period/0vGXxVln724L",
                         "text": "Trajanic"}]"#,
                    "[]",
                ),
            )
            .with("projects/0001_gap.json", &project_json(r#"[{"en": "Trajanic"}]"#, "[]"))
            .with(
                "chronontology-periods.json",
                r#"{"0vGXxVln724L": {"hasTimespan": [{"begin": {"at": "98"}, "end": {"at": "117"}}]}}"#,
            );
        let errors = fixture.errors();
        assert_eq!(errors.len(), 1, "expected the gap reported exactly once, got: {errors:?}");
        assert_eq!(
            errors[0],
            format!(
                "{}: temporalCoverage 'Trajanic' has no resolved date \
                 (add a W3CDTF range to temporal-coverage-enrichment.json, \
                 or mark source=\"unresolved\" if not a time period)",
                fixture.path_of("projects/0001_gap.json")
            )
        );
    }

    #[test]
    fn counts_feed_the_validated_summary_line() {
        let fixture = Fixture::new()
            .with(
                "projects/0000_test.json",
                &project_json("[]", r#"[{"contributor": "ada", "contributorType": ["Project Leader"]}]"#),
            )
            .with("persons/ada.json", &person_json("ada", "[]"))
            .with(
                "organizations/unibas.json",
                r#"{"id": "unibas", "name": "University of Basel", "url": "https://unibas.ch"}"#,
            )
            // Records are counted per entry, not per file.
            .with("records/0000.json", "[]")
            .with(
                "records/0001.json",
                &format!("[{}, {}]", record_json("one"), record_json("two")),
            );
        let report = fixture.report();
        assert!(report.errors.is_empty(), "expected no errors, got: {:?}", report.errors);
        assert_eq!(report.project_count, 1);
        assert_eq!(report.record_count, 2);
        assert_eq!(report.person_count, 1);
        assert_eq!(report.org_count, 1);
    }
}
