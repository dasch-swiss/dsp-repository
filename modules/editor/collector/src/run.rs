//! The per-record loop: the skip rule, the force-push guard, and the report.
//!
//! Collection state is derived from GitHub and keyed on the project's
//! shortcode, never on the record id. A record id identifies a row the editor
//! may legitimately replace, while the conflict being avoided is per project
//! file, so keying on the record would let a superseded record's pull request
//! go unseen and a second one open against the same file.

use std::collections::BTreeSet;
use std::path::Path;

use editor_core::agents::Agents;
use editor_core::canonical::{write_entity, write_project};
use editor_core::collection::{ApprovedRecordView, CollectionReport, ProposedEntityView};
use editor_core::proposals::{entity_id_number, ProposalKind};
use editor_core::records::PullRequestState;
use serde_json::Value;
use shared_metadata::project::{is_valid_shortcode, ProjectRaw};

use crate::config::Config;
use crate::editor::Editor;
use crate::enrichment::{self, ENRICHMENT_FILE};
use crate::forge::{branch_for, CollectionPullRequest, Forge, COLLECTED_BY_TRAILER};
use crate::manifest::{self, MANIFEST_FILE};
use crate::{layout, renumber};

/// What one run did, for the operator reading the job log.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub considered: usize,
    pub published: usize,
    pub skipped: usize,
    pub failed: usize,
    /// Records whose outcome could not be reported back. The work landed; the
    /// editor's picture of it did not, so a run with any of these exits
    /// non-zero even when every record published.
    pub unreported: usize,
}

/// What happened to one record.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// Published, or force-pushed onto a pull request that was already open.
    Published {
        url: String,
    },
    /// Not acted on, but its pull request's state is worth re-reporting.
    Inspected {
        url: String,
        state: PullRequestState,
    },
    Failed {
        reason: String,
    },
    /// Nothing to say: no pull request has ever been opened for this project.
    Silent,
}

/// Fetch, publish and report.
pub fn collect(config: &Config, editor: &Editor, forge: &dyn Forge) -> Result<Summary, String> {
    let records = editor.approved_records()?;
    let mut summary = Summary::default();
    let mut pull_requests = forge.collection_pull_requests()?;

    for record in &records {
        let outcome =
            collect_one(config, forge, record, &pull_requests).unwrap_or_else(|reason| Outcome::Failed { reason });
        finish(editor, record, &outcome, &mut summary);

        // Re-derived after each record, not read once per run: the pull request
        // this record just opened claims entity ids the next record must not
        // allocate again, and another run may have opened one meanwhile.
        pull_requests = forge.collection_pull_requests()?;
    }
    Ok(summary)
}

/// Re-report pull request states, writing nothing.
///
/// A pull request merged with reviewer edits after the last collect leaves its
/// record reading `open`, with nobody prompted to re-run; this is what
/// eventually classifies it.
pub fn refresh(editor: &Editor, forge: &dyn Forge) -> Result<Summary, String> {
    let records = editor.approved_records()?;
    let pull_requests = forge.collection_pull_requests()?;
    let mut summary = Summary::default();

    for record in &records {
        let branch = branch_for(&record.shortcode);
        let outcome = match newest_on(&pull_requests, &branch) {
            Some(pull_request) => Outcome::Inspected { url: pull_request.url.clone(), state: pull_request.state },
            None => Outcome::Silent,
        };
        finish(editor, record, &outcome, &mut summary);
    }
    Ok(summary)
}

