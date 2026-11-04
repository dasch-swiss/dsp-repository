//! The place and period lists a project can record a `spatialCoverage` or
//! `temporalCoverage` reference from, bundled as JSON under `data/`, never edited by hand.
//!
//! Regenerate with `python3 scripts/build-coverage-vocabularies.py`; a new period needs the full
//! mirror of `python3 scripts/fetch-chronontology-periods.py` first. A period is offered only when
//! DPE's slim `chronontology-periods.json` holds it, since submit resolves against that file.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::disciplines;
use crate::form::{registry, REFERENCE_KIND, SNSF_KIND};

/// One bundled authority entry, stored as a `spatialCoverage` or `temporalCoverage` reference.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Entry {
    #[serde(rename = "type")]
    pub type_: String,
    pub url: String,
    pub text: String,
    /// What tells entries of one name apart in a search, never stored: a place's source and id,
    /// a period's years and source.
    pub detail: Option<String>,
}

impl Entry {
    /// The value a pick posts: `"{url} {label}"`. One url can carry several labels, so the label
    /// belongs to the pick; no bundled url holds whitespace, so the first space splits the two.
    pub fn pick_value(&self) -> String {
        format!("{} {}", self.url, self.text)
    }
}

/// The bundled places, sorted by text.
pub fn places() -> &'static [Entry] {
    static PLACES: OnceLock<Vec<Entry>> = OnceLock::new();
    PLACES.get_or_init(|| {
        serde_json::from_str(include_str!("../data/places.json"))
            .expect("data/places.json is the output of scripts/build-coverage-vocabularies.py")
    })
}

/// The bundled periods, sorted by text.
pub fn periods() -> &'static [Entry] {
    static PERIODS: OnceLock<Vec<Entry>> = OnceLock::new();
    PERIODS.get_or_init(|| {
        serde_json::from_str(include_str!("../data/periods.json"))
            .expect("data/periods.json is the output of scripts/build-coverage-vocabularies.py")
    })
}

/// The entry a [`Entry::pick_value`] names, or `None` for a value without a space or one the
/// list does not hold.
fn picked<'a>(entries: &'a [Entry], value: &str) -> Option<&'a Entry> {
    let (url, text) = value.split_once(' ')?;
    entries.iter().find(|entry| entry.url == url && entry.text == text)
}

/// Entries whose text or detail line holds every word of `query`, case-insensitively: those whose
/// text starts with the query first, then the rest, each in list order. A blank query matches
/// nothing.
pub fn search<'a>(entries: &'a [Entry], query: &str) -> Vec<&'a Entry> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let words: Vec<&str> = query.split_whitespace().collect();
    let mut found: Vec<(&Entry, bool)> = entries
        .iter()
        .filter_map(|entry| {
            let text = entry.text.to_lowercase();
            // The detail line is searched too: dozens of periods share a name and differ only
            // there.
            let detail = entry.detail.as_deref().unwrap_or_default().to_lowercase();
            words
                .iter()
                .all(|word| text.contains(word) || detail.contains(word))
                .then(|| (entry, text.starts_with(&query)))
        })
        .collect();
    found.sort_by_key(|&(_, leading)| !leading);
    found.into_iter().map(|(entry, _)| entry).collect()
}

/// The list a field's picker searches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    Disciplines,
    Places,
    Periods,
}

/// The list `field` searches, or `None` for a field without a picker.
pub fn list_for(field: &str) -> Option<List> {
    registry::field(field).and_then(|field| field.vocabulary)
}

/// One search result, whatever the list. The `value` is what picking it posts back to
/// [`List::posted`]; the rest is borrowed from the entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match<'a> {
    pub value: String,
    pub label: &'a str,
    pub detail: &'a str,
    pub group: Option<&'a str>,
}

