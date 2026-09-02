//! What the form knows about each project field, and how the fields are grouped.
//!
//! Data, not rendering. Every editable `ProjectRaw` member has one [`Field`]
//! here — its label, its help text, whether it is required, whether it is
//! display-only, and who may see it — and every [`Section`] is an ordered list
//! of field ids. Which *control* a field renders is a separate concern, keyed by
//! the same ids; that split is the prototype's `FIELD_META` / `FIELD_RENDERERS`
//! shape, and it is what lets the grouping change without touching a renderer.
//!
//! ## The vocabulary is the prototype's
//!
//! Labels and hints are taken from `dsp-incubator/metadata-editor-v2`'s actual
//! screens rather than paraphrased, because REQ-2.1 and REQ-2.2 make the
//! depositor-facing wording normative and the prototype is what was validated
//! with users. Where the prototype's own summary and its screens disagree, the
//! screens win.
//!
//! ## Grouping
//!
//! One scheme, the prototype's `dpe` default: the sections mirror the published
//! project page, so the structure a depositor edits in is the structure they
//! read in. The prototype carried two further schemes behind a developer panel
//! for comparison; they are not product surface and are not here.
//!
//! ## Three fields are absent on purpose
//!
//! `records`, `clusters` and `collections` are omitted entirely (REQ-1.6) —
//! [`OMITTED`] names them so the completeness test can tell "decided against"
//! from "forgotten". They still ride through a draft untouched (REQ-1.7); the
//! omission is from the *form*, not from the data.

use Audience::{Everyone, RduOnly};
use Obligation::{Optional, Recommended, Required};

/// How much a field is expected of a depositor.
///
/// The prototype distinguished "required" from "required before publishing" and
/// then collapsed the two, because a depositor cannot act on the difference: a
/// draft may be missing anything (REQ-1.9), and everything in both tiers has to
/// be there to submit. What is left is a single required tier plus two degrees
/// of encouragement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Obligation {
    /// Must be present to submit.
    Required,
    /// Encouraged, and never blocks a submission.
    Recommended,
    /// Add if relevant.
    Optional,
}

impl Obligation {
    /// The word shown beside the field. Always a word, never colour alone.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Required => "Required",
            Self::Recommended => "Recommended",
            Self::Optional => "Optional",
        }
    }

    /// The one-line explanation of the tier, for the obligation key.
    #[must_use]
    pub const fn note(self) -> &'static str {
        match self {
            Self::Required => "needed for a complete record",
            Self::Recommended => "encouraged, but not required",
            Self::Optional => "add if relevant",
        }
    }
}

/// Who a field is shown to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {
    /// Every account that may reach the project.
    Everyone,
    /// RDU members only.
    ///
    /// Not a permission — a depositor cannot reach these whatever they post,
    /// because the decoder consults this registry too. It is here rather than in
    /// the renderer so that the form and the decoder cannot disagree about which
    /// fields a depositor owns.
    RduOnly,
}

/// One project field, as the form treats it.
#[derive(Debug, Clone, Copy)]
pub struct Field {
    /// The `ProjectRaw` JSON member name, or a dotted path into one. Also the
    /// key a renderer and a per-field error are looked up by.
    pub id: &'static str,
    /// The label, as the prototype's screens word it.
    pub label: &'static str,
    /// Help text below the control, or `None` where the label says it all.
    pub hint: Option<&'static str>,
    /// `None` for a display-only field: nothing is expected of a reader who
    /// cannot change it, and a "Required" pill beside a value they cannot
    /// supply is an instruction they cannot follow.
    pub obligation: Option<Obligation>,
    /// Shown as a value, never as a control (REQ-1.5). Written back unchanged
    /// (REQ-1.7).
    pub display_only: bool,
    pub audience: Audience,
}

impl Field {
    /// Whether a depositor may change this field.
    #[must_use]
    pub const fn editable_by_depositor(&self) -> bool {
        !self.display_only && matches!(self.audience, Audience::Everyone)
    }
}