fn collect_one(
    config: &Config,
    forge: &dyn Forge,
    record: &ApprovedRecordView,
    pull_requests: &[CollectionPullRequest],
) -> Result<Outcome, String> {
    // The shortcode arrives over the wire and becomes both a git ref and a
    // path under the data directory, so it is checked before either is built.
    if !is_valid_shortcode(&record.shortcode) {
        return Ok(Outcome::Failed {
            reason: format!("{:?} is not a usable shortcode", record.shortcode),
        });
    }

    let branch = branch_for(&record.shortcode);
    let on_branch: Vec<&CollectionPullRequest> = pull_requests
        .iter()
        .filter(|pull_request| pull_request.head_ref == branch)
        .collect();

    // Merged: the change has landed, and reconciliation discards the record
    // when a release carries it. Its state is still reported.
    if let Some(merged) = on_branch
        .iter()
        .find(|pull_request| pull_request.state == PullRequestState::Merged)
    {
        return Ok(Outcome::Inspected { url: merged.url.clone(), state: PullRequestState::Merged });
    }

    let Some(project) = record.project.as_ref() else {
        return Ok(Outcome::Failed {
            reason: record
                .problem
                .clone()
                .unwrap_or_else(|| "the record carries no publishable project".to_string()),
        });
    };

    // Before anything is written: a reviewer's fixups on this branch must not be
    // silently overwritten by the next trigger.
    if let Some(message) = forge.remote_tip_message(&branch)? {
        if !message.contains(COLLECTED_BY_TRAILER) {
            return Ok(Outcome::Failed {
                reason: format!(
                    "{branch} has a tip commit this collector did not make; refusing to force-push over it"
                ),
            });
        }
    }

    // Every entity is checked before any is written. A record failing halfway
    // would otherwise leave an untracked file behind — `git checkout -B` does
    // not remove one — and the next record's manifest would count it.
    if let Err(reason) = check_entities(&record.entities) {
        return Ok(Outcome::Failed { reason });
    }

    forge.start_branch(&branch, &config.data_dir)?;

    let taken = taken_ids(&config.data_dir, pull_requests, &branch)?;
    let mapping = renumber::plan(&record.entities, &taken);
    let mut written = Vec::new();

    for entity in &record.entities {
        let id = mapping.get(&entity.id).unwrap_or(&entity.id).clone();
        let mut body = entity.body.clone();
        renumber::rewrite(&mut body, &mapping);
        let Value::Object(members) = &mut body else {
            unreachable!("check_entities rejected every non-object body")
        };
        // The filename stem must equal the stored id, so the renumbered id is
        // written into the body rather than left as the editor sent it.
        members.insert("id".to_string(), Value::String(id.clone()));

        let path = layout::entity_path(&config.data_dir, &entity.kind, &id);
        let rendered = write_entity(&body).map_err(|error| format!("could not write {id}: {error}"))?;
        write_file(&path, &rendered)?;
        written.push(path);
    }

    let mut value =
        serde_json::to_value(project).map_err(|error| format!("could not read the published project: {error}"))?;
    renumber::rewrite(&mut value, &mapping);
    let project: ProjectRaw = serde_json::from_value(value)
        .map_err(|error| format!("renumbering produced an unreadable project: {error}"))?;

    let path = layout::project_path(&config.data_dir, &record.shortcode, &project.name);
    let rendered = write_project(&project).map_err(|error| format!("could not write {}: {error}", record.shortcode))?;
    write_file(&path, &rendered)?;
    written.push(path);

    let rows = enrichment::add_skeleton_rows(&config.data_dir, &project)?;
    if !rows.is_empty() {
        written.push(config.data_dir.join(ENRICHMENT_FILE));
    }
    manifest::rewrite(&config.data_dir)?;
    written.push(config.data_dir.join(MANIFEST_FILE));

    forge.commit(&written, &commit_message(record, &mapping, &rows))?;
    forge.force_push(&branch)?;

    // An open pull request now carries the newest approved state. Opening a
    // second would orphan the first, with no record referencing it.
    if let Some(open) = on_branch
        .iter()
        .find(|pull_request| pull_request.state == PullRequestState::Open)
    {
        return Ok(Outcome::Published { url: open.url.clone() });
    }

    let title = subject(record);
    let url = forge.open_pull_request(&branch, &title, &pull_request_body(record, &mapping, &rows))?;
    Ok(Outcome::Published { url })
}

