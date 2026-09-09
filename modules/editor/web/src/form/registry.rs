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
//! **One deliberate departure: a hint never states an obligation.** The
//! prototype's hints carried phrases like "Required before publishing" and
//! "required even for a draft", which were written against its two required
//! tiers. The pill now states the tier and [`Obligation::Required`] is enforced
//! at submit, so a hint repeating it is at best redundant and was at worst
//! wrong in two directions at once: a hint saying "before publishing" beside a
//! pill that blocks a *submission* understates the gate, and "required even for
//! a draft" contradicts the draft's own permissiveness — a draft may be missing anything, which is
//! why no control here carries `required`. The hints say what goes in the field;
//! the pill says whether it has to be there.
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

use editor_core::draft::UrlSlot;
use editor_core::form::{ChoiceSet, Shape, WhenCleared};
use editor_core::multilingual::UI_LANGUAGES;
use platform_metadata::project::{ACCESS_RIGHTS_VALUES, PROJECT_STATUS_VALUES, TYPE_OF_DATA_VALUES};
use Audience::{Everyone, RduOnly};
use Obligation::{Optional, Recommended, Required};
use Shape::{
    AgentRows, AttributionRows, Choice, FundingRows, Multilingual, MultilingualRows, PublicationRows, ReferenceRows,
    StringList, StringRows, Text, TextOrReferenceRows, Url,
};

/// A required contract `String` whose empty state the committed data spells as a
/// sentinel. Named because the distinction reads as the UI tier beside it and is
/// not: `officialName` is `Recommended` and still needs this.
const REQUIRED_STRING: Shape = Text(WhenCleared::Placeholder);

/// An `Option<String>` on the contract, where absent is what unset means.
const OPTIONAL_STRING: Shape = Text(WhenCleared::Drop);

/// The authority sources a `spatialCoverage` reference may come from, spelled as the committed data
/// spells them.
const PLACE_SOURCES: &[&str] = &["Geonames", "Pleiades", "Gazetteer", "URL"];

/// The authority sources a `temporalCoverage` reference may come from, spelled as the committed
/// data spells them.
///
/// A closed offer, unlike the role vocabulary beside it: these are the
/// resolvers the platform actually consults, and an unknown one is a link
/// nothing can dereference. `URL` is kept because a committed entry uses it and
/// dropping it would refuse that project.
const PERIOD_SOURCES: &[&str] = &["Chronontology", "Periodo", "URL"];

/// The authority sources a `disciplines` reference may come from. Every committed reference is
/// `Skos` — the SNSF and UNESCO vocabularies are both published as SKOS.
const DISCIPLINE_SOURCES: &[&str] = &["Skos"];

/// How much a field is expected of a depositor.
///
/// The prototype distinguished "required" from "required before publishing" and
/// then collapsed the two, because a depositor cannot act on the difference: a
/// draft may be missing anything, and everything in both tiers has to be there
/// to submit. What is left is a single required tier plus two degrees of
/// encouragement.
///
/// ## The required tier is bounded by what the published corpus satisfies
///
/// **A field is `Required` only if the published corpus answers it.** [`Obligation::Required`] is a
/// literal submit gate, so tiering a field the published projects leave unset would refuse every
/// one of them — an unenforceable tier that therefore goes unenforced. Such a field is
/// `Recommended`, with its publication requirement stated in the hint: a different gate, owned by
/// RDU at publication.
///
/// `obligation::tests::the_required_fields_the_committed_corpus_does_not_answer_are_the_measured_ones` keeps it
/// true — it fails the day a field is tiered `Required` that the corpus cannot answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Obligation {
    /// Must be present to submit — a literal gate, not an encouragement.
    Required,
    /// Encouraged, and never blocks a submission.
    ///
    /// Also where a field the repository needs before *publishing* sits, when
    /// the published corpus shows it is not there today. The hint carries that;
    /// see the tier note above.
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
    /// How a posted body is read back into this field, or `None` for a field no
    /// applier touches — a display-only one, or one whose control has not landed
    /// ([`NOT_READ_YET`]).
    ///
    /// The single declaration of the field's contract shape and of what clearing
    /// it means; the round-trip test derives its table from here rather than
    /// keeping a second copy.
    pub shape: Option<Shape>,
}

