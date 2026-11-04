//! Where a depositor opened an entity proposal from, so "Back to the project" and a discard return
//! there instead of to the project's first section.

use editor_core::form::registry::{self, Audience, Field, Section};
use editor_core::form::{FormBody, Shape};

/// The query parameter, and the entity form's hidden input, naming the section.
pub(crate) const SECTION: &str = "section";
/// The query parameter, and the entity form's hidden input, naming the agent field whose card
/// opened the proposal.
pub(crate) const FIELD: &str = "field";

/// The section a proposal was opened from and, when it was a card's control, the agent field that
/// card sits in. It travels as ids rather than as a URL, and only ids the registry lists for the
/// reader's audience survive [`Origin::parse`], so every link built from it stays on this project's
/// own sections: an open redirect cannot be spelled in it.
#[derive(Debug, Clone, Copy)]
pub struct Origin {
    pub section: &'static Section,
    pub field: Option<&'static Field>,
}

impl Origin {
    pub(crate) const fn section(section: &'static Section) -> Self {
        Self { section, field: None }
    }

    /// `None` for a section this audience does not see or the registry does not list. A field that
    /// is not one of that section's agent fields is dropped and the section kept: the anchor is a
    /// convenience, the section is the destination.
    pub(crate) fn parse(audience: Audience, section: Option<&str>, field: Option<&str>) -> Option<Self> {
        let section = registry::sections_for(audience).find(|candidate| Some(candidate.id) == section)?;
        let field = field.and_then(|id| {
            section
                .fields_for(audience)
                .find(|candidate| candidate.id == id && candidate.shape.is_some_and(Shape::has_agent_picker))
        });
        Some(Self { section, field })
    }

    pub(crate) fn posted(audience: Audience, body: &FormBody) -> Option<Self> {
        Self::parse(audience, body.get(SECTION), body.get(FIELD))
    }

    /// The query string that carries this origin, without the leading `?`.
    pub(crate) fn query(&self) -> String {
        match self.field {
            Some(field) => format!("{SECTION}={}&{FIELD}={}", self.section.id, field.id),
            None => format!("{SECTION}={}", self.section.id),
        }
    }

    /// The section page, with `query` when given, at the card for `entity_id` when this origin
    /// names a field.
    pub(crate) fn section_url(&self, shortcode: &str, query: Option<&str>, entity_id: Option<&str>) -> String {
        let mut url = format!("/projects/{shortcode}/sections/{}", self.section.id);
        if let Some(query) = query {
            url.push('?');
            url.push_str(query);
        }
        if let (Some(field), Some(entity_id)) = (self.field, entity_id) {
            url.push('#');
            url.push_str(&crate::form::widgets::card_anchor(field.id, entity_id));
        }
        url
    }
}

pub(crate) fn entity_url(shortcode: &str, entity_id: &str, origin: Option<Origin>) -> String {
    match origin {
        Some(origin) => format!("/projects/{shortcode}/entities/{entity_id}?{}", origin.query()),
        None => format!("/projects/{shortcode}/entities/{entity_id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_and_agent_field_the_audience_sees_survive() {
        let origin = Origin::parse(Audience::Everyone, Some("contributors"), Some("attributions")).expect("an origin");
        assert_eq!(origin.query(), "section=contributors&field=attributions");
        assert_eq!(
            origin.section_url("0801", None, Some("person-001")),
            "/projects/0801/sections/contributors#attributions-card-person-001"
        );
    }

    #[test]
    fn anything_but_a_listed_section_is_no_origin() {
        for section in ["", "//evil.example", "https://evil.example", "../people", "nope"] {
            assert!(Origin::parse(Audience::Everyone, Some(section), None).is_none(), "{section}");
        }
    }

    #[test]
    fn a_field_that_is_not_an_agent_field_of_that_section_is_dropped() {
        for field in ["description", "nope", "\"><script>"] {
            let origin = Origin::parse(Audience::Everyone, Some("contributors"), Some(field)).expect("an origin");
            assert!(origin.field.is_none(), "{field}");
            assert_eq!(
                origin.section_url("0801", None, Some("person-001")),
                "/projects/0801/sections/contributors"
            );
        }
    }
}