/// A group of fields, in the order they are shown.
#[derive(Debug, Clone, Copy)]
pub struct Section {
    /// The URL segment: `/projects/{shortcode}/sections/{id}`.
    pub id: &'static str,
    pub title: &'static str,
    /// Field ids, in display order. Every one resolves through [`field`].
    pub fields: &'static [&'static str],
    /// RDU-only sections are absent from a depositor's rail entirely, rather
    /// than present and empty.
    pub audience: Audience,
}

/// Shorthand for an editable field.
///
/// Every editable field has a hint, which is why there is no hintless variant:
/// a control whose label alone is enough has not turned up, and the fields where
/// it might (`name`, `status`) are exactly the ones a depositor most needs told
/// what goes in them.
const fn hinted(
    id: &'static str,
    label: &'static str,
    hint: &'static str,
    obligation: Obligation,
    audience: Audience,
) -> Field {
    Field {
        id,
        label,
        hint: Some(hint),
        obligation: Some(obligation),
        display_only: false,
        audience,
    }
}

/// Shorthand for a display-only field (REQ-1.5).
const fn shown(id: &'static str, label: &'static str, hint: Option<&'static str>, audience: Audience) -> Field {
    Field {
        id,
        label,
        hint,
        obligation: None,
        display_only: true,
        audience,
    }
}

