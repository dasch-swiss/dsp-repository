//! The per-record loop: the skip rule, the force-push guard, and the job summary.
//!
//! Collection state is derived from GitHub and keyed on the project's
//! shortcode, never on the record id. A record id identifies a row the editor
//! may legitimately replace, while the conflict being avoided is per project
//! file, so keying on the record would let a superseded record's pull request
//! go unseen and a second one open against the same file.
//!
//! The one exception: a merged pull request speaks only for the record its
//! body names (`pull_requests_for`).

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use editor_core::agents::Agents;
use editor_core::canonical::{write_entity, write_project};
use editor_core::collection::{ApprovedRecordView, ProposedEntityView};
use editor_core::proposals::{entity_id_number, ProposalKind};
use editor_core::records::PullRequestState;
use serde_json::Value;
use shared_metadata::project::{is_valid_shortcode, ProjectRaw};

use crate::config::Config;
use crate::editor::Editor;
use crate::enrichment::{self, ENRICHMENT_FILE};
use crate::forge::{branch_for, record_line, CollectionPullRequest, Forge, COLLECTED_BY_TRAILER};
use crate::manifest::{self, MANIFEST_FILE};
use crate::{layout, renumber};

/// What one run did, for the operator reading the job log.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub considered: usize,
    pub opened: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
}

/// What happened to one record.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// A new pull request.
    Opened {
        url: String,
    },
    /// Force-pushed onto a pull request that was already open.
    Updated {
        url: String,
    },
    /// Not acted on: a merged pull request already speaks for this record.
    Skipped {
        url: String,
    },
    Failed {
        reason: String,
    },
}

/// The job summary's title and table header. A table needs the blank line
/// before it, or the title and the header row fuse into one paragraph.
const SUMMARY_HEADER: &str =
    "## Editor collection\n\n| Project | Record | Outcome | Pull request | Reason |\n| --- | --- | --- | --- | --- |\n";

/// Written in place of the table when the editor holds no approved record.
const SUMMARY_NO_RECORDS: &str = "## Editor collection\n\nThe editor holds no approved records.\n";

/// Fetch and publish.
pub fn collect(config: &Config, editor: &Editor, forge: &dyn Forge) -> Result<Summary, String> {
    let records = editor.approved_records()?;
    collect_records(config, &records, forge)
}

/// Each record's row is appended to the job summary as soon as that record is
/// handled, so a run that aborts on a later listing error keeps the rows it
/// already wrote. A summary that cannot be written never stops the loop: the
/// first write failure is returned only after every record has been handled.
fn collect_records(config: &Config, records: &[ApprovedRecordView], forge: &dyn Forge) -> Result<Summary, String> {
    let mut summary = Summary::default();
    let summary_path = config.step_summary.as_deref();
    let mut write_failure: Option<String> = None;
    let mut write = |text: &str| {
        if let Some(path) = summary_path {
            if let Err(error) = append_to_summary(path, text) {
                eprintln!("{error}");
                write_failure.get_or_insert(error);
            }
        }
    };

    write(if records.is_empty() {
        SUMMARY_NO_RECORDS
    } else {
        SUMMARY_HEADER
    });
    let mut pull_requests = forge.collection_pull_requests()?;

    for record in records {
        let outcome =
            collect_one(config, forge, record, &pull_requests).unwrap_or_else(|reason| Outcome::Failed { reason });
        finish(record, &outcome, &mut summary);
        write(&summary_row(record, &outcome));

        // Re-derived after each record, not read once per run: the pull request
        // this record just opened claims entity ids the next record must not
        // allocate again, and another run may have opened one meanwhile.
        pull_requests = forge.collection_pull_requests()?;
    }
    match write_failure {
        Some(error) => Err(error),
        None => Ok(summary),
    }
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
    let on_branch = pull_requests_for(pull_requests, record);

    // Merged for this record: the change has landed, and reconciliation
    // discards the record when a release carries it.
    if let Some(merged) = on_branch
        .iter()
        .find(|pull_request| pull_request.state == PullRequestState::Merged)
    {
        return Ok(Outcome::Skipped { url: merged.url.clone() });
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
                    "{branch} has a tip commit this collector did not make; refusing to force-push over it. \
                     Merge its pull request, or close it and delete the branch, then run again."
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
    let body = pull_request_body(record, &mapping, &rows);

    // An open pull request now carries the newest approved state. Opening a
    // second would orphan the first, with no record referencing it. Its body
    // is renamed after the push, never before: a body naming a record the
    // branch does not carry would, once merged, skip that record uncollected.
    if let Some(open) = on_branch
        .iter()
        .find(|pull_request| pull_request.state == PullRequestState::Open)
    {
        forge.edit_pull_request_body(&open.url, &body).map_err(|error| {
            format!("pushed onto {}, but its body still names the earlier record: {error}", open.url)
        })?;
        return Ok(Outcome::Updated { url: open.url.clone() });
    }

    let title = subject(record);
    let url = forge.open_pull_request(&branch, &title, &body)?;
    Ok(Outcome::Opened { url })
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

/// The pull requests on this record's branch that speak for it: all except a
/// merged one naming another record. The skip rule starts here.
fn pull_requests_for<'a>(
    pull_requests: &'a [CollectionPullRequest],
    record: &ApprovedRecordView,
) -> Vec<&'a CollectionPullRequest> {
    let branch = branch_for(&record.shortcode);
    let id = record.id.to_string();
    pull_requests
        .iter()
        .filter(|pull_request| pull_request.head_ref == branch)
        .filter(|pull_request| {
            pull_request.state != PullRequestState::Merged || pull_request.record.as_deref() == Some(id.as_str())
        })
        .collect()
}

