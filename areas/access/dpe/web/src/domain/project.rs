// ProjectQuery: the projects-list query/filter state, parsed from the URL query
// string by `ProjectQuery::from_query_str`. The domain types live elsewhere —
// `ProjectRaw` in shared-metadata, `Project` in dpe-core.

use shared_metadata::ACCESS_RIGHTS_VALUES;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ProjectQuery {
    pub ongoing: Option<bool>,
    pub finished: Option<bool>,
    pub search: Option<String>,
    pub page: Option<i32>,
    pub type_of_data: Option<String>,
    pub data_language: Option<String>,
    pub access_rights: Option<String>,
    pub dialog: Option<bool>,
}

impl ProjectQuery {
    /// Parses a raw query string. A multi-select facet accepts repeated keys (what a checkbox
    /// form submits) and comma-joined values (the canonical form, and every URL bookmarked so
    /// far), merged in first-seen order without duplicates. A malformed or unknown value is
    /// ignored rather than refused, so a mangled bookmark still renders.
    pub fn from_query_str(raw: &str) -> Self {
        let mut query = Self::default();
        let mut type_of_data = Vec::new();
        let mut data_language = Vec::new();
        let mut access_rights = Vec::new();
        for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
            match key.as_ref() {
                "ongoing" => query.ongoing = (value == "true").then_some(true),
                "finished" => query.finished = (value == "true").then_some(true),
                "dialog" => query.dialog = (value == "true").then_some(true),
                "search" => query.search = Some(value.into_owned()),
                "page" => query.page = value.parse::<i32>().ok().filter(|p| *p > 0),
                "type_of_data" => push_values(&mut type_of_data, &value),
                "data_language" => push_values(&mut data_language, &value),
                "access_rights" => push_values(&mut access_rights, &value),
                _ => {}
            }
        }
        query.type_of_data = join_values(type_of_data);
        query.data_language = join_values(data_language);
        query.access_rights = join_values(access_rights);
        query
    }

    /// Whether a facet, `search` or `dialog` is set: the URL is a filtered view, not the list
    /// or one of its pages.
    pub fn has_filter_params(&self) -> bool {
        self.type_of_data.is_some()
            || self.data_language.is_some()
            || self.access_rights.is_some()
            || self.ongoing()
            || self.finished()
            || self.search.as_deref().is_some_and(|s| !s.is_empty())
            || self.dialog == Some(true)
    }

    pub fn ongoing(&self) -> bool {
        self.ongoing.unwrap_or(false)
    }

    pub fn finished(&self) -> bool {
        self.finished.unwrap_or(false)
    }

    // Unconditional (the body has nothing SSR-specific) — sibling accessors
    // like `type_of_data`, `data_language`, `access_rights` are also ungated.
    pub fn search(&self) -> String {
        self.search.clone().unwrap_or_default()
    }

    pub fn page(&self) -> i32 {
        self.page.unwrap_or(1)
    }

    pub fn type_of_data(&self) -> Vec<String> {
        self.type_of_data
            .as_deref()
            .map(|s| s.split(',').map(str::to_string).collect())
            .unwrap_or_default()
    }

    pub fn data_language(&self) -> Vec<String> {
        self.data_language
            .as_deref()
            .map(|s| s.split(',').map(str::to_string).collect())
            .unwrap_or_default()
    }

    pub fn access_rights(&self) -> Vec<String> {
        self.access_rights
            .as_deref()
            .map(|s| s.split(',').map(str::to_string).collect())
            .unwrap_or_default()
    }

    pub fn with_page(self, page: i32) -> Self {
        Self { page: Some(page), ..self }
    }

    /// Return a new `ProjectQuery` with the given status param toggled, page reset to 1.
    pub fn with_status_toggled(&self, param: &str) -> Self {
        let ongoing = self.ongoing();
        let finished = self.finished();
        Self {
            ongoing: Some(if param == "ongoing" { !ongoing } else { ongoing }),
            finished: Some(if param == "finished" { !finished } else { finished }),
            search: self.search.clone(),
            page: Some(1),
            type_of_data: self.type_of_data.clone(),
            data_language: self.data_language.clone(),
            access_rights: self.access_rights.clone(),
            dialog: self.dialog,
        }
    }

    /// Build access rights filter items: `(label, checked, href)` for each access rights value.
    pub fn access_rights_filter_items(&self) -> Vec<(String, bool, String)> {
        ACCESS_RIGHTS_VALUES
            .iter()
            .map(|&v| {
                let checked = self.access_rights().contains(&v.to_string());
                let href = format!("/dpe/projects{}", self.with_access_rights_toggled(v).to_query_string());
                (v.to_string(), checked, href)
            })
            .collect()
    }

    /// Build status filter items: `(label, checked, href)` for "Ongoing" and "Finished".
    pub fn status_filter_items(&self) -> Vec<(String, bool, String)> {
        [
            ("ongoing", "Ongoing", self.ongoing()),
            ("finished", "Finished", self.finished()),
        ]
        .iter()
        .map(|(param, label, checked)| {
            let href = format!("/dpe/projects{}", self.with_status_toggled(param).to_query_string());
            (label.to_string(), *checked, href)
        })
        .collect()
    }

    /// Return a new `ProjectQuery` with `value` toggled in/out of `type_of_data`, page reset to 1.
    pub fn with_type_of_data_toggled(&self, value: &str) -> Self {
        let mut selected = self.type_of_data();
        if selected.contains(&value.to_string()) {
            selected.retain(|v| v != value);
        } else {
            selected.push(value.to_string());
        }
        Self {
            ongoing: self.ongoing,
            finished: self.finished,
            search: self.search.clone(),
            page: Some(1),
            type_of_data: if selected.is_empty() {
                None
            } else {
                Some(selected.join(","))
            },
            data_language: self.data_language.clone(),
            access_rights: self.access_rights.clone(),
            dialog: self.dialog,
        }
    }

    /// Return a new `ProjectQuery` with `value` toggled in/out of `data_language`, page reset to 1.
    pub fn with_data_language_toggled(&self, value: &str) -> Self {
        let mut selected = self.data_language();
        if selected.contains(&value.to_string()) {
            selected.retain(|v| v != value);
        } else {
            selected.push(value.to_string());
        }
        Self {
            ongoing: self.ongoing,
            finished: self.finished,
            search: self.search.clone(),
            page: Some(1),
            type_of_data: self.type_of_data.clone(),
            data_language: if selected.is_empty() {
                None
            } else {
                Some(selected.join(","))
            },
            access_rights: self.access_rights.clone(),
            dialog: self.dialog,
        }
    }

    /// Return a new `ProjectQuery` with `value` toggled in/out of `access_rights`, page reset to 1.
    pub fn with_access_rights_toggled(&self, value: &str) -> Self {
        let mut selected = self.access_rights();
        if selected.contains(&value.to_string()) {
            selected.retain(|v| v != value);
        } else {
            selected.push(value.to_string());
        }
        Self {
            ongoing: self.ongoing,
            finished: self.finished,
            search: self.search.clone(),
            page: Some(1),
            type_of_data: self.type_of_data.clone(),
            data_language: self.data_language.clone(),
            access_rights: if selected.is_empty() {
                None
            } else {
                Some(selected.join(","))
            },
            dialog: self.dialog,
        }
    }

    /// Return a new `ProjectQuery` with `dialog` set to `open`, preserving all other fields.
    pub fn with_dialog(self, open: bool) -> Self {
        Self { dialog: if open { Some(true) } else { None }, ..self }
    }

    /// The canonical query string, `?` included, or empty. Every value is percent-encoded and
    /// multi-select values are joined with a literal `,`, so the output holds only
    /// `A-Z a-z 0-9 - _ . ~ % , & = ?` and is safe in an `href` and a single-quoted JS string.
    pub fn to_query_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(true) = self.ongoing {
            parts.push("ongoing=true".to_string());
        }
        if let Some(true) = self.finished {
            parts.push("finished=true".to_string());
        }
        if let Some(ref search) = self.search {
            if !search.is_empty() {
                parts.push(format!("search={}", urlencoding::encode(search)));
            }
        }
        if let Some(page) = self.page {
            if page > 1 {
                parts.push(format!("page={}", page));
            }
        }
        if let Some(ref type_of_data) = self.type_of_data {
            if !type_of_data.is_empty() {
                parts.push(format!("type_of_data={}", encode_values(type_of_data)));
            }
        }
        if let Some(ref data_language) = self.data_language {
            if !data_language.is_empty() {
                parts.push(format!("data_language={}", encode_values(data_language)));
            }
        }
        if let Some(ref access_rights) = self.access_rights {
            if !access_rights.is_empty() {
                parts.push(format!("access_rights={}", encode_values(access_rights)));
            }
        }
        if let Some(true) = self.dialog {
            parts.push("dialog=true".to_string());
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("?{}", parts.join("&"))
        }
    }
}

