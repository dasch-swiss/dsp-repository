use std::io;
use std::path::PathBuf;

use cpe_ports::contract::Violation;

use crate::ArkError;

/// Why a known project's snapshot file or curation file could not be served; `path` names the file
/// at fault. `LiveArchiveProjection` returns it as the source of `ProjectionError::Unavailable`.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("cannot read {}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// `line` is 1-based.
    #[error("{} is not valid N-Quads at line {line}", path.display())]
    Syntax {
        path: PathBuf,
        line: u64,
        #[source]
        source: oxttl::TurtleSyntaxError,
    },
    /// The file breaks `FORMAT.md` on a fact the port serves, or a rule the whole file must keep.
    /// `subject` is the node that carries it: a value node, a list node, a representation, or the
    /// resource for a rule about the resource as a whole.
    #[error("{} is not a valid snapshot at {subject}: {reason}", path.display())]
    Invalid { path: PathBuf, subject: String, reason: InvalidFact },
    #[error("{} holds no resource", path.display())]
    Empty { path: PathBuf },
    /// The curation file breaks its format: the first fault met.
    #[error("{} is not a valid curation file at {fault}", path.display())]
    Curation { path: PathBuf, fault: CurationFault },
    /// The mapped snapshot breaks the port's contract: a cross-node rule `sync-store` leaves to
    /// `cpe_ports::contract`, or anything else the mapping let through. `path` is the snapshot
    /// file.
    #[error("{} breaks the port's contract: {}", path.display(), join_violations(violations))]
    Contract { path: PathBuf, violations: Vec<Violation> },
}

fn join_violations(violations: &[Violation]) -> String {
    violations.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ")
}

/// The rule a fact breaks. IRIs are full, as the file writes them; a blank node is `_:<id>`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidFact {
    #[error("a blank-node subject")]
    BlankNodeSubject,
    #[error("a blank-node object")]
    BlankNodeObject,
    #[error("a quad outside the project graph, in {graph}")]
    ForeignGraph { graph: String },
    /// The resource's IRI is not one dsp-api derives a data ARK from.
    #[error("a resource whose IRI has no data ARK: {reason}")]
    NoDataArk { reason: ArkError },
    #[error("a resource without a class")]
    MissingClass,
    #[error("a resource also typed {class}")]
    ExtraResourceType { class: String },
    #[error("a resource part of {parent}, which is not a resource")]
    UnknownParent { parent: String },
    #[error("a resource without a label")]
    MissingLabel,
    #[error("a resource with two labels")]
    RepeatedLabel,
    #[error("a resource whose label is language-tagged")]
    TaggedLabel,
    #[error("a value node without a UUID")]
    MissingValueUuid,
    #[error("a value node with two values")]
    RepeatedValue,
    #[error("a value node with two source properties")]
    RepeatedSourceProperty,
    #[error("a value node without a source property")]
    MissingSourceProperty,
    #[error("a value node whose source property is not the predicate of its edge {edge}")]
    UnfitSourceProperty { edge: String },
    /// The UUID must be base64url and the suffix of the node's IRI.
    #[error("a value node whose UUID {uuid:?} does not fit its IRI")]
    UnfitValueUuid { uuid: String },
    #[error("a value node with neither a value nor a list node")]
    MissingValueContent,
    #[error("a list value that also has a value")]
    ListValueWithContent,
    /// The value node is reached from two resources; the error's `subject` is the second.
    #[error("two resources share the value node with UUID {uuid}")]
    DuplicateValueUuid { uuid: String },
    #[error("{predicate} has {lexical:?}, which does not fit its type")]
    UnfitLiteral { predicate: String, lexical: String },
    #[error("a date without {missing}")]
    IncompleteDate { missing: String },
    #[error("{predicate} has the unknown term {lexical:?}")]
    UnknownDateTerm { predicate: String, lexical: String },
    #[error("a list value naming {node}, which is not a list node")]
    UnknownListNode { node: String },
    #[error("a list node whose parent {parent} is not a list node")]
    UnknownListParent { parent: String },
    #[error("a list node without a parent but with a position")]
    PositionedListRoot,
    /// Untagged counts as one language.
    #[error("a list node with two labels in one language")]
    DuplicateListLabel,
    #[error("a resource whose representation {node} is not typed as one")]
    UnknownRepresentation { node: String },
    #[error("a representation without {missing}")]
    IncompleteRepresentation { missing: String },
    /// The error's `subject` is the second resource.
    #[error("two resources share the representation {node}")]
    DuplicateRepresentation { node: String },
    #[error("an annotation without a motivation")]
    MissingMotivation,
    #[error("an annotation with the unknown motivation {motivation}")]
    UnknownMotivation { motivation: String },
    #[error("an annotation without a target")]
    MissingAnnotationTarget,
    /// The target is outside the file, or a node of it that is not a `dao:Resource`.
    #[error("an annotation targeting {target}, which is not a resource")]
    UnknownTarget { target: String },
    #[error("an annotation that is not a resource")]
    UnservedAnnotation,
    /// The error's `subject` can be any node: a resource, a value node or a representation.
    #[error("{predicate} on a node that is not an annotation")]
    StrayAnnotationFact { predicate: String },
    /// Any predicate `FORMAT.md` allows once on its subject, other than those with their own
    /// variant above.
    #[error("{predicate} more than once")]
    RepeatedPredicate { predicate: String },
}

/// A fault of a curation file. `line` is 1-based; the header is line 1.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {reason}")]
pub struct CurationFault {
    pub line: usize,
    pub reason: InvalidCuration,
}

/// The rule a curation file breaks.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidCuration {
    #[error("bytes that are not UTF-8")]
    NotUtf8,
    /// The file has no line, or its first line is empty.
    #[error("no header")]
    MissingHeader,
    /// A control character other than the line feed that ends a line, or U+2028 or U+2029.
    #[error("the control or line-break character {character:?}")]
    ControlCharacter { character: char },
    #[error("a quote inside a bare cell, or text after a closing quote")]
    StrayQuote,
    /// The fault's line is the one the quote opens on.
    #[error("a quoted cell that does not end on its line")]
    UnterminatedQuote,
    #[error("a first column named {found:?}, not iri")]
    FirstColumnNotIri { found: String },
    #[error("a column named {column:?}, which is neither key, key@lang nor a comment")]
    MalformedColumn { column: String },
    #[error("the column {column:?} twice")]
    DuplicateColumn { column: String },
    #[error("a blank line")]
    BlankLine,
    #[error("a row of {found} cells under a header of {expected}")]
    WrongCellCount { expected: usize, found: usize },
    #[error("a row for {iri:?}, which is not a resource of the snapshot")]
    UnknownResource { iri: String },
    /// `first_line` is the line of the IRI's first row.
    #[error("a second row for {iri:?}, after line {first_line}")]
    DuplicateRow { iri: String, first_line: usize },
    /// The value starts or ends with white space, U+200B or U+FEFF.
    #[error("a value under {column} that starts or ends with white space or an invisible character")]
    PaddedValue { column: String },
}