/// Counts the outcome, and names a failure in the job log.
///
/// Never stops the run: the remaining records still deserve their own attempt.
fn finish(record: &ApprovedRecordView, outcome: &Outcome, summary: &mut Summary) {
    summary.considered += 1;
    match outcome {
        Outcome::Opened { .. } => summary.opened += 1,
        Outcome::Updated { .. } => summary.updated += 1,
        Outcome::Skipped { .. } => summary.skipped += 1,
        Outcome::Failed { reason } => {
            summary.failed += 1;
            eprintln!("{}: {reason}", record.shortcode);
        }
    }
}

/// One job-summary table row. The shortcode and the reason are free text and go through [`cell`];
/// the pull request URL stays plain so GitHub links it.
fn summary_row(record: &ApprovedRecordView, outcome: &Outcome) -> String {
    let (word, url, reason) = match outcome {
        Outcome::Opened { url } => ("opened", url.as_str(), ""),
        Outcome::Updated { url } => ("updated", url.as_str(), ""),
        Outcome::Skipped { url } => ("skipped", url.as_str(), "a pull request for this record is already merged"),
        Outcome::Failed { reason } => ("failed", "", reason.as_str()),
    };
    format!(
        "| {} | {} | {word} | {} | {} |\n",
        cell(&record.shortcode),
        record.id,
        url,
        cell(reason)
    )
}

/// Free text made safe to sit in one table cell. GitHub renders the job summary
/// as Markdown with HTML, so a link or tag in the text would render live; inside
/// a code span it is shown as text. A pipe is escaped as `\|` because a table
/// row splits on it even inside a code span. Empty text stays an empty cell.
fn cell(value: &str) -> String {
    let text = one_line(value);
    if text.is_empty() {
        return text;
    }
    format!("`{}`", text.replace('|', "\\|"))
}

fn append_to_summary(path: &Path, text: &str) -> Result<(), String> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .map_err(|error| format!("could not write the job summary {}: {error}", path.display()))
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
        "Approved in the metadata editor on {}.\n\n{}\n\n{}\n",
        record.approved_at.format("%Y-%m-%d"),
        notes(mapping, rows),
        record_line(&record.id.to_string())
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

