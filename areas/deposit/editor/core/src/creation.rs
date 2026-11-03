//! A project RDU creates in the editor: what is derived from the shortcode and name RDU enters,
//! and the draft the project opens with.
//!
//! `id` and `pid` are derived, not minted: the ARK resolves only once DSP knows the project.
//! `howToCite` and `legalInfo` are display-only, so a depositor cannot fill them in, and
//! [`crate::draft::ProjectDraft::to_raw`] needs both at submit; the seed carries them, and RDU
//! completes them in the collection pull request.

use serde_json::{json, Value};

use crate::draft::ProjectDraft;
use crate::published::PublishedProjects;

/// The host of every published project's ARK `pid`.
const ARK_HOST: &str = "https://ark.dasch.swiss";

/// The `pid` a created project carries: the ARK host, [`shared_metadata::ARK_PATH_PREFIX`], then
/// the shortcode.
#[must_use]
pub fn pid_for(shortcode: &str) -> String {
    format!("{ARK_HOST}/{}{shortcode}", shared_metadata::ARK_PATH_PREFIX)
}

/// The published project whose `pid` is the one `shortcode` derives, if any: the `0801a`-`0801e`
/// split shares `0801`'s ARK, so a shortcode no published project carries can still name one.
#[must_use]
pub fn pid_holder<'a>(published: &'a PublishedProjects, shortcode: &str) -> Option<&'a str> {
    let pid = pid_for(shortcode);
    published
        .summaries()
        .filter_map(|summary| published.get(summary.shortcode))
        .find(|project| project.pid.eq_ignore_ascii_case(&pid))
        .map(|project| project.shortcode.as_str())
}

/// The citation a created project opens with; RDU adds its authors and year before it is
/// published.
#[must_use]
pub fn how_to_cite(name: &str, pid: &str) -> String {
    format!("{name} [database] (DaSCH), {pid}.")
}

/// The draft a project opens with when RDU creates it.
#[must_use]
pub fn seed(shortcode: &str, name: &str) -> ProjectDraft {
    let mut draft = reseed(shortcode, name);
    draft.set("name", Value::String(name.to_string()));
    draft
}

/// What "Discard draft" leaves of a created project: [`seed`] without the name, which is a
/// creation-time value a depositor may have changed. `registered_name` is the name RDU entered,
/// which the citation is built from.
#[must_use]
pub fn reseed(shortcode: &str, registered_name: &str) -> ProjectDraft {
    let pid = pid_for(shortcode);
    let mut draft = ProjectDraft::default();
    draft.set("shortcode", Value::String(shortcode.to_string()));
    draft.set("id", Value::String(shortcode.to_string()));
    draft.set("howToCite", Value::String(how_to_cite(registered_name, &pid)));
    draft.set("pid", Value::String(pid));
    draft.set("legalInfo", json!([]));
    draft
}

/// A created project's `member` (`shortcode` or `name`) as the editor shows it: the draft's, once
/// it holds a non-blank one, else `registered`, the registration's.
#[must_use]
pub fn shown<'a>(draft: Option<&'a ProjectDraft>, member: &str, registered: &'a str) -> &'a str {
    draft
        .and_then(|draft| draft.get(member))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or(registered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_derives_id_pid_and_citation_from_shortcode_and_name() {
        let draft = seed("1111", "Basler Fotoarchiv");
        assert_eq!(draft.get("shortcode"), Some(&json!("1111")));
        assert_eq!(draft.get("id"), Some(&json!("1111")));
        assert_eq!(draft.get("pid"), Some(&json!("https://ark.dasch.swiss/ark:/72163/1/1111")));
        assert_eq!(draft.get("name"), Some(&json!("Basler Fotoarchiv")));
        assert_eq!(
            draft.get("howToCite"),
            Some(&json!(
                "Basler Fotoarchiv [database] (DaSCH), https://ark.dasch.swiss/ark:/72163/1/1111."
            ))
        );
        assert_eq!(draft.get("legalInfo"), Some(&json!([])));
    }

    #[test]
    fn test_reseed_keeps_everything_but_the_name() {
        let mut seeded = seed("1111", "Basler Fotoarchiv");
        seeded.remove("name");
        assert_eq!(reseed("1111", "Basler Fotoarchiv"), seeded);
    }

    #[test]
    fn test_a_shortcode_whose_ark_a_published_project_carries_is_found() {
        let dir = crate::checkout_dpe_data_dir().join("projects");
        let (published, _) = crate::published::PublishedProjects::load_from(&dir);
        assert!(published.get("0801").is_none(), "0801 itself is not published");
        assert!(pid_holder(&published, "0801").is_some_and(|holder| holder.starts_with("0801")));
        assert_eq!(pid_holder(&published, "08AB"), None);
    }

    #[test]
    fn test_every_committed_pid_that_names_its_own_shortcode_is_the_derived_one() {
        // The derivation is the published convention, not an invention: every committed project
        // whose ARK ends in its own shortcode spells it exactly so. The `0801a`-`0801e` split
        // shares `0801`'s ARK, which a created shortcode never does.
        let dir = crate::checkout_dpe_data_dir().join("projects");
        let (published, errors) = crate::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "{errors:?}");
        let mut checked = 0;
        for summary in published.summaries() {
            let raw = published.get(summary.shortcode).unwrap();
            if raw.pid.ends_with(&format!("/{}", raw.shortcode)) {
                assert_eq!(raw.pid, pid_for(&raw.shortcode), "{}", raw.shortcode);
                assert_eq!(raw.id, raw.shortcode, "{}", raw.shortcode);
                checked += 1;
            }
        }
        assert!(checked > 0, "no committed project was checked");
    }
}
