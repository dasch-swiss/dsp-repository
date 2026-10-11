//! Why `build` refused a snapshot.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Declares [`Rule`] with its name per variant, and [`Rule::ALL`] from the same list, so that
/// `ALL` cannot miss a variant.
macro_rules! rules {
    ($($(#[$doc:meta])* $variant:ident => $name:literal,)+) => {
        /// One refusal case of the builder.
        ///
        /// A new variant gets a case in `tests/it/refusals.rs` or in the case table of
        /// `tests/it/hookless.rs` or of `tests/it/hooks.rs`: the completeness test in the
        /// first compares the rules the cases of all three produce with [`Rule::ALL`].
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum Rule {
            $($(#[$doc])* $variant,)+
        }

        impl Rule {
            /// Every variant, in declaration order.
            pub const ALL: &'static [Rule] = &[$(Rule::$variant,)+];

            pub fn name(self) -> &'static str {
                match self {
                    $(Rule::$variant => $name,)+
                }
            }
        }
    };
}

rules! {
    /// The snapshot breaks an invariant of the port's contract.
    ContractViolation => "contract-violation",
    /// A text holds U+0000, which a SQL text literal cannot carry.
    InteriorNul => "interior-nul",
    /// A resource that is no annotation has a class the mapping does not declare, or an
    /// annotation stands in a snapshot whose mapping has no `annotations` node.
    UnmappedClass => "unmapped-class",
    /// An annotation's class is not the one its motivation belongs to.
    AnnotationClassMismatch => "annotation-class-mismatch",
    /// A value under a property the builder reads is not of the kind the property's rule
    /// expects; a list node never is.
    UnexpectedValueKind => "unexpected-value-kind",
    /// A resource whose id is a curated slug, with no source to make it from otherwise, has no
    /// curated value at all.
    BookWithoutCuration => "book-without-curation",
    /// A resource has not exactly one curated slug under the key of its class's `id`; where the
    /// `id` names a source to fall back on, it has more than one.
    SlugCount => "slug-count",
    /// A curated slug is empty once the whitespace around it is gone.
    EmptySlug => "empty-slug",
    /// Two resources have the same curated slug.
    DuplicateSlug => "duplicate-slug",
    /// A value holds `http` where no address belongs.
    HttpInValue => "http-in-value",
    /// A text holds the sentinel where it would be taken for what it stands in for: a curated
    /// value, a label that is read, or a value of a property that refuses the sentinel, alone or
    /// within a longer text.
    MissingInValue => "missing-in-value",
    /// A resource of a class with a `cover` has not exactly one curated cover.
    CoverCount => "cover-count",
    /// A resource's curated cover is not one of its own members.
    CoverNotOwnPage => "cover-not-own-page",
    /// A resource's curated cover is a member that a hook dropped.
    CoverIsCalibrationScan => "cover-is-calibration-scan",
    /// Two resources have the same curated cover.
    SharedCover => "shared-cover",
    /// A member's parents are not exactly one resource of the class its own class is
    /// `member-of`.
    PageParent => "page-parent",
    /// A member has no `seqnum`.
    PageWithoutSeqnum => "page-without-seqnum",
    /// A rule of Incunabula's hook: a calibration scan has a description, a comment or a border
    /// strip.
    CalibrationScanContent => "calibration-scan-content",
    /// The text a resource's id is made from has no character an id can be made of.
    EmptyIdSlug => "empty-id-slug",
    /// Two resources of declared classes have the same id.
    DuplicateId => "duplicate-id",
    /// A resource of a class with an image has no file, or one that is not a still image.
    NotStillImage => "not-still-image",
    /// A file's name has no stem to make the Image's id of.
    EmptyStem => "empty-stem",
    /// A file's name does not end in `.jp2`.
    NotJp2 => "not-jp2",
    /// A still image is zero pixels wide or high.
    ZeroDimension => "zero-dimension",
    /// An Image's id is also the id of a resource, or resources of two classes have the same
    /// file.
    ImageIdTaken => "image-id-taken",
    /// A resource has not as many curated values under a key as the key takes: one under a
    /// `curated` node that requires a value and under a date's `display-key`, and at most one
    /// under any other key a node of its class reads; an annotation and an annotation link one
    /// `keep` and at most one language, an annotation link at most one name.
    CuratedCount => "curated-count",
    /// The curated display text of a date has no language.
    DateDisplayWithoutLanguage => "date-display-without-language",
    /// The curated display text of a date is in none of the site's languages.
    UnknownDateDisplayLanguage => "unknown-date-display-language",
    /// A rule of Incunabula's hook: a Book's curated sort key has whitespace around it.
    PaddedSortKey => "padded-sort-key",
    /// A rule of Incunabula's hook: a Book's sort key, curated or its title, does not start
    /// with an ASCII character.
    SortKeyNotAscii => "sort-key-not-ascii",
    /// No value of a property that leaves out the label (`exclude-label`) is the resource's
    /// label.
    LabelNotATitle => "label-not-a-title",
    /// A rule of Incunabula's hook: a Book's catalogue address is overridden, and the archive
    /// no longer has the address the override replaces.
    StaleCatalogueOverride => "stale-catalogue-override",
    /// A rule of Incunabula's hook: a Book has a curated catalogue address without the address
    /// it replaces, or the address to replace without the one to write.
    UnpairedCatalogueOverride => "unpaired-catalogue-override",
    /// A date's two bounds have different precisions.
    MixedDatePrecision => "mixed-date-precision",
    /// A date has a bound outside the years 1 to 9999.
    DateOutOfRange => "date-out-of-range",
    /// A link's target is no resource of the class the property names as its `target`.
    StripLinkTarget => "strip-link-target",
    /// A rule of Incunabula's hook: a Page's border strip on one side is not a strip of that
    /// side.
    StripSideMismatch => "strip-side-mismatch",
    /// A curated value has a key that nothing reads: no node of a class, not the hook, and not
    /// the builder for an annotation.
    UnknownCuratedKey => "unknown-curated-key",
    /// A curated value is on a resource of a class its key is not curated on.
    CuratedKeyOnWrongClass => "curated-key-on-wrong-class",
    /// An annotation's or an annotation link's curated `keep` is neither `yes` nor `no`.
    KeepNotYesOrNo => "keep-not-yes-or-no",
    /// An annotation's or an annotation link's curated language is none of the site's
    /// languages.
    UnknownAnnotationLanguage => "unknown-annotation-language",
    /// Two of the annotations and annotation links have the same id, the last part of their
    /// IRIs.
    DuplicateAnnotationId => "duplicate-annotation-id",
    /// An annotation has not exactly one geometry and one colour, or an annotation or an
    /// annotation link has more than one comment.
    AnnotationValueCount => "annotation-value-count",
    /// An annotation's geometry is no JSON object with a list of points, each an `x` and a
    /// `y`, or is a circle without such a radius.
    GeometryNotRead => "geometry-not-read",
    /// An annotation's geometry is none of rectangle, polygon and circle.
    UnknownShape => "unknown-shape",
    /// An annotation's geometry has not the points its shape takes: a rectangle two, a polygon
    /// three or more, a circle one.
    ShapePointCount => "shape-point-count",
    /// An annotation's box has no width or no height.
    EmptyBox => "empty-box",
    /// An annotation's box reaches outside the image.
    BoxOutsideImage => "box-outside-image",
    /// A number of an annotation's geometry is not finite, or would be written with an
    /// exponent.
    UnwritableNumber => "unwritable-number",
    /// An annotation's colour is not `#rrggbb`.
    ColourNotHex => "colour-not-hex",
    /// An annotation's remark holds U+001E between its first and its last character.
    RemarkHoldsRecordSeparator => "remark-holds-record-separator",
    /// An annotation is not on exactly one resource, of a class the mapping names under `on`.
    RegionTarget => "region-target",
    /// An annotation the editors keep is on a resource that a hook dropped, which has no row.
    RegionOnDroppedPage => "region-on-dropped-page",
    /// An annotation link links to a resource that is neither an annotation nor of a class the
    /// mapping names under `link-targets`.
    LinkTarget => "link-target",
    /// An annotation link links to nothing.
    LinkWithoutTarget => "link-without-target",
    /// An annotation link the editors keep links to a resource that a hook dropped, which has
    /// no row.
    LinkToDroppedPage => "link-to-dropped-page",
    /// An annotation link the editors keep has neither a curated name nor a comment to be
    /// named by.
    LinkWithoutName => "link-without-name",
    /// A text that must not be empty is empty once the whitespace around it is gone: a label
    /// that is read as a text, or a value of a property that refuses an empty one.
    EmptyValue => "empty-value",
    /// A resource has not as many values under a property as the property takes.
    ValueCount => "value-count",
    /// A curated text that must name a node of a list names none of its nodes.
    UnknownListNode => "unknown-list-node",
    /// The curated texts of a property that has one per language are not one in each of the
    /// site's languages.
    CuratedLanguages => "curated-languages",
    /// Two resources of one class have the same file.
    SharedFile => "shared-file",
    /// The mapping declares a hook, and the builder was given none for the project.
    HookNotRegistered => "hook-not-registered",
    /// The builder was given a hook for a project whose mapping declares none.
    HookNotDeclared => "hook-not-declared",
    /// A hook read or changed what the mapping's `hook` node does not declare.
    HookExceedsDeclaration => "hook-exceeds-declaration",
    /// A link points at a resource that a hook dropped, which has no row.
    LinkTargetWithoutRow => "link-target-without-row",
    /// The id of a resource, an Image, an annotation or an annotation link is empty or holds a
    /// character outside `A-Z a-z 0-9 - . _ ~`, which a URL carries unchanged.
    IdNotUrlSafe => "id-not-url-safe",
}

/// One refusal. `Ord` is the rule in declaration order, then the subject, then the detail.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuildError {
    pub rule: Rule,
    /// What the refusal is about: a resource IRI or a CPE id.
    pub subject: String,
    pub detail: Option<String>,
}

impl BuildError {
    /// The refusal of `subject` by `rule`, with a detail.
    pub(crate) fn refused(rule: Rule, subject: &str, detail: impl Into<String>) -> Self {
        Self {
            rule,
            subject: subject.to_string(),
            detail: Some(detail.into()),
        }
    }

    /// The name of the rule that refused.
    pub fn kind(&self) -> &'static str {
        self.rule.name()
    }
}

/// Refuses by `rule` every key of `owners` that more than one owner has. The subject is the
/// key, and the detail names its owners.
pub(crate) fn refuse_shared_keys(rule: Rule, owners: &BTreeMap<&str, Vec<&str>>, errors: &mut Vec<BuildError>) {
    for (key, owners) in owners {
        if owners.len() > 1 {
            errors.push(BuildError::refused(rule, key, owners.join(", ")));
        }
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind(), self.subject)?;
        match &self.detail {
            Some(detail) => write!(f, ": {detail}"),
            None => Ok(()),
        }
    }
}

