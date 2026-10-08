use std::io;
use std::path::PathBuf;

use cpe_ports::contract::Violation;

/// Why a known project's snapshot file could not be served. `LiveArchiveProjection` returns it as
/// the source of `ProjectionError::Unavailable`.
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
    /// The mapped snapshot breaks the port's contract: a cross-node rule `sync-store` leaves to
    /// `cpe_ports::contract`, or anything else the mapping let through.
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