impl Field {
    /// Whether a depositor may change this field.
    #[must_use]
    pub const fn editable_by_depositor(&self) -> bool {
        !self.display_only && matches!(self.audience, Audience::Everyone)
    }

    /// Whether the form renders a control for this field at all.
    ///
    /// False for a display-only field and for one the form does not read yet;
    /// the two render differently (a value against a stated note), which is why
    /// [`Self::display_only`] stays a separate fact rather than being inferred
    /// from an absent shape.
    #[must_use]
    pub const fn is_editable(&self) -> bool {
        self.shape.is_some()
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
    shape: Option<Shape>,
) -> Field {
    Field {
        id,
        label,
        hint: Some(hint),
        obligation: Some(obligation),
        display_only: false,
        audience,
        shape,
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
        // Nothing reads a display-only field back, which is what REQ-1.5 and
        // REQ-1.7 together say: shown as a value, written back unchanged.
        shape: None,
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
    hinted(
        "name",
        "Name",
        "Full, human-readable project title.",
        Required,
        Everyone,
        Some(REQUIRED_STRING),
    ),
    hinted(
        "officialName",
        "Official name",
        "Official legal name of the project — the formal registered title, for example as filed with the \
         funder. Kept for records, and not shown on the public project page.",
        Recommended,
        Everyone,
        Some(REQUIRED_STRING),
    ),
    hinted(
        "alternativeNames",
        "Alternative names",
        "Acronyms or alternate spellings — one value per language.",
        Optional,
        Everyone,
        Some(MultilingualRows),
    ),
    // --- Descriptions -------------------------------------------------------
    hinted(
        "shortDescription",
        "Short description",
        "One-line teaser for cards and listings. Up to 200 characters.",
        Required,
        Everyone,
        Some(REQUIRED_STRING),
    ),
    hinted(
        "description",
        "Description",
        "Long form — at least one language.",
        Required,
        Everyone,
        Some(Multilingual),
    ),
    hinted(
        "abstract",
        "Abstract",
        "Short, citation-ready summary.",
        Recommended,
        Everyone,
        Some(Multilingual),
    ),
    hinted(
        "keywords",
        "Keywords",
        "At least one keyword — each is one multilingual term.",
        Required,
        Everyone,
        Some(MultilingualRows),
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
    // `Recommended`, not `Required`: published projects without a `url` exist, so a submit gate on it would refuse
    // them. See `Obligation`.
    hinted(
        "url",
        "DaSCH project URL",
        "Link to this project in the DaSCH platform — usually an app.dasch.swiss address. Needed before the \
         project can be published.",
        Recommended,
        RduOnly,
        Some(Url(UrlSlot::Primary)),
    ),
    hinted(
        "secondaryUrl",
        "External project website",
        "The project's own website outside DaSCH, if it has one.",
        Optional,
        Everyone,
        Some(Url(UrlSlot::Secondary)),
    ),
    // --- Status, dates and access ------------------------------------------
    hinted(
        "status",
        "Status",
        "Whether the project is ongoing or finished.",
        Required,
        Everyone,
        Some(Choice(PROJECT_STATUS_VALUES)),
    ),
    hinted(
        "startDate",
        "Start date",
        "When the project began.",
        Required,
        Everyone,
        Some(REQUIRED_STRING),
    ),
    hinted(
        "endDate",
        "End date",
        "Leave empty while the project is ongoing.",
        Optional,
        Everyone,
        Some(REQUIRED_STRING),
    ),
    hinted(
        "dataPublicationYear",
        "Data publication year",
        "Year the dataset was published.",
        Recommended,
        Everyone,
        Some(OPTIONAL_STRING),
    ),
    // The id is the nested member holding the choice, not the object around it:
    // `accessRights` is `{accessRights, embargoDate}`, so a shape on the object
    // would have to write the whole thing and would take the embargo date with
    // it. `ProjectDraft` follows dotted ids, so naming the member is all this
    // needs — the same shape `accessRights.embargoDate` beside it already had.
    hinted(
        "accessRights.accessRights",
        "Access rights",
        "How openly the data can be accessed.",
        Required,
        Everyone,
        Some(Choice(ACCESS_RIGHTS_VALUES)),
    ),
    // `Optional`, and deliberately not gated on the choice beside it: almost no published project with "Embargoed
    // Access" carries a date, so a rule requiring one would refuse them all. Same lesson as the tier note on
    // `Obligation`.
    hinted(
        "accessRights.embargoDate",
        "Embargo release date",
        "When the data becomes openly available. Needed for embargoed access.",
        Optional,
        Everyone,
        Some(OPTIONAL_STRING),
    ),
    hinted(
        "dataManagementPlan",
        "Data management plan",
        "Link to the DMP.",
        Optional,
        Everyone,
        Some(OPTIONAL_STRING),
    ),
    // --- The dataset --------------------------------------------------------
    // `Recommended` for both: `082C_decoso` is published with neither, so a
    // submit gate on them would refuse it. See `Obligation`.
    hinted(
        "typeOfData",
        "Type of data",
        "Kind or kinds of data in the dataset — needed before the project can be published.",
        Recommended,
        Everyone,
        Some(StringList(ChoiceSet::Closed(TYPE_OF_DATA_VALUES))),
    ),
    hinted(
        "dataLanguage",
        "Data languages",
        "The languages of the data itself. Pick from the list or add any other language. Needed before the \
         project can be published.",
        Recommended,
        Everyone,
        Some(StringList(ChoiceSet::Open(&UI_LANGUAGES))),
    ),
    hinted(
        "disciplines",
        "Disciplines",
        "At least one — pick from the SNSF or UNESCO discipline lists, or add your own if nothing fits.",
        Required,
        Everyone,
        Some(TextOrReferenceRows(DISCIPLINE_SOURCES)),
    ),
    hinted(
        "temporalCoverage",
        "Temporal coverage",
        "The time period the data covers. Search for a recognised period, for example Bronze Age or Siècle \
         des Lumières, and we'll record the authority link — or add your own term per language.",
        Required,
        Everyone,
        Some(TextOrReferenceRows(PERIOD_SOURCES)),
    ),
    hinted(
        "spatialCoverage",
        "Spatial coverage",
        "Search for a place; we'll record the standard reference link for you.",
        Required,
        Everyone,
        Some(ReferenceRows(PLACE_SOURCES)),
    ),
    hinted(
        "provenance",
        "Provenance",
        "Where the data came from, or how it was produced.",
        Recommended,
        Everyone,
        Some(OPTIONAL_STRING),
    ),
    // `Recommended`: no published project has it, which makes it the clearest case of the tier note on
    // `Obligation` — RDU compiles it, and gating a submission on it would refuse the entire live corpus.
    hinted(
        "documentationMaterial",
        "Documentation material",
        "Documentation, codebooks, guides. Compiled by RDU on the depositor's behalf.",
        Recommended,
        RduOnly,
        Some(StringRows),
    ),
    hinted(
        "additionalMaterial",
        "Additional material",
        "Any additional material related to the dataset — related datasets, mirrors, sister projects.",
        Optional,
        Everyone,
        Some(StringRows),
    ),
    // --- People and publications -------------------------------------------
    hinted(
        "attributions",
        "Contributors",
        "The people and organisations involved, each with their role or roles.",
        Required,
        Everyone,
        Some(AttributionRows),
    ),
    // `Recommended`: published projects without a `contactPoint` exist. See `Obligation`.
    hinted(
        "contactPoint",
        "Contact point",
        "A person or organisation users should contact about the data. Needed before the project can be \
         published.",
        Recommended,
        Everyone,
        Some(AgentRows),
    ),
    hinted(
        "publications",
        "Publications",
        "Bibliographic references, each with an optional persistent identifier.",
        Optional,
        Everyone,
        Some(PublicationRows),
    ),
    // --- Funding ------------------------------------------------------------
    hinted(
        "funding",
        "Funding",
        "At least one funder — funder organisation or organisations, plus grant number, programme and \
         URL.",
        Required,
        Everyone,
        Some(FundingRows),
    ),
    // --- Image and legal ----------------------------------------------------
    hinted(
        "imageCredit",
        "Project image credit",
        "Who the project image belongs to and on what terms it may be shown — a licence and a copyright \
         holder at minimum. The licence governs the image, separately from the dataset licence.",
        Optional,
        Everyone,
        Some(OPTIONAL_STRING),
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
            "accessRights.accessRights",
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

/// The section a field is shown in, or `None` for a field no section lists.
///
/// The form is sectioned and submit validation is whole-project, so an error
/// can name a field the reader is not currently looking at. Without this the
/// refusal says "the fields below say what needs changing" and nothing below
/// says anything — a dead end.
#[must_use]
pub fn section_of(field_id: &str) -> Option<&'static Section> {
    SECTIONS.iter().find(|section| section.fields.contains(&field_id))
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
    use std::collections::{BTreeMap, BTreeSet};

    use platform_metadata::project::{CONTRIBUTOR_ROLES, ROLES_NOT_OFFERED};

    use super::*;

    /// The contract as `ProjectRaw` serializes it, read off a real committed
    /// project rather than listed here — a field added to the contract has to
    /// show up without this file being edited.
    ///
    /// `ProjectRaw` carries no `skip_serializing_if`, so an unset `Option`
    /// serializes as `null` and is still a member: this is the whole contract
    /// and not just the parts this project happens to fill in.
    fn contract() -> serde_json::Value {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");
        let project = published.get("0801d").expect("0801d is in the committed corpus");
        serde_json::to_value(project).expect("ProjectRaw serializes")
    }

    /// The contract's top-level members.
    fn contract_members() -> BTreeSet<String> {
        contract()
            .as_object()
            .expect("ProjectRaw is an object")
            .keys()
            .cloned()
            .collect()
    }

    /// A registry id's top-level contract member: `accessRights.embargoDate` is
    /// part of `accessRights`.
    fn root_of(id: &str) -> &str {
        id.split_once('.').map_or(id, |(root, _)| root)
    }

    /// Whether a registry id resolves segment by segment, so a dotted id is
    /// checked against the nested member it names and not merely against its
    /// root. A `null` anywhere on the path counts as unresolved: the sample
    /// project cannot answer for that member, and a check that cannot check
    /// should say so rather than pass.
    fn resolves(contract: &serde_json::Value, id: &str) -> bool {
        let mut current = contract;
        for segment in id.split('.') {
            match current.as_object().and_then(|object| object.get(segment)) {
                Some(next) => current = next,
                None => return false,
            }
        }
        true
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
        // entry that renders a control posting to nothing. Dotted ids are
        // followed the whole way down, because checking only the root would pass
        // `accessRights.embargoDate` on the strength of `accessRights` existing,
        // while the nested member could be renamed or dropped with nothing
        // failing.
        let contract = contract();
        let unknown: Vec<&str> = FIELDS
            .iter()
            .map(|field| field.id)
            .filter(|id| !resolves(&contract, id))
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

    /// Every committed project, parsed as the contract sees it — nulls intact,
    /// because an unset `Option` serializes as `null` and that is the only way to
    /// tell an `Option` member from a required one.
    fn contracts() -> Vec<serde_json::Value> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");
        let contracts: Vec<serde_json::Value> = published
            .summaries()
            .map(|summary| {
                let project = published.get(summary.shortcode).expect("a summary names a loaded project");
                serde_json::to_value(project).expect("ProjectRaw serializes")
            })
            .collect();
        assert_eq!(contracts.len(), 85, "the corpus should be all 85 committed projects");
        contracts
    }

    /// The fields the form does not read back yet.
    ///
    /// **Empty: every editable field has a control.** The test below is what keeps it so — a new
    /// `ProjectRaw` member placed in a section without a shape lands here and fails, rather
    /// than rendering as a note nobody notices.
    ///
    /// The rendering it names is still in `widgets::stated`, unreached by any field today and
    /// deliberately kept: it is what a *new* field falls back to, and a depositor who cannot
    /// find a field the published page shows would otherwise conclude the form lost it.
    const NOT_READ_YET_IDS: &[&str] = &[];

    /// `typeOfData` is a closed vocabulary with no enum behind it, so nothing but this test says
    /// the offered set still covers the committed data. A project holding a kind the form does
    /// not offer has it dropped on the first save.
    ///
    /// Here rather than beside the slice in `platform-metadata`, because a platform crate takes no
    /// path into a service's data directory (`.github/scripts/check-platform-paths.sh`).
    #[test]
    fn the_offered_data_kinds_cover_every_value_the_corpus_holds() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");

        let mut unknown: Vec<&str> = Vec::new();
        for summary in published.summaries() {
            let project = published.get(summary.shortcode).expect("a summary names a loaded project");
            for kind in project.type_of_data.iter().flatten() {
                if !TYPE_OF_DATA_VALUES.contains(&kind.as_str()) {
                    unknown.push(kind);
                }
            }
        }
        unknown.sort_unstable();
        unknown.dedup();
        assert!(unknown.is_empty(), "committed data kinds the form does not offer: {unknown:?}");
    }

    /// Every role several projects share is either offered or deliberately not.
    ///
    /// Measured by project spread rather than by use count: a role one project repeats is that
    /// project's wording, and the open set is what carries it. Three projects is the line.
    /// Case-insensitive, because supplying one casing where the data has several is the offer's
    /// whole purpose.
    #[test]
    fn the_offered_roles_cover_the_roles_several_projects_share() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");

        let mut projects_per_role: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
        for summary in published.summaries() {
            let project = published.get(summary.shortcode).expect("a summary names a loaded project");
            for attribution in &project.attributions {
                for role in &attribution.contributor_type {
                    projects_per_role
                        .entry(role.trim().to_lowercase())
                        .or_default()
                        .insert(summary.shortcode);
                }
            }
        }

        let offered: BTreeSet<String> = CONTRIBUTOR_ROLES.iter().map(|role| role.to_lowercase()).collect();
        let excused: BTreeSet<&str> = ROLES_NOT_OFFERED.iter().map(|(role, _)| *role).collect();
        let mut missing: Vec<(&str, usize)> = projects_per_role
            .iter()
            .filter(|(role, projects)| {
                projects.len() >= 3 && !offered.contains(role.as_str()) && !excused.contains(role.as_str())
            })
            .map(|(role, projects)| (role.as_str(), projects.len()))
            .collect();
        missing.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        assert!(
            missing.is_empty(),
            "roles shared by three or more projects that are neither offered nor in ROLES_NOT_OFFERED: {missing:?}"
        );

        // The other direction: an excuse for a role the corpus has stopped sharing is a stale
        // decision, and one for a role that *is* offered is a contradiction.
        for (role, reason) in ROLES_NOT_OFFERED {
            assert!(!offered.contains(*role), "{role} is both offered and excused ({reason})");
            assert!(
                projects_per_role.get(*role).is_some_and(|projects| projects.len() >= 3),
                "{role} is excused but no longer shared by three projects; drop the entry"
            );
        }
    }

    #[test]
    fn the_fields_the_form_does_not_read_yet_are_named() {
        // A field with no shape renders a note instead of a control, and no
        // applier touches it — so a *new* contract field defaulting into this
        // state would be silently uneditable while looking registered. The
        // completeness test above only asks that a field be placed in a section;
        // this is what asks whether the form can actually read it. The list is
        // empty, so this now asserts that every editable field is readable.
        let unread: BTreeSet<&str> = FIELDS
            .iter()
            .filter(|field| !field.display_only && field.shape.is_none())
            .map(|field| field.id)
            .collect();
        assert_eq!(unread, NOT_READ_YET_IDS.iter().copied().collect::<BTreeSet<&str>>());
    }

    #[test]
    fn a_display_only_field_declares_no_shape() {
        // REQ-1.5 and REQ-1.7 together: shown as a value, written back
        // unchanged. A shape here would put an applier on a field the reader
        // cannot change, so an empty control would clear a value nobody touched.
        for field in FIELDS.iter().filter(|field| field.display_only) {
            assert!(field.shape.is_none(), "{} should declare no shape", field.id);
            assert!(!field.is_editable(), "{} should render no control", field.id);
        }
    }

    #[test]
    fn a_scalar_shape_s_empty_state_matches_whether_the_contract_requires_the_field() {
        // The inversion this declaration exists to prevent, checked against the
        // committed data rather than against a list repeated here: `Placeholder`
        // is for a member the contract types as a `String`, so it is present in
        // all 85 files, and `Drop` is for an `Option`, so it is null or absent in
        // at least one. Getting it backwards is invisible — `Drop` on a required
        // `String` leaves every ongoing project unpublishable until an end date
        // it does not have is entered, and nothing fails.
        let contracts = contracts();
        for field in FIELDS {
            let Some(Shape::Text(when_cleared)) = field.shape else {
                continue;
            };
            let unset = contracts
                .iter()
                .filter(|contract| contract.get(field.id).is_none_or(serde_json::Value::is_null))
                .count();
            match when_cleared {
                WhenCleared::Placeholder => assert_eq!(
                    unset,
                    0,
                    "{} declares the placeholder empty state, so the contract must require it — but it is \
                     null or absent in {unset} of {} committed projects, which makes it an Option",
                    field.id,
                    contracts.len()
                ),
                WhenCleared::Drop => assert!(
                    unset > 0,
                    "{} declares that clearing drops the member, so the contract must type it as an Option \
                     — but it is set in all {} committed projects, which is what a required String looks like",
                    field.id,
                    contracts.len()
                ),
            }
        }
    }

    #[test]
    fn a_declared_shape_matches_the_json_kind_the_contract_holds() {
        // A `Multilingual` shape on a string member, or a `Text` shape on a
        // language map, renders a control that posts under names the applier
        // never reads: the field looks editable and every save is a no-op.
        for contract in contracts() {
            for field in FIELDS {
                let Some(shape) = field.shape else { continue };
                let Some(value) = contract.get(field.id).filter(|value| !value.is_null()) else {
                    continue;
                };
                match shape {
                    Shape::Text(_) => assert!(
                        value.is_string(),
                        "{} declares a scalar shape but the contract holds {value}",
                        field.id
                    ),
                    Shape::Multilingual => assert!(
                        value.is_object(),
                        "{} declares a language map but the contract holds {value}",
                        field.id
                    ),
                    // Stronger than the arms above: a choice must not merely be
                    // a string, it must be one the applier accepts back. A
                    // committed value outside the offered set is a control that
                    // silently resets the field on the first save.
                    // The pair is stored positionally on 74 projects and as
                    // members on 11, so the only thing worth asserting is that
                    // it is *not* a bare string — which is the one shape
                    // `url_slot` cannot read either way.
                    Shape::AgentRows | Shape::StringRows => {
                        let rows = value.as_array().unwrap_or_else(|| {
                            panic!("{} declares string rows but the contract holds {value}", field.id)
                        });
                        for row in rows {
                            assert!(row.is_string(), "{} declares string rows but a row holds {row}", field.id);
                        }
                    }
                    Shape::ReferenceRows(sources) => {
                        let rows = value.as_array().unwrap_or_else(|| {
                            panic!("{} declares reference rows but the contract holds {value}", field.id)
                        });
                        for row in rows {
                            let source = row.get("type").and_then(serde_json::Value::as_str).unwrap_or("");
                            assert!(
                                sources.contains(&source),
                                "{} holds a reference from {source:?}, which is not one of {sources:?}",
                                field.id
                            );
                        }
                    }
                    Shape::PublicationRows => {
                        let rows = value.as_array().unwrap_or_else(|| {
                            panic!("{} declares publication rows but the contract holds {value}", field.id)
                        });
                        for row in rows {
                            assert!(
                                row.get("text").is_some_and(serde_json::Value::is_string),
                                "{} declares publication rows but a row holds {row}",
                                field.id
                            );
                        }
                    }
                    Shape::FundingRows => assert!(
                        value.is_array() || value.is_string(),
                        "{} declares funding but the contract holds {value}",
                        field.id
                    ),
                    Shape::TextOrReferenceRows(sources) => {
                        let rows = value.as_array().unwrap_or_else(|| {
                            panic!("{} declares variant rows but the contract holds {value}", field.id)
                        });
                        for row in rows {
                            // A reference must name a source the form offers,
                            // or the committed entry is one no control can
                            // reproduce; a text row is a language map.
                            match row.get("url") {
                                Some(_) => {
                                    let source = row.get("type").and_then(serde_json::Value::as_str).unwrap_or("");
                                    assert!(
                                        sources.contains(&source),
                                        "{} holds a reference from {source:?}, which is not one of {sources:?}",
                                        field.id
                                    );
                                }
                                None => assert!(
                                    row.is_object(),
                                    "{} declares variant rows but a text row holds {row}",
                                    field.id
                                ),
                            }
                        }
                    }
                    Shape::AttributionRows => {
                        let rows = value.as_array().unwrap_or_else(|| {
                            panic!("{} declares contributor rows but the contract holds {value}", field.id)
                        });
                        for row in rows {
                            assert!(
                                row.get("contributor").is_some_and(serde_json::Value::is_string),
                                "{} declares contributor rows but a row holds {row}",
                                field.id
                            );
                        }
                    }
                    Shape::MultilingualRows => {
                        let rows = value.as_array().unwrap_or_else(|| {
                            panic!("{} declares a list of maps but the contract holds {value}", field.id)
                        });
                        for row in rows {
                            assert!(
                                row.is_object(),
                                "{} declares a list of language maps but a row holds {row}",
                                field.id
                            );
                        }
                    }
                    Shape::StringList(set) => {
                        let items = value
                            .as_array()
                            .unwrap_or_else(|| panic!("{} declares a list but the contract holds {value}", field.id));
                        // A closed vocabulary must cover what the corpus holds,
                        // or the applier drops a committed value on the first
                        // save. An open one is open, so there is nothing to
                        // check beyond the kind.
                        let unknown: Vec<&str> = items
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .filter(|item| !set.accepts(item))
                            .collect();
                        assert!(
                            unknown.is_empty(),
                            "{} holds {unknown:?}, which its closed shape does not offer",
                            field.id
                        );
                    }
                    Shape::Url(_) => assert!(
                        value.is_array() || value.is_object(),
                        "{} declares a URL slot but the contract holds {value}",
                        field.id
                    ),
                    Shape::Choice(values) => {
                        let held = value.as_str().unwrap_or_default();
                        assert!(
                            values.contains(&held),
                            "{} holds {held:?}, not one of the {values:?} its shape offers",
                            field.id
                        );
                    }
                }
            }
        }
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
    fn a_hint_never_states_an_obligation() {
        // The pill states the tier and submit enforces it, so a hint repeating it is redundant
        // where it agrees and a second, contradictory source of truth where it does not.
        //
        // "Needed before the project can be published" is deliberately still allowed, and is what
        // the `Recommended` fields carry: a different gate, owned by RDU at publication.
        // The tier note on `Obligation` is where that distinction is written down.
        for field in FIELDS {
            let Some(hint) = field.hint else { continue };
            assert!(
                !hint.to_ascii_lowercase().contains("required"),
                "{}: a hint must not state an obligation — the pill does: {hint:?}",
                field.id
            );
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