/// Splits one occurrence on `,` and appends its non-empty parts not seen yet.
fn push_values(acc: &mut Vec<String>, value: &str) {
    for part in value.split(',').filter(|p| !p.is_empty()) {
        if !acc.iter().any(|v| v == part) {
            acc.push(part.to_string());
        }
    }
}

fn join_values(values: Vec<String>) -> Option<String> {
    (!values.is_empty()).then(|| values.join(","))
}

/// Percent-encodes each comma-joined value on its own, so the separator stays a literal `,`.
fn encode_values(joined: &str) -> String {
    joined.split(',').map(|v| urlencoding::encode(v).into_owned()).collect::<Vec<_>>().join(",")
}

#[cfg(test)]
mod tests {
    use super::ProjectQuery;

    fn parse(raw: &str) -> ProjectQuery {
        ProjectQuery::from_query_str(raw)
    }

    #[test]
    fn repeated_keys_and_comma_joined_values_give_the_same_query() {
        let repeated = parse("type_of_data=Text&type_of_data=Image&data_language=en&data_language=de");
        let joined = parse("type_of_data=Text,Image&data_language=en,de");
        assert_eq!(repeated, joined);
        assert_eq!(joined.type_of_data(), vec!["Text", "Image"]);
    }

    #[test]
    fn mixed_forms_merge_in_first_seen_order_without_duplicates() {
        let q = parse("access_rights=A,B&access_rights=C&access_rights=B,,A");
        assert_eq!(q.access_rights.as_deref(), Some("A,B,C"));
    }

