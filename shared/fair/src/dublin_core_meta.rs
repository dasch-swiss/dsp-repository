//! The `<meta name="DC.*">` tags a landing page carries.
//!
//! A second rendering of the Dublin Core record `oai_dc` already serves, not a
//! second mapping: [`project_to_dublin_core`] produces the record and this
//! module only says which of its fields become `<meta>` tags and in what order.
//! The one value it does not take from there is `DC.accessRights`, which is a
//! COAR URI the OAI record has no field for.

use crate::helpers::{coar_access_right, real};
use crate::project_graph::ProjectGraph;
use crate::{project_to_dublin_core, DublinCoreRecord};

/// `DC.description` is cut to this many characters. A `<meta>` attribute of
/// several kilobytes is legal and pointless; the full text is in the JSON-LD,
/// the OAI record and the page itself.
const DESCRIPTION_LIMIT: usize = 1000;

/// The name/content pairs for a project's `<meta name="DC.*">` tags.
///
/// Placeholders are dropped. `project_to_dublin_core` carries them through on
/// purpose — the committed `oai_dc` output has always shown them and moving
/// that is not this plan's business — but a landing page asserting
/// `DC.title = "MISSING"` to a harvester is a different claim, and a false one.
pub fn project_to_dublin_core_meta(graph: &ProjectGraph) -> Vec<(&'static str, String)> {
    let dc: DublinCoreRecord = project_to_dublin_core(graph);
    let mut tags = Vec::new();

    push_all(&mut tags, "DC.title", &dc.titles);
    push_all(&mut tags, "DC.creator", &dc.creators);
    push_all(&mut tags, "DC.publisher", std::slice::from_ref(&dc.publisher));
    push_all(&mut tags, "DC.identifier", &dc.identifiers);
    push_all(&mut tags, "DC.type", std::slice::from_ref(&dc.resource_type));
    push_all(&mut tags, "DC.date", &dc.dates);
    push_all(&mut tags, "DC.language", &dc.languages);

    for description in &dc.descriptions {
        if let Some(text) = real(description) {
            tags.push(("DC.description", truncate_on_char_boundary(text, DESCRIPTION_LIMIT)));
        }
    }

    push_all(&mut tags, "DC.rights", &dc.rights);
    tags.push(("DC.accessRights", coar_access_right(&graph.access_rights).to_string()));

    tags
}

