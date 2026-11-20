//! The closed-world violation report (§11): a run collects every breach and fails
//! once, with all of them, instead of stopping at the first.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// `(rule, subject, iri)`: see [`Violations`].
pub(crate) type Violation = (Rule, String, String);

/// How many violations of one rule the report prints; the report value keeps all.
const SHOWN_PER_RULE: usize = 20;

/// The closed set of violation kinds. `Ord`, so the report groups by rule. A new
/// variant is appended last, to [`Rule::ALL`] too, and named in `FORMAT.md` in a
/// section [`Rule::section`] cites, with a §16 line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Rule {
    /// A blank node as subject or object in the data graph (§1).
    BlankNode,
    /// A data-graph subject with no `rdf:type` and no kind by position (§11).
    UntypedSubject,
    /// A subject with more than one `rdf:type` (§11).
    MultipleTypes,
    /// An `rdf:type` that is neither a project class nor a knora-base class of §11.
    UnknownClass,
    /// A resource-level predicate that fits no §6 rule and is not in §11's resource list.
    UnknownPredicate,
    /// A resource without `rdfs:label` (§2).
    MissingLabel,
    /// A resource with more than one `rdfs:label` (§2).
    RepeatedLabel,
    /// A resource whose `rdfs:label` is not a plain string literal (§2).
    NonPlainLabel,
    /// A language tag that is not valid BCP 47 (§1).
    InvalidLanguageTag,
    /// A `kb:valueHasUUID` that is not non-empty base64url (§3).
    InvalidUuid,
    /// Two emitted values sharing a `kb:valueHasUUID`, so one node (§3).
    DuplicateUuid,
    /// A value or file value without a term §3–§5, §7 or §8 require: its UUID, its
    /// kind's source literal or list node, a seqnum's integer, a file name or a
    /// still image's dimension.
    MissingSourceLiteral,
    /// A value or file value with two of a term it may carry once: a literal, or a
    /// list value's list node (§3–§5, §7, §8).
    RepeatedSourceLiteral,
    /// A source term that is not what its §4 kind, §5, §7 or §8 requires: an IRI
    /// where a literal belongs or the reverse, the wrong datatype or a language
    /// tag, or a lexical outside that datatype (a dimension outside `u32`).
    InvalidLiteral,
    /// A value-node predicate outside §11's list for its value class.
    UnknownValuePredicate,
    /// A date in a calendar other than GREGORIAN or JULIAN (§4.6).
    UnknownCalendar,
    /// A date precision other than YEAR, MONTH or DAY (§4.6).
    UnknownPrecision,
    /// A date whose start JDN is after its end JDN (§4.6).
    InvertedDate,
    /// A list value naming no live list node: one missing from the dump,
    /// deleted, or inside a deleted subtree (§4.7).
    ListValueTarget,
    /// A direct link triple that no LinkValue, live or dropped, describes (§6).
    /// Not reported under a subject and predicate whose LinkValue lacks its one
    /// IRI `rdf:object`: that LinkValue is reported instead.
    MissingLinkValue,
    /// A live LinkValue whose direct triple is not in the dump (§6).
    UnpairedLinkValue,
    /// A live LinkValue without exactly one IRI `rdf:subject`, `rdf:predicate`
    /// and `rdf:object` (§6).
    InvalidLinkValue,
    /// A resource on a `dao:isPartOf` cycle (§7).
    MembershipCycle,
    /// A resource with two live seqnum values (§7).
    RepeatedSeqnum,
    /// A seqnum whose object is not an `IntValue` (§7).
    InvalidSeqnum,
    /// A resource with two live file values (§8).
    MultipleRepresentations,
    /// A list-node predicate outside §11's list-node list.
    UnknownListNodePredicate,
    /// A file-value predicate outside §11's file-value list.
    UnknownFileValuePredicate,
    /// A LinkValue predicate outside §11's LinkValue list.
    UnknownLinkValuePredicate,
    /// A list-node `rdfs:label` that is not a plain or language-tagged string (§9).
    InvalidListLabel,
    /// Two labels on one list node with the same tag, or both untagged (§9).
    DuplicateListLabel,
    /// A `kb:hasSubListNode` whose object is not a list node of the dump (§9).
    DanglingSubListNode,
    /// A non-root list node that no `kb:hasSubListNode` names (§9).
    OrphanListNode,
    /// A list node that two `kb:hasSubListNode` triples name (§9).
    MultipleParents,
    /// A root list node that a `kb:hasSubListNode` names (§9).
    RootWithParent,
    /// A root list node with a `kb:listNodePosition` (§9).
    RootWithPosition,
    /// A `kb:listNodePosition` that is not one `xsd:integer` fitting a `u32` (§9).
    InvalidListPosition,
    /// Two children of one list node sharing a position (§9).
    DuplicateSiblingPosition,
    /// A list node on a `skos:broader` cycle (§9).
    ListNodeCycle,
    /// An annotation with no carried link of its target property (§10).
    MissingAnnotationTarget,
    /// A resource whose class reaches both `kb:Region` and `kb:LinkObj`, so it has
    /// no one motivation (§10).
    AmbiguousAnnotationClass,
    /// A text value with standoff, a mapping or a formatted `kb:hasTextValueType`
    /// but no `kb:valueHasXml` (§4.1).
    MissingTextXml,
    /// A `kb:hasTextValueType` naming none of knora-base's four text types (§4.1).
    UnknownTextType,
    /// A text value typed `kb:UnformattedText` with XML, standoff or a mapping (§4.1).
    TextTypeMismatch,
}

