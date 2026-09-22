//! GitHub and git, reached through the `gh` and `git` executables.
//!
//! Subprocesses rather than an API client crate: `release-please.yml` is the
//! in-repo precedent for querying pull requests with `gh`, and a trait keeps the
//! run loop testable without either binary on the machine.

use std::path::{Path, PathBuf};
use std::process::Command;

use editor_core::records::PullRequestState;

/// The prefix every branch this collector owns carries.
pub const BRANCH_PREFIX: &str = "editor-collect/";

/// The trailer marking a commit as this collector's.
///
/// The force-push guard reads it off the branch tip. A reviewer's fixup does
/// not carry it, which is what stops the next run from silently overwriting
/// their work; nothing else about a commit distinguishes the two reliably,
/// since the bot identity is not exclusive to this collector.
pub const COLLECTED_BY_TRAILER: &str = "Collected-by: editor-collector";

/// How many collection pull requests one listing may return.
///
/// A truncated listing would hide an open pull request and let a second one
/// open against the same project file — the state that keying on the shortcode
/// exists to make unreachable — so reaching the cap is an error, not a page
/// boundary to read past.
const PULL_REQUEST_LIMIT: usize = 200;

/// The branch one project's collection lives on.
#[must_use]
pub fn branch_for(shortcode: &str) -> String {
    format!("{BRANCH_PREFIX}{shortcode}")
}

/// One pull request opened from a collection branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionPullRequest {
    /// Higher is newer, which is how the newest of several closed pull
    /// requests on one branch is chosen.
    pub number: u64,
    pub url: String,
    pub head_ref: String,
    pub state: PullRequestState,
    /// Repository-relative paths the pull request changes. The proposed-id
    /// collision check reads entity ids out of these, so a sibling pull request
    /// that has not merged still holds its ids.
    pub files: Vec<String>,
}

/// Everything the run loop needs from git and GitHub.
pub trait Forge {
    /// Every pull request opened from an `editor-collect/*` branch, in any
    /// state.
    ///
    /// One call serves both the skip rule and the sibling-pull-request id
    /// collision check, and it is re-issued after each record so a pull request
    /// this run just opened is visible to the next one.
    fn collection_pull_requests(&self) -> Result<Vec<CollectionPullRequest>, String>;

    /// The tip commit message of `origin/<branch>`, or `None` when the remote
    /// has no such branch.
    fn remote_tip_message(&self, branch: &str) -> Result<Option<String>, String>;

    /// Checks out `branch` reset to the base branch, discarding whatever was
    /// there — including a file left untracked under `data_dir` by a record
    /// that failed after writing. Each record starts from the base so its pull
    /// request carries only its own change.
    fn start_branch(&self, branch: &str, data_dir: &Path) -> Result<(), String>;

    /// Stages exactly `paths` and commits them. Never `git add -A`: the
    /// checkout also holds this collector's own build output.
    fn commit(&self, paths: &[PathBuf], message: &str) -> Result<(), String>;

    fn force_push(&self, branch: &str) -> Result<(), String>;

    /// Opens a pull request and returns its URL.
    fn open_pull_request(&self, branch: &str, title: &str, body: &str) -> Result<String, String>;
}

/// The real thing: `gh` and `git` against the checkout the workflow made.
pub struct CommandForge {
    repository: String,
    base_branch: String,
    working_dir: PathBuf,
}

impl CommandForge {
    pub fn new(repository: &str, base_branch: &str, working_dir: &Path) -> Self {
        Self {
            repository: repository.to_string(),
            base_branch: base_branch.to_string(),
            working_dir: working_dir.to_path_buf(),
        }
    }