/// Whether every entity in a record can be written, checked in one pass.
///
/// `kind` and `id` both become a path under the data directory, so both are
/// held to the id grammar before one is built: `entity_id_number` accepts only
/// `{kind}-{number}`, which can neither traverse nor be absolute.
fn check_entities(entities: &[ProposedEntityView]) -> Result<(), String> {
    for entity in entities {
        let Ok(kind) = entity.kind.parse::<ProposalKind>() else {
            return Err(format!("{:?} is not an entity kind", entity.kind));
        };
        if entity_id_number(kind, &entity.id).is_none() {
            return Err(format!("{:?} is not a usable {kind} id", entity.id));
        }
        if !entity.body.is_object() {
            return Err(format!("the body of {} is not a JSON object", entity.id));
        }
    }
    Ok(())
}

/// Every entity id a proposed one may not collide with: the published set, plus
/// the ids held by sibling pull requests that are still open.
///
/// A published entity file that will not load is an error, not an empty slot.
/// [`Agents`] reports such a file and carries on without it, which would leave
/// its id looking free and let an allocation overwrite a committed entity.
fn taken_ids(
    data_dir: &Path,
    pull_requests: &[CollectionPullRequest],
    own_branch: &str,
) -> Result<BTreeSet<String>, String> {
    let (published, errors) = Agents::load_from(&data_dir.join("persons"), &data_dir.join("organizations"));
    if !errors.is_empty() {
        let reported: Vec<String> = errors.iter().map(ToString::to_string).collect();
        return Err(format!(
            "the published entity set did not load cleanly, so a proposed id cannot be checked against it: {}",
            reported.join("; ")
        ));
    }
    let mut taken: BTreeSet<String> = published.all().map(|agent| agent.id.clone()).collect();

    for pull_request in pull_requests {
        // This record's own branch is about to be force-pushed over, so the ids
        // it holds are this record's to reuse. A merged pull request's files are
        // already in the published set above.
        if pull_request.head_ref == own_branch || pull_request.state != PullRequestState::Open {
            continue;
        }
        taken.extend(pull_request.files.iter().filter_map(|path| entity_id_at(path)));
    }
    Ok(taken)
}

/// The entity id a changed path claims, or `None` for a path that is not an
/// entity file.
fn entity_id_at(path: &str) -> Option<String> {
    let (directory, file) = path.rsplit_once('/')?;
    if !directory.ends_with("persons") && !directory.ends_with("organizations") {
        return None;
    }
    Some(file.strip_suffix(".json")?.to_string())
}

/// The pull request whose state is worth reporting for one branch: a merged one
/// first, then an open one, then the newest closed one.
fn newest_on<'a>(pull_requests: &'a [CollectionPullRequest], branch: &str) -> Option<&'a CollectionPullRequest> {
    let on_branch: Vec<&CollectionPullRequest> = pull_requests
        .iter()
        .filter(|pull_request| pull_request.head_ref == branch)
        .collect();

    on_branch
        .iter()
        .find(|pull_request| pull_request.state == PullRequestState::Merged)
        .or_else(|| {
            on_branch
                .iter()
                .find(|pull_request| pull_request.state == PullRequestState::Open)
        })
        .or_else(|| on_branch.iter().max_by_key(|pull_request| pull_request.number))
        .copied()
}

/// Sends the report and counts the outcome.
///
/// A failing report never stops the run: the remaining records still deserve
/// their own attempt, and one that already published must not be published
/// twice.
fn finish(editor: &Editor, record: &ApprovedRecordView, outcome: &Outcome, summary: &mut Summary) {
    summary.considered += 1;
    match outcome {
        Outcome::Published { .. } => summary.published += 1,
        Outcome::Inspected { .. } => summary.skipped += 1,
        Outcome::Failed { reason } => {
            summary.failed += 1;
            eprintln!("{}: {reason}", record.shortcode);
        }
        Outcome::Silent => summary.skipped += 1,
    }

    let Some(report) = report_for(record, outcome) else {
        return;
    };
    if let Err(error) = editor.report(&report) {
        summary.unreported += 1;
        eprintln!("{}: could not report: {error}", record.shortcode);
    }
}