/// Every field the form knows, in no particular order — [`SECTIONS`] decides
/// display order.
pub const FIELDS: &[Field] = &[
    // --- Identity -----------------------------------------------------------
    shown(
        "id",
        "Internal ID",
        Some("The record's identifier in this repository."),
        RduOnly,
    ),
    shown(
        "pid",
        "PID",
        Some("The project's persistent identifier. Assigned by DaSCH on first publication."),
        Everyone,
    ),
    shown(
        "shortcode",
        "Shortcode",
        Some("Short project identifier — set by DaSCH when the project is created."),
        RduOnly,
    ),
    hinted("name", "Name", "Full, human-readable project title.", Required, Everyone),
    hinted(
        "officialName",
        "Official name",
        "Official legal name of the project — the formal registered title, for example as filed with the \
         funder. Kept for records, and not shown on the public project page.",
        Recommended,
        Everyone,
    ),
    hinted(
        "alternativeNames",
        "Alternative names",
        "Acronyms or alternate spellings — one value per language.",
        Optional,
        Everyone,
    ),
    // --- Descriptions -------------------------------------------------------
    hinted(
        "shortDescription",
        "Short description",
        "One-line teaser for cards and listings. Up to 200 characters.",
        Required,
        Everyone,
    ),
    hinted(
        "description",
        "Description",
        "Long form — at least one language. Required before publishing.",
        Required,
        Everyone,
    ),
    hinted("abstract", "Abstract", "Short, citation-ready summary.", Recommended, Everyone),
    hinted(
        "keywords",
        "Keywords",
        "At least one keyword — each is one multilingual term.",
        Required,
        Everyone,
    ),
    // --- Links and citation -------------------------------------------------
    shown(
        "howToCite",
        "How to cite",
        Some(
            "Composed automatically from your contributors, title, dates and PID — finalized when your \
             project is published. You don't need to fill this in.",
        ),
        Everyone,
    ),
    hinted(
        "url",
        "DaSCH project URL",
        "Link to this project in the DaSCH platform — usually an app.dasch.swiss address. Required before \
         publishing.",
        Required,
        RduOnly,
    ),
    hinted(
        "secondaryUrl",
        "External project website",
        "The project's own website outside DaSCH, if it has one.",
        Optional,
        Everyone,
    ),
    // --- Status, dates and access ------------------------------------------
    hinted(
        "status",
        "Status",
        "Whether the project is ongoing or finished — required even for a draft.",
        Required,
        Everyone,
    ),
    hinted(
        "startDate",
        "Start date",
        "When the project began. Required even for a draft.",
        Required,
        Everyone,
    ),
    hinted(
        "endDate",
        "End date",
        "Leave empty while the project is ongoing.",
        Optional,
        Everyone,
    ),
    hinted(
        "dataPublicationYear",
        "Data publication year",
        "Year the dataset was published.",
        Recommended,
        Everyone,
    ),
    hinted(
        "accessRights",
        "Access rights",
        "How openly the data can be accessed.",
        Required,
        Everyone,
    ),
    hinted(
        "accessRights.embargoDate",
        "Embargo release date",
        "When the data becomes openly available. Needed for embargoed access.",
        Optional,
        Everyone,
    ),
    hinted(
        "dataManagementPlan",
        "Data management plan",
        "Link to the DMP.",
        Optional,
        Everyone,
    ),
    // --- The dataset --------------------------------------------------------
    hinted(
        "typeOfData",
        "Type of data",
        "Kind or kinds of data in the dataset — required before publishing.",
        Required,
        Everyone,
    ),
    hinted(
        "dataLanguage",
        "Data languages",
        "The languages of the data itself. Pick from the list or add any other language. Required before \
         publishing.",
        Required,
        Everyone,
    ),
    hinted(
        "disciplines",
        "Disciplines",
        "At least one — pick from the SNSF or UNESCO discipline lists, or add your own if nothing fits.",
        Required,
        Everyone,
    ),
    hinted(
        "temporalCoverage",
        "Temporal coverage",
        "The time period the data covers. Search for a recognised period, for example Bronze Age or Siècle \
         des Lumières, and we'll record the authority link — or add your own term per language. Required \
         before publishing.",
        Required,
        Everyone,
    ),
    hinted(
        "spatialCoverage",
        "Spatial coverage",
        "Search for a place; we'll record the standard reference link for you. Required before publishing.",
        Required,
        Everyone,
    ),
    hinted(
        "provenance",
        "Provenance",
        "Where the data came from, or how it was produced.",
        Recommended,
        Everyone,
    ),
    hinted(
        "documentationMaterial",
        "Documentation material",
        "Documentation, codebooks, guides. Compiled by RDU on the depositor's behalf.",
        Required,
        RduOnly,
    ),
    hinted(
        "additionalMaterial",
        "Additional material",
        "Any additional material related to the dataset — related datasets, mirrors, sister projects.",
        Optional,
        Everyone,
    ),
    // --- People and publications -------------------------------------------
    hinted(
        "attributions",
        "Contributors",
        "The people and organisations involved, each with their role or roles. Required before publishing.",
        Required,
        Everyone,
    ),
    hinted(
        "contactPoint",
        "Contact point",
        "A person or organisation users should contact about the data.",
        Required,
        Everyone,
    ),
    hinted(
        "publications",
        "Publications",
        "Bibliographic references, each with an optional persistent identifier.",
        Optional,
        Everyone,
    ),
    // --- Funding ------------------------------------------------------------
    hinted(
        "funding",
        "Funding",
        "At least one funder is required before publishing — funder organisation or organisations, plus \
         grant number, programme and URL.",
        Required,
        Everyone,
    ),
    // --- Image and legal ----------------------------------------------------
    hinted(
        "imageCredit",
        "Project image credit",
        "Who the project image belongs to and on what terms it may be shown — a licence and a copyright \
         holder at minimum. The licence governs the image, separately from the dataset licence.",
        Optional,
        Everyone,
    ),
    shown(
        "legalInfo",
        "Legal info",
        Some("The dataset's licence, copyright holder and authorship. Maintained by RDU."),
        RduOnly,
    ),
];

/// The `ProjectRaw` members the form does not show at all (REQ-1.6).
///
/// Named rather than merely absent, so [`tests::every_contract_field_is_placed_or_omitted`]
/// can tell a deliberate omission from a forgotten field.
pub const OMITTED: &[&str] = &["records", "clusters", "collections"];