    #[test]
    fn plus_decodes_to_a_space_and_encoded_comma_splits() {
        let q = parse("access_rights=Full+Open+Access%2CMetadata+only+Access&search=a+b");
        assert_eq!(q.access_rights(), vec!["Full Open Access", "Metadata only Access"]);
        assert_eq!(q.search(), "a b");
    }

    #[test]
    fn empty_values_leave_the_facet_unset() {
        let q = parse("type_of_data=&data_language=,");
        assert_eq!(q.type_of_data, None);
        assert_eq!(q.data_language, None);
    }

    #[test]
    fn malformed_and_unknown_values_are_ignored() {
        let q = parse("page=abc&ongoing=yes&finished=1&dialog=maybe&datastar=%7B%7D&foo=bar");
        assert_eq!(q, ProjectQuery::default());
        assert_eq!(parse("page=-2").page, None);
        assert_eq!(parse("page=0").page, None);
    }

    #[test]
    fn last_occurrence_wins_for_single_valued_keys() {
        let q = parse("page=2&page=3&search=a&search=b&ongoing=true&ongoing=false");
        assert_eq!(q.page, Some(3));
        assert_eq!(q.search(), "b");
        assert_eq!(q.ongoing, None);
        let q = parse("finished=true&dialog=true");
        assert!(q.finished());
        assert_eq!(q.dialog, Some(true));
    }

    #[test]
    fn has_filter_params_is_true_for_each_facet_status_search_and_dialog() {
        for raw in [
            "type_of_data=Text",
            "data_language=en",
            "access_rights=A",
            "ongoing=true",
            "finished=true",
            "search=x",
            "dialog=true",
        ] {
            assert!(parse(raw).has_filter_params(), "{raw}");
        }
    }

    #[test]
    fn has_filter_params_is_false_for_the_list_and_a_page_alone() {
        assert!(!ProjectQuery::default().has_filter_params());
        assert!(!parse("page=2").has_filter_params());
        assert!(!parse("search=").has_filter_params());
    }

    #[test]
    fn to_query_string_percent_encodes_every_value_and_keeps_the_comma_separator() {
        let q = ProjectQuery {
            search: Some("a b".to_string()),
            access_rights: Some("Full Open Access,x'<y&z".to_string()),
            ..Default::default()
        };
        assert_eq!(
            q.to_query_string(),
            "?search=a%20b&access_rights=Full%20Open%20Access,x%27%3Cy%26z"
        );
    }

    #[test]
    fn to_query_string_keeps_the_parameter_order() {
        let q = parse("access_rights=A&data_language=en&type_of_data=T&page=2&search=s&finished=true&ongoing=true");
        assert_eq!(
            q.to_query_string(),
            "?ongoing=true&finished=true&search=s&page=2&type_of_data=T&data_language=en&access_rights=A"
        );
    }

    #[test]
    fn to_query_string_holds_only_the_url_safe_character_set() {
        let q = ProjectQuery {
            search: Some("</script>'\"`ü".to_string()),
            type_of_data: Some("a b,c\\d".to_string()),
            data_language: Some("x-cpa,ka".to_string()),
            access_rights: Some("é,<>".to_string()),
            page: Some(4),
            ongoing: Some(true),
            ..Default::default()
        };
        let qs = q.to_query_string();
        assert!(
            qs.chars().all(|c| c.is_ascii_alphanumeric() || "-_.~%,&=?".contains(c)),
            "{qs}"
        );
    }

    #[test]
    fn canonical_string_round_trips_through_the_parser() {
        let q = parse("type_of_data=a+b&type_of_data=c&data_language=ka,x-cpa&search=%27x%27&page=3");
        assert_eq!(parse(&q.to_query_string()[1..]), q);
    }

    #[test]
    fn no_committed_facet_option_value_contains_a_comma() {
        let corpus = crate::test_support::test_corpus();
        let values = super::super::list_type_of_data(corpus)
            .into_iter()
            .chain(super::super::list_data_languages(corpus).into_iter().map(|(code, _)| code))
            .chain(shared_metadata::ACCESS_RIGHTS_VALUES.iter().map(|v| v.to_string()));
        for value in values {
            assert!(!value.contains(','), "a comma-joined facet cannot round-trip {value:?}");
        }
    }
}