/// The report one outcome produces, or `None` when there is nothing to say.
///
/// A failure report records `last_failure` and leaves the URL and state alone:
/// it says nothing about that pull request, and erasing a reference an earlier
/// report established would leave the record reading as uncollected while its
/// pull request is still open.
fn report_for(record: &ApprovedRecordView, outcome: &Outcome) -> Option<CollectionReport> {
    match outcome {
        Outcome::Published { url } => Some(CollectionReport {
            record: record.id,
            pull_request: Some(url.clone()),
            state: Some(PullRequestState::Open),
            failure: None,
        }),
        Outcome::Inspected { url, state } => Some(CollectionReport {
            record: record.id,
            pull_request: Some(url.clone()),
            state: Some(*state),
            failure: None,
        }),
        Outcome::Failed { reason } => Some(CollectionReport {
            record: record.id,
            pull_request: None,
            state: None,
            failure: Some(reason.clone()),
        }),
        Outcome::Silent => None,
    }
}

fn subject(record: &ApprovedRecordView) -> String {
    format!("feat(dpe-data): publish the approved metadata for {}", record.shortcode)
}

/// The commit message, carrying the trailer the force-push guard reads back.
fn commit_message(
    record: &ApprovedRecordView,
    mapping: &std::collections::BTreeMap<String, String>,
    rows: &[String],
) -> String {
    let mut message = format!("{}\n\n{}\n", subject(record), notes(mapping, rows));
    message.push_str(&format!("\n{COLLECTED_BY_TRAILER}\n"));
    message
}

fn pull_request_body(
    record: &ApprovedRecordView,
    mapping: &std::collections::BTreeMap<String, String>,
    rows: &[String],
) -> String {
    format!(
        "Approved in the metadata editor on {}.\n\n{}\n",
        record.approved_at.format("%Y-%m-%d"),
        notes(mapping, rows)
    )
}

/// What a reviewer needs to know beyond the diff: which ids moved, and which
/// temporal-coverage values still need a range.
fn notes(mapping: &std::collections::BTreeMap<String, String>, rows: &[String]) -> String {
    let mut lines = Vec::new();
    for (from, to) in mapping {
        lines.push(format!("Renumbered {from} to {to}, which was already taken."));
    }
    for name in rows {
        lines.push(format!(
            "`{}` has no date range yet and was added to {ENRICHMENT_FILE} as unresolved; replace it with a W3CDTF range if it is a period.",
            one_line(name)
        ));
    }
    if lines.is_empty() {
        lines.push("No entity ids were renumbered and every temporal coverage value already resolves.".to_string());
    }
    lines.join("\n")
}