impl Rule {
    /// Every rule in declaration order, for the test that `FORMAT.md` names each
    /// one; a new variant is appended here too.
    #[cfg(test)]
    pub(crate) const ALL: [Rule; 44] = [
        Rule::BlankNode,
        Rule::UntypedSubject,
        Rule::MultipleTypes,
        Rule::UnknownClass,
        Rule::UnknownPredicate,
        Rule::MissingLabel,
        Rule::RepeatedLabel,
        Rule::NonPlainLabel,
        Rule::InvalidLanguageTag,
        Rule::InvalidUuid,
        Rule::DuplicateUuid,
        Rule::MissingSourceLiteral,
        Rule::RepeatedSourceLiteral,
        Rule::InvalidLiteral,
        Rule::UnknownValuePredicate,
        Rule::UnknownCalendar,
        Rule::UnknownPrecision,
        Rule::InvertedDate,
        Rule::ListValueTarget,
        Rule::MissingLinkValue,
        Rule::UnpairedLinkValue,
        Rule::InvalidLinkValue,
        Rule::MembershipCycle,
        Rule::RepeatedSeqnum,
        Rule::InvalidSeqnum,
        Rule::MultipleRepresentations,
        Rule::UnknownListNodePredicate,
        Rule::UnknownFileValuePredicate,
        Rule::UnknownLinkValuePredicate,
        Rule::InvalidListLabel,
        Rule::DuplicateListLabel,
        Rule::DanglingSubListNode,
        Rule::OrphanListNode,
        Rule::MultipleParents,
        Rule::RootWithParent,
        Rule::RootWithPosition,
        Rule::InvalidListPosition,
        Rule::DuplicateSiblingPosition,
        Rule::ListNodeCycle,
        Rule::MissingAnnotationTarget,
        Rule::AmbiguousAnnotationClass,
        Rule::MissingTextXml,
        Rule::UnknownTextType,
        Rule::TextTypeMismatch,
    ];

    /// Whether this is the enum's last variant. The `match` has no wildcard, so a
    /// new variant fails to compile here; making it the last one makes
    /// `format_doc_tests` fail until it is appended to [`Rule::ALL`] too.
    #[cfg(test)]
    pub(crate) fn is_last(self) -> bool {
        match self {
            Rule::TextTypeMismatch => true,
            Rule::BlankNode
            | Rule::UntypedSubject
            | Rule::MultipleTypes
            | Rule::UnknownClass
            | Rule::UnknownPredicate
            | Rule::MissingLabel
            | Rule::RepeatedLabel
            | Rule::NonPlainLabel
            | Rule::InvalidLanguageTag
            | Rule::InvalidUuid
            | Rule::DuplicateUuid
            | Rule::MissingSourceLiteral
            | Rule::RepeatedSourceLiteral
            | Rule::InvalidLiteral
            | Rule::UnknownValuePredicate
            | Rule::UnknownCalendar
            | Rule::UnknownPrecision
            | Rule::InvertedDate
            | Rule::ListValueTarget
            | Rule::MissingLinkValue
            | Rule::UnpairedLinkValue
            | Rule::InvalidLinkValue
            | Rule::MembershipCycle
            | Rule::RepeatedSeqnum
            | Rule::InvalidSeqnum
            | Rule::MultipleRepresentations
            | Rule::UnknownListNodePredicate
            | Rule::UnknownFileValuePredicate
            | Rule::UnknownLinkValuePredicate
            | Rule::InvalidListLabel
            | Rule::DuplicateListLabel
            | Rule::DanglingSubListNode
            | Rule::OrphanListNode
            | Rule::MultipleParents
            | Rule::RootWithParent
            | Rule::RootWithPosition
            | Rule::InvalidListPosition
            | Rule::DuplicateSiblingPosition
            | Rule::ListNodeCycle
            | Rule::MissingAnnotationTarget
            | Rule::AmbiguousAnnotationClass
            | Rule::MissingTextXml
            | Rule::UnknownTextType => false,
        }
    }