impl List {
    pub fn label(self) -> &'static str {
        match self {
            Self::Disciplines => "Search the SNSF list",
            Self::Places => "Search for a place",
            Self::Periods => "Search for a period",
        }
    }

    /// What the list is searched by, for the combobox hint.
    pub fn hint(self) -> &'static str {
        match self {
            Self::Disciplines => "By name, code or research domain.",
            Self::Places | Self::Periods => "By name.",
        }
    }

    /// Matches for `query` in search order. Disciplines: value = code, label = name, detail = code,
    /// group = research domain. Places and periods: value = [`Entry::pick_value`], label = text,
    /// detail = the entry's own or its url, no group.
    pub fn search(self, query: &str) -> Vec<Match<'static>> {
        match self {
            Self::Disciplines => disciplines::search(query)
                .into_iter()
                .map(|discipline| Match {
                    value: discipline.code.clone(),
                    label: &discipline.name,
                    detail: &discipline.code,
                    group: Some(&discipline.group),
                })
                .collect(),
            Self::Places => entry_matches(search(places(), query)),
            Self::Periods => entry_matches(search(periods(), query)),
        }
    }

    /// The pairs a pick of `value` posts for the row `prefix` (`disciplines.r0`), the same ones the
    /// row's own controls post, or `None` when the list does not hold `value`. A place or period
    /// value is [`Entry::pick_value`]: the entry is found by url and text.
    pub fn posted(self, prefix: &str, value: &str) -> Option<Vec<(String, String)>> {
        let pair = |name: &str, value: &str| (format!("{prefix}.{name}"), value.to_string());
        let reference = |entry: &Entry| {
            vec![
                pair("ref.type", &entry.type_),
                pair("ref.url", &entry.url),
                pair("ref.label", &entry.text),
            ]
        };
        match self {
            Self::Disciplines => {
                let discipline = disciplines::snsf_discipline(value)?;
                Some(vec![pair("kind", SNSF_KIND), pair("snsf", &discipline.code)])
            }
            Self::Places => picked(places(), value).map(reference),
            Self::Periods => picked(periods(), value).map(|entry| {
                let mut pairs = vec![pair("kind", REFERENCE_KIND)];
                pairs.extend(reference(entry));
                pairs
            }),
        }
    }
}

