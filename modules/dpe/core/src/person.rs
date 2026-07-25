use serde::{Deserialize, Serialize};

use super::models::AuthorityFileReference;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Person {
    pub id: String,
    #[serde(rename = "givenNames")]
    pub given_names: Vec<String>,
    #[serde(rename = "familyNames")]
    pub family_names: Vec<String>,
    #[serde(rename = "jobTitles")]
    pub job_titles: Vec<String>,
    #[serde(default)]
    pub affiliations: Vec<String>,
    #[serde(rename = "sameAs", default)]
    pub same_as: Vec<AuthorityFileReference>,
    #[serde(default)]
    pub email: Option<String>,
}

/// Project-contribution role words that belong in a project's `attributions`
/// (`contributorType`), never in a person's `jobTitles`. A role stored in
/// `jobTitles` is invisible to the OAI-PMH creator/contributor logic, which
/// only reads `attributions`.
///
/// The list is deliberately curated to unambiguous *project roles*. Words that
/// can equally be genuine occupations (e.g. "Editor", "Author", "Developer",
/// "Designer", "Collector", "Consultant") are intentionally excluded to avoid
/// false positives. Extend it as further role-as-job-title cases are confirmed.
///
/// All entries are lowercase; matching is case-insensitive on the trimmed
/// value (see [`is_role_job_title`]).
pub const JOB_TITLE_ROLE_WORDS: &[&str] = &[
    "project leader",
    "project lead",
    "projektleiter",
    "projektleitung",
    "project manager",
    "project member",
    "projektmitglied",
    "projektmitarbeit",
    "project staff",
    "project coordinator",
    "project coordination",
    "project assistant",
    "creator",
    "data collector",
    "principal investigator",
    "principal investigator (pi)",
    "pi",
    "co-applicant",
    "applicant",
    "contributor",
];

/// Returns `true` if `job_title` is a project-contribution role word (a member
/// of [`JOB_TITLE_ROLE_WORDS`]). Such a value belongs in a project's
/// `attributions` (`contributorType`), not in a person's `jobTitles`.
pub fn is_role_job_title(job_title: &str) -> bool {
    let normalized = job_title.trim().to_lowercase();
    JOB_TITLE_ROLE_WORDS.contains(&normalized.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_known_project_roles() {
        assert!(is_role_job_title("Project Leader"));
        assert!(is_role_job_title("Project staff"));
        assert!(is_role_job_title("Project manager"));
        assert!(is_role_job_title("Project coordinator"));
        assert!(is_role_job_title("project assistant"));
        assert!(is_role_job_title("Creator"));
        assert!(is_role_job_title("Data Collector"));
    }

    #[test]
    fn matching_is_case_insensitive_and_trimmed() {
        assert!(is_role_job_title("  PROJECT STAFF  "));
        assert!(is_role_job_title("cReAtOr"));
    }

    #[test]
    fn does_not_flag_genuine_occupations() {
        assert!(!is_role_job_title("Full professor"));
        assert!(!is_role_job_title("PhD student"));
        assert!(!is_role_job_title("Curator photo archive"));
        assert!(!is_role_job_title("Teaching and research associate"));
        assert!(!is_role_job_title("Member of the editorial management"));
    }

    #[test]
    fn does_not_flag_borderline_words_left_untouched() {
        // These remain in jobTitles pending case-by-case review; the guard
        // must not flag them or `validate` would fail on current data.
        assert!(!is_role_job_title("Developer"));
        assert!(!is_role_job_title("Designer"));
        assert!(!is_role_job_title("Project Consultant"));
        assert!(!is_role_job_title("Collector"));
    }

    #[test]
    fn does_not_flag_substring_matches() {
        // Exact match only: "program coordinator" must not match "project
        // coordinator", and a compound occupation containing a role word
        // must not be flagged.
        assert!(!is_role_job_title("Manging director and program coordinator of eikones"));
    }
}