    /// The name the report prints; `FORMAT.md` names each rule by it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Rule::BlankNode => "blank-node",
            Rule::UntypedSubject => "untyped-subject",
            Rule::MultipleTypes => "multiple-types",
            Rule::UnknownClass => "unknown-class",
            Rule::UnknownPredicate => "unknown-predicate",
            Rule::MissingLabel => "missing-label",
            Rule::RepeatedLabel => "repeated-label",
            Rule::NonPlainLabel => "non-plain-label",
            Rule::InvalidLanguageTag => "invalid-language-tag",
            Rule::InvalidUuid => "invalid-uuid",
            Rule::DuplicateUuid => "duplicate-uuid",
            Rule::MissingSourceLiteral => "missing-source-literal",
            Rule::RepeatedSourceLiteral => "repeated-source-literal",
            Rule::InvalidLiteral => "invalid-literal",
            Rule::UnknownValuePredicate => "unknown-value-predicate",
            Rule::UnknownCalendar => "unknown-calendar",
            Rule::UnknownPrecision => "unknown-precision",
            Rule::InvertedDate => "inverted-date",
            Rule::ListValueTarget => "list-value-target",
            Rule::MissingLinkValue => "missing-link-value",
            Rule::UnpairedLinkValue => "unpaired-link-value",
            Rule::InvalidLinkValue => "invalid-link-value",
            Rule::MembershipCycle => "membership-cycle",
            Rule::RepeatedSeqnum => "repeated-seqnum",
            Rule::InvalidSeqnum => "invalid-seqnum",
            Rule::MultipleRepresentations => "multiple-representations",
            Rule::UnknownListNodePredicate => "unknown-list-node-predicate",
            Rule::UnknownFileValuePredicate => "unknown-file-value-predicate",
            Rule::UnknownLinkValuePredicate => "unknown-link-value-predicate",
            Rule::InvalidListLabel => "invalid-list-label",
            Rule::DuplicateListLabel => "duplicate-list-label",
            Rule::DanglingSubListNode => "dangling-sub-list-node",
            Rule::OrphanListNode => "orphan-list-node",
            Rule::MultipleParents => "multiple-parents",
            Rule::RootWithParent => "root-with-parent",
            Rule::RootWithPosition => "root-with-position",
            Rule::InvalidListPosition => "invalid-list-position",
            Rule::DuplicateSiblingPosition => "duplicate-sibling-position",
            Rule::ListNodeCycle => "list-node-cycle",
            Rule::MissingAnnotationTarget => "missing-annotation-target",
            Rule::AmbiguousAnnotationClass => "ambiguous-annotation-class",
            Rule::MissingTextXml => "missing-text-xml",
            Rule::UnknownTextType => "unknown-text-type",
            Rule::TextTypeMismatch => "text-type-mismatch",
        }
    }

    /// The `FORMAT.md` sections the report cites; one of them names the rule.
    pub(crate) fn section(self) -> &'static str {
        match self {
            Rule::BlankNode | Rule::InvalidLanguageTag => "§1",
            Rule::UnknownClass | Rule::MissingLabel | Rule::RepeatedLabel | Rule::NonPlainLabel => "§2",
            Rule::InvalidUuid | Rule::DuplicateUuid => "§3",
            Rule::MissingSourceLiteral | Rule::RepeatedSourceLiteral | Rule::InvalidLiteral => "§3–§5, §7, §8",
            Rule::MissingTextXml | Rule::UnknownTextType | Rule::TextTypeMismatch => "§4.1",
            Rule::UnknownCalendar | Rule::UnknownPrecision | Rule::InvertedDate => "§4.6",
            Rule::ListValueTarget => "§4.7",
            Rule::UnknownPredicate | Rule::MissingLinkValue | Rule::UnpairedLinkValue | Rule::InvalidLinkValue => "§6",
            Rule::MembershipCycle | Rule::RepeatedSeqnum | Rule::InvalidSeqnum => "§7",
            Rule::MultipleRepresentations => "§8",
            Rule::InvalidListLabel
            | Rule::DuplicateListLabel
            | Rule::DanglingSubListNode
            | Rule::OrphanListNode
            | Rule::MultipleParents
            | Rule::RootWithParent
            | Rule::RootWithPosition
            | Rule::InvalidListPosition
            | Rule::DuplicateSiblingPosition
            | Rule::ListNodeCycle => "§9",
            Rule::MissingAnnotationTarget | Rule::AmbiguousAnnotationClass => "§10",
            Rule::UntypedSubject
            | Rule::MultipleTypes
            | Rule::UnknownValuePredicate
            | Rule::UnknownListNodePredicate
            | Rule::UnknownLinkValuePredicate
            | Rule::UnknownFileValuePredicate => "§11",
        }
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (FORMAT.md {})", self.name(), self.section())
    }
}