fn entry_matches<'a>(found: Vec<&'a Entry>) -> Vec<Match<'a>> {
    found
        .into_iter()
        .map(|entry| Match {
            value: entry.pick_value(),
            label: &entry.text,
            detail: entry.detail.as_deref().unwrap_or(&entry.url),
            group: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use shared_metadata::project::TemporalCoverage;
    use shared_metadata::AuthorityFileReference;

    use super::*;
    use crate::checkout_dpe_data_dir;
    use crate::form::Shape;
    use crate::test_support::sample_raw;

    fn corpus() -> Vec<serde_json::Value> {
        let projects = checkout_dpe_data_dir().join("projects");
        let mut files = Vec::new();
        for entry in fs::read_dir(&projects).expect("the committed corpus is readable") {
            let path = entry.expect("directory entry").path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let text = fs::read_to_string(&path).expect("project file is readable");
            files.push(serde_json::from_str(&text).expect("project file is JSON"));
        }
        assert!(!files.is_empty(), "no projects under {}", projects.display());
        files
    }

    /// The `http` references of `field` across the corpus as `(type, url, text)`.
    fn corpus_references(field: &str) -> Vec<(String, String, String)> {
        let mut references = Vec::new();
        for project in corpus() {
            let entries = project.get(field).and_then(|value| value.as_array());
            for entry in entries.into_iter().flatten().filter_map(|entry| entry.as_object()) {
                let member = |name: &str| entry.get(name).and_then(|value| value.as_str()).map(str::to_string);
                let Some(url) = member("url").filter(|url| url.starts_with("http")) else {
                    continue;
                };
                references.push((member("type").unwrap_or_default(), url, member("text").unwrap_or_default()));
            }
        }
        references
    }

    fn texts(entries: &[&Entry]) -> Vec<String> {
        entries.iter().map(|entry| entry.text.clone()).collect()
    }

    /// The numeric id in a GeoNames url, which the corpus spells in several forms.
    fn geonames_id(url: &str) -> Option<&str> {
        let path = url.split_once("geonames.org").map_or(url, |(_, path)| path);
        path.split('/')
            .find(|segment| !segment.is_empty() && segment.bytes().all(|byte| byte.is_ascii_digit()))
    }

    #[test]
    fn every_committed_place_reference_is_an_option_by_place_and_label() {
        let references = corpus_references("spatialCoverage");
        assert!(!references.is_empty(), "no spatialCoverage references in the corpus");
        for (type_, url, text) in &references {
            let same_place = |option: &Entry| match (type_.as_str(), geonames_id(url)) {
                ("Geonames", Some(id)) => geonames_id(&option.url) == Some(id),
                _ => &option.url == url,
            };
            assert!(
                places()
                    .iter()
                    .any(|option| &option.type_ == type_ && &option.text == text && same_place(option)),
                "`{text}` at `{url}` is not a bundled place; regenerate with scripts/build-coverage-vocabularies.py"
            );
        }
        // Switzerland (GeoNames 2658434) has four url spellings: one option per label.
        let switzerland: Vec<&Entry> = places()
            .iter()
            .filter(|option| geonames_id(&option.url) == Some("2658434"))
            .collect();
        assert_eq!(
            switzerland.iter().map(|option| option.text.as_str()).collect::<Vec<_>>(),
            ["Suisse", "Switzerland"]
        );
        assert!(switzerland.iter().all(|option| option.type_ == "Geonames"), "{switzerland:?}");
    }

    #[test]
    fn every_committed_period_reference_is_an_option_verbatim() {
        let references = corpus_references("temporalCoverage");
        assert!(!references.is_empty(), "no temporalCoverage references in the corpus");
        for (type_, url, text) in references {
            let option = periods().iter().find(|option| option.url == url).unwrap_or_else(|| {
                panic!("`{url}` is not a bundled period; regenerate with scripts/build-coverage-vocabularies.py")
            });
            assert_eq!(
                (&option.type_, &option.url, &option.text),
                (&type_, &url, &text),
                "`{url}` differs from the committed reference; regenerate with scripts/build-coverage-vocabularies.py"
            );
        }
    }

    #[test]
    fn every_period_option_resolves_at_submit() {
        let coverage = periods()
            .iter()
            .map(|entry| {
                TemporalCoverage::Reference(AuthorityFileReference {
                    type_: entry.type_.clone(),
                    url: entry.url.clone(),
                    text: Some(entry.text.clone()),
                })
            })
            .collect();
        let mut project = sample_raw();
        project.temporal_coverage = coverage;
        let dir = checkout_dpe_data_dir();
        let chronontology = shared_metadata::chronontology::load_from(&dir);
        let enrichment = shared_metadata::temporal_enrichment::load_from(&dir);

        let unresolved = crate::submission::unresolved_temporal_coverage(&project, &chronontology, &enrichment);

        assert!(
            unresolved.is_empty(),
            "period options that do not resolve: {unresolved:?}; regenerate with scripts/build-coverage-vocabularies.py"
        );
    }

    #[test]
    fn a_word_of_the_detail_line_reaches_a_period_past_the_cap() {
        let named: Vec<&Entry> = periods().iter().filter(|entry| entry.text == "Neolithic").collect();
        assert!(named.len() > 25, "{} Neolithic periods", named.len());
        for entry in named {
            let detail = entry.detail.as_deref().expect("a period has a detail line");
            let found = search(periods(), &format!("neolithic {detail}"));
            assert!(found.len() <= 25 && found.contains(&entry), "`{detail}`: {} found", found.len());
        }
    }

    #[test]
    fn search_finds_a_place_whatever_the_case() {
        for query in ["switz", "SWITZ", "  Switz  "] {
            let found = search(places(), query);
            assert_eq!(found.first().map(|entry| entry.text.as_str()), Some("Switzerland"), "{query}");
        }
    }

    #[test]
    fn a_blank_search_matches_nothing() {
        assert!(search(places(), "").is_empty());
        assert!(search(places(), "   ").is_empty());
    }

    #[test]
    fn a_leading_match_sorts_before_a_holding_match() {
        // `Argentina` precedes `Geneva` in list order and only holds "gen".
        let found = texts(&search(places(), "gen"));
        let position = |text: &str| found.iter().position(|found| found == text).unwrap_or_else(|| panic!("{text}"));
        assert!(position("Geneva") < position("Argentina"), "{found:?}");
    }

    #[test]
    fn a_discipline_matches_by_code_under_its_research_domain() {
        let found = List::Disciplines.search("10404");
        let visual_arts = found.iter().find(|found| found.value == "10404").expect("10404 is listed");
        assert_eq!(visual_arts.label, "Visual arts and Art history");
        assert_eq!(visual_arts.detail, "10404");
        assert!(visual_arts.group.is_some(), "{visual_arts:?}");
    }

    #[test]
    fn a_place_matches_by_url_and_name_without_a_group() {
        let found = List::Places.search("switz");
        let first = found.first().expect("Switzerland matches");
        assert_eq!(first.label, "Switzerland");
        assert_eq!(first.detail, "GeoNames 2658434");
        assert!(first.value.starts_with("http"), "{first:?}");
        assert!(first.value.ends_with(" Switzerland"), "{first:?}");
        assert_eq!(first.group, None);
    }

    #[test]
    fn a_discipline_pick_posts_the_snsf_kind_and_code() {
        assert_eq!(
            List::Disciplines.posted("disciplines.r0", "10404"),
            Some(vec![
                ("disciplines.r0.kind".to_string(), "snsf".to_string()),
                ("disciplines.r0.snsf".to_string(), "10404".to_string()),
            ])
        );
    }

    #[test]
    fn a_place_pick_posts_the_reference_a_row_posts() {
        let entry = places()
            .iter()
            .find(|entry| entry.text == "Switzerland")
            .expect("Switzerland is listed");
        assert_eq!(
            List::Places.posted("spatialCoverage.r1", &entry.pick_value()),
            Some(vec![
                ("spatialCoverage.r1.ref.type".to_string(), entry.type_.clone()),
                ("spatialCoverage.r1.ref.url".to_string(), entry.url.clone()),
                ("spatialCoverage.r1.ref.label".to_string(), "Switzerland".to_string()),
            ])
        );
    }

    #[test]
    fn a_period_pick_also_posts_the_reference_kind() {
        let entry = periods().first().expect("periods are bundled");
        let pairs = List::Periods
            .posted("temporalCoverage.r0", &entry.pick_value())
            .expect("the period is listed");
        assert_eq!(pairs[0], ("temporalCoverage.r0.kind".to_string(), "reference".to_string()));
        assert_eq!(
            &pairs[1..],
            [
                ("temporalCoverage.r0.ref.type".to_string(), entry.type_.clone()),
                ("temporalCoverage.r0.ref.url".to_string(), entry.url.clone()),
                ("temporalCoverage.r0.ref.label".to_string(), entry.text.clone()),
            ]
        );
    }

    #[test]
    fn a_pick_of_a_value_the_list_does_not_hold_posts_nothing() {
        assert_eq!(List::Disciplines.posted("disciplines.r0", "10400"), None);
        assert_eq!(
            List::Places.posted("spatialCoverage.r0", "https://example.org/nowhere Nowhere"),
            None
        );
        assert_eq!(
            List::Periods.posted("temporalCoverage.r0", "https://example.org/never Never"),
            None
        );
    }

    #[test]
    fn a_place_or_period_pick_names_url_and_label() {
        for (list, entries, prefix) in [
            (List::Places, places(), "spatialCoverage.r0"),
            (List::Periods, periods(), "temporalCoverage.r0"),
        ] {
            let entry = entries.first().expect("entries are bundled");
            assert_eq!(list.posted(prefix, &entry.url), None, "a url alone");
            assert_eq!(
                list.posted(prefix, &format!("{} Wrong label", entry.url)),
                None,
                "a label the url does not carry"
            );
        }
    }

    fn posted_value<'a>(pairs: &'a [(String, String)], prefix: &str, name: &str) -> Option<&'a str> {
        let key = format!("{prefix}.{name}");
        pairs.iter().find(|(posted, _)| *posted == key).map(|(_, value)| value.as_str())
    }

    #[test]
    fn every_discipline_option_posts_its_own_code() {
        for discipline in disciplines::snsf() {
            let found = List::Disciplines.search(&discipline.code);
            let option = found
                .iter()
                .find(|option| option.label == discipline.name)
                .unwrap_or_else(|| panic!("{} is not an option", discipline.code));
            let pairs = List::Disciplines
                .posted("disciplines.r0", &option.value)
                .unwrap_or_else(|| panic!("{} does not post", discipline.code));
            assert_eq!(posted_value(&pairs, "disciplines.r0", "snsf"), Some(discipline.code.as_str()));
        }
    }

    #[test]
    fn every_place_option_posts_its_own_type_url_and_label() {
        for entry in places() {
            let found = List::Places.search(&entry.text);
            let option = found
                .iter()
                .find(|option| option.value == entry.pick_value())
                .unwrap_or_else(|| panic!("{} is not an option", entry.pick_value()));
            let pairs = List::Places
                .posted("spatialCoverage.r0", &option.value)
                .unwrap_or_else(|| panic!("{} does not post", option.value));
            let posted = |name| posted_value(&pairs, "spatialCoverage.r0", name);
            assert_eq!(
                (posted("ref.type"), posted("ref.url"), posted("ref.label")),
                (Some(entry.type_.as_str()), Some(entry.url.as_str()), Some(entry.text.as_str()))
            );
        }
    }

    #[test]
    fn every_period_option_posts_its_own_type_url_and_label() {
        for entry in periods() {
            let pairs = List::Periods
                .posted("temporalCoverage.r0", &entry.pick_value())
                .unwrap_or_else(|| panic!("{} does not post", entry.pick_value()));
            let posted = |name| posted_value(&pairs, "temporalCoverage.r0", name);
            assert_eq!(
                (posted("ref.type"), posted("ref.url"), posted("ref.label")),
                (Some(entry.type_.as_str()), Some(entry.url.as_str()), Some(entry.text.as_str()))
            );
        }
    }

    #[test]
    fn a_bundled_entry_is_one_a_row_can_store_and_a_pick_can_split() {
        for field in ["spatialCoverage", "temporalCoverage"] {
            let entries = match list_for(field) {
                Some(List::Places) => places(),
                Some(List::Periods) => periods(),
                other => panic!("{field} searches {other:?}"),
            };
            let sources = match registry::field(field).and_then(|field| field.shape) {
                Some(Shape::ReferenceRows(sources) | Shape::TextOrReferenceRows(sources)) => sources,
                other => panic!("{field} has the shape {other:?}"),
            };
            for entry in entries {
                let url = &entry.url;
                assert!(
                    url.starts_with("http://") || url.starts_with("https://"),
                    "`{url}` is not an http(s) url"
                );
                assert!(
                    !url.contains(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\')),
                    "`{url}` holds a character a link or a pick cannot carry"
                );
                assert!(
                    !entry.text.is_empty() && entry.text == entry.text.trim(),
                    "`{}` at `{url}` is blank or padded",
                    entry.text
                );
                assert!(
                    sources.contains(&entry.type_.as_str()),
                    "`{url}` has the type `{}`, which {field} does not offer ({sources:?})",
                    entry.type_
                );
            }
        }
    }
}