/// The sections, in the order the rail shows them.
///
/// The prototype's `dpe` scheme: the grouping mirrors the published project
/// page, so the structure a depositor edits in is the structure they read in.
pub const SECTIONS: &[Section] = &[
    Section {
        id: "overview",
        title: "Overview",
        audience: Everyone,
        fields: &[
            "id",
            "pid",
            "shortcode",
            "name",
            "officialName",
            "alternativeNames",
            "shortDescription",
            "description",
            "abstract",
            "url",
            "secondaryUrl",
            "status",
            "startDate",
            "endDate",
        ],
    },
    Section {
        id: "dataset",
        title: "Dataset",
        audience: Everyone,
        fields: &[
            "typeOfData",
            "dataLanguage",
            "dataPublicationYear",
            "keywords",
            "disciplines",
            "temporalCoverage",
            "spatialCoverage",
            "documentationMaterial",
            "additionalMaterial",
            "provenance",
        ],
    },
    Section {
        id: "publications",
        title: "Publications",
        audience: Everyone,
        fields: &["publications"],
    },
    Section {
        id: "contributors",
        title: "Contributors",
        audience: Everyone,
        fields: &["attributions", "contactPoint"],
    },
    Section {
        id: "access",
        title: "Access, citation and funding",
        audience: Everyone,
        fields: &[
            "accessRights",
            "accessRights.embargoDate",
            "howToCite",
            "funding",
            "dataManagementPlan",
        ],
    },
    Section {
        id: "image",
        title: "Project image",
        audience: Everyone,
        fields: &["imageCredit"],
    },
    // Display-only and RDU-only: a depositor's rail does not show it at all.
    Section {
        id: "legal",
        title: "Legal info",
        audience: RduOnly,
        fields: &["legalInfo"],
    },
];

/// One field by id, or `None` for an id the form does not know.
#[must_use]
pub fn field(id: &str) -> Option<&'static Field> {
    FIELDS.iter().find(|field| field.id == id)
}

/// One section by id.
#[must_use]
pub fn section(id: &str) -> Option<&'static Section> {
    SECTIONS.iter().find(|section| section.id == id)
}

/// The sections this audience sees, in rail order.
pub fn sections_for(audience: Audience) -> impl Iterator<Item = &'static Section> {
    SECTIONS
        .iter()
        .filter(move |section| audience == Audience::RduOnly || section.audience == Audience::Everyone)
}

/// The first section this audience sees — where `/projects/{shortcode}` lands.
#[must_use]
pub fn first_section(audience: Audience) -> &'static Section {
    sections_for(audience).next().expect("every audience sees at least one section")
}