/// A depositor's free text, made safe to sit in a commit message and a pull
/// request body. It is interpolated inside a code span, so a backtick would
/// break out of it and a newline would end the line — and GitHub links an
/// `@name` or a `#123` that escapes one.
fn one_line(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character == '`' || character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    std::fs::write(path, contents).map_err(|error| format!("could not write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use chrono::Utc;
    use editor_core::collection::CollectionStateView;
    use uuid::Uuid;

    use super::*;

    const CREATED_URL: &str = "https://github.com/dasch-swiss/dsp-repository/pull/999";
    const EXISTING_URL: &str = "https://github.com/dasch-swiss/dsp-repository/pull/412";

    /// A forge that answers from fixed state and records what it was asked to
    /// do, so a test can assert that a skip wrote nothing.
    #[derive(Default)]
    struct FakeForge {
        pull_requests: Vec<CollectionPullRequest>,
        tip_message: Option<String>,
        calls: RefCell<Vec<String>>,
    }

    impl FakeForge {
        fn did(&self, what: &str) -> bool {
            self.calls.borrow().iter().any(|call| call == what)
        }
    }

    impl Forge for FakeForge {
        fn collection_pull_requests(&self) -> Result<Vec<CollectionPullRequest>, String> {
            Ok(self.pull_requests.clone())
        }

        fn remote_tip_message(&self, _branch: &str) -> Result<Option<String>, String> {
            Ok(self.tip_message.clone())
        }

        fn start_branch(&self, _branch: &str, _data_dir: &Path) -> Result<(), String> {
            self.calls.borrow_mut().push("start".to_string());
            Ok(())
        }

        fn commit(&self, _paths: &[PathBuf], _message: &str) -> Result<(), String> {
            self.calls.borrow_mut().push("commit".to_string());
            Ok(())
        }

        fn force_push(&self, _branch: &str) -> Result<(), String> {
            self.calls.borrow_mut().push("push".to_string());
            Ok(())
        }

        fn open_pull_request(&self, _branch: &str, _title: &str, _body: &str) -> Result<String, String> {
            self.calls.borrow_mut().push("create".to_string());
            Ok(CREATED_URL.to_string())
        }
    }

    fn pull_request(branch: &str, state: PullRequestState, url: &str) -> CollectionPullRequest {
        CollectionPullRequest {
            number: 412,
            url: url.to_string(),
            head_ref: branch.to_string(),
            state,
            files: Vec::new(),
        }
    }

    /// A data directory holding one real committed project, so the publish path
    /// writes a project the canonical writer accepts rather than a stub.
    fn fixture(name: &str) -> Config {
        let dir = std::env::temp_dir().join(format!("editor-collector-run-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["projects", "persons", "organizations"] {
            std::fs::create_dir_all(dir.join(sub)).expect("a fixture directory");
        }
        Config {
            editor_base_url: "https://editor.example".to_string(),
            collection_token: "token".to_string(),
            repository: "dasch-swiss/dsp-repository".to_string(),
            data_dir: dir,
        }
    }

    fn committed_project() -> ProjectRaw {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../dpe/server/data/projects/0803_incunabula.json"
        );
        let json = std::fs::read_to_string(path).expect("a committed project should be readable");
        serde_json::from_str(&json).expect("a committed project should parse")
    }

    fn record(shortcode: &str, project: Option<ProjectRaw>, problem: Option<String>) -> ApprovedRecordView {
        ApprovedRecordView {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            approved_at: Utc::now(),
            project,
            entities: Vec::new(),
            problem,
            collection: CollectionStateView {
                collected_at: None,
                pull_request: None,
                state: None,
                last_failure: None,
            },
        }
    }

    /// The change has landed; reconciliation discards the record when a release
    /// carries it. Nothing may be written, and the state is still reported.
    #[test]
    fn a_merged_pull_request_is_reported_and_nothing_is_written() {
        let config = fixture("merged");
        let forge = FakeForge {
            pull_requests: vec![pull_request(
                "editor-collect/0803",
                PullRequestState::Merged,
                EXISTING_URL,
            )],
            ..FakeForge::default()
        };

        let outcome = collect_one(
            &config,
            &forge,
            &record("0803", Some(committed_project()), None),
            &forge.pull_requests,
        )
        .expect("the skip rule decides without failing");

        assert_eq!(
            outcome,
            Outcome::Inspected {
                url: EXISTING_URL.to_string(),
                state: PullRequestState::Merged
            }
        );
        assert!(!forge.did("start") && !forge.did("commit") && !forge.did("push"));
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// The contract's load-bearing difference from the issue body: an open pull
    /// request is force-pushed onto, never joined by a second one.
    #[test]
    fn an_open_pull_request_is_force_pushed_onto_and_no_second_one_is_opened() {
        let config = fixture("open");
        let forge = FakeForge {
            pull_requests: vec![pull_request(
                "editor-collect/0803",
                PullRequestState::Open,
                EXISTING_URL,
            )],
            ..FakeForge::default()
        };

        let outcome = collect_one(
            &config,
            &forge,
            &record("0803", Some(committed_project()), None),
            &forge.pull_requests,
        )
        .expect("an open pull request is updated");

        assert_eq!(outcome, Outcome::Published { url: EXISTING_URL.to_string() });
        assert!(forge.did("push"), "the branch is force-pushed");
        assert!(!forge.did("create"), "a second pull request must not be opened");
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    #[test]
    fn no_pull_request_force_pushes_and_opens_a_new_one() {
        let config = fixture("none");
        let forge = FakeForge::default();

        let outcome = collect_one(&config, &forge, &record("0803", Some(committed_project()), None), &[])
            .expect("a new pull request is opened");

        assert_eq!(outcome, Outcome::Published { url: CREATED_URL.to_string() });
        assert!(forge.did("push") && forge.did("create"));
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// Reopening a closed pull request is never correct: somebody deliberately
    /// ended that review, so the retry is a new one.
    #[test]
    fn a_closed_unmerged_pull_request_gets_a_new_one_rather_than_being_reopened() {
        let config = fixture("closed");
        let forge = FakeForge {
            pull_requests: vec![pull_request(
                "editor-collect/0803",
                PullRequestState::Closed,
                EXISTING_URL,
            )],
            ..FakeForge::default()
        };

        let outcome = collect_one(
            &config,
            &forge,
            &record("0803", Some(committed_project()), None),
            &forge.pull_requests,
        )
        .expect("a closed pull request is retried");

        assert_eq!(outcome, Outcome::Published { url: CREATED_URL.to_string() });
        assert!(forge.did("create"));
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// Reviewer edits are an expected part of this flow, so a tip this
    /// collector did not make stops the run for that record rather than being
    /// overwritten.
    #[test]
    fn a_branch_tip_the_collector_did_not_make_is_refused_before_anything_is_written() {
        let config = fixture("guard");
        let forge = FakeForge {
            tip_message: Some("fix(dpe-data): correct the contact address\n".to_string()),
            ..FakeForge::default()
        };

        let outcome = collect_one(&config, &forge, &record("0803", Some(committed_project()), None), &[])
            .expect("the guard reports rather than erroring");

        let Outcome::Failed { reason } = outcome else {
            panic!("expected a refusal, got {outcome:?}")
        };
        assert!(reason.contains("editor-collect/0803"), "{reason}");
        assert!(!forge.did("start") && !forge.did("push"), "nothing may be written or pushed");
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    #[test]
    fn a_tip_this_collector_made_is_force_pushed_over() {
        let config = fixture("own-tip");
        let forge = FakeForge {
            tip_message: Some(format!("feat(dpe-data): publish 0803\n\n{COLLECTED_BY_TRAILER}\n")),
            ..FakeForge::default()
        };

        collect_one(&config, &forge, &record("0803", Some(committed_project()), None), &[])
            .expect("its own tip is replaceable");

        assert!(forge.did("push"));
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// A record whose draft could not be converted is still served by the
    /// editor, carrying its error; it becomes a failure report, not a panic and
    /// not a silent skip.
    #[test]
    fn a_record_with_no_publishable_project_becomes_a_failure_report() {
        let config = fixture("problem");
        let forge = FakeForge::default();
        let record = record("0803", None, Some("shortDescription is required".to_string()));

        let outcome = collect_one(&config, &forge, &record, &[]).expect("a problem is reported, not raised");

        assert_eq!(outcome, Outcome::Failed { reason: "shortDescription is required".to_string() });
        let report = report_for(&record, &outcome).expect("a failure is reported");
        // A failure report says nothing about the pull request: erasing a
        // reference an earlier report established would leave the record
        // reading as uncollected while its pull request is still open.
        assert!(report.pull_request.is_none() && report.state.is_none());
        assert_eq!(report.failure.as_deref(), Some("shortDescription is required"));
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// The guard reads this back off the branch tip, so a commit without it
    /// would make the collector refuse to update its own branch.
    #[test]
    fn every_commit_carries_the_trailer_the_guard_reads() {
        let message = commit_message(&record("0803", None, None), &std::collections::BTreeMap::new(), &[]);
        assert!(message.contains(COLLECTED_BY_TRAILER), "{message}");
        assert!(
            message.starts_with("feat(dpe-data): publish the approved metadata for 0803\n"),
            "{message}"
        );
    }

    /// A shortcode with no pull request has no state to re-report, and a
    /// failure report would record a failure that did not happen.
    #[test]
    fn refresh_says_nothing_about_a_project_that_was_never_collected() {
        assert!(newest_on(&[], "editor-collect/0803").is_none());
    }

    #[test]
    fn refresh_prefers_a_merged_pull_request_over_an_open_one_on_the_same_branch() {
        let branch = "editor-collect/0803";
        let pull_requests = vec![
            pull_request(branch, PullRequestState::Open, EXISTING_URL),
            pull_request(branch, PullRequestState::Merged, CREATED_URL),
        ];
        assert_eq!(
            newest_on(&pull_requests, branch).map(|p| p.state),
            Some(PullRequestState::Merged)
        );
    }

    #[test]
    fn a_sibling_open_pull_requests_entity_ids_are_taken() {
        let config = fixture("siblings");
        let pull_requests = vec![CollectionPullRequest {
            number: 1,
            url: EXISTING_URL.to_string(),
            head_ref: "editor-collect/0804".to_string(),
            state: PullRequestState::Open,
            files: vec![
                "modules/dpe/server/data/persons/person-417.json".to_string(),
                "modules/dpe/server/data/projects/0804_x.json".to_string(),
            ],
        }];

        let taken = taken_ids(&config.data_dir, &pull_requests, "editor-collect/0803").expect("the fixture loads");

        assert!(taken.contains("person-417"), "a sibling's proposed id is taken");
        assert!(!taken.contains("0804_x"), "a project file is not an entity id");
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// Its own branch is about to be force-pushed over, so the ids that branch
    /// holds are this record's to reuse rather than collide with.
    #[test]
    fn the_records_own_branch_does_not_make_its_ids_taken() {
        let config = fixture("own-ids");
        let pull_requests = vec![CollectionPullRequest {
            number: 1,
            url: EXISTING_URL.to_string(),
            head_ref: "editor-collect/0803".to_string(),
            state: PullRequestState::Open,
            files: vec!["modules/dpe/server/data/persons/person-417.json".to_string()],
        }];

        let taken = taken_ids(&config.data_dir, &pull_requests, "editor-collect/0803").expect("the fixture loads");

        assert!(!taken.contains("person-417"));
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// An entity's `kind` and `id` both become a path under the data directory.
    /// Unchecked, either escapes it, and this collector runs in CI holding a
    /// token with write access to the repository.
    #[test]
    fn an_entity_kind_or_id_that_would_escape_the_data_directory_is_refused() {
        let config = fixture("traversal");
        let forge = FakeForge::default();

        let escapes = [
            ("person", "/tmp/collector-should-never-write-here"),
            ("person", "../../../etc/shadow"),
            ("../../../etc", "person-001"),
            ("person", "person-001/../../../x"),
            ("person", "organization-001"),
        ];
        for (kind, id) in escapes {
            let mut record = record("0803", Some(committed_project()), None);
            record.entities = vec![ProposedEntityView {
                kind: kind.to_string(),
                operation: "new".to_string(),
                id: id.to_string(),
                body: serde_json::json!({ "givenNames": ["Ada"] }),
            }];

            let outcome = collect_one(&config, &forge, &record, &[]).expect("a bad entity is reported, not raised");

            assert!(
                matches!(outcome, Outcome::Failed { .. }),
                "{kind:?}/{id:?} produced {outcome:?}"
            );
        }
        assert!(!forge.did("push"), "nothing may be pushed");
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// A record is all-or-nothing on disk. The second entity here is an
    /// ordinary malformed proposal — `ProposedEntityView::from_proposal` gives
    /// an unparseable payload a null body — and writing the first before
    /// noticing it would leave an untracked file that `git checkout -B` does
    /// not remove and the next record's manifest would count.
    #[test]
    fn a_record_that_fails_on_a_later_entity_writes_none_of_them() {
        let config = fixture("all-or-nothing");
        let forge = FakeForge::default();

        let mut record = record("0803", Some(committed_project()), None);
        record.entities = vec![
            ProposedEntityView {
                kind: "person".to_string(),
                operation: "new".to_string(),
                id: "person-001".to_string(),
                body: serde_json::json!({"givenNames": ["Ada"], "familyNames": ["Lovelace"]}),
            },
            ProposedEntityView {
                kind: "person".to_string(),
                operation: "new".to_string(),
                id: "person-002".to_string(),
                body: Value::Null,
            },
        ];

        let outcome = collect_one(&config, &forge, &record, &[]).expect("a bad body is reported, not raised");

        assert!(matches!(outcome, Outcome::Failed { .. }), "{outcome:?}");
        assert!(
            !config.data_dir.join("persons/person-001.json").exists(),
            "the first entity must not survive the second's rejection"
        );
        assert!(!forge.did("start"), "the branch is not even started");
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// The shortcode becomes `editor-collect/<shortcode>` and a path under the
    /// data directory, so a value carrying a slash or a leading dash would
    /// reach `git` and the filesystem as something other than a name.
    #[test]
    fn a_shortcode_that_is_not_a_shortcode_is_refused_before_a_ref_is_built() {
        let config = fixture("bad-shortcode");
        let forge = FakeForge::default();

        for candidate in ["../../etc", "0803/../0804", "", "has space"] {
            let outcome = collect_one(&config, &forge, &record(candidate, Some(committed_project()), None), &[])
                .expect("a bad shortcode is reported, not raised");
            assert!(matches!(outcome, Outcome::Failed { .. }), "{candidate:?} produced {outcome:?}");
        }
        assert!(!forge.did("start") && !forge.did("push"));
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// A committed entity file that will not load leaves its id looking free,
    /// which would let an allocation overwrite it. The record fails instead.
    #[test]
    fn an_unloadable_published_entity_set_stops_the_record_rather_than_risking_an_overwrite() {
        let config = fixture("bad-corpus");
        std::fs::write(config.data_dir.join("persons/person-001.json"), "{ not json")
            .expect("a broken published person");

        let error = taken_ids(&config.data_dir, &[], "editor-collect/0803")
            .expect_err("a corpus that does not load is refused");

        assert!(error.contains("person-001"), "{error}");
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// A `new` entity colliding with the published set is renumbered, and the
    /// project's reference to it moves with it.
    #[test]
    fn a_colliding_entity_is_renumbered_and_the_project_reference_follows() {
        let config = fixture("renumber");
        std::fs::write(
            config.data_dir.join("persons/person-001.json"),
            r#"{"id": "person-001", "givenNames": ["Taken"], "familyNames": ["Already"], "jobTitles": []}"#,
        )
        .expect("a published person");

        let mut record = record("0803", Some(committed_project()), None);
        record.entities = vec![ProposedEntityView {
            kind: "person".to_string(),
            operation: "new".to_string(),
            id: "person-001".to_string(),
            body: serde_json::json!({"id": "person-001", "givenNames": ["Ada"], "familyNames": ["Lovelace"]}),
        }];
        if let Some(project) = record.project.as_mut() {
            project.contact_point = Some(vec!["person-001".to_string()]);
        }

        let forge = FakeForge::default();
        collect_one(&config, &forge, &record, &[]).expect("the record publishes");

        let renumbered = std::fs::read_to_string(config.data_dir.join("persons/person-002.json"))
            .expect("the proposed person is written under the next free id");
        assert!(renumbered.contains("Lovelace"), "{renumbered}");
        assert!(
            renumbered.contains("\"id\": \"person-002\""),
            "the body carries the new id: {renumbered}"
        );

        let published = std::fs::read_to_string(config.data_dir.join("persons/person-001.json"))
            .expect("the published person is readable");
        assert!(published.contains("Already"), "the published person must not be overwritten");

        // The fixture starts with no committed project for 0803, so the file
        // is the new-project path `layout` derived from the shortcode and name.
        let written_project = std::fs::read_dir(config.data_dir.join("projects"))
            .expect("the projects directory is readable")
            .flatten()
            .map(|entry| entry.path())
            .next()
            .expect("the project is written");
        let project = std::fs::read_to_string(&written_project).expect("the project is readable");
        assert!(
            project.contains("person-002"),
            "the reference follows the renumbering: {project}"
        );
        assert!(!project.contains("person-001"), "no reference is left pointing at the taken id");
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }
}