/// Every closed-world breach found in one run, as `(rule, subject, iri)`, where
/// `iri` names the offending predicate, class or term (empty when the subject
/// alone is the finding). Keyed rule-first, so the per-rule counts read off the order.
#[derive(Default)]
pub(crate) struct Violations(BTreeSet<Violation>);

impl Violations {
    pub(crate) fn record(&mut self, rule: Rule, subject: &str, iri: &str) {
        self.0.insert((rule, subject.to_string(), iri.to_string()));
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    /// `Err` with the full report when anything was recorded.
    pub(crate) fn into_result(self) -> Result<(), ViolationReport> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ViolationReport(self))
        }
    }
}

/// The run's single failure: `Display` opens with a count per rule, then lists
/// the first [`SHOWN_PER_RULE`] of each rule in sorted order and "… and K more".
pub struct ViolationReport(Violations);

impl ViolationReport {
    /// Every violation, sorted, uncapped.
    #[cfg(test)]
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Violation> {
        self.0.0.iter()
    }

    /// The number of violations per rule, in rule order, each rule as its name and `FORMAT.md`
    /// section. Unlike the report's text, it names no subject, so it may be logged.
    pub fn counts(&self) -> Vec<(String, usize)> {
        self.per_rule()
            .into_iter()
            .map(|(rule, items)| (rule.to_string(), items.len()))
            .collect()
    }

    fn per_rule(&self) -> Vec<(Rule, Vec<&Violation>)> {
        let mut groups: Vec<(Rule, Vec<&Violation>)> = Vec::new();
        for v in &self.0.0 {
            match groups.last_mut() {
                Some((rule, items)) if *rule == v.0 => items.push(v),
                _ => groups.push((v.0, vec![v])),
            }
        }
        groups
    }
}

impl fmt::Display for ViolationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let groups = self.per_rule();
        writeln!(f, "snapshot: {} violation(s), no snapshot (FORMAT.md §11)", self.0.len())?;
        for (rule, items) in &groups {
            writeln!(f, "  {rule}: {}", items.len())?;
        }
        for (rule, items) in &groups {
            writeln!(f, "{rule}:")?;
            for (_, subject, iri) in items.iter().take(SHOWN_PER_RULE) {
                if iri.is_empty() {
                    writeln!(f, "  {subject}")?;
                } else {
                    writeln!(f, "  {subject} {iri}")?;
                }
            }
            if items.len() > SHOWN_PER_RULE {
                writeln!(f, "  … and {} more", items.len() - SHOWN_PER_RULE)?;
            }
        }
        Ok(())
    }
}

/// An error printed with `{:?}` stays readable this way, instead of one escaped
/// line.
impl fmt::Debug for ViolationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for ViolationReport {}