impl Section {
    /// This section's fields, resolved and filtered to what `audience` sees, in
    /// display order.
    pub fn fields_for(&self, audience: Audience) -> impl Iterator<Item = &'static Field> + '_ {
        self.fields.iter().filter_map(move |id| {
            let field = field(id)?;
            match (audience, field.audience) {
                (Audience::RduOnly, _) | (_, Audience::Everyone) => Some(field),
                (Audience::Everyone, Audience::RduOnly) => None,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// Every JSON member `ProjectRaw` serializes, read off a real committed
    /// project rather than listed here — a field added to the contract has to
    /// show up without this file being edited.
    ///
    /// `ProjectRaw` carries no `skip_serializing_if`, so an unset `Option`
    /// serializes as `null` and is still a member: the set is the whole contract
    /// and not just the parts this project happens to fill in.
    fn contract_members() -> BTreeSet<String> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");
        let project = published.get("0801d").expect("0801d is in the committed corpus");
        let value = serde_json::to_value(project).expect("ProjectRaw serializes");
        value.as_object().expect("ProjectRaw is an object").keys().cloned().collect()
    }

    /// A registry id's top-level contract member: `accessRights.embargoDate` is
    /// part of `accessRights`.
    fn root_of(id: &str) -> &str {
        id.split_once('.').map_or(id, |(root, _)| root)
    }

    #[test]
    fn every_contract_field_is_either_placed_in_the_form_or_deliberately_omitted() {
        // REQ-1.4: the form exposes every field that is not display-only or
        // hidden. This is the test that fails when a field is added to
        // `ProjectRaw` and nobody decides where it goes — without it the new
        // field is silently uneditable, and the only symptom is a depositor
        // unable to enter something.
        let placed: BTreeSet<&str> = FIELDS.iter().map(|field| root_of(field.id)).collect();
        let omitted: BTreeSet<&str> = OMITTED.iter().copied().collect();
        let unplaced: Vec<String> = contract_members()
            .into_iter()
            .filter(|member| !placed.contains(member.as_str()) && !omitted.contains(member.as_str()))
            .collect();
        assert!(
            unplaced.is_empty(),
            "these ProjectRaw fields are in neither FIELDS nor OMITTED: {unplaced:?}"
        );
    }

    #[test]
    fn nothing_is_registered_that_the_contract_does_not_have() {
        // The other direction: a field renamed in the contract leaves a registry
        // entry that renders a control posting to nothing.
        let members = contract_members();
        let unknown: Vec<&str> = FIELDS
            .iter()
            .map(|field| root_of(field.id))
            .filter(|root| !members.contains(*root))
            .collect();
        assert!(unknown.is_empty(), "these registry ids are not ProjectRaw members: {unknown:?}");
    }

    #[test]
    fn the_omitted_fields_are_exactly_the_three_req_1_6_names() {
        assert_eq!(OMITTED, ["records", "clusters", "collections"]);
        for omitted in OMITTED {
            assert!(field(omitted).is_none(), "{omitted} must not be a form field");
        }
    }

    #[test]
    fn the_display_only_fields_are_exactly_the_five_req_1_5_names() {
        // REQ-1.5 names id, pid, shortcode, howToCite and legalInfo. A sixth
        // would be a field a depositor can no longer edit, which is a
        // requirement change rather than an implementation detail.
        let display_only: BTreeSet<&str> =
            FIELDS.iter().filter(|field| field.display_only).map(|field| field.id).collect();
        assert_eq!(
            display_only,
            BTreeSet::from(["id", "pid", "shortcode", "howToCite", "legalInfo"])
        );
    }

    #[test]
    fn a_display_only_field_carries_no_obligation() {
        // A "Required" pill beside a value the reader cannot supply is an
        // instruction they cannot follow.
        for field in FIELDS.iter().filter(|field| field.display_only) {
            assert_eq!(field.obligation, None, "{} should carry no obligation", field.id);
        }
    }

    #[test]
    fn an_editable_field_always_carries_an_obligation() {
        // The rail's per-section state is computed from these, so a field with
        // none is invisible to it.
        for field in FIELDS.iter().filter(|field| !field.display_only) {
            assert!(field.obligation.is_some(), "{} should carry an obligation", field.id);
        }
    }

    #[test]
    fn every_field_id_is_unique() {
        let mut seen = BTreeSet::new();
        for field in FIELDS {
            assert!(seen.insert(field.id), "duplicate field id {}", field.id);
        }
    }

    #[test]
    fn every_field_appears_in_exactly_one_section() {
        // Twice and it renders twice, posting two values for one field; never
        // and it is unreachable while looking registered.
        let mut placements: Vec<(&str, Vec<&str>)> = FIELDS
            .iter()
            .map(|field| {
                let sections: Vec<&str> = SECTIONS
                    .iter()
                    .filter(|section| section.fields.contains(&field.id))
                    .map(|section| section.id)
                    .collect();
                (field.id, sections)
            })
            .collect();
        placements.retain(|(_, sections)| sections.len() != 1);
        assert!(
            placements.is_empty(),
            "each field belongs in exactly one section: {placements:?}"
        );
    }

    #[test]
    fn every_section_field_id_resolves() {
        // A typo here renders a section with a field silently missing.
        for section in SECTIONS {
            for id in section.fields {
                assert!(field(id).is_some(), "section {} names unknown field {id}", section.id);
            }
        }
    }

    #[test]
    fn every_section_id_is_unique_and_url_safe() {
        // Section ids are a URL segment: `/projects/{shortcode}/sections/{id}`.
        let mut seen = BTreeSet::new();
        for section in SECTIONS {
            assert!(seen.insert(section.id), "duplicate section id {}", section.id);
            assert!(
                section.id.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "section id {} is not URL-safe",
                section.id
            );
        }
    }

    #[test]
    fn no_section_is_empty_for_the_audience_that_can_see_it() {
        // An empty section is a rail entry leading to a blank page.
        for section in sections_for(Audience::Everyone) {
            assert!(
                section.fields_for(Audience::Everyone).count() > 0,
                "section {} is empty for a depositor",
                section.id
            );
        }
        for section in sections_for(Audience::RduOnly) {
            assert!(
                section.fields_for(Audience::RduOnly).count() > 0,
                "section {} is empty for RDU",
                section.id
            );
        }
    }

    #[test]
    fn a_depositor_does_not_see_the_rdu_only_section_at_all() {
        // Present and empty would be a rail entry that goes nowhere.
        let visible: Vec<&str> = sections_for(Audience::Everyone).map(|s| s.id).collect();
        assert!(!visible.contains(&"legal"), "{visible:?}");
        let rdu: Vec<&str> = sections_for(Audience::RduOnly).map(|s| s.id).collect();
        assert!(rdu.contains(&"legal"), "{rdu:?}");
    }

    #[test]
    fn an_rdu_only_field_is_filtered_out_of_a_depositor_s_section() {
        let overview = section("overview").expect("overview");
        let depositor: Vec<&str> = overview.fields_for(Audience::Everyone).map(|f| f.id).collect();
        assert!(!depositor.contains(&"shortcode"), "{depositor:?}");
        assert!(!depositor.contains(&"url"), "{depositor:?}");
        assert!(depositor.contains(&"name"), "{depositor:?}");

        let rdu: Vec<&str> = overview.fields_for(Audience::RduOnly).map(|f| f.id).collect();
        assert!(rdu.contains(&"shortcode"), "{rdu:?}");
        assert!(rdu.contains(&"url"), "{rdu:?}");
    }

    #[test]
    fn fields_are_returned_in_the_section_s_declared_order() {
        let overview = section("overview").expect("overview");
        let ids: Vec<&str> = overview.fields_for(Audience::RduOnly).map(|f| f.id).collect();
        let declared: Vec<&str> = overview.fields.to_vec();
        assert_eq!(ids, declared);
    }

    #[test]
    fn the_first_section_is_the_same_one_for_both_audiences() {
        // `/projects/{shortcode}` redirects there, and a redirect that depends
        // on the role is one more thing to get wrong in a shared link.
        assert_eq!(first_section(Audience::Everyone).id, "overview");
        assert_eq!(first_section(Audience::RduOnly).id, "overview");
    }

    #[test]
    fn a_depositor_cannot_edit_a_display_only_or_rdu_only_field() {
        assert!(!field("legalInfo").expect("legalInfo").editable_by_depositor());
        assert!(!field("shortcode").expect("shortcode").editable_by_depositor());
        assert!(!field("url").expect("url").editable_by_depositor());
        assert!(field("name").expect("name").editable_by_depositor());
    }

    #[test]
    fn a_hint_reads_as_a_sentence() {
        // These are the depositor-facing wording REQ-2.1 makes normative, and a
        // hint that is a fragment or a stray placeholder reads as unfinished.
        for field in FIELDS {
            if let Some(hint) = field.hint {
                assert!(!hint.is_empty(), "{}: empty hint", field.id);
                assert!(
                    hint.ends_with('.') || hint.ends_with('?'),
                    "{}: hint should be a sentence: {hint:?}",
                    field.id
                );
                assert!(
                    hint.chars().next().is_some_and(char::is_uppercase),
                    "{}: hint should start with a capital: {hint:?}",
                    field.id
                );
                assert!(!hint.contains("TODO"), "{}: unfinished hint", field.id);
            }
            assert!(!field.label.is_empty(), "{}: empty label", field.id);
        }
    }

    #[test]
    fn obligation_labels_and_notes_are_stated_in_words() {
        // The tier is always a word beside the colour, never colour alone.
        for tier in [Required, Recommended, Optional] {
            assert!(!tier.label().is_empty());
            assert!(!tier.note().is_empty());
        }
        assert_eq!(Required.label(), "Required");
    }
}
