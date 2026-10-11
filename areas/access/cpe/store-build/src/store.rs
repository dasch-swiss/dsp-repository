//! The row model: one plain struct per table of `cpe-store`'s schema, with that table's columns
//! in the order the store file writes them. `representations.id` is left out: SQLite assigns it.

/// The ten tables, in the order the store file writes them: a table stands after every table it
/// references, because the file is loaded with foreign keys on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    Lists,
    ListNodes,
    ListNodeLabels,
    Resources,
    Properties,
    Representations,
    CompoundParts,
    Annotations,
    AnnotationLinks,
    AnnotationLinkTargets,
}

impl Table {
    pub const ALL: [Table; 10] = [
        Table::Lists,
        Table::ListNodes,
        Table::ListNodeLabels,
        Table::Resources,
        Table::Properties,
        Table::Representations,
        Table::CompoundParts,
        Table::Annotations,
        Table::AnnotationLinks,
        Table::AnnotationLinkTargets,
    ];

    /// The table's name in the schema.
    pub fn name(self) -> &'static str {
        match self {
            Table::Lists => "lists",
            Table::ListNodes => "list_nodes",
            Table::ListNodeLabels => "list_node_labels",
            Table::Resources => "resources",
            Table::Properties => "properties",
            Table::Representations => "representations",
            Table::CompoundParts => "compound_parts",
            Table::Annotations => "annotations",
            Table::AnnotationLinks => "annotation_links",
            Table::AnnotationLinkTargets => "annotation_link_targets",
        }
    }
}

/// A store's rows, each table in the order the file writes them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Store {
    pub lists: Vec<ListRow>,
    pub list_nodes: Vec<ListNodeRow>,
    pub list_node_labels: Vec<ListNodeLabelRow>,
    pub resources: Vec<ResourceRow>,
    pub properties: Vec<PropertyRow>,
    pub representations: Vec<RepresentationRow>,
    pub compound_parts: Vec<CompoundPartRow>,
    pub annotations: Vec<AnnotationRow>,
    pub annotation_links: Vec<AnnotationLinkRow>,
    pub annotation_link_targets: Vec<AnnotationLinkTargetRow>,
}

impl Store {
    pub fn row_count(&self, table: Table) -> usize {
        match table {
            Table::Lists => self.lists.len(),
            Table::ListNodes => self.list_nodes.len(),
            Table::ListNodeLabels => self.list_node_labels.len(),
            Table::Resources => self.resources.len(),
            Table::Properties => self.properties.len(),
            Table::Representations => self.representations.len(),
            Table::CompoundParts => self.compound_parts.len(),
            Table::Annotations => self.annotations.len(),
            Table::AnnotationLinks => self.annotation_links.len(),
            Table::AnnotationLinkTargets => self.annotation_link_targets.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListRow {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListNodeRow {
    pub id: String,
    pub list_id: String,
    pub parent_id: Option<String>,
    pub seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListNodeLabelRow {
    pub node_id: String,
    pub lang: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRow {
    pub id: String,
    pub class: String,
    pub ark: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyRow {
    pub resource_id: String,
    pub name: String,
    pub seq: i64,
    pub value_type: String,
    pub value_text: Option<String>,
    pub value_lang: Option<String>,
    pub value_int: Option<i64>,
    pub value_real: Option<Real>,
    pub date_start: Option<String>,
    pub date_end: Option<String>,
    pub date_precision: Option<String>,
    pub date_display: Option<String>,
    pub date_calendar: Option<String>,
    pub target_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationRow {
    pub resource_id: String,
    pub kind: String,
    pub uri: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub alt: Option<String>,
    pub attribution: Option<String>,
    pub seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundPartRow {
    pub parent_id: String,
    pub child_id: String,
    pub seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationRow {
    pub id: String,
    pub resource_id: String,
    pub shape: String,
    pub x: Real,
    pub y: Real,
    pub w: Real,
    pub h: Real,
    pub points: Option<String>,
    pub color: Option<String>,
    pub remark: Option<String>,
    pub remark_lang: Option<String>,
    pub ark: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationLinkRow {
    pub id: String,
    pub name: String,
    pub name_lang: Option<String>,
    pub comment: Option<String>,
    pub comment_lang: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationLinkTargetRow {
    pub link_id: String,
    pub seq: i64,
    pub annotation_id: Option<String>,
    pub resource_id: Option<String>,
}

/// A `REAL` value as the text the store file carries: the shortest decimal form that reads back
/// as the same `f64`.
///
/// Holding the text, not the number, lets the rows be `Eq` and the writers infallible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Real(String);

/// Why [`Real::new`] refused a number; each holds the number as it would have been written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RealError {
    #[error("{0} is not a finite number")]
    NonFinite(String),
    #[error("{0} is written with an exponent")]
    Exponent(String),
}

impl Real {
    /// Refuses a non-finite number, and one whose text has an exponent: the store's numbers are
    /// in the one form Rust's `{:?}` and Python's `repr` share, and the two spell an exponent
    /// differently.
    pub fn new(value: f64) -> Result<Self, RealError> {
        let text = format!("{value:?}");
        if !value.is_finite() {
            return Err(RealError::NonFinite(text));
        }
        if text.contains('e') {
            return Err(RealError::Exponent(text));
        }
        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_new_whole_number_keeps_its_fraction_digit() {
        assert_eq!(Real::new(1.0).map(|real| real.0), Ok("1.0".to_string()));
    }

    #[test]
    fn test_real_new_seventeen_digit_value_is_written_in_full() {
        assert_eq!(Real::new(0.1 + 0.2).map(|real| real.0), Ok("0.30000000000000004".to_string()));
    }

    #[test]
    fn test_real_new_exponent_form_is_refused() {
        assert_eq!(Real::new(1e-5), Err(RealError::Exponent("1e-5".to_string())));
        assert_eq!(Real::new(1e16), Err(RealError::Exponent("1e16".to_string())));
    }

    #[test]
    fn test_real_new_non_finite_value_is_refused() {
        assert_eq!(Real::new(f64::NAN), Err(RealError::NonFinite("NaN".to_string())));
        assert_eq!(Real::new(f64::NEG_INFINITY), Err(RealError::NonFinite("-inf".to_string())));
    }
}
