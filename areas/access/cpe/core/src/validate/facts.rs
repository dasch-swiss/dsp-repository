//! The facts the data-dependent boot checks read from a project's data: the
//! [`ProjectFacts`] seam, its error, and the rows it returns.
//!
//! This module is in `cpe-core` and imports nothing outside the domain. The
//! validator declares what it needs to know about the data here; the store
//! implements it (`LiveProjectFacts`) and `Project::load` injects it.
//! A new data check that needs a new lookup adds a method here and its
//! forwarding in the store.

use std::collections::HashSet;

use crate::diagnostic::{Diagnostic, DiagnosticKind, Severity, ValidationReport};

/// A lookup against the project's data failed to execute.
///
/// Wraps the store's own error and displays exactly as it does, so a
/// diagnostic that formats `{e}` reads the same whichever store answered.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FactsError(Box<dyn std::error::Error + Send + Sync>);

impl FactsError {
    /// Wrap a store error.
    pub fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(Box::new(source))
    }
}

/// What the data-dependent boot checks ask of a project's data.
///
/// Every check takes `&dyn ProjectFacts`: the per-project registry's check
/// functions are fn pointers and cannot be generic, so keep the trait
/// dyn-compatible (no generic methods, no `impl Trait` returns, no `Self` by
/// value).
pub trait ProjectFacts {
    fn resource_exists(&self, id: &str) -> Result<bool, FactsError>;

    fn properties_present_on_class_of(&self, id: &str) -> Result<HashSet<String>, FactsError>;

    fn malformed_property_rows(&self) -> Result<Vec<(String, String, String, &'static str)>, FactsError>;

    fn malformed_annotation_rows(&self) -> Result<AnnotationCheckRows, FactsError>;

    fn malformed_data_ark_rows(&self) -> Result<Vec<DataArkRow>, FactsError>;

    fn dangling_link_targets(&self) -> Result<Vec<(String, String)>, FactsError>;

    fn class_exists(&self, class: &str) -> Result<bool, FactsError>;

    fn compound_has_parts(&self, parent_id: &str) -> Result<bool, FactsError>;

    fn compound_member_classes(&self, parent_id: Option<&str>) -> Result<Vec<String>, FactsError>;

    fn property_exists_anywhere(&self, name: &str) -> Result<bool, FactsError>;

    fn properties_present_on_class(&self, class: &str) -> Result<HashSet<String>, FactsError>;

    fn property_value_types_on_class(&self, class: &str, property: &str) -> Result<HashSet<String>, FactsError>;

    fn class_of(&self, id: &str) -> Result<Option<String>, FactsError>;

    fn link_target_classes_on_class(&self, class: &str, property: &str) -> Result<HashSet<String>, FactsError>;

    fn class_has_representations(&self, class: &str) -> Result<bool, FactsError>;

    fn linking_resource_lacks_property(
        &self,
        class: &str,
        link_properties: &[String],
        property: &str,
    ) -> Result<bool, FactsError>;

    fn showcase_annotation(&self, id: &str, media_property: &str) -> Result<Option<ShowcaseAnnotationRow>, FactsError>;

    fn property_texts(&self, resource_id: &str, name: &str) -> Result<Vec<String>, FactsError>;

    fn link_targets(&self, link_id: &str) -> Result<Option<Vec<LinkTargetRow>>, FactsError>;
}

/// Push an `InternalQueryError` diagnostic (Error severity) for a validator
/// query that failed to execute against the freshly-loaded DB. `context` names
/// the check (goes in the kind); `message` is the full human-readable text.
pub(crate) fn push_internal_query_error(report: &mut ValidationReport, context: &str, message: String) {
    report.push(Diagnostic {
        kind: DiagnosticKind::InternalQueryError { context: context.to_owned() },
        message,
        span: None,
        severity: Severity::Error,
    });
}

/// One `annotations` row as stored, for the `MalformedAnnotation` boot check.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationCheckRow {
    pub id: String,
    pub shape: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub points: Option<String>,
    pub color: Option<String>,
}

/// The inputs of the `MalformedAnnotation` boot check: every `annotations`
/// row, unjudged (the tolerance and the points grammar are the check's), and
/// the ids of links with no target row. Both ordered by id.
#[derive(Debug, Default, PartialEq)]
pub struct AnnotationCheckRows {
    pub annotations: Vec<AnnotationCheckRow>,
    pub empty_links: Vec<String>,
}

/// One stored data ARK, for the `MalformedDataArk` boot check: the table it
/// sits in (`"resources"` or `"annotations"`), the row's id and the value.
#[derive(Debug, Clone, PartialEq)]
pub struct DataArkRow {
    pub table: &'static str,
    pub id: String,
    pub ark: String,
}

/// A showcase annotation as the boot checks read it.
#[derive(Debug, Clone, PartialEq)]
pub struct ShowcaseAnnotationRow {
    /// The annotated resource.
    pub resource_id: String,
    pub remark: Option<String>,
    /// The stored size of that resource's image, reached through its
    /// `media_property`; `None` when either is unknown.
    pub width: Option<i64>,
    pub height: Option<i64>,
    /// That image's stored URI, `None` when there is no image.
    pub uri: Option<String>,
}

/// One target of an annotation link: exactly one of the two is set.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkTargetRow {
    pub annotation_id: Option<String>,
    pub resource_id: Option<String>,
}