    /// Runs one command, returning stdout, or stderr folded into the error.
    fn run(&self, program: &str, arguments: &[&str]) -> Result<String, String> {
        let output = Command::new(program)
            .args(arguments)
            .current_dir(&self.working_dir)
            .output()
            .map_err(|error| format!("could not run `{program}`: {error}"))?;

        if !output.status.success() {
            return Err(format!(
                "`{program} {}` failed: {}",
                arguments.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// What `gh pr list --json` returns, as much of it as is read here.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhPullRequest {
    number: u64,
    url: String,
    head_ref_name: String,
    state: String,
    #[serde(default)]
    files: Vec<GhFile>,
}

#[derive(serde::Deserialize)]
struct GhFile {
    path: String,
}

impl Forge for CommandForge {
    fn collection_pull_requests(&self) -> Result<Vec<CollectionPullRequest>, String> {
        let search = format!("head:{BRANCH_PREFIX}");
        let json = self.run(
            "gh",
            &[
                "pr",
                "list",
                "--repo",
                &self.repository,
                "--state",
                "all",
                "--search",
                &search,
                "--limit",
                &PULL_REQUEST_LIMIT.to_string(),
                "--json",
                "number,url,headRefName,state,files",
            ],
        )?;

        let listed: Vec<GhPullRequest> =
            serde_json::from_str(&json).map_err(|error| format!("could not read `gh pr list` output: {error}"))?;

        if listed.len() >= PULL_REQUEST_LIMIT {
            return Err(format!(
                "`gh pr list` returned its {PULL_REQUEST_LIMIT}-row limit, so the listing may be incomplete; \
                 page it before trusting the skip rule"
            ));
        }

        Ok(listed
            .into_iter()
            // `head:` is a search qualifier, not an exact filter, so the prefix
            // is re-checked here: a loose match must not widen the set of
            // branches this collector believes it owns.
            .filter(|pull_request| pull_request.head_ref_name.starts_with(BRANCH_PREFIX))
            .map(|pull_request| CollectionPullRequest {
                number: pull_request.number,
                url: pull_request.url,
                head_ref: pull_request.head_ref_name,
                state: match pull_request.state.as_str() {
                    "MERGED" => PullRequestState::Merged,
                    "CLOSED" => PullRequestState::Closed,
                    _ => PullRequestState::Open,
                },
                files: pull_request.files.into_iter().map(|file| file.path).collect(),
            })
            .collect())
    }

    fn remote_tip_message(&self, branch: &str) -> Result<Option<String>, String> {
        // `ls-remote` rather than a local ref: the checkout may predate a branch
        // a concurrent run pushed, and a stale local ref would let the guard
        // pass on a tip it has never seen.
        let listed = self.run("git", &["ls-remote", "--heads", "origin", branch])?;
        if listed.trim().is_empty() {
            return Ok(None);
        }
        self.run("git", &["fetch", "--no-tags", "origin", branch])?;
        let message = self.run("git", &["log", "-1", "--format=%B", "FETCH_HEAD"])?;
        Ok(Some(message))
    }

    fn start_branch(&self, branch: &str, data_dir: &Path) -> Result<(), String> {
        let base = format!("origin/{}", self.base_branch);
        self.run("git", &["fetch", "--no-tags", "origin", &self.base_branch])?;
        self.run("git", &["checkout", "-B", branch, &base])?;
        // Scoped to the data directory, and without `-x`, so the runner's build
        // output and every other ignored file are left alone.
        let data_dir = data_dir.to_string_lossy().into_owned();
        self.run("git", &["clean", "-fd", "--", &data_dir])?;
        Ok(())
    }

    fn commit(&self, paths: &[PathBuf], message: &str) -> Result<(), String> {
        for path in paths {
            let path = path.to_string_lossy().into_owned();
            self.run("git", &["add", "--", &path])?;
        }
        self.run("git", &["commit", "-m", message])?;
        Ok(())
    }

    fn force_push(&self, branch: &str) -> Result<(), String> {
        // `--force`, not `--force-with-lease`: the branch was just reset to the
        // base branch, so there is no local ref for a lease to compare against.
        // The guard that makes this safe is the tip-message check the run loop
        // performs before calling this.
        let refspec = format!("{branch}:{branch}");
        self.run("git", &["push", "--force", "origin", &refspec])?;
        Ok(())
    }

    fn open_pull_request(&self, branch: &str, title: &str, body: &str) -> Result<String, String> {
        let url = self.run(
            "gh",
            &[
                "pr",
                "create",
                "--repo",
                &self.repository,
                "--base",
                &self.base_branch,
                "--head",
                branch,
                "--title",
                title,
                "--body",
                body,
            ],
        )?;
        Ok(url.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_branch_is_the_prefix_and_the_shortcode() {
        assert_eq!(branch_for("0803"), "editor-collect/0803");
        assert_eq!(branch_for("0801d"), "editor-collect/0801d");
    }
}