fn push_all(tags: &mut Vec<(&'static str, String)>, name: &'static str, values: &[String]) {
    for value in values {
        if let Some(value) = real(value) {
            tags.push((name, value.to_string()));
        }
    }
}

/// The first `limit` **characters**, with an ellipsis when anything was cut.
///
/// Characters, not bytes: this corpus is German, French and Latin, and
/// `&text[..limit]` panics the moment the cut lands inside a multi-byte
/// character. That is the bug `extract_year` had.
fn truncate_on_char_boundary(text: &str, limit: usize) -> String {
    match text.char_indices().nth(limit) {
        None => text.to_string(),
        Some((byte_index, _)) => format!("{}…", &text[..byte_index]),
    }
}

#[cfg(test)]
mod tests {
    use shared_metadata::{AccessRights, AccessRightsType, Multilingual, ProjectRaw};

    use super::*;
    use crate::test_support::{build, english, legal, project};

    fn tags(raw: &ProjectRaw) -> Vec<(&'static str, String)> {
        project_to_dublin_core_meta(&build(raw, &[]))
    }

    fn values<'a>(tags: &'a [(&'static str, String)], name: &str) -> Vec<&'a str> {
        tags.iter()
            .filter(|(tag, _)| *tag == name)
            .map(|(_, value)| value.as_str())
            .collect()
    }

    #[test]
    fn the_core_elements_are_emitted() {
        let tags = tags(&project());
        // `oai_dc` prefers `officialName` and adds only the recorded
        // alternative names; it does not repeat `name` as an alternative the
        // way DataCite does. That choice is the writer's, and this is a second
        // rendering of that writer, not a second mapping.
        assert_eq!(
            values(&tags, "DC.title"),
            vec!["Rural Land Use in the Swiss Midlands, 1920-1950"]
        );
        assert_eq!(values(&tags, "DC.publisher"), vec!["DaSCH"]);
        assert_eq!(
            values(&tags, "DC.identifier"),
            vec!["https://ark.dasch.swiss/ark:/72163/1/0001"]
        );
        assert_eq!(values(&tags, "DC.type"), vec!["Project"]);
        assert_eq!(values(&tags, "DC.date"), vec!["2008-06-01"]);
        assert_eq!(values(&tags, "DC.language"), vec!["de", "fr"]);
        assert_eq!(values(&tags, "DC.description"), vec!["A study of rural land use."]);
    }

    #[test]
    fn access_rights_is_the_coar_uri_for_the_level() {
        for (rights, uri) in [
            (AccessRightsType::FullOpenAccess, "http://purl.org/coar/access_right/c_abf2"),
            (
                AccessRightsType::OpenAccessWithRestrictions,
                "http://purl.org/coar/access_right/c_16ec",
            ),
            (AccessRightsType::EmbargoedAccess, "http://purl.org/coar/access_right/c_f1cf"),
            (AccessRightsType::MetadataOnlyAccess, "http://purl.org/coar/access_right/c_14cb"),
        ] {
            let raw = ProjectRaw {
                access_rights: AccessRights { access_rights: rights, embargo_date: None },
                ..project()
            };
            assert_eq!(values(&tags(&raw), "DC.accessRights"), vec![uri]);
        }
    }

    #[test]
    fn one_rights_tag_per_license_beside_the_access_level() {
        let raw = ProjectRaw {
            legal_info: vec![
                legal("CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/"),
                legal("CC0-1.0", "https://creativecommons.org/publicdomain/zero/1.0/"),
            ],
            ..project()
        };
        assert_eq!(
            values(&tags(&raw), "DC.rights"),
            vec![
                "Full Open Access",
                "https://creativecommons.org/licenses/by/4.0/",
                "https://creativecommons.org/publicdomain/zero/1.0/"
            ]
        );
    }

    #[test]
    fn no_placeholder_becomes_a_meta_tag() {
        let raw = ProjectRaw {
            name: "MISSING".to_string(),
            official_name: "MISSING".to_string(),
            description: english("MISSING"),
            ..project()
        };
        let tags = tags(&raw);
        assert!(values(&tags, "DC.title").is_empty(), "{tags:?}");
        assert!(values(&tags, "DC.description").is_empty(), "{tags:?}");
    }

    /// Two characters, chosen for what a byte slice at `DESCRIPTION_LIMIT` does
    /// to each.
    ///
    /// `ä` is two bytes, so byte 1000 is a character boundary: `&text[..1000]`
    /// would not panic, it would silently return half the characters asked for.
    /// `—` is three bytes, so byte 1000 lands *inside* a character and
    /// `&text[..1000]` panics. Both are wrong, in different ways, and this
    /// corpus is full of both kinds of character — so both are pinned.
    #[test]
    fn a_long_description_is_cut_on_a_character_boundary() {
        for filler in ['ä', '—'] {
            assert!(filler.len_utf8() > 1, "the filler should be multi-byte");
            let text: String = filler.to_string().repeat(1500);
            let raw = ProjectRaw { description: english(&text), ..project() };
            let cut = values(&tags(&raw), "DC.description")[0].to_string();
            assert_eq!(cut.chars().count(), DESCRIPTION_LIMIT + 1, "{filler}");
            assert!(cut.ends_with('…'), "{filler}: {cut}");
            assert_eq!(cut.chars().filter(|c| *c == filler).count(), DESCRIPTION_LIMIT, "{filler}");
        }
    }

    /// The bug this guards against, stated directly: a byte slice at the limit
    /// is either a panic or a silent truncation to the wrong length, depending
    /// on how wide the characters happen to be.
    #[test]
    fn a_byte_slice_at_the_limit_would_have_been_wrong_both_ways() {
        let two_byte: String = "ä".repeat(1500);
        assert!(two_byte.is_char_boundary(DESCRIPTION_LIMIT));
        assert_eq!(two_byte[..DESCRIPTION_LIMIT].chars().count(), DESCRIPTION_LIMIT / 2);

        let three_byte: String = "—".repeat(1500);
        assert!(!three_byte.is_char_boundary(DESCRIPTION_LIMIT));
    }

    #[test]
    fn a_description_at_the_limit_is_not_cut() {
        for filler in ['ä', '—'] {
            let text: String = filler.to_string().repeat(DESCRIPTION_LIMIT);
            let raw = ProjectRaw { description: english(&text), ..project() };
            assert_eq!(values(&tags(&raw), "DC.description"), vec![text.as_str()], "{filler}");
        }
    }

    /// The same text in every representation, whatever language it is recorded
    /// in: the graph resolved it once with `multilingual_value`, which prefers
    /// English and otherwise takes the smallest language tag.
    #[test]
    fn a_non_english_description_is_the_one_every_writer_carries() {
        let raw = ProjectRaw {
            description: Multilingual::from([
                ("de".to_string(), "Eine Studie über ländliche Bodennutzung.".to_string()),
                ("fr".to_string(), "Une étude de l'utilisation des terres.".to_string()),
            ]),
            ..project()
        };
        let graph = build(&raw, &[]);
        let expected = "Eine Studie über ländliche Bodennutzung.";
        assert_eq!(graph.description.as_deref(), Some(expected));
        assert_eq!(values(&project_to_dublin_core_meta(&graph), "DC.description"), vec![expected]);
    }
}