impl Error for BuildError {}

/// Every refusal of one `build` call: sorted, and never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildErrors(Vec<BuildError>);

impl BuildErrors {
    /// `None` when there is no error to report.
    pub fn new(mut errors: Vec<BuildError>) -> Option<Self> {
        if errors.is_empty() {
            return None;
        }
        errors.sort();
        Some(Self(errors))
    }

    pub fn as_slice(&self) -> &[BuildError] {
        &self.0
    }
}

/// One error per line.
impl fmt::Display for BuildErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.0.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

impl Error for BuildErrors {}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(rule: Rule, subject: &str, detail: Option<&str>) -> BuildError {
        BuildError {
            rule,
            subject: subject.to_string(),
            detail: detail.map(str::to_string),
        }
    }

    #[test]
    fn test_new_no_errors_returns_none() {
        assert_eq!(BuildErrors::new(Vec::new()), None);
    }

    #[test]
    fn test_new_sorts_by_rule_then_subject_then_detail() {
        let errors = BuildErrors::new(vec![
            error(Rule::InteriorNul, "a", None),
            error(Rule::ContractViolation, "b", Some("z")),
            error(Rule::ContractViolation, "b", None),
            error(Rule::ContractViolation, "a", Some("y")),
        ])
        .expect("four errors");

        assert_eq!(
            errors.as_slice(),
            [
                error(Rule::ContractViolation, "a", Some("y")),
                error(Rule::ContractViolation, "b", None),
                error(Rule::ContractViolation, "b", Some("z")),
                error(Rule::InteriorNul, "a", None),
            ]
        );
    }

    #[test]
    fn test_display_names_rule_subject_and_detail_one_error_per_line() {
        let errors = BuildErrors::new(vec![
            error(Rule::InteriorNul, "http://rdfh.ch/0803/b", Some("label")),
            error(Rule::ContractViolation, "http://rdfh.ch/0803/a", None),
        ])
        .expect("two errors");

        assert_eq!(
            errors.to_string(),
            "contract-violation: http://rdfh.ch/0803/a\ninterior-nul: http://rdfh.ch/0803/b: label"
        );
    }
}