/// A depositor's free text, made safe to sit in a commit message, a pull
/// request body and a job-summary table cell. It is interpolated inside a code
/// span, so a backtick would break out of it and a newline would end the line
/// (or the table row) — and GitHub links an `@name` or a `#123` that escapes
/// one.
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
    use crate::forge::record_in;

    const CREATED_URL: &str = "https://github.com/dasch-swiss/dsp-repository/pull/999";
    const EXISTING_URL: &str = "https://github.com/dasch-swiss/dsp-repository/pull/412";

    /// A forge that answers from fixed state and records what it was asked to
    /// do, so a test can assert that a skip wrote nothing.
    #[derive(Default)]
    struct FakeForge {
        pull_requests: Vec<CollectionPullRequest>,
        tip_message: Option<String>,
        calls: RefCell<Vec<String>>,
        bodies: RefCell<Vec<String>>,
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

        fn open_pull_request(&self, _branch: &str, _title: &str, body: &str) -> Result<String, String> {
            self.calls.borrow_mut().push("create".to_string());
            self.bodies.borrow_mut().push(body.to_string());
            Ok(CREATED_URL.to_string())
        }

        fn edit_pull_request_body(&self, _url: &str, body: &str) -> Result<(), String> {
            self.calls.borrow_mut().push("edit".to_string());
            self.bodies.borrow_mut().push(body.to_string());
            Ok(())
        }
    }

    fn pull_request(branch: &str, state: PullRequestState, url: &str) -> CollectionPullRequest {
        CollectionPullRequest {
            number: 412,
            url: url.to_string(),
            head_ref: branch.to_string(),
            state,
            files: Vec::new(),
            record: None,
        }
    }

    fn naming(mut pull_request: CollectionPullRequest, record: &ApprovedRecordView) -> CollectionPullRequest {
        pull_request.record = Some(record.id.to_string());
        pull_request
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
            repository: "dasch-swiss/dsp-repository".to_string(),
            step_summary: None,
            data_dir: dir,
        }
    }

    fn committed_project() -> ProjectRaw {
        let path = editor_core::checkout_dpe_data_dir().join("projects/0803_incunabula.json");
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
    /// carries it. Nothing may be written.
    #[test]
    fn a_merged_pull_request_skips_the_record_and_nothing_is_written() {
        let config = fixture("merged");
        let record = record("0803", Some(committed_project()), None);
        let forge = FakeForge {
            pull_requests: vec![naming(
                pull_request("editor-collect/0803", PullRequestState::Merged, EXISTING_URL),
                &record,
            )],
            ..FakeForge::default()
        };

        let outcome =
            collect_one(&config, &forge, &record, &forge.pull_requests).expect("the skip rule decides without failing");

        assert_eq!(outcome, Outcome::Skipped { url: EXISTING_URL.to_string() });
        assert!(!forge.did("start") && !forge.did("commit") && !forge.did("push"));
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// A project approved again after its first collection was merged and
    /// released.
    #[test]
    fn a_merged_pull_request_naming_an_earlier_record_does_not_skip_a_newer_one() {
        let config = fixture("merged-earlier");
        let earlier = record("0803", Some(committed_project()), None);
        let newer = record("0803", Some(committed_project()), None);
        let forge = FakeForge {
            pull_requests: vec![naming(
                pull_request("editor-collect/0803", PullRequestState::Merged, EXISTING_URL),
                &earlier,
            )],
            ..FakeForge::default()
        };

        let outcome =
            collect_one(&config, &forge, &newer, &forge.pull_requests).expect("the newer record is collected");

        assert_eq!(outcome, Outcome::Opened { url: CREATED_URL.to_string() });
        assert!(forge.did("push") && forge.did("create"));
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    /// The contract's load-bearing difference from the issue body: an open pull
    /// request is force-pushed onto, never joined by a second one, and renamed
    /// to the record now on its branch.
    #[test]
    fn an_open_pull_request_is_force_pushed_onto_and_renamed_and_no_second_one_is_opened() {
        let config = fixture("open");
        let earlier = record("0803", Some(committed_project()), None);
        let newer = record("0803", Some(committed_project()), None);
        let forge = FakeForge {
            pull_requests: vec![naming(
                pull_request("editor-collect/0803", PullRequestState::Open, EXISTING_URL),
                &earlier,
            )],
            ..FakeForge::default()
        };

        let outcome =
            collect_one(&config, &forge, &newer, &forge.pull_requests).expect("an open pull request is updated");

        assert_eq!(outcome, Outcome::Updated { url: EXISTING_URL.to_string() });
        assert!(forge.did("push"), "the branch is force-pushed");
        assert!(!forge.did("create"), "a second pull request must not be opened");
        let bodies = forge.bodies.borrow();
        assert_eq!(bodies.len(), 1, "the open pull request's body is edited once");
        assert_eq!(record_in(&bodies[0]), Some(newer.id.to_string()));
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    #[test]
    fn no_pull_request_force_pushes_and_opens_a_new_one() {
        let config = fixture("none");
        let forge = FakeForge::default();

        let record = record("0803", Some(committed_project()), None);

        let outcome = collect_one(&config, &forge, &record, &[]).expect("a new pull request is opened");

        assert_eq!(outcome, Outcome::Opened { url: CREATED_URL.to_string() });
        assert!(forge.did("push") && forge.did("create"));
        assert_eq!(record_in(&forge.bodies.borrow()[0]), Some(record.id.to_string()));
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

        assert_eq!(outcome, Outcome::Opened { url: CREATED_URL.to_string() });
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
    /// editor, carrying its error; it becomes a failure, not a panic and not a
    /// silent skip.
    #[test]
    fn a_record_with_no_publishable_project_fails() {
        let config = fixture("problem");
        let forge = FakeForge::default();
        let record = record("0803", None, Some("shortDescription is required".to_string()));

        let outcome = collect_one(&config, &forge, &record, &[]).expect("a problem is returned as a failure");

        assert_eq!(outcome, Outcome::Failed { reason: "shortDescription is required".to_string() });
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

    #[test]
    fn each_outcome_renders_its_word_url_and_reason() {
        let record = record("0803", None, None);
        let id = record.id;

        let opened = summary_row(&record, &Outcome::Opened { url: CREATED_URL.to_string() });
        assert_eq!(opened, format!("| `0803` | {id} | opened | {CREATED_URL} |  |\n"));

        let updated = summary_row(&record, &Outcome::Updated { url: EXISTING_URL.to_string() });
        assert_eq!(updated, format!("| `0803` | {id} | updated | {EXISTING_URL} |  |\n"));

        let skipped = summary_row(&record, &Outcome::Skipped { url: EXISTING_URL.to_string() });
        assert_eq!(
            skipped,
            format!(
                "| `0803` | {id} | skipped | {EXISTING_URL} | `a pull request for this record is already merged` |\n"
            )
        );

        let failed = summary_row(&record, &Outcome::Failed { reason: "no usable project".to_string() });
        assert_eq!(failed, format!("| `0803` | {id} | failed |  | `no usable project` |\n"));
    }

    #[test]
    fn a_depositors_text_cannot_reshape_the_row_or_render_live() {
        let piped = record("08|3\n", None, None);
        let id = piped.id;
        let row = summary_row(&piped, &Outcome::Failed { reason: "bad `a|b`\nsecond line".to_string() });
        assert_eq!(row, format!("| `08\\|3` | {id} | failed |  | `bad  a\\|b  second line` |\n"));
        assert_eq!(row.matches('\n').count(), 1, "one row is one line: {row:?}");

        let linked = record("[x](https://evil.example)", None, None);
        let id = linked.id;
        let row = summary_row(&linked, &Outcome::Failed { reason: "<img src=x>|".to_string() });
        assert_eq!(
            row,
            format!("| `[x](https://evil.example)` | {id} | failed |  | `<img src=x>\\|` |\n")
        );
    }

    fn config_writing_summary_to(name: &str, path: &Path) -> Config {
        Config { step_summary: Some(path.to_path_buf()), ..fixture(name) }
    }

    #[test]
    fn the_summary_holds_the_header_then_one_row_per_record_in_order() {
        let path = std::env::temp_dir().join(format!("editor-collector-summary-rows-{}.md", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let config = config_writing_summary_to("summary-rows", &path);
        let forge = FakeForge::default();
        let records = [
            record("0803", Some(committed_project()), None),
            record("0804", None, Some("shortDescription is required".to_string())),
        ];

        let summary = collect_records(&config, &records, &forge).expect("both records are handled");

        assert_eq!((summary.opened, summary.failed), (1, 1));
        let written = std::fs::read_to_string(&path).expect("the summary file exists");
        let rows = written.strip_prefix(SUMMARY_HEADER).expect("the header comes first");
        let rows: Vec<&str> = rows.lines().collect();
        assert_eq!(rows.len(), 2, "{written}");
        assert!(rows[0].contains("`0803`") && rows[0].contains("| opened |") && rows[0].contains(CREATED_URL));
        assert!(
            rows[1].contains("`0804`")
                && rows[1].contains("| failed |")
                && rows[1].contains("`shortDescription is required`")
        );
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    #[test]
    fn no_records_writes_the_no_records_line_and_no_table() {
        let path = std::env::temp_dir().join(format!("editor-collector-summary-empty-{}.md", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let config = config_writing_summary_to("summary-empty", &path);

        collect_records(&config, &[], &FakeForge::default()).expect("nothing to handle");

        let written = std::fs::read_to_string(&path).expect("the summary file exists");
        assert_eq!(written, SUMMARY_NO_RECORDS);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    #[test]
    fn an_unwritable_summary_does_not_stop_the_records_but_fails_the_run() {
        let path = std::env::temp_dir()
            .join(format!("editor-collector-no-such-dir-{}", std::process::id()))
            .join("summary.md");
        let config = config_writing_summary_to("summary-unwritable", &path);
        let forge = FakeForge::default();
        let records = [
            record("0803", Some(committed_project()), None),
            record("0803", Some(committed_project()), None),
        ];

        let result = collect_records(&config, &records, &forge);

        let error = result.expect_err("a summary that cannot be written fails the run");
        assert!(error.contains("job summary"), "{error}");
        let created = forge.calls.borrow().iter().filter(|call| *call == "create").count();
        assert_eq!(created, 2, "every record is still attempted");
        let _ = std::fs::remove_dir_all(&config.data_dir);
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
                format!("{}/persons/person-417.json", editor_core::DPE_DATA_DIR),
                format!("{}/projects/0804_x.json", editor_core::DPE_DATA_DIR),
            ],
            record: None,
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
            files: vec![format!("{}/persons/person-417.json", editor_core::DPE_DATA_DIR)],
            record: None,
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

            let outcome = collect_one(&config, &forge, &record, &[]).expect("a bad entity is returned as a failure");

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

        let outcome = collect_one(&config, &forge, &record, &[]).expect("a bad body is returned as a failure");

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
                .expect("a bad shortcode is returned as a failure");
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
