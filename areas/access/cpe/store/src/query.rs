//! Query layer: domain-shaped functions over the EAV `properties` table.
//!
//! The query functions take a `&rusqlite::Connection` directly (no Axum, no
//! async) and are `pub(crate)` or private; their callers lock. The composite
//! reads over a project's store, and every lock a request takes, live in the
//! child module `reads` (`ProjectReads`); `cpe-web`'s `Project` delegates to
//! it. The child module `page` builds one page request's view data over them.
//!
//! No `pub` fn in this file takes or returns a `rusqlite` type, and
//! `QueryError` carries the driver's error wrapped (`error::SqliteError`).
//! There is intentionally no `From<rusqlite::Error>` impl; call sites use
//! `.map_err(QueryError::query)`.

pub mod page;
pub mod reads;

use std::collections::{BTreeSet, HashMap, HashSet};

use cpe_core::ir::{
    parse_annotation_points, Annotation, AnnotationCrop, AnnotationLink, AnnotationPartner, AnnotationShape, Calendar,
    CropImage, DataArks, DatePrecision, GroupKey, IncomingGroupBy, IncomingLink, IncomingLinks, IncomingLinksSpec,
    IncomingOrder, LaneNode, MemberHeader, MemberParent, PagePlate, Property, PropertyValue, Representation,
    RepresentationKind, Resource, ResourceLink, Sort, SortDir, SortKey, Value, WhereClause,
};
use cpe_core::lang::Lang;
use cpe_core::util::fts_match_expr;
use cpe_core::validate::facts::{
    AnnotationCheckRow, AnnotationCheckRows, DataArkRow, FactsError, LinkTargetRow, ProjectFacts, ShowcaseAnnotationRow,
};
use rusqlite::{Connection, OptionalExtension};

use crate::db::ProjectStore;
use crate::error::SqliteError;

// Error type

/// Errors that can occur while executing a query against the project's
/// in-memory SQLite.
///
/// `Query` and `SearchIndexBuild` carry the driver's error, wrapped, as
/// their source. There is no `From<rusqlite::Error>` impl, so nothing
/// converts to `QueryError` implicitly. This mirrors the `DbError` pattern in
/// `store/src/db.rs`.
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("database query failed: {0}")]
    Query(#[source] SqliteError),
    /// A `properties.value_type` string was not one of the schema-declared
    /// values. The schema CHECK constraint makes this unreachable in practice;
    /// the variant exists so the unknown-type arm can return a typed error
    /// instead of constructing a fake rusqlite error.
    #[error("unknown value_type in properties table: {0}")]
    UnknownValueType(String),
    /// A `Value::Integer(i128)` could not be safely narrowed to `i64` for
    /// SQLite binding. The schema stores integers as i64; an i128 value
    /// outside `[i64::MIN, i64::MAX]` cannot be represented.
    #[error("integer value {0} is out of the i64 range required for SQLite binding")]
    ValueOutOfRange(i128),
    /// Building a `search-bar`'s FTS5 virtual table (schema creation or the
    /// boot-time content insert) failed. Distinct from `Query` so a boot
    /// failure building the search index can be reported with its own
    /// context rather than a generic "database query failed" message.
    #[error("failed to build FTS5 search index: {0}")]
    SearchIndexBuild(#[source] SqliteError),
}

impl QueryError {
    pub(crate) fn query(error: rusqlite::Error) -> Self {
        Self::Query(SqliteError::new(error))
    }

    pub(crate) fn search_index_build(error: rusqlite::Error) -> Self {
        Self::SearchIndexBuild(SqliteError::new(error))
    }
}

impl From<QueryError> for FactsError {
    fn from(e: QueryError) -> Self {
        FactsError::new(e)
    }
}

// The validator's facts

/// The live store's answer to the validator's [`ProjectFacts`]: each method
/// forwards to the query function of the same name (`showcase_annotation` to
/// `showcase_annotation_row`), locking the store for that one query.
/// `ProjectStore::facts` builds it; `Project::load` hands it to the validator.
pub struct LiveProjectFacts<'store> {
    store: &'store ProjectStore,
}

impl<'store> LiveProjectFacts<'store> {
    pub(crate) fn new(store: &'store ProjectStore) -> Self {
        Self { store }
    }
}

impl ProjectFacts for LiveProjectFacts<'_> {
    fn resource_exists(&self, id: &str) -> Result<bool, FactsError> {
        Ok(resource_exists(&self.store.lock(), id)?)
    }

    fn properties_present_on_class_of(&self, id: &str) -> Result<HashSet<String>, FactsError> {
        Ok(properties_present_on_class_of(&self.store.lock(), id)?)
    }

    fn malformed_property_rows(&self) -> Result<Vec<(String, String, String, &'static str)>, FactsError> {
        Ok(malformed_property_rows(&self.store.lock())?)
    }

    fn malformed_annotation_rows(&self) -> Result<AnnotationCheckRows, FactsError> {
        Ok(malformed_annotation_rows(&self.store.lock())?)
    }

    fn malformed_data_ark_rows(&self) -> Result<Vec<DataArkRow>, FactsError> {
        Ok(malformed_data_ark_rows(&self.store.lock())?)
    }

    fn dangling_link_targets(&self) -> Result<Vec<(String, String)>, FactsError> {
        Ok(dangling_link_targets(&self.store.lock())?)
    }

    fn class_exists(&self, class: &str) -> Result<bool, FactsError> {
        Ok(class_exists(&self.store.lock(), class)?)
    }

    fn compound_has_parts(&self, parent_id: &str) -> Result<bool, FactsError> {
        Ok(compound_has_parts(&self.store.lock(), parent_id)?)
    }

    fn compound_member_classes(&self, parent_id: Option<&str>) -> Result<Vec<String>, FactsError> {
        Ok(compound_member_classes(&self.store.lock(), parent_id)?)
    }

    fn property_exists_anywhere(&self, name: &str) -> Result<bool, FactsError> {
        Ok(property_exists_anywhere(&self.store.lock(), name)?)
    }

    fn properties_present_on_class(&self, class: &str) -> Result<HashSet<String>, FactsError> {
        Ok(properties_present_on_class(&self.store.lock(), class)?)
    }

    fn property_value_types_on_class(&self, class: &str, property: &str) -> Result<HashSet<String>, FactsError> {
        Ok(property_value_types_on_class(&self.store.lock(), class, property)?)
    }

    fn class_of(&self, id: &str) -> Result<Option<String>, FactsError> {
        Ok(class_of(&self.store.lock(), id)?)
    }

    fn link_target_classes_on_class(&self, class: &str, property: &str) -> Result<HashSet<String>, FactsError> {
        Ok(link_target_classes_on_class(&self.store.lock(), class, property)?)
    }

    fn class_has_representations(&self, class: &str) -> Result<bool, FactsError> {
        Ok(class_has_representations(&self.store.lock(), class)?)
    }

    fn linking_resource_lacks_property(
        &self,
        class: &str,
        link_properties: &[String],
        property: &str,
    ) -> Result<bool, FactsError> {
        Ok(linking_resource_lacks_property(
            &self.store.lock(),
            class,
            link_properties,
            property,
        )?)
    }

    fn showcase_annotation(&self, id: &str, media_property: &str) -> Result<Option<ShowcaseAnnotationRow>, FactsError> {
        Ok(showcase_annotation_row(&self.store.lock(), id, media_property)?)
    }

    fn property_texts(&self, resource_id: &str, name: &str) -> Result<Vec<String>, FactsError> {
        Ok(property_texts(&self.store.lock(), resource_id, name)?)
    }

    fn link_targets(&self, link_id: &str) -> Result<Option<Vec<LinkTargetRow>>, FactsError> {
        Ok(link_targets(&self.store.lock(), link_id)?)
    }
}

// Crate-internal functions
//
// These functions take a `&rusqlite::Connection` directly and never lock.
// They are `pub(crate)` or private, so they are not part of `cpe-store`'s
// public surface. Their callers hold the lock: `ProjectStore` for the two
// load steps (`build_search_indexes`, `data_arks`), `LiveProjectFacts` for a
// fact, `ProjectReads` (`query::reads`) for a read.

/// Fetch a `Resource` by id from the project database.
///
/// Returns `Ok(None)` when the id is not present in `resources` — a
/// deliberate "not found" that the caller should map to HTTP 404.
/// Returns `Err(QueryError)` only for unexpected database failures.
///
/// Language resolution for labels:
/// - `text`/`richtext` values are returned as-is (all langs); the renderer picks the matching
///   language tag from the multi-valued set.
/// - `link` target labels and `list_value` node labels are resolved to `lang` inside this function
///   (with fallback: requested → any first present).
pub(crate) fn get_resource(conn: &Connection, id: &str, lang: &Lang) -> Result<Option<Resource>, QueryError> {
    // Step 1 — confirm the resource exists and fetch its class.
    let class: Option<String> = conn
        .query_row("SELECT class FROM resources WHERE id = ?1", [id], |row| row.get(0))
        .optional()
        .map_err(QueryError::query)?;

    let class = match class {
        None => return Ok(None),
        Some(c) => c,
    };

    // Step 2 — fetch all property rows ordered by (name, seq).
    let mut stmt = conn
        .prepare(
            "SELECT name, seq, value_type, \
                    value_text, value_lang, value_int, value_real, \
                    date_start, date_end, date_precision, target_id, \
                    date_display, date_calendar \
             FROM properties \
             WHERE resource_id = ?1 \
             ORDER BY name, seq",
        )
        .map_err(QueryError::query)?;

    // Step 3 — build PropertyValue per row, group into Property by name.
    let mut properties: Vec<Property> = Vec::new();

    let mut rows = stmt.query([id]).map_err(QueryError::query)?;
    while let Some(row) = rows.next().map_err(QueryError::query)? {
        let name: String = row.get(0).map_err(QueryError::query)?;
        let _seq: i64 = row.get(1).map_err(QueryError::query)?; // ORDER BY handles ordering; kept
                                                                // for clarity
        let value_type: String = row.get(2).map_err(QueryError::query)?;
        let value_text: Option<String> = row.get(3).map_err(QueryError::query)?;
        let value_lang: Option<String> = row.get(4).map_err(QueryError::query)?;
        let value_int: Option<i64> = row.get(5).map_err(QueryError::query)?;
        let value_real: Option<f64> = row.get(6).map_err(QueryError::query)?;
        let date_start: Option<String> = row.get(7).map_err(QueryError::query)?;
        let date_end: Option<String> = row.get(8).map_err(QueryError::query)?;
        let date_precision: Option<String> = row.get(9).map_err(QueryError::query)?;
        let target_id: Option<String> = row.get(10).map_err(QueryError::query)?;
        let date_display: Option<String> = row.get(11).map_err(QueryError::query)?;
        let date_calendar: Option<String> = row.get(12).map_err(QueryError::query)?;

        let value = build_property_value(
            &value_type,
            value_text,
            value_lang,
            value_int,
            value_real,
            DateColumns {
                start: date_start,
                end: date_end,
                precision: date_precision,
                display: date_display,
                calendar: date_calendar,
            },
            target_id,
            conn,
            lang,
        )?;

        // Group consecutive rows with the same name into one Property.
        match properties.last_mut() {
            Some(p) if p.name == name => {
                p.values.push(value);
            }
            _ => {
                properties.push(Property { name, values: vec![value] });
            }
        }
    }

    // Step 4 — fetch representation rows ordered by seq.
    let representations = get_representations(conn, id)?;

    Ok(Some(Resource { id: id.to_owned(), class, properties, representations }))
}

/// Fetch representation rows for a resource, ordered by `seq`.
fn get_representations(conn: &Connection, resource_id: &str) -> Result<Vec<Representation>, QueryError> {
    let mut stmt = conn
        .prepare(
            "SELECT kind, uri, width, height, alt, attribution, seq \
             FROM representations \
             WHERE resource_id = ?1 \
             ORDER BY seq",
        )
        .map_err(QueryError::query)?;

    let rows = stmt
        .query_map([resource_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;

    let mut representations = Vec::new();
    for (kind_str, uri, width, height, alt, attribution, seq) in rows {
        // Unknown kinds are silently skipped; the schema CHECK prevents them
        // in practice, but be defensive if the DB is loaded from external data.
        if let Some(kind) = RepresentationKind::from_schema_str(&kind_str) {
            representations.push(Representation { kind, uri, width, height, alt, attribution, seq });
        }
    }
    Ok(representations)
}

/// Return `true` when a resource with the given id exists in the database.
pub(crate) fn resource_exists(conn: &Connection, id: &str) -> Result<bool, QueryError> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM resources WHERE id = ?1", [id], |row| row.get(0))
        .map_err(QueryError::query)?;
    Ok(count > 0)
}

/// Return the class of the resource with the given id, or `None` when no such
/// resource exists. Used when resolving link `target_class` and by the FK-
/// existence validator.
///
/// Returns `Ok(None)` for an absent id (not an error). Returns `Err` only for
/// unexpected database failures.
pub(crate) fn class_of(conn: &Connection, id: &str) -> Result<Option<String>, QueryError> {
    conn.query_row("SELECT class FROM resources WHERE id = ?1", [id], |row| row.get(0))
        .optional()
        .map_err(QueryError::query)
}

/// Return the union of distinct property names across all resources that share
/// the same class as the resource identified by `id`.
///
/// When `id` does not exist this returns an empty set (no error, no panic).
pub(crate) fn properties_present_on_class_of(conn: &Connection, id: &str) -> Result<HashSet<String>, QueryError> {
    // Look up the class of the given resource.
    let class: Option<String> = conn
        .query_row("SELECT class FROM resources WHERE id = ?1", [id], |row| row.get(0))
        .optional()
        .map_err(QueryError::query)?;

    let class = match class {
        None => return Ok(HashSet::new()),
        Some(c) => c,
    };

    // Collect distinct property names across all resources of that class.
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT p.name \
             FROM properties p \
             JOIN resources r ON r.id = p.resource_id \
             WHERE r.class = ?1",
        )
        .map_err(QueryError::query)?;

    let names: HashSet<String> = stmt
        .query_map([&class], |row| row.get(0))
        .map_err(QueryError::query)?
        .collect::<Result<HashSet<String>, _>>()
        .map_err(QueryError::query)?;

    Ok(names)
}

// Resource listing functions

/// Return `true` when at least one resource with the given `class` exists.
///
/// Used by boot validation to catch unknown class names on `resource-list`
/// before attempting to serve a page. Returns `Ok(false)` for an absent class
/// (not an error).
pub(crate) fn class_exists(conn: &Connection, class: &str) -> Result<bool, QueryError> {
    let exists: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM resources WHERE class = ?1)", [class], |row| {
            row.get(0)
        })
        .map_err(QueryError::query)?;
    Ok(exists)
}

/// Return `true` when at least one property row with the given `name` exists
/// anywhere in the database (across all resources and classes).
///
/// Used by boot validation to distinguish a property name that is absent from a
/// specific class (sparse → warn) from one that doesn't exist at all (hard-fail).
pub(crate) fn property_exists_anywhere(conn: &Connection, name: &str) -> Result<bool, QueryError> {
    let exists: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM properties WHERE name = ?1)", [name], |row| {
            row.get(0)
        })
        .map_err(QueryError::query)?;
    Ok(exists)
}

/// Return the union of distinct property names across all resources that share
/// the given `class`.
///
/// Class-keyed sibling of the id-keyed `properties_present_on_class_of`. Returns
/// an empty set when no resources of `class` exist (not an error).
///
/// Used by boot validation to warn about sparse properties on a `resource-list`:
/// a sort or where property that exists somewhere but not on this class.
pub(crate) fn properties_present_on_class(conn: &Connection, class: &str) -> Result<HashSet<String>, QueryError> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT p.name \
             FROM properties p \
             JOIN resources r ON r.id = p.resource_id \
             WHERE r.class = ?1",
        )
        .map_err(QueryError::query)?;

    let names: HashSet<String> = stmt
        .query_map([class], |row| row.get(0))
        .map_err(QueryError::query)?
        .collect::<Result<HashSet<String>, _>>()
        .map_err(QueryError::query)?;

    Ok(names)
}

/// The exact SQL `count_resources` runs, given a `build_where` fragment.
///
/// Factored out so the query-plan regression test
/// (`target_in_where_uses_composite_target_index`) can `EXPLAIN QUERY PLAN`
/// the **real** statement rather than a hand-copied lookalike that could
/// drift away from it.
fn count_resources_sql(where_fragment: &str) -> String {
    format!(
        "SELECT COUNT(DISTINCT r.id) \
         FROM resources r \
         WHERE r.class = ?1{where_fragment}"
    )
}

/// Return the count of distinct resource ids matching `class` + `wheres`.
///
/// Shares the exact same `build_where` fragment+params builder as
/// `list_resource_ids`, so the two can never diverge.
pub(crate) fn count_resources(conn: &Connection, class: &str, wheres: &[WhereClause]) -> Result<usize, QueryError> {
    let (where_fragment, where_params) = build_where(wheres)?;

    let sql = count_resources_sql(&where_fragment);

    // class goes in slot ?1; where params follow.
    let mut all_params: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Text(class.to_owned())];
    all_params.extend(where_params);

    let count: i64 = conn
        .query_row(&sql, rusqlite::params_from_iter(&all_params), |row| row.get(0))
        .map_err(QueryError::query)?;

    // COUNT(*) is always ≥ 0 and fits in usize on any supported platform.
    Ok(usize::try_from(count).unwrap_or(0))
}

/// Return a page of distinct resource ids of `class`, filtered by `wheres`,
/// sorted by `sorts`, paginated with `limit` and `offset`.
///
/// ## Duplicate-row safety (EAV multi-valued properties)
///
/// A resource may have multiple rows for its sort property (e.g. Alice's `name`
/// has EN and DE rows). A plain JOIN would multiply the resource, corrupting
/// both the result count and feeding duplicates to hydration. This function
/// avoids that by collapsing each resource to one row via GROUP BY + MIN
/// aggregate inside a subquery:
///
/// ```sql
/// SELECT id
/// FROM (
///     SELECT r.id AS id,
///            COALESCE(
///                MIN(CASE WHEN pS.value_lang = ?lang THEN pS.value_text END),
///                MIN(CASE WHEN pS.value_lang IS NULL THEN pS.value_text END),
///                MIN(pS.value_text),
///                MIN(pS.value_int),
///                MIN(pS.value_real),
///                MIN(pS.date_start)
///            ) AS sort_val
///     FROM resources r
///     LEFT JOIN properties pS ON pS.resource_id = r.id AND pS.name = ?
///     WHERE r.class = ? [AND <where EXISTS clauses>]
///     GROUP BY r.id
/// )
/// ORDER BY (sort_val IS NULL), sort_val <ASC|DESC>, id
/// LIMIT ? OFFSET ?
/// ```
///
/// The language-aware COALESCE picks the sort key in this priority order:
/// 1. Requested language (exact lang match on `value_lang`).
/// 2. Universal / untagged text (`value_lang IS NULL`).
/// 3. Any text (cross-language fallback).
/// 4. Integer (`value_int`) — native numeric affinity, no text cast → correct numeric ordering (1,
///    2, … 10, 11, not lexical 1, 10, 11, 2…).
/// 5. Decimal (`value_real`).
/// 6. Date (`date_start`).
///
/// A property is single-typed in practice, so for a text property the text
/// arms resolve and the numeric/real/date arms are NULL. For an integer
/// property the text arms are NULL (no `value_text`) and it falls through to
/// `MIN(value_int)`, which SQLite orders with NUMERIC affinity — correct
/// numeric order without any text cast.
///
/// The outer ORDER BY references the **aliased aggregate** `sort_val` (not the
/// raw join column). NULLs sort last regardless of direction via
/// `(sort_val IS NULL)`; `id` is the stable tiebreak.
///
/// For multiple sort criteria, each `SortKey::Property` clause adds its own
/// LEFT JOIN (with a distinct alias `pS0`, `pS1`, …) and its own lang-aware
/// aggregate column. `SortKey::Class`/`SortKey::Id` clauses instead alias the
/// bare `r.class`/`r.id` column directly (no join, no bound params, no
/// `(sort_val{i} IS NULL)` ORDER BY term — neither column is ever NULL). Field
/// and property sorts can be freely mixed in one call; field variants
/// contribute zero placeholders, so the left-to-right bind order (lang
/// params, then join-name params, then `class`, then `where` params, then
/// `limit`/`offset`) is unaffected by which sorts are field vs. property.
///
/// `where` clauses map to EXISTS subqueries (not joins) to prevent row
/// multiplication on multi-valued filter properties:
/// `AND EXISTS (SELECT 1 FROM properties pW WHERE pW.resource_id = r.id AND
///              pW.name = ? AND <predicate>)`.
pub(crate) fn list_resource_ids(
    conn: &Connection,
    class: &str,
    wheres: &[WhereClause],
    sorts: &[Sort],
    limit: u32,
    offset: u32,
    lang: &Lang,
) -> Result<Vec<String>, QueryError> {
    let (where_fragment, where_params) = build_where(wheres)?;

    // Build the inner SELECT (GROUP BY subquery)
    //
    // Each sort clause contributes:
    //   - one LEFT JOIN alias (pS0, pS1, …)
    //   - one lang-aware COALESCE aggregate column (sort_val0, sort_val1, …)
    //   - one ?lang bind parameter (the requested language code)
    // When there are no sort clauses, no JOIN is emitted and the outer ORDER BY
    // falls back to `id` only.

    let mut join_clauses = String::new();
    let mut select_sort_cols = String::new();
    // The inner SQL has the form:
    //   SELECT r.id, COALESCE(MIN(CASE WHEN pS0.value_lang = ? ...), ...) AS sort_val0, ...
    //   FROM resources r LEFT JOIN properties pS0 ON ... AND pS0.name = ?, ...
    //   WHERE r.class = ? ...
    //
    // Positional ? params bind left-to-right: the COALESCE CASE ?-lang params appear
    // FIRST (in the SELECT) and the JOIN ?-property-name params appear SECOND (in the
    // FROM clause). They must be collected separately and combined in that order.
    let mut lang_params: Vec<rusqlite::types::Value> = Vec::new(); // for CASE WHEN ... = ?
    let mut join_name_params: Vec<rusqlite::types::Value> = Vec::new(); // for ON ... name = ?

    for (i, sort) in sorts.iter().enumerate() {
        match &sort.key {
            SortKey::Property(name) => {
                let alias = format!("pS{i}");
                join_clauses.push_str(&format!(
                    " LEFT JOIN properties {alias} ON {alias}.resource_id = r.id AND {alias}.name = ?"
                ));
                // Language-aware sort key, in priority order:
                //   1. Requested language (exact value_lang match — ?lang param)
                //   2. Universal/untagged text (value_lang IS NULL)
                //   3. Any text (cross-language fallback)
                //   4. Integer — native NUMERIC affinity, no text cast (correct numeric order)
                //   5. Decimal
                //   6. Date
                select_sort_cols.push_str(&format!(
                    ", COALESCE(\
                       MIN(CASE WHEN {alias}.value_lang = ? THEN {alias}.value_text END), \
                       MIN(CASE WHEN {alias}.value_lang IS NULL THEN {alias}.value_text END), \
                       MIN({alias}.value_text), \
                       MIN({alias}.value_int), \
                       MIN({alias}.value_real), \
                       MIN({alias}.date_start)\
                       ) AS sort_val{i}"
                ));
                // lang param feeds the `CASE WHEN value_lang = ?` in select_sort_cols (SELECT).
                // property name feeds the `AND name = ?` in join_clauses (FROM).
                // They are collected separately so params bind in SQL left-to-right order:
                //   all lang params (SELECT) come before all join-name params (FROM clause).
                lang_params.push(rusqlite::types::Value::Text(lang.as_str().to_owned()));
                join_name_params.push(rusqlite::types::Value::Text(name.clone()));
            }
            SortKey::Class => {
                // No join, no bound params — `class` is a plain `resources` column
                // and is NOT NULL, so no `(sort_val{i} IS NULL)` ORDER BY term either.
                select_sort_cols.push_str(&format!(", r.class AS sort_val{i}"));
            }
            SortKey::Id => {
                // No join, no bound params — `id` is a plain `resources` column
                // and is NOT NULL, so no `(sort_val{i} IS NULL)` ORDER BY term either.
                select_sort_cols.push_str(&format!(", r.id AS sort_val{i}"));
            }
        }
    }

    // Combine params in SQL left-to-right order:
    //   lang_params (SELECT CASE), join_name_params (JOIN ON), class (WHERE), where_params.
    let mut all_params: Vec<rusqlite::types::Value> = lang_params;
    all_params.extend(join_name_params);
    all_params.push(rusqlite::types::Value::Text(class.to_owned()));
    all_params.extend(where_params);

    // Build ORDER BY for the outer query
    let order_by = if sorts.is_empty() {
        "id".to_owned()
    } else {
        let terms: Vec<String> = sorts
            .iter()
            .enumerate()
            .flat_map(|(i, sort)| {
                let dir = match sort.direction {
                    SortDir::Asc => "ASC",
                    SortDir::Desc => "DESC",
                };
                match &sort.key {
                    SortKey::Property(_) => {
                        // NULLs last regardless of direction, then the sort value, then id.
                        // The leading `(sort_val IS NULL)` bool (0/1) sorts NULLs to the end.
                        // Text sorts are case-insensitive: NOCASE folds ASCII `A–Z` only
                        // (an `Ä…` still files after `z…`), and SQLite applies a
                        // collation only when comparing two TEXT values, so integer and
                        // ISO-date sort values keep their order.
                        vec![
                            format!("(sort_val{i} IS NULL)"),
                            format!("sort_val{i} COLLATE NOCASE {dir}"),
                        ]
                    }
                    SortKey::Class | SortKey::Id => {
                        // `class`/`id` are NOT NULL — no `(sort_val{i} IS NULL)` term.
                        vec![format!("sort_val{i} {dir}")]
                    }
                }
            })
            .collect();
        // Append `id` as the stable tiebreak.
        format!("{}, id", terms.join(", "))
    };

    let inner_sql = format!(
        "SELECT r.id AS id{select_sort_cols} \
         FROM resources r{join_clauses} \
         WHERE r.class = ?{where_fragment} \
         GROUP BY r.id"
    );

    let sql = format!(
        "SELECT id FROM ({inner_sql}) \
         ORDER BY {order_by} \
         LIMIT ? OFFSET ?"
    );

    // Append LIMIT and OFFSET params.
    all_params.push(rusqlite::types::Value::Integer(i64::from(limit)));
    all_params.push(rusqlite::types::Value::Integer(i64::from(offset)));

    let mut stmt = conn.prepare(&sql).map_err(QueryError::query)?;
    let ids: Vec<String> = stmt
        .query_map(rusqlite::params_from_iter(&all_params), |row| row.get(0))
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;

    Ok(ids)
}

// FTS search index building

/// Build FTS5 virtual tables for every `search-bar` in `config.pages` and
/// populate them from the loaded data. Called once at boot
/// (`ProjectStore::build_search_indexes`, from `Project::load` after
/// validation).
///
/// Each search-bar's `target` resolves to its sibling `ResourceList` in the
/// SAME page's body (`id == Some(target)`) to recover the searched `class`.
/// Distinct `(class, searches)` pairs are deduped by the table name
/// `search_index_name` would derive for them — using it directly as the
/// dedup key (rather than a separate `(class, sorted searches)` tuple)
/// guarantees the dedup key and the schema table name can never disagree.
/// Two search-bars sharing a class + prop-set therefore share one FTS5
/// table; this is deliberate future-proofing (a project may declare the
/// same search surface on two pages), not a confused pattern carried over
/// from facets.
///
/// Every `INSERT` across every table runs inside a single transaction — per
/// the project's "avoid O(n²) row-by-row autocommit inserts" convention —
/// not one transaction per table.
///
/// A `target` that fails to resolve to a sibling `ResourceList` is skipped
/// defensively: `lower_search_bar`'s `UnresolvedTarget` hard-fail already
/// guarantees a validated `config.pages` never contains a dangling target,
/// so this is an unreachable-in-practice guard, not a silent-drop of real
/// config.
pub(crate) fn build_search_indexes(
    conn: &Connection,
    config: &cpe_core::ir::ValidatedConfig,
) -> Result<(), QueryError> {
    use cpe_core::ir::Component;

    // Resolve every search-bar to its (table, class, searches), deduped by
    // table name.
    let mut indexes: Vec<(String, String, Vec<String>)> = Vec::new();
    let mut seen_tables: HashSet<String> = HashSet::new();

    for page in &config.pages {
        for component in &page.body {
            let Component::SearchBar { target, searches, .. } = component else {
                continue;
            };
            let class = page.body.iter().find_map(|c| match c {
                Component::ResourceList { id, class, .. } if id.as_deref() == Some(target.as_str()) => {
                    Some(class.clone())
                }
                _ => None,
            });
            let Some(class) = class else {
                continue;
            };
            let table = search_index_name(&class, searches);
            if seen_tables.insert(table.clone()) {
                indexes.push((table, class, searches.clone()));
            }
        }
    }

    if indexes.is_empty() {
        return Ok(());
    }

    // Schema creation — one CREATE VIRTUAL TABLE per distinct table, outside
    // the transaction (DDL is auto-committed by SQLite regardless).
    for (table, _, _) in &indexes {
        conn.execute(
            &format!("CREATE VIRTUAL TABLE {table} USING fts5(resource_id UNINDEXED, content)"),
            [],
        )
        .map_err(QueryError::search_index_build)?;
    }

    // Content population — every INSERT across every table in ONE
    // transaction (never row-by-row autocommit; see the project's SQLite
    // text-seed learning on O(n²) row-by-row inserts).
    let tx = conn.unchecked_transaction().map_err(QueryError::search_index_build)?;

    for (table, class, searches) in &indexes {
        let resource_ids: Vec<String> = {
            let mut stmt = tx
                .prepare("SELECT id FROM resources WHERE class = ?1")
                .map_err(QueryError::search_index_build)?;
            let ids = stmt
                .query_map([class], |row| row.get::<_, String>(0))
                .map_err(QueryError::search_index_build)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(QueryError::search_index_build)?;
            ids
        };

        // Prepared once, reused per resource — not a fresh SQL string per row.
        let placeholders = searches.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let mut select_stmt = tx
            .prepare(&format!(
                "SELECT value_type, value_text FROM properties \
                 WHERE resource_id = ? AND name IN ({placeholders})"
            ))
            .map_err(QueryError::search_index_build)?;
        let mut insert_stmt = tx
            .prepare(&format!("INSERT INTO {table} (resource_id, content) VALUES (?, ?)"))
            .map_err(QueryError::search_index_build)?;

        for resource_id in &resource_ids {
            let mut query_params: Vec<&str> = vec![resource_id.as_str()];
            query_params.extend(searches.iter().map(String::as_str));

            let mut fragments: Vec<String> = Vec::new();
            let mut rows = select_stmt
                .query(rusqlite::params_from_iter(query_params))
                .map_err(QueryError::search_index_build)?;
            // Every row for the resource+searches is collected regardless of
            // `value_lang` — a language-agnostic index, matching how
            // `WhereClause::Search` ignores `value_lang` at query time. A
            // multi-lang property therefore contributes all its language
            // variants' text, concatenated with the rest.
            while let Some(row) = rows.next().map_err(QueryError::search_index_build)? {
                let value_type: String = row.get(0).map_err(QueryError::search_index_build)?;
                let value_text: Option<String> = row.get(1).map_err(QueryError::search_index_build)?;
                let Some(text) = value_text else {
                    continue;
                };
                let fragment = if value_type == "richtext" {
                    cpe_core::util::markdown_to_text(&text)
                } else {
                    text
                };
                if !fragment.is_empty() {
                    fragments.push(fragment);
                }
            }

            let content = fragments.join(" ");
            // Skip resources with no searchable text at all — inserting an
            // empty FTS row wouldn't be wrong, just wasted space; skipping
            // is the simpler choice.
            if content.is_empty() {
                continue;
            }

            insert_stmt
                .execute(rusqlite::params![resource_id, content])
                .map_err(QueryError::search_index_build)?;
        }
    }

    tx.commit().map_err(QueryError::search_index_build)?;

    Ok(())
}

// Private helpers

/// Build the `WHERE` fragment and bound params for a slice of `WhereClause`s.
///
/// Returns a `(sql_fragment, params)` pair where `sql_fragment` is zero or more
/// `AND EXISTS (…)` clauses (without a leading `WHERE` — callers append it after
/// their own `WHERE r.class = ?` term). Each `?` placeholder in the fragment
/// maps positionally to an entry in `params`.
///
/// Property names are always bound as values (`pW.name = ?`), never interpolated.
/// The type column (`value_text` / `value_int` / `value_real` / `date_start`) is
/// chosen by a closed `match` on the `Value` variant — the only non-bindable SQL
/// pieces are column names, which come from Rust enums, not external strings.
///
/// `Value::Integer(i128)` is converted with `i64::try_from(n)` and returns
/// `QueryError::ValueOutOfRange` on overflow (never `as i64`).
fn build_where(wheres: &[WhereClause]) -> Result<(String, Vec<rusqlite::types::Value>), QueryError> {
    let mut fragment = String::new();
    let mut params: Vec<rusqlite::types::Value> = Vec::new();

    for clause in wheres {
        match clause {
            WhereClause::Equals { property, value } => {
                let (col, val) = value_col_and_param(value)?;
                fragment.push_str(&format!(
                    " AND EXISTS (SELECT 1 FROM properties pW \
                     WHERE pW.resource_id = r.id AND pW.name = ? AND pW.{col} = ?)"
                ));
                params.push(rusqlite::types::Value::Text(property.clone()));
                params.push(val);
            }
            WhereClause::In { property, values } => {
                if values.is_empty() {
                    // An empty IN list can never match — emit a false condition.
                    fragment.push_str(" AND 0");
                    continue;
                }
                // All values in an In clause must share the same type column.
                // Pick from the first value; mixed types would yield no rows
                // rather than an error (accepted).
                let (col, _) = value_col_and_param(&values[0])?;
                let placeholders = values.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                fragment.push_str(&format!(
                    " AND EXISTS (SELECT 1 FROM properties pW \
                     WHERE pW.resource_id = r.id AND pW.name = ? AND pW.{col} IN ({placeholders}))"
                ));
                params.push(rusqlite::types::Value::Text(property.clone()));
                for v in values {
                    let (_, val) = value_col_and_param(v)?;
                    params.push(val);
                }
            }
            WhereClause::Range { property, start, end } => {
                // Determine the type column from whichever bound is present.
                let col = match (start, end) {
                    (Some(v), _) | (None, Some(v)) => value_col_and_param(v)?.0,
                    (None, None) => {
                        // No bounds — tautology, skip.
                        continue;
                    }
                };

                match (start, end) {
                    (Some(s), Some(e)) => {
                        let (_, sv) = value_col_and_param(s)?;
                        let (_, ev) = value_col_and_param(e)?;
                        fragment.push_str(&format!(
                            " AND EXISTS (SELECT 1 FROM properties pW \
                             WHERE pW.resource_id = r.id AND pW.name = ? \
                             AND pW.{col} BETWEEN ? AND ?)"
                        ));
                        params.push(rusqlite::types::Value::Text(property.clone()));
                        params.push(sv);
                        params.push(ev);
                    }
                    (Some(s), None) => {
                        let (_, sv) = value_col_and_param(s)?;
                        fragment.push_str(&format!(
                            " AND EXISTS (SELECT 1 FROM properties pW \
                             WHERE pW.resource_id = r.id AND pW.name = ? \
                             AND pW.{col} >= ?)"
                        ));
                        params.push(rusqlite::types::Value::Text(property.clone()));
                        params.push(sv);
                    }
                    (None, Some(e)) => {
                        let (_, ev) = value_col_and_param(e)?;
                        fragment.push_str(&format!(
                            " AND EXISTS (SELECT 1 FROM properties pW \
                             WHERE pW.resource_id = r.id AND pW.name = ? \
                             AND pW.{col} <= ?)"
                        ));
                        params.push(rusqlite::types::Value::Text(property.clone()));
                        params.push(ev);
                    }
                    (None, None) => unreachable!("handled above"),
                }
            }
            WhereClause::TargetIn { property, target_ids } => {
                if target_ids.is_empty() {
                    // An empty TargetIn list can never match — emit a false condition.
                    // The caller (query::page's page builder) must guard against pushing
                    // an empty-ids TargetIn; this branch is a defensive dead path.
                    fragment.push_str(" AND 0");
                    continue;
                }
                let placeholders = target_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                // ⚠️ PERFORMANCE: this is a CORRELATED subquery — SQLite re-runs
                // it once per candidate row, so its inner lookup must be an
                // index *seek*, never a scan. That is what
                // `idx_properties_target(target_id, resource_id)` buys: with a
                // single-column `(target_id)` index SQLite seeks target_id
                // and then scans the whole bucket for that target hunting for
                // `resource_id = r.id`, making every narrowed browse cost
                // O(candidate rows x rows sharing the target) — 15.6 s for HdM's
                // `?source=ib` (5,160 of 8,942 Performances). Don't narrow that
                // index; `target_in_where_uses_composite_target_index` guards it.
                fragment.push_str(&format!(
                    " AND EXISTS (SELECT 1 FROM properties pW \
                     WHERE pW.resource_id = r.id AND pW.name = ? \
                     AND pW.target_id IN ({placeholders}))"
                ));
                params.push(rusqlite::types::Value::Text(property.clone()));
                for id in target_ids {
                    params.push(rusqlite::types::Value::Text(id.clone()));
                }
            }
            WhereClause::Search { table, query } => {
                // `table` is a boot-sanitized identifier by construction
                // (`search_index_name`), so it's interpolated the same way
                // `col` is interpolated in the arms above — never external
                // input. The MATCH string is still bound as a `?` param.
                match fts_match_expr(query) {
                    None => {
                        // Zero usable terms — no clause at all (never emit
                        // an empty/malformed MATCH). Mirrors the empty-`In`/
                        // `TargetIn`/no-bound-`Range` arms above, except this
                        // one has no false-condition to emit: an inactive
                        // search means "no filter", not "match nothing".
                        continue;
                    }
                    Some(match_str) => {
                        fragment.push_str(&format!(
                            " AND r.id IN (SELECT resource_id FROM {table} \
                             WHERE {table} MATCH ?)"
                        ));
                        params.push(rusqlite::types::Value::Text(match_str));
                    }
                }
            }
        }
    }

    Ok((fragment, params))
}

/// Derive the deterministic FTS5 virtual table name for a `(class,
/// searches)` pair.
///
/// Sorts `searches` internally so callers can pass config-declaration order
/// and still get the same table name every time (the boot builder and the
/// host's request-time lookup must derive the *same* name or a request hits
/// `no such table`). Sanitizes `class` and each search property name to
/// `[A-Za-z0-9_]` (every other character replaced with `_`), then returns
/// `search_<sanitized class>_<sorted sanitized names joined by _>`.
///
/// Deterministic but not provably injective — e.g. `"a-b"` and `"a_b"` both
/// sanitize to `"a_b"`, so two distinct (class, searches) inputs could
/// theoretically collide on the same table name. An accepted
/// limitation: a hash suffix is the remedy if a real project needs it.
pub fn search_index_name(class: &str, searches: &[String]) -> String {
    fn sanitize(s: &str) -> String {
        s.chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
            .collect()
    }

    let mut sorted = searches.to_vec();
    sorted.sort();

    let sanitized_class = sanitize(class);
    let sanitized_names: Vec<String> = sorted.iter().map(|s| sanitize(s)).collect();

    format!("search_{sanitized_class}_{}", sanitized_names.join("_"))
}

/// Map a `Value` to its corresponding SQLite column name (a `&'static str`
/// chosen from a closed Rust enum — not interpolated from external input) and a
/// `rusqlite::types::Value` ready to bind.
///
/// `Value::Null` has no corresponding column and is mapped to `value_text` with
/// a SQL NULL — callers should avoid relying on Null predicates (they will yield
/// no rows rather than an error).
///
/// `Value::Integer(i128)` is converted with `i64::try_from(n)` and returns
/// `QueryError::ValueOutOfRange` on overflow.
fn value_col_and_param(value: &Value) -> Result<(&'static str, rusqlite::types::Value), QueryError> {
    match value {
        Value::String(s) => Ok(("value_text", rusqlite::types::Value::Text(s.clone()))),
        Value::Integer(n) => {
            let i = i64::try_from(*n).map_err(|_| QueryError::ValueOutOfRange(*n))?;
            Ok(("value_int", rusqlite::types::Value::Integer(i)))
        }
        Value::Float(f) => Ok(("value_real", rusqlite::types::Value::Real(*f))),
        Value::Bool(b) => Ok(("value_int", rusqlite::types::Value::Integer(*b as i64))),
        Value::Null => Ok(("value_text", rusqlite::types::Value::Null)),
    }
}

/// Return `(source_resource_id, target_id)` pairs for every `link`-typed
/// property value whose `target_id` does not exist in the `resources` table.
///
/// An empty `Vec` means all link targets are present. Results are ordered
/// deterministically by `(resource_id, target_id)` so tests can assert stable
/// output without sorting.
///
/// This is a full data-level foreign-key check: every `link` row in the
/// `properties` table is checked, not just those reachable from the config.
pub(crate) fn dangling_link_targets(conn: &Connection) -> Result<Vec<(String, String)>, QueryError> {
    // The `p.target_id IS NOT NULL` guard excludes malformed rows (NULL
    // target_id on a link row) that are already caught by
    // `malformed_property_rows`. Those rows would produce a Rusqlite NULL-
    // column error when we try to SELECT p.target_id as TEXT, so we skip
    // them here. Only non-NULL target_ids that are absent from `resources`
    // are FK violations of interest.
    let mut stmt = conn
        .prepare(
            "SELECT p.resource_id, p.target_id \
             FROM properties p \
             LEFT JOIN resources r ON r.id = p.target_id \
             WHERE p.value_type = 'link' \
               AND p.target_id IS NOT NULL \
               AND r.id IS NULL \
             ORDER BY p.resource_id, p.target_id",
        )
        .map_err(QueryError::query)?;

    let pairs: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;

    Ok(pairs)
}

/// Return every `properties` row whose required value column is NULL for its
/// `value_type`. The required column per type:
///
/// | value_type        | required column |
/// |-------------------|-----------------|
/// | text / richtext   | value_text      |
/// | integer / boolean | value_int       |
/// | decimal           | value_real      |
/// | date              | date_start      |
/// | link / list_value | target_id       |
///
/// Rows are returned as `(resource_id, name, value_type, missing_column)` tuples
/// where `missing_column` is the column-name literal (`&'static str`).
/// Results are ordered by `(resource_id, name, seq)` for determinism.
///
/// A clean database returns an empty `Vec`.
pub(crate) fn malformed_property_rows(
    conn: &Connection,
) -> Result<Vec<(String, String, String, &'static str)>, QueryError> {
    // Fetch every row that violates the type→column constraint.
    // We detect the violation type in SQL via CASE so we can do it in a single
    // query, then map the detected bucket back to a &'static str column name in
    // Rust (avoids constructing dynamic column-name strings from SQL).
    let mut stmt = conn
        .prepare(
            "SELECT resource_id, name, seq, value_type, \
                    value_text, value_int, value_real, date_start, target_id \
             FROM properties \
             WHERE \
               (value_type IN ('text', 'richtext')   AND value_text IS NULL) OR \
               (value_type IN ('integer', 'boolean')  AND value_int  IS NULL) OR \
               (value_type = 'decimal'                AND value_real IS NULL) OR \
               (value_type = 'date'                   AND date_start IS NULL) OR \
               (value_type IN ('link', 'list_value')  AND target_id  IS NULL) \
             ORDER BY resource_id, name, seq",
        )
        .map_err(QueryError::query)?;

    let rows: Vec<(String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?, // resource_id
                row.get::<_, String>(1)?, // name
                row.get::<_, String>(3)?, // value_type (col index 3; seq is 2)
            ))
        })
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;

    // Map value_type → missing column name (&'static str) in Rust.
    let result = rows
        .into_iter()
        .map(|(resource_id, name, value_type)| {
            let missing_column: &'static str = match value_type.as_str() {
                "text" | "richtext" => "value_text",
                "integer" | "boolean" => "value_int",
                "decimal" => "value_real",
                "date" => "date_start",
                "link" | "list_value" => "target_id",
                other => unreachable!(
                    "value_type '{other}' cannot appear: the WHERE clause only \
                     selects the five known violating type families"
                ),
            };
            (resource_id, name, value_type, missing_column)
        })
        .collect();

    Ok(result)
}

/// Read the `MalformedAnnotation` boot check's inputs (see
/// [`AnnotationCheckRows`]). A database with no annotation rows returns both
/// lists empty.
pub(crate) fn malformed_annotation_rows(conn: &Connection) -> Result<AnnotationCheckRows, QueryError> {
    let mut stmt = conn
        .prepare("SELECT id, shape, x, y, w, h, points, color FROM annotations ORDER BY id")
        .map_err(QueryError::query)?;
    let annotations = stmt
        .query_map([], |row| {
            Ok(AnnotationCheckRow {
                id: row.get(0)?,
                shape: row.get(1)?,
                x: row.get(2)?,
                y: row.get(3)?,
                w: row.get(4)?,
                h: row.get(5)?,
                points: row.get(6)?,
                color: row.get(7)?,
            })
        })
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;

    let mut stmt = conn
        .prepare(
            "SELECT l.id FROM annotation_links l \
             WHERE NOT EXISTS (SELECT 1 FROM annotation_link_targets t WHERE t.link_id = l.id) \
             ORDER BY l.id",
        )
        .map_err(QueryError::query)?;
    let empty_links = stmt
        .query_map([], |row| row.get(0))
        .map_err(QueryError::query)?
        .collect::<Result<Vec<String>, _>>()
        .map_err(QueryError::query)?;

    Ok(AnnotationCheckRows { annotations, empty_links })
}

/// Read the `MalformedDataArk` boot check's inputs: every non-NULL `ark` of
/// `resources`, then of `annotations`, each ordered by id, unjudged (the
/// pattern is the check's).
pub(crate) fn malformed_data_ark_rows(conn: &Connection) -> Result<Vec<DataArkRow>, QueryError> {
    let mut rows = Vec::new();
    for (table, sql) in [
        ("resources", "SELECT id, ark FROM resources WHERE ark IS NOT NULL ORDER BY id"),
        (
            "annotations",
            "SELECT id, ark FROM annotations WHERE ark IS NOT NULL ORDER BY id",
        ),
    ] {
        let mut stmt = conn.prepare(sql).map_err(QueryError::query)?;
        let found = stmt
            .query_map([], |row| Ok(DataArkRow { table, id: row.get(0)?, ark: row.get(1)? }))
            .map_err(QueryError::query)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(QueryError::query)?;
        rows.extend(found);
    }
    Ok(rows)
}

/// Every non-NULL `ark` of `resources` and of `annotations`, keyed by id in
/// two separate maps (the two id namespaces may overlap). Read once at load
/// into `Project::data_arks`; the values are whatever is stored, so callers
/// rely on the `MalformedDataArk` boot check for their shape.
pub(crate) fn data_arks(conn: &Connection) -> Result<DataArks, QueryError> {
    let read = |sql: &str| -> Result<HashMap<String, String>, QueryError> {
        let mut stmt = conn.prepare(sql).map_err(QueryError::query)?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(QueryError::query)?
            .collect::<Result<HashMap<String, String>, _>>()
            .map_err(QueryError::query)?;
        Ok(rows)
    };
    Ok(DataArks::from_stored(
        read("SELECT id, ark FROM resources WHERE ark IS NOT NULL")?,
        read("SELECT id, ark FROM annotations WHERE ark IS NOT NULL")?,
    ))
}

// Compound-parts functions

/// Return `true` when at least one `compound_parts` row exists for `parent_id`.
///
/// Mirrors `class_exists` — uses `EXISTS` so SQLite can short-circuit on the
/// first matching row. Returns `Ok(false)` for an absent parent (not an error).
///
/// Called from the boot validator (`check_compound_viewer_data`).
pub(crate) fn compound_has_parts(conn: &Connection, parent_id: &str) -> Result<bool, QueryError> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM compound_parts WHERE parent_id = ?)",
            [parent_id],
            |row| row.get(0),
        )
        .map_err(QueryError::query)?;
    Ok(exists)
}

/// Return the distinct classes of compound members, sorted.
///
/// `Some(parent_id)` → the classes of that parent's `compound_parts`
/// children; `None` → the classes of **every** `compound_parts` child in the
/// project (the `$param`-sourced `compound-viewer` case, whose parent is only
/// known at request time). A child id with no `resources` row contributes no
/// class. Empty result ⇒ no members to validate against (not an error).
///
/// Used by the `compound-viewer media="…"` boot validator
/// to pick the class(es) `validate::media_property::check_single_media_property`
/// runs against.
pub(crate) fn compound_member_classes(conn: &Connection, parent_id: Option<&str>) -> Result<Vec<String>, QueryError> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT r.class \
             FROM compound_parts cp \
             JOIN resources r ON r.id = cp.child_id \
             WHERE ?1 IS NULL OR cp.parent_id = ?1 \
             ORDER BY r.class",
        )
        .map_err(QueryError::query)?;

    let classes = stmt
        .query_map([parent_id], |row| row.get(0))
        .map_err(QueryError::query)?
        .collect::<Result<Vec<String>, _>>()
        .map_err(QueryError::query)?;

    Ok(classes)
}

/// Return one `MemberHeader` per member of a compound Resource, in
/// `compound_parts.seq, child_id` order.
///
/// Resolves each member's display `name` from the literal `"name"` property
/// via a GROUP BY + lang-aware `COALESCE(MIN(…))` aggregate — the same
/// duplicate-row-safe technique `list_resource_ids` uses for its sort key
/// (a plain JOIN would multiply a row for a multi-valued `name`). Fallback
/// chain: requested `lang` → universal (`value_lang IS NULL`) → any. When a
/// member has no `name` value at all, the aggregate is `NULL` and this
/// function falls back to the member's `id` — `name` is never empty.
///
/// The literal `"name"` here mirrors `compound_viewer`/`resource_list`,
/// which hardcode `Some("name")` via `resolve_title_str`. A per-class
/// configured `title-property` is not built.
///
/// Returns an empty `Vec` when `parent_id` has no `compound_parts` rows
/// (not an error — mirrors `compound_has_parts`'s empty-not-error semantics).
pub(crate) fn compound_part_headers(
    conn: &Connection,
    parent_id: &str,
    lang: &Lang,
) -> Result<Vec<MemberHeader>, QueryError> {
    let mut stmt = conn
        .prepare(
            "SELECT cp.child_id AS id, \
                    r.class AS class, \
                    COALESCE(\
                        MIN(CASE WHEN p.value_lang = ? THEN p.value_text END), \
                        MIN(CASE WHEN p.value_lang IS NULL THEN p.value_text END), \
                        MIN(p.value_text)\
                    ) AS name \
             FROM compound_parts cp \
             JOIN resources r ON r.id = cp.child_id \
             LEFT JOIN properties p ON p.resource_id = cp.child_id AND p.name = 'name' \
             WHERE cp.parent_id = ? \
             GROUP BY cp.child_id, cp.seq, r.class \
             ORDER BY cp.seq, cp.child_id",
        )
        .map_err(QueryError::query)?;

    let lang_str = lang.as_str().to_owned();
    let rows: Vec<(String, String, Option<String>)> = stmt
        .query_map([lang_str, parent_id.to_owned()], |row| {
            let id: String = row.get(0)?;
            let class: String = row.get(1)?;
            let name: Option<String> = row.get(2)?;
            Ok((id, class, name))
        })
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;

    let headers = rows
        .into_iter()
        .map(|(id, class, name)| {
            let name = name.unwrap_or_else(|| id.clone());
            MemberHeader { id, class, name }
        })
        .collect();

    Ok(headers)
}

// Incoming links (reverse link resolution)

/// Result-column indices of the OPTIONAL `incoming_links` columns.
/// The eight base columns are always `0..=7`; each optional column is
/// appended only when its spec field asks for it, so its index varies.
#[derive(Debug, Default, Clone, Copy)]
struct IncomingExtraCols {
    /// `group-by="<property>"`: the first `list_value` node id.
    group_node: Option<usize>,
    /// `group-by="<property>"`: that node's `list_nodes.seq`.
    group_seq: Option<usize>,
    /// `group-by="<property>"`: that node's label (lang → any).
    group_label: Option<usize>,
    /// `group-by="<property>"`: the first `text` value (lang → universal → any).
    group_text: Option<usize>,
    /// `row-label`: the first `text` value (lang → universal → any).
    label: Option<usize>,
    /// `row-aside`: the first `text` value, else the first `list_value` label.
    aside: Option<usize>,
}

/// The fully-bound statement `incoming_links` runs: the SQL, every positional
/// parameter in textual `?` order, and where the optional columns landed.
struct IncomingLinksStatement {
    sql: String,
    params: Vec<rusqlite::types::Value>,
    cols: IncomingExtraCols,
}

/// A scalar correlated subquery selecting the FIRST `text` value of the
/// source's `?`-bound property, language-picked: requested lang → universal
/// (`value_lang IS NULL`) → any, then `seq`. Binds `(name, lang)`.
const INCOMING_TEXT_PICK: &str = "(SELECT p.value_text FROM properties p \
     WHERE p.resource_id = s.id AND p.name = ? AND p.value_type = 'text' \
     ORDER BY CASE WHEN p.value_lang = ? THEN 0 WHEN p.value_lang IS NULL THEN 1 ELSE 2 END, \
     p.seq LIMIT 1)";

/// A scalar correlated subquery selecting the LABEL of the source's first
/// (`seq`) `list_value` node of the `?`-bound property: requested lang → any
/// (`MIN`, deterministic). `list_node_labels.lang` is NOT NULL, so there is
/// no universal rung (the `get_list_node_label` rule). Binds `(lang, name)`.
const INCOMING_LIST_LABEL_PICK: &str = "(SELECT COALESCE(\
     (SELECT lnl.label FROM list_node_labels lnl WHERE lnl.node_id = p.target_id AND lnl.lang = ?), \
     (SELECT MIN(lnl.label) FROM list_node_labels lnl WHERE lnl.node_id = p.target_id)) \
     FROM properties p \
     WHERE p.resource_id = s.id AND p.name = ? AND p.value_type = 'list_value' \
     ORDER BY p.seq LIMIT 1)";

/// A scalar correlated subquery selecting the source's first (`seq`)
/// `list_value` node id of the `?`-bound property. Binds `(name)`.
const INCOMING_LIST_NODE_PICK: &str = "(SELECT p.target_id FROM properties p \
     WHERE p.resource_id = s.id AND p.name = ? AND p.value_type = 'list_value' \
     ORDER BY p.seq LIMIT 1)";

/// A scalar correlated subquery selecting the `list_nodes.seq` of the
/// source's first (`seq`) `list_value` node of the `?`-bound property. A LEFT
/// JOIN so an orphan first node yields NULL rather than silently skipping to
/// the second value. Binds `(name)`.
const INCOMING_LIST_SEQ_PICK: &str = "(SELECT ln.seq FROM properties p \
     LEFT JOIN list_nodes ln ON ln.id = p.target_id \
     WHERE p.resource_id = s.id AND p.name = ? AND p.value_type = 'list_value' \
     ORDER BY p.seq LIMIT 1)";

/// Build the exact statement `incoming_links` runs for `spec`, `target_id`
/// and `lang` (`spec.properties` non-empty).
///
/// Property names are never interpolated; only the `?` placeholder list's
/// length varies. Base positional params, in order: `lang`, `target_id`, the
/// property names, `source_class`, `limit`. The optional columns' params come
/// FIRST because their `?`s sit in the outer select list, textually before
/// the inner subquery's `lang`.
///
/// - The unary `+` on `l.name` and `r.class` keeps SQLite (with no `ANALYZE` stats) from preferring
///   `idx_properties_name*` / `idx_resources_class` to drive the join over the
///   `idx_properties_target` seek.
/// - One parent per source: the inner subquery picks the first `parent_id` and the outer `LEFT JOIN
///   compound_parts` fetches THAT pair's `seq`; two independent `MIN()`s would not guarantee the
///   pairing.
/// - `total` is a `COUNT(*) OVER ()` window on the outer query, evaluated before `LIMIT ?`, so one
///   query yields the uncapped count and the capped rows. The limit is `spec.limit` verbatim (the
///   grouped default is resolved at lowering).
///
/// Optional lookups (`group-by`, `row-label`, `row-aside`, `sort-property`)
/// are emitted only when set, so an ungrouped spec yields the plain string.
/// Each is a scalar correlated subquery (`ORDER BY seq LIMIT 1`), never a
/// `LEFT JOIN`: a join on a multi-valued property would multiply rows and
/// corrupt the `COUNT(*) OVER ()` total. A multi-valued property contributes
/// its first value only.
///
/// A property group key is the list path (first `list_value` node id, seq,
/// label) when present, else the first text value; text keys arrive
/// case-canonicalised (first spelling wins) by `incoming_links`. `sort-property` orders by its
/// first text value `COLLATE NOCASE` with missing values last, and wins over
/// `order=`.
fn incoming_links_statement(spec: &IncomingLinksSpec, target_id: &str, lang: &Lang) -> IncomingLinksStatement {
    use rusqlite::types::Value as V;
    let lang_v = || V::Text(lang.as_str().to_owned());
    let name_v = |n: &str| V::Text(n.to_owned());

    // Optional outer-select columns, appended after the eight base columns.
    let mut extra_sql = String::new();
    let mut params: Vec<V> = Vec::new();
    let mut cols = IncomingExtraCols::default();
    let mut next = 8usize;
    let mut push_col = |sql: &str, extra_sql: &mut String| {
        extra_sql.push_str(", ");
        extra_sql.push_str(sql);
        let idx = next;
        next += 1;
        idx
    };

    if let Some(grouping) = &spec.grouping {
        if let IncomingGroupBy::Property(prop) = &grouping.by {
            cols.group_node = Some(push_col(INCOMING_LIST_NODE_PICK, &mut extra_sql));
            params.push(name_v(prop));
            cols.group_seq = Some(push_col(INCOMING_LIST_SEQ_PICK, &mut extra_sql));
            params.push(name_v(prop));
            cols.group_label = Some(push_col(INCOMING_LIST_LABEL_PICK, &mut extra_sql));
            params.push(lang_v());
            params.push(name_v(prop));
            cols.group_text = Some(push_col(INCOMING_TEXT_PICK, &mut extra_sql));
            params.push(name_v(prop));
            params.push(lang_v());
        }
    }
    if let Some(prop) = &spec.row_label {
        cols.label = Some(push_col(INCOMING_TEXT_PICK, &mut extra_sql));
        params.push(name_v(prop));
        params.push(lang_v());
    }
    if let Some(prop) = &spec.row_aside {
        let aside = format!("COALESCE({INCOMING_TEXT_PICK}, {INCOMING_LIST_LABEL_PICK})");
        cols.aside = Some(push_col(&aside, &mut extra_sql));
        params.push(name_v(prop));
        params.push(lang_v());
        params.push(lang_v());
        params.push(name_v(prop));
    }
    let order_by = if let Some(prop) = &spec.sort_property {
        // Selected (and ignored by Rust) so ORDER BY can name it; always the
        // LAST optional column.
        push_col(&format!("{INCOMING_TEXT_PICK} AS sort_val"), &mut extra_sql);
        params.push(name_v(prop));
        params.push(lang_v());
        "(sort_val IS NULL), sort_val COLLATE NOCASE, s.id, s.via"
    } else {
        match spec.order {
            // Case-insensitive like the resource-list text sort (ASCII-only NOCASE).
            IncomingOrder::Name => "COALESCE(s.name, s.id) COLLATE NOCASE, s.id, s.via",
            IncomingOrder::Id => "s.id, s.via",
            IncomingOrder::CompoundSeq => "(s.parent_id IS NULL), s.parent_id, cp.seq, s.id, s.via",
        }
    };

    let placeholders = vec!["?"; spec.properties.len()].join(", ");
    let sql = format!(
        "SELECT s.id, s.class, s.via, s.name, s.parent_id, pr.class, cp.seq, \
                COUNT(*) OVER () AS total{extra_sql} \
         FROM ( \
             SELECT l.resource_id AS id, \
                    r.class AS class, \
                    l.name AS via, \
                    COALESCE(\
                        MIN(CASE WHEN n.value_lang = ? THEN n.value_text END), \
                        MIN(CASE WHEN n.value_lang IS NULL THEN n.value_text END), \
                        MIN(n.value_text)\
                    ) AS name, \
                    (SELECT cpf.parent_id FROM compound_parts cpf \
                      WHERE cpf.child_id = l.resource_id \
                      ORDER BY cpf.parent_id LIMIT 1) AS parent_id \
             FROM properties l \
             JOIN resources r ON r.id = l.resource_id \
             LEFT JOIN properties n ON n.resource_id = l.resource_id AND n.name = 'name' \
             WHERE l.target_id = ? AND l.value_type = 'link' \
               AND +l.name IN ({placeholders}) \
               AND +r.class = ? \
             GROUP BY l.resource_id, l.name, r.class \
         ) s \
         LEFT JOIN compound_parts cp ON cp.parent_id = s.parent_id AND cp.child_id = s.id \
         LEFT JOIN resources pr ON pr.id = s.parent_id \
         ORDER BY {order_by} \
         LIMIT ?"
    );

    params.push(lang_v());
    params.push(V::Text(target_id.to_owned()));
    params.extend(spec.properties.iter().map(|p| name_v(p)));
    params.push(V::Text(spec.class.clone()));
    params.push(V::Integer(i64::from(spec.limit)));

    IncomingLinksStatement { sql, params, cols }
}

/// Return every `spec.class` resource that links TO `target_id` through one
/// of `spec.properties` (`link`-typed rows only), one row per
/// `(source, property)` pair, in `spec.order` (or by `spec.sort_property`,
/// which wins), capped at `spec.limit`.
///
/// Display `name` resolves from the literal `"name"` property exactly as in
/// `compound_part_headers` (requested lang → universal → any, duplicate-row
/// safe via GROUP BY), falling back to the source id. `parent` is the
/// source's lexicographically-first compound parent with that parent's own
/// `seq`, or `None` for a non-member. `group`/`label`/`aside` are filled only
/// when the spec asks for them (see [`incoming_links_statement`]).
///
/// An empty `properties` list returns `IncomingLinks { rows: vec![], total: 0 }`
/// without querying (`IN ()` is a syntax error). A target nothing links to is
/// also `total: 0`, not an error.
pub(crate) fn incoming_links(
    conn: &Connection,
    spec: &IncomingLinksSpec,
    target_id: &str,
    lang: &Lang,
) -> Result<IncomingLinks, QueryError> {
    if spec.properties.is_empty() {
        return Ok(IncomingLinks { rows: vec![], total: 0 });
    }

    let IncomingLinksStatement { sql, params, cols } = incoming_links_statement(spec, target_id, lang);

    /// Optional-column cell reader: `None` when the column was not emitted.
    fn opt<T: rusqlite::types::FromSql>(row: &rusqlite::Row<'_>, idx: Option<usize>) -> rusqlite::Result<Option<T>> {
        match idx {
            Some(i) => row.get(i),
            None => Ok(None),
        }
    }

    let mut stmt = conn.prepare(&sql).map_err(QueryError::query)?;
    let mut total: usize = 0;
    let mut links: Vec<IncomingLink> = stmt
        .query_map(rusqlite::params_from_iter(&params), |row| {
            let id: String = row.get(0)?;
            let class: String = row.get(1)?;
            let via: String = row.get(2)?;
            let name: Option<String> = row.get(3)?;
            let parent_id: Option<String> = row.get(4)?;
            let parent_class: Option<String> = row.get(5)?;
            let seq: Option<i64> = row.get(6)?;
            let row_total: i64 = row.get(7)?;
            let group_node: Option<String> = opt(row, cols.group_node)?;
            let group_seq: Option<i64> = opt(row, cols.group_seq)?;
            let group_label: Option<String> = opt(row, cols.group_label)?;
            let group_text: Option<String> = opt(row, cols.group_text)?;
            let label: Option<String> = opt(row, cols.label)?;
            let aside: Option<String> = opt(row, cols.aside)?;

            let name = name.unwrap_or_else(|| id.clone());
            // All three are present together for a member: the
            // `compound_parts` FKs guarantee the parent resource exists and
            // the (parent, child) pair row is the one the subquery found.
            let parent = match (parent_id, parent_class, seq) {
                (Some(id), Some(class), Some(seq)) => Some(MemberParent { id, class, seq }),
                _ => None,
            };
            // The group key's COALESCE: the list path wins, else the text path.
            let group = match (group_node, group_text) {
                (Some(key), _) => Some(GroupKey::List {
                    label: group_label.unwrap_or_else(|| key.clone()),
                    seq: group_seq.unwrap_or(i64::MAX),
                    key,
                }),
                (None, Some(key)) => Some(GroupKey::Text { key }),
                (None, None) => None,
            };
            Ok((
                usize::try_from(row_total).unwrap_or(0),
                IncomingLink { id, class, name, via, parent, group, label, aside },
            ))
        })
        .map_err(QueryError::query)?
        .map(|r| {
            r.map(|(row_total, link)| {
                total = row_total;
                link
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;

    // Text group keys compare case-insensitively: every case variant takes
    // the first spelling met in row order, so grouping is plain equality.
    let mut first_spelling: HashMap<String, String> = HashMap::new();
    for link in &mut links {
        if let Some(GroupKey::Text { key }) = &mut link.group {
            let canonical = first_spelling.entry(key.to_lowercase()).or_insert_with(|| key.clone());
            key.clone_from(canonical);
        }
    }

    Ok(IncomingLinks { rows: links, total })
}

// Annotations
// dsp-incubator cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md

/// Every annotation with its register number `n`: per resource, larger box
/// first, ties by id. The one place an annotation's number is derived —
/// `annotations_on` and `annotation_partners` both read `n` from it.
const NUMBERED_ANNOTATIONS: &str = "(SELECT *, ROW_NUMBER() OVER \
     (PARTITION BY resource_id ORDER BY w * h DESC, id) AS n FROM annotations)";

const ANNOTATION_COLUMNS: &str = "id, resource_id, shape, x, y, w, h, points, color, remark, remark_lang, n";

/// A scalar subquery for the display name of the resource whose id is the
/// SQL expression `res`: its literal `"name"` property, requested lang (`?2`)
/// → universal → any (`MIN`) — the `compound_part_headers` rule. NULL when
/// the resource has no `name`.
fn name_pick_sql(res: &str) -> String {
    format!(
        "(SELECT COALESCE(\
             MIN(CASE WHEN np.value_lang = ?2 THEN np.value_text END), \
             MIN(CASE WHEN np.value_lang IS NULL THEN np.value_text END), \
             MIN(np.value_text)) \
          FROM properties np WHERE np.resource_id = {res} AND np.name = 'name')"
    )
}

/// A row-decoding failure for column `col`, so a malformed row surfaces as a
/// `QueryError` rather than a panic under the connection lock (which would
/// poison it for every later request).
fn bad_annotation_column(col: usize, what: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        col,
        rusqlite::types::Type::Text,
        format!("{what} — boot validation (MalformedAnnotation) should have rejected it").into(),
    )
}

/// Map one `ANNOTATION_COLUMNS` row to its register number and annotation.
/// An unknown shape or a polygon without parseable points is an `Err`
/// (both are excluded by the table's `CHECK` and the boot check).
fn annotation_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(u32, Annotation)> {
    let shape: String = row.get(2)?;
    let shape = AnnotationShape::from_db(&shape).ok_or_else(|| bad_annotation_column(2, "unknown shape"))?;
    let points: Option<String> = row.get(7)?;
    let points = match shape {
        AnnotationShape::Polygon => points
            .as_deref()
            .and_then(parse_annotation_points)
            .ok_or_else(|| bad_annotation_column(7, "polygon points do not parse"))?,
        AnnotationShape::Rectangle | AnnotationShape::Circle => Vec::new(),
    };
    let n: i64 = row.get(11)?;
    let number = u32::try_from(n).map_err(|_| bad_annotation_column(11, "number out of range"))?;
    Ok((
        number,
        Annotation {
            id: row.get(0)?,
            resource_id: row.get(1)?,
            shape,
            x: row.get(3)?,
            y: row.get(4)?,
            w: row.get(5)?,
            h: row.get(6)?,
            points,
            color: row.get(8)?,
            remark: row.get(9)?,
            remark_lang: row.get(10)?,
        },
    ))
}

/// Every annotation on `resource_id` with its 1-based register number
/// (larger box first, ties by id), in that order. Empty when it has none.
pub(crate) fn annotations_on(conn: &Connection, resource_id: &str) -> Result<Vec<(u32, Annotation)>, QueryError> {
    let sql = format!(
        "SELECT {ANNOTATION_COLUMNS} FROM {NUMBERED_ANNOTATIONS} \
         WHERE resource_id = ?1 ORDER BY n"
    );
    let mut stmt = conn.prepare(&sql).map_err(QueryError::query)?;
    let rows = stmt
        .query_map([resource_id], annotation_from_row)
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;
    Ok(rows)
}

/// A register entry whose partners `annotation_partners` looks up.
#[derive(Debug, Clone, Copy)]
pub(crate) enum EntryTarget<'a> {
    /// A numbered entry: the annotation with this id.
    Annotation(&'a str),
    /// A page-level entry: links targeting the resource with this id.
    Resource(&'a str),
}

/// Every other target of every link that targets `entry`, one row per
/// `(link, target)`, ordered by link id then target `seq`.
///
/// A target equal to the entry itself is skipped, so a link with no other
/// target yields nothing. Two links joining the same pair yield two rows
/// (deduplicating is the renderer's). An annotation partner carries its
/// number on its own resource; every partner carries its resource's label,
/// its first compound parent (as in `incoming_links`), that parent's display
/// name, and whether it is that parent's first member.
pub(crate) fn annotation_partners(
    conn: &Connection,
    entry: EntryTarget<'_>,
    lang: &Lang,
) -> Result<Vec<AnnotationPartner>, QueryError> {
    let (col, id) = match entry {
        EntryTarget::Annotation(id) => ("annotation_id", id),
        EntryTarget::Resource(id) => ("resource_id", id),
    };
    let label = name_pick_sql("r.id");
    let parent_title = name_pick_sql("pp.parent_id");
    let sql = format!(
        "SELECT l.id, l.name, l.name_lang, o.annotation_id, an.n, r.id, r.class, {label}, \
                pp.parent_id, pr.class, cp.seq, {parent_title}, \
                cp.child_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM compound_parts f \
                    WHERE f.parent_id = cp.parent_id \
                      AND (f.seq < cp.seq OR (f.seq = cp.seq AND f.child_id < cp.child_id))) \
         FROM (SELECT DISTINCT link_id FROM annotation_link_targets WHERE {col} = ?1) m \
         JOIN annotation_links l ON l.id = m.link_id \
         JOIN annotation_link_targets o ON o.link_id = l.id \
         LEFT JOIN {NUMBERED_ANNOTATIONS} an ON an.id = o.annotation_id \
         JOIN resources r ON r.id = COALESCE(an.resource_id, o.resource_id) \
         LEFT JOIN (SELECT child_id, MIN(parent_id) AS parent_id \
                    FROM compound_parts GROUP BY child_id) pp ON pp.child_id = r.id \
         LEFT JOIN compound_parts cp ON cp.parent_id = pp.parent_id AND cp.child_id = r.id \
         LEFT JOIN resources pr ON pr.id = pp.parent_id \
         WHERE o.{col} IS NOT ?1 \
         ORDER BY l.id, o.seq"
    );
    let mut stmt = conn.prepare(&sql).map_err(QueryError::query)?;
    let rows = stmt
        .query_map(rusqlite::params![id, lang.as_str()], |row| {
            let resource_id: String = row.get(5)?;
            let label: Option<String> = row.get(7)?;
            let number: Option<i64> = row.get(4)?;
            let parent_id: Option<String> = row.get(8)?;
            let parent_class: Option<String> = row.get(9)?;
            let seq: Option<i64> = row.get(10)?;
            // All three are present together for a member: the
            // `compound_parts` FKs guarantee the parent resource exists.
            let parent = match (parent_id, parent_class, seq) {
                (Some(id), Some(class), Some(seq)) => Some(MemberParent { id, class, seq }),
                _ => None,
            };
            Ok(AnnotationPartner {
                link_id: row.get(0)?,
                link_name: row.get(1)?,
                link_name_lang: row.get(2)?,
                annotation_id: row.get(3)?,
                number: number.and_then(|n| u32::try_from(n).ok()),
                resource_label: label.unwrap_or_else(|| resource_id.clone()),
                resource_id,
                resource_class: row.get(6)?,
                parent,
                parent_title: row.get(11)?,
                first_member: row.get(12)?,
            })
        })
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;
    Ok(rows)
}

/// Every link with a target equal to the resource `resource_id` (its
/// page-level entries), in link-id order.
pub(crate) fn links_targeting(conn: &Connection, resource_id: &str) -> Result<Vec<AnnotationLink>, QueryError> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT l.id, l.name, l.name_lang, l.comment, l.comment_lang \
             FROM annotation_links l \
             JOIN annotation_link_targets t ON t.link_id = l.id \
             WHERE t.resource_id = ?1 \
             ORDER BY l.id",
        )
        .map_err(QueryError::query)?;
    let rows = stmt
        .query_map([resource_id], |row| {
            Ok(AnnotationLink {
                id: row.get(0)?,
                name: row.get(1)?,
                name_lang: row.get(2)?,
                comment: row.get(3)?,
                comment_lang: row.get(4)?,
            })
        })
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;
    Ok(rows)
}

/// The members of compound `parent_id` that have a register: at least one
/// annotation, or a link targeting the member itself. One query per compound.
pub(crate) fn annotated_members(conn: &Connection, parent_id: &str) -> Result<BTreeSet<String>, QueryError> {
    let mut stmt = conn
        .prepare(
            "SELECT cp.child_id FROM compound_parts cp \
             WHERE cp.parent_id = ?1 \
               AND (EXISTS (SELECT 1 FROM annotations a WHERE a.resource_id = cp.child_id) \
                 OR EXISTS (SELECT 1 FROM annotation_link_targets t \
                            WHERE t.resource_id = cp.child_id))",
        )
        .map_err(QueryError::query)?;
    let ids = stmt
        .query_map([parent_id], |row| row.get(0))
        .map_err(QueryError::query)?
        .collect::<Result<BTreeSet<String>, _>>()
        .map_err(QueryError::query)?;
    Ok(ids)
}

/// The resource-level partners of `resource_id`: every OTHER resource target
/// of every link that targets it as a resource, ordered by link id then
/// target `seq`. Annotation targets are omitted.
pub(crate) fn resource_links(
    conn: &Connection,
    resource_id: &str,
    lang: &Lang,
) -> Result<Vec<ResourceLink>, QueryError> {
    let label = name_pick_sql("r.id");
    // `<>`, not `IS NOT` as in `annotation_partners`: an annotation target
    // (NULL `resource_id`) must drop out here.
    let sql = format!(
        "SELECT l.id, l.name, l.name_lang, r.id, r.class, {label} \
         FROM (SELECT DISTINCT link_id FROM annotation_link_targets WHERE resource_id = ?1) m \
         JOIN annotation_links l ON l.id = m.link_id \
         JOIN annotation_link_targets o ON o.link_id = l.id \
         JOIN resources r ON r.id = o.resource_id \
         WHERE o.resource_id <> ?1 \
         ORDER BY l.id, o.seq"
    );
    let mut stmt = conn.prepare(&sql).map_err(QueryError::query)?;
    let rows = stmt
        .query_map(rusqlite::params![resource_id, lang.as_str()], |row| {
            let id: String = row.get(3)?;
            let label: Option<String> = row.get(5)?;
            Ok(ResourceLink {
                link_id: row.get(0)?,
                link_name: row.get(1)?,
                link_name_lang: row.get(2)?,
                resource_label: label.unwrap_or_else(|| id.clone()),
                resource_id: id,
                resource_class: row.get(4)?,
            })
        })
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;
    Ok(rows)
}

/// A scalar subquery for the representation id of the image of the resource
/// whose id is the SQL expression `res`: among the targets of its
/// `link`-typed property named by the parameter `prop` that have an image
/// representation, the lowest-`seq` target's lowest-`seq` image. A first
/// target without one is passed over for the next, unlike
/// `media_thumb_uris`, which reads only the first target. NULL when no
/// target has an image.
fn first_image_sql(res: &str, prop: &str) -> String {
    format!(
        "(SELECT rp.id FROM properties ip \
          JOIN representations rp ON rp.resource_id = ip.target_id AND rp.kind = 'image' \
          WHERE ip.resource_id = {res} AND ip.name = {prop} AND ip.value_type = 'link' \
          ORDER BY ip.seq, rp.seq LIMIT 1)"
    )
}

/// `?3, ?4, …` for `n` ids bound after two leading parameters.
fn id_placeholders_from_3(n: usize) -> String {
    (3..n + 3).map(|i| format!("?{i}")).collect::<Vec<_>>().join(", ")
}

/// The annotations `ids` with their crop sources, in the order of `ids`
/// (a repeated id repeats); an id with no annotation is omitted. Each carries its resource's label,
/// first compound parent (as in `annotation_partners`) and that parent's title, and
/// the resource's image through its `link`-typed `media_property`, picked as
/// `first_image_sql` says.
pub(crate) fn annotation_crops(
    conn: &Connection,
    ids: &[String],
    media_property: &str,
    lang: &Lang,
) -> Result<Vec<AnnotationCrop>, QueryError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let label = name_pick_sql("r.id");
    let parent_title = name_pick_sql("pp.parent_id");
    let image = first_image_sql("r.id", "?1");
    let placeholders = id_placeholders_from_3(ids.len());
    let sql = format!(
        "SELECT a.id, a.resource_id, a.shape, a.x, a.y, a.w, a.h, a.points, a.color, \
                a.remark, a.remark_lang, a.n, \
                r.class, {label}, pp.parent_id, pr.class, cp.seq, {parent_title}, \
                rep.uri, rep.width, rep.height \
         FROM {NUMBERED_ANNOTATIONS} a \
         JOIN resources r ON r.id = a.resource_id \
         LEFT JOIN (SELECT child_id, MIN(parent_id) AS parent_id \
                    FROM compound_parts GROUP BY child_id) pp ON pp.child_id = r.id \
         LEFT JOIN compound_parts cp ON cp.parent_id = pp.parent_id AND cp.child_id = r.id \
         LEFT JOIN resources pr ON pr.id = pp.parent_id \
         LEFT JOIN representations rep ON rep.id = {image} \
         WHERE a.id IN ({placeholders})"
    );
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(ids.len() + 2);
    let lang_code = lang.as_str();
    params.push(&media_property);
    params.push(&lang_code);
    for id in ids {
        params.push(id);
    }
    let mut stmt = conn.prepare(&sql).map_err(QueryError::query)?;
    let found: HashMap<String, AnnotationCrop> = stmt
        .query_map(params.as_slice(), |row| {
            let (_, annotation) = annotation_from_row(row)?;
            let label: Option<String> = row.get(13)?;
            let parent_id: Option<String> = row.get(14)?;
            let parent_class: Option<String> = row.get(15)?;
            let seq: Option<i64> = row.get(16)?;
            let parent = match (parent_id, parent_class, seq) {
                (Some(id), Some(class), Some(seq)) => Some(MemberParent { id, class, seq }),
                _ => None,
            };
            let uri: Option<String> = row.get(18)?;
            let image = uri.map(|uri| -> rusqlite::Result<CropImage> {
                Ok(CropImage { uri, width: row.get(19)?, height: row.get(20)? })
            });
            Ok(AnnotationCrop {
                resource_label: label.unwrap_or_else(|| annotation.resource_id.clone()),
                annotation,
                resource_class: row.get(12)?,
                parent,
                parent_title: row.get(17)?,
                image: image.transpose()?,
            })
        })
        .map_err(QueryError::query)?
        .map(|crop| crop.map(|c| (c.annotation.id.clone(), c)))
        .collect::<Result<_, _>>()
        .map_err(QueryError::query)?;
    Ok(ids.iter().filter_map(|id| found.get(id).cloned()).collect())
}

/// The resources `ids` with what a whole-page figure needs, in the order of
/// `ids` (a repeated id repeats); an id with no resource is omitted. Each
/// carries its label, first compound parent and that parent's title, and its
/// image through its `link`-typed `media_property`, read exactly as
/// [`annotation_crops`] reads them.
pub(crate) fn page_plates(
    conn: &Connection,
    ids: &[String],
    media_property: &str,
    lang: &Lang,
) -> Result<Vec<PagePlate>, QueryError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let label = name_pick_sql("r.id");
    let parent_title = name_pick_sql("pp.parent_id");
    let image = first_image_sql("r.id", "?1");
    let placeholders = id_placeholders_from_3(ids.len());
    let sql = format!(
        "SELECT r.id, r.class, {label}, pp.parent_id, pr.class, cp.seq, {parent_title}, \
                rep.uri, rep.width, rep.height \
         FROM resources r \
         LEFT JOIN (SELECT child_id, MIN(parent_id) AS parent_id \
                    FROM compound_parts GROUP BY child_id) pp ON pp.child_id = r.id \
         LEFT JOIN compound_parts cp ON cp.parent_id = pp.parent_id AND cp.child_id = r.id \
         LEFT JOIN resources pr ON pr.id = pp.parent_id \
         LEFT JOIN representations rep ON rep.id = {image} \
         WHERE r.id IN ({placeholders})"
    );
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(ids.len() + 2);
    let lang_code = lang.as_str();
    params.push(&media_property);
    params.push(&lang_code);
    for id in ids {
        params.push(id);
    }
    let mut stmt = conn.prepare(&sql).map_err(QueryError::query)?;
    let found: HashMap<String, PagePlate> = stmt
        .query_map(params.as_slice(), |row| {
            let id: String = row.get(0)?;
            let label: Option<String> = row.get(2)?;
            let parent_id: Option<String> = row.get(3)?;
            let parent_class: Option<String> = row.get(4)?;
            let seq: Option<i64> = row.get(5)?;
            let parent = match (parent_id, parent_class, seq) {
                (Some(id), Some(class), Some(seq)) => Some(MemberParent { id, class, seq }),
                _ => None,
            };
            let uri: Option<String> = row.get(7)?;
            let image = uri.map(|uri| -> rusqlite::Result<CropImage> {
                Ok(CropImage { uri, width: row.get(8)?, height: row.get(9)? })
            });
            Ok(PagePlate {
                resource_label: label.unwrap_or_else(|| id.clone()),
                resource_id: id,
                resource_class: row.get(1)?,
                parent,
                parent_title: row.get(6)?,
                image: image.transpose()?,
            })
        })
        .map_err(QueryError::query)?
        .map(|plate| plate.map(|p| (p.resource_id.clone(), p)))
        .collect::<Result<_, _>>()
        .map_err(QueryError::query)?;
    Ok(ids.iter().filter_map(|id| found.get(id).cloned()).collect())
}

/// The annotation links `ids`, in the order of `ids` (a repeated id repeats);
/// an id with no link is omitted.
pub(crate) fn annotation_links_by_id(conn: &Connection, ids: &[String]) -> Result<Vec<AnnotationLink>, QueryError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT id, name, name_lang, comment, comment_lang FROM annotation_links \
         WHERE id IN ({placeholders})"
    );
    let mut stmt = conn.prepare(&sql).map_err(QueryError::query)?;
    let found: HashMap<String, AnnotationLink> = stmt
        .query_map(rusqlite::params_from_iter(ids), |row| {
            Ok(AnnotationLink {
                id: row.get(0)?,
                name: row.get(1)?,
                name_lang: row.get(2)?,
                comment: row.get(3)?,
                comment_lang: row.get(4)?,
            })
        })
        .map_err(QueryError::query)?
        .map(|link| link.map(|l| (l.id.clone(), l)))
        .collect::<Result<_, _>>()
        .map_err(QueryError::query)?;
    Ok(ids.iter().filter_map(|id| found.get(id).cloned()).collect())
}

/// The annotation `id` for the showcase boot checks; `None` when it does not
/// exist.
pub(crate) fn showcase_annotation_row(
    conn: &Connection,
    id: &str,
    media_property: &str,
) -> Result<Option<ShowcaseAnnotationRow>, QueryError> {
    let image = first_image_sql("a.resource_id", "?2");
    let sql = format!(
        "SELECT a.resource_id, a.remark, rep.width, rep.height, rep.uri FROM annotations a \
         LEFT JOIN representations rep ON rep.id = {image} \
         WHERE a.id = ?1"
    );
    conn.query_row(&sql, rusqlite::params![id, media_property], |row| {
        Ok(ShowcaseAnnotationRow {
            resource_id: row.get(0)?,
            remark: row.get(1)?,
            width: row.get(2)?,
            height: row.get(3)?,
            uri: row.get(4)?,
        })
    })
    .optional()
    .map_err(QueryError::query)
}

/// Every `text`/`richtext` value of `resource_id`'s property `name`, in
/// `seq` order, all languages.
pub(crate) fn property_texts(conn: &Connection, resource_id: &str, name: &str) -> Result<Vec<String>, QueryError> {
    let mut stmt = conn
        .prepare(
            "SELECT value_text FROM properties \
             WHERE resource_id = ?1 AND name = ?2 \
               AND value_type IN ('text', 'richtext') AND value_text IS NOT NULL \
             ORDER BY seq",
        )
        .map_err(QueryError::query)?;
    let rows = stmt
        .query_map([resource_id, name], |row| row.get(0))
        .map_err(QueryError::query)?
        .collect::<Result<Vec<String>, _>>()
        .map_err(QueryError::query)?;
    Ok(rows)
}

/// The targets of annotation link `link_id`, in `seq` order; `None` when the
/// link does not exist.
pub(crate) fn link_targets(conn: &Connection, link_id: &str) -> Result<Option<Vec<LinkTargetRow>>, QueryError> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM annotation_links WHERE id = ?1)",
            [link_id],
            |row| row.get(0),
        )
        .map_err(QueryError::query)?;
    if !exists {
        return Ok(None);
    }
    let mut stmt = conn
        .prepare(
            "SELECT annotation_id, resource_id FROM annotation_link_targets \
             WHERE link_id = ?1 ORDER BY seq",
        )
        .map_err(QueryError::query)?;
    let rows = stmt
        .query_map([link_id], |row| {
            Ok(LinkTargetRow { annotation_id: row.get(0)?, resource_id: row.get(1)? })
        })
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;
    Ok(Some(rows))
}

// Embedded-media hydration

/// Return the ordered list of target Image resource ids a host resource
/// points at via its `link`-typed `property`, in `properties.seq` order.
///
/// `properties` is EAV: a multi-valued `link` property is multiple rows
/// sharing `(resource_id, name)`, distinguished by `seq` — exactly the shape
/// `compound_parts.seq` orders (see `compound_part_headers` above), except
/// here the order comes from the property row itself rather than a join
/// table. Returns an empty `Vec` when `resource_id` has no rows for
/// `property` (not an error — a host resource with no media values simply
/// has nothing to hydrate).
pub(crate) fn media_item_ids(conn: &Connection, resource_id: &str, property: &str) -> Result<Vec<String>, QueryError> {
    let mut stmt = conn
        .prepare(
            "SELECT target_id FROM properties \
             WHERE resource_id = ?1 AND name = ?2 AND value_type = 'link' \
             ORDER BY seq",
        )
        .map_err(QueryError::query)?;

    let ids: Vec<String> = stmt
        .query_map([resource_id, property], |row| row.get(0))
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;

    Ok(ids)
}

/// Batch-fetch, for each id in `ids`, the representation URI of the
/// **first** (lowest-seqnum) target Image linked via `class_media_property`
/// — one query, not N+1.
///
/// For each host id: the lowest-`seq` `link`-typed `properties` row named
/// `class_media_property` gives the target Image id; the lowest-`seq`
/// `kind='image'` `representations` row on that target gives the thumb URI.
/// Both "lowest seq" picks are expressed as correlated `MIN(...)` subquery
/// predicates rather than two round-trips per id, so the whole batch runs as
/// one SQL statement regardless of `ids.len()`.
///
/// Returns a map with an entry only for ids that resolved to at least one
/// image — a host resource with no media values (or whose first media
/// target has no image Representation) is simply absent from the map, never
/// an error.
///
/// Each entry also carries the target image's own stored pixel `width`
/// (nullable — `representations.width`), needed by the caller to clamp the
/// IIIF thumbnail request size (see `cpe_core::ir::MediaThumb`'s doc comment for
/// why: Sipi/IIIF v3 400s a `/full/<w>,/` request wider than the image's
/// native width).
pub(crate) fn media_thumb_uris(
    conn: &Connection,
    class_media_property: &str,
    ids: &[String],
) -> Result<HashMap<String, cpe_core::ir::MediaThumb>, QueryError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT p.resource_id, r.uri, r.width, r.height \
         FROM properties p \
         JOIN representations r ON r.resource_id = p.target_id \
         WHERE p.name = ?1 \
           AND p.value_type = 'link' \
           AND p.resource_id IN ({placeholders}) \
           AND p.seq = ( \
               SELECT MIN(p2.seq) FROM properties p2 \
               WHERE p2.resource_id = p.resource_id \
                 AND p2.name = p.name \
                 AND p2.value_type = 'link' \
           ) \
           AND r.kind = 'image' \
           AND r.seq = ( \
               SELECT MIN(r2.seq) FROM representations r2 \
               WHERE r2.resource_id = p.target_id AND r2.kind = 'image' \
           )"
    );

    let mut stmt = conn.prepare(&sql).map_err(QueryError::query)?;

    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(ids.len() + 1);
    params.push(&class_media_property);
    for id in ids {
        params.push(id);
    }

    let rows: Vec<(String, String, Option<i64>, Option<i64>)> = stmt
        .query_map(params.as_slice(), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;

    Ok(rows
        .into_iter()
        .map(|(id, uri, width, height)| (id, cpe_core::ir::MediaThumb { uri, width, height }))
        .collect())
}

// Internal helpers

/// The five date columns of one property row, grouped so
/// `build_property_value`'s signature stays readable. Every field is the raw
/// column value; only meaningful when `value_type = 'date'`.
struct DateColumns {
    start: Option<String>,
    end: Option<String>,
    precision: Option<String>,
    display: Option<String>,
    calendar: Option<String>,
}

/// Build a `PropertyValue` from the columns of one property row.
///
/// `value_type` is validated at data-load time by the schema CHECK constraint;
/// an unrecognised value here indicates data corruption and returns an error.
#[allow(clippy::too_many_arguments)]
fn build_property_value(
    value_type: &str,
    value_text: Option<String>,
    value_lang: Option<String>,
    value_int: Option<i64>,
    value_real: Option<f64>,
    date: DateColumns,
    target_id: Option<String>,
    conn: &Connection,
    lang: &Lang,
) -> Result<PropertyValue, QueryError> {
    match value_type {
        "text" => Ok(PropertyValue::Text {
            text: value_text.expect(
                "value_text is non-NULL for value_type='text' \
                 — enforced by boot validation (MalformedPropertyRow)",
            ),
            lang: value_lang,
        }),
        "richtext" => Ok(PropertyValue::Richtext {
            text: value_text.expect(
                "value_text is non-NULL for value_type='richtext' \
                 — enforced by boot validation (MalformedPropertyRow)",
            ),
            lang: value_lang,
        }),
        "integer" => Ok(PropertyValue::Integer(value_int.expect(
            "value_int is non-NULL for value_type='integer' \
             — enforced by boot validation (MalformedPropertyRow)",
        ))),
        "decimal" => Ok(PropertyValue::Decimal(value_real.expect(
            "value_real is non-NULL for value_type='decimal' \
             — enforced by boot validation (MalformedPropertyRow)",
        ))),
        "boolean" => Ok(PropertyValue::Boolean(
            value_int.expect(
                "value_int is non-NULL for value_type='boolean' \
                 — enforced by boot validation (MalformedPropertyRow)",
            ) != 0,
        )),
        "date" => {
            let start = date.start.expect(
                "date_start is non-NULL for value_type='date' \
                 — enforced by boot validation (MalformedPropertyRow)",
            );
            let precision = date
                .precision
                .as_deref()
                .and_then(DatePrecision::from_schema_str)
                // NULL date_precision legitimately means day precision — not a violation.
                .unwrap_or(DatePrecision::Day);
            // date_display / date_calendar are always optional — NULL is never
            // a violation, so `malformed_property_rows` does not check them.
            let calendar = date.calendar.as_deref().map(|s| {
                Calendar::from_schema_str(s).expect(
                    "date_calendar is NULL, 'julian' or 'gregorian' \
                     — enforced by the schema CHECK constraint at load",
                )
            });
            Ok(PropertyValue::Date {
                start,
                end: date.end,
                precision,
                display: date.display,
                // For a date row, `value_lang` names the language the
                // display string is written in.
                display_lang: value_lang,
                calendar,
            })
        }
        "link" => {
            let tid = target_id.expect(
                "target_id is non-NULL for value_type='link' \
                 — enforced by boot validation (MalformedPropertyRow)",
            );
            // N+1: one fetch per link target. Label and class resolved together
            // from the same target row to avoid a second round-trip.
            let target_label = get_resource_label(conn, &tid, lang)?;
            let target_class = class_of(conn, &tid)?;
            Ok(PropertyValue::Link { target_id: tid, target_label, target_class })
        }
        "list_value" => {
            let node_id = target_id.expect(
                "target_id is non-NULL for value_type='list_value' \
                 — enforced by boot validation (MalformedPropertyRow)",
            );
            let label = get_list_node_label(conn, &node_id, lang)?;
            Ok(PropertyValue::ListValue { node_id, label })
        }
        unknown => {
            // The schema CHECK constraint prevents unknown value_type strings,
            // but return a dedicated error variant rather than panicking if
            // data is corrupted. `UnknownValueType` carries the bad string for
            // diagnostics without reusing a rusqlite error shape.
            Err(QueryError::UnknownValueType(unknown.to_owned()))
        }
    }
}

/// Fetch the display label for a link target: the target resource's `name`
/// property, language-picked for `lang`.
///
/// Fallback order: requested lang → universal (value_lang IS NULL) → first
/// present. Returns `None` when the target does not exist or has no `name`
/// property.
///
/// N+1: called once per link row.
fn get_resource_label(conn: &Connection, target_id: &str, lang: &Lang) -> Result<Option<String>, QueryError> {
    // Fetch all `name` rows for the target (implicitly confirms existence).
    let mut stmt = conn
        .prepare(
            "SELECT value_text, value_lang \
             FROM properties \
             WHERE resource_id = ?1 AND name = 'name' \
             ORDER BY seq",
        )
        .map_err(QueryError::query)?;

    let rows: Vec<(Option<String>, Option<String>)> = stmt
        .query_map([target_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;

    if rows.is_empty() {
        // Target either doesn't exist or has no `name` property.
        return Ok(None);
    }

    // Priority 1: exact lang match.
    if let Some(text) = rows
        .iter()
        .find(|(_, l)| l.as_deref() == Some(lang.as_str()))
        .and_then(|(v, _)| v.clone())
    {
        return Ok(Some(text));
    }

    // Priority 2: universal (value_lang IS NULL).
    if let Some(text) = rows.iter().find(|(_, l)| l.is_none()).and_then(|(v, _)| v.clone()) {
        return Ok(Some(text));
    }

    // Priority 3: first present.
    Ok(rows.into_iter().find_map(|(v, _)| v))
}

/// Fetch the label for a list node, resolved to `lang` with fallback.
///
/// Fallback order: requested lang → any first present label.
/// `list_node_labels.lang` is NOT NULL, so there is no universal/NULL-lang
/// step here (unlike text/richtext property values).
fn get_list_node_label(conn: &Connection, node_id: &str, lang: &Lang) -> Result<Option<String>, QueryError> {
    let mut stmt = conn
        .prepare("SELECT lang, label FROM list_node_labels WHERE node_id = ?1")
        .map_err(QueryError::query)?;

    let rows: Vec<(String, String)> = stmt
        .query_map([node_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;

    if rows.is_empty() {
        return Ok(None);
    }

    // Requested lang first, then any first present.
    let label = rows
        .iter()
        .find(|(l, _)| l == lang.as_str())
        .or_else(|| rows.first())
        .map(|(_, label)| label.clone());

    Ok(label)
}

// Facet query functions

/// Return `(node_id, label, count)` tuples for every distinct `target_id` on
/// the given `property` across resources of `class`, subject to `base_wheres`.
///
/// Each tuple is:
/// - `node_id` — the `target_id` column value (list node id).
/// - `label` — resolved display label for `lang` with fallback (requested lang → any first
///   present); `None` when the node has no labels (orphan node id).
/// - `count` — number of **distinct** resources of `class` that have a `list_value` property row
///   for this node (subject to `base_wheres`).
///
/// Results are ordered `cnt DESC, label ASC, target_id ASC` — most-populated
/// options first; `label` and `target_id` are stable tiebreaks.
///
/// **Param-binding order** (left-to-right as `?`s appear in the SQL):
///   `lang` (SELECT CASE) → `property` (JOIN ON) → `class` (WHERE) →
///   `build_where` fragment params.
///
/// A NULL label (orphan node — no label rows) is returned as `None`; the
/// caller (`ProjectReads::facet_options`) applies the node-id fallback at that
/// level, not here.
pub(crate) fn list_facet_values(
    conn: &Connection,
    class: &str,
    property: &str,
    base_wheres: &[WhereClause],
    lang: &Lang,
) -> Result<Vec<(String, Option<String>, usize)>, QueryError> {
    let (where_fragment, where_params) = build_where(base_wheres)?;

    let sql = format!(
        "SELECT p.target_id, \
                COALESCE(MIN(CASE WHEN lnl.lang = ? THEN lnl.label END), \
                         MIN(lnl.label)) AS label, \
                COUNT(DISTINCT r.id) AS cnt \
         FROM resources r \
         JOIN properties p ON p.resource_id = r.id AND p.name = ? AND p.value_type = 'list_value' \
         LEFT JOIN list_node_labels lnl ON lnl.node_id = p.target_id \
         WHERE r.class = ?{where_fragment} \
         GROUP BY p.target_id \
         ORDER BY cnt DESC, label, p.target_id"
    );

    // Param-binding order: lang → property → class → base_where_params.
    // Collected in separate vecs first (explicit discipline, mirrors list_resource_ids).
    let lang_param: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Text(lang.as_str().to_owned())];
    let prop_param: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Text(property.to_owned())];
    let class_param: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Text(class.to_owned())];

    let mut all_params: Vec<rusqlite::types::Value> = lang_param;
    all_params.extend(prop_param);
    all_params.extend(class_param);
    all_params.extend(where_params);

    let mut stmt = conn.prepare(&sql).map_err(QueryError::query)?;
    let rows: Vec<(String, Option<String>, usize)> = stmt
        .query_map(rusqlite::params_from_iter(&all_params), |row| {
            let node_id: String = row.get(0)?;
            let label: Option<String> = row.get(1)?;
            let cnt: i64 = row.get(2)?;
            Ok((node_id, label, cnt))
        })
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?
        .into_iter()
        .map(|(node_id, label, cnt)| {
            // COUNT(DISTINCT …) is always ≥ 0 and fits in usize on any supported platform.
            let count = usize::try_from(cnt).unwrap_or(0);
            (node_id, label, count)
        })
        .collect();

    Ok(rows)
}

/// The list nodes that `class`'s `property` values point at, in vocabulary
/// order (`list_nodes.seq, node_id`), each with its label in `lang`, else any
/// label (the same pick [`list_facet_values`] makes).
///
/// Deliberately a separate query from [`list_facet_values`]: that one orders
/// by value count for the facet panel, and a `timeline`'s lanes follow the
/// vocabulary's own order. A `target_id` with no `list_nodes` row has no
/// `seq`, so it is not returned (the timeline puts such a marker in its `—`
/// lane).
pub(crate) fn list_nodes_for_property(
    conn: &Connection,
    class: &str,
    property: &str,
    lang: &Lang,
) -> Result<Vec<LaneNode>, QueryError> {
    let sql = "SELECT ln.id, \
                COALESCE(MIN(CASE WHEN lnl.lang = ?1 THEN lnl.label END), \
                         MIN(lnl.label)) AS label \
         FROM list_nodes ln \
         LEFT JOIN list_node_labels lnl ON lnl.node_id = ln.id \
         WHERE ln.id IN ( \
             SELECT p.target_id \
             FROM properties p \
             JOIN resources r ON r.id = p.resource_id \
             WHERE r.class = ?2 AND p.name = ?3 AND p.value_type = 'list_value') \
         GROUP BY ln.id, ln.seq \
         ORDER BY ln.seq, ln.id";
    let mut stmt = conn.prepare(sql).map_err(QueryError::query)?;
    let rows = stmt
        .query_map(rusqlite::params![lang.as_str(), class, property], |row| {
            Ok(LaneNode { node_id: row.get(0)?, label: row.get(1)? })
        })
        .map_err(QueryError::query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(QueryError::query)?;
    Ok(rows)
}

/// Return the set of distinct `value_type` strings for the given `property`
/// across all resources of `class`.
///
/// An empty set means the property does not appear on this class at all (it
/// may exist on other classes — see `property_exists_anywhere`). A non-empty
/// set tells the validator which storage types are actually in use for this
/// property on this class (e.g. `{"list_value"}` for a facet target, or
/// `{"text"}` for a plain text property).
///
/// Used by boot validation to check facet kind↔value-type compatibility
/// (`IncompatibleFacetKind`, `SparseFacetProperty`).
pub(crate) fn property_value_types_on_class(
    conn: &Connection,
    class: &str,
    property: &str,
) -> Result<HashSet<String>, QueryError> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT p.value_type \
             FROM properties p \
             JOIN resources r ON r.id = p.resource_id \
             WHERE r.class = ? AND p.name = ?",
        )
        .map_err(QueryError::query)?;

    let types: HashSet<String> = stmt
        .query_map([class, property], |row| row.get(0))
        .map_err(QueryError::query)?
        .collect::<Result<HashSet<String>, _>>()
        .map_err(QueryError::query)?;

    Ok(types)
}

/// Return the set of distinct target-resource classes that a `link`-typed
/// `property` resolves to, across all resources of `class`.
///
/// Only rows with a non-NULL `target_id` that resolves to an existing
/// resource contribute (a dangling target contributes no class — that case
/// is separately flagged by `dangling_link_targets`/`UnknownLinkTarget`).
///
/// Used by the `media="…"` boot validator
/// to check that a `media` property's link targets belong to a class
/// whose resources carry `representations` rows (image-like).
pub(crate) fn link_target_classes_on_class(
    conn: &Connection,
    class: &str,
    property: &str,
) -> Result<HashSet<String>, QueryError> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT r2.class \
             FROM properties p \
             JOIN resources r1 ON r1.id = p.resource_id \
             JOIN resources r2 ON r2.id = p.target_id \
             WHERE r1.class = ?1 AND p.name = ?2 AND p.value_type = 'link'",
        )
        .map_err(QueryError::query)?;

    let classes: HashSet<String> = stmt
        .query_map([class, property], |row| row.get(0))
        .map_err(QueryError::query)?
        .collect::<Result<HashSet<String>, _>>()
        .map_err(QueryError::query)?;

    Ok(classes)
}

/// Return `true` when at least one resource of `class` that carries a `link`
/// value through any of `link_properties` has NO value for `property` — i.e.
/// some `incoming-links` row would lack it
/// (`SparseIncomingLinksRowProperty`). Scoped to the linking resources, not the
/// whole class: a source resource that links nowhere can never be a row.
/// `link_properties` empty ⇒ `false`.
pub(crate) fn linking_resource_lacks_property(
    conn: &Connection,
    class: &str,
    link_properties: &[String],
    property: &str,
) -> Result<bool, QueryError> {
    if link_properties.is_empty() {
        return Ok(false);
    }
    let placeholders = vec!["?"; link_properties.len()].join(", ");
    let sql = format!(
        "SELECT EXISTS( \
             SELECT 1 FROM resources r \
             WHERE r.class = ? \
               AND EXISTS (SELECT 1 FROM properties l \
                           WHERE l.resource_id = r.id AND l.value_type = 'link' \
                             AND l.name IN ({placeholders})) \
               AND NOT EXISTS (SELECT 1 FROM properties p \
                               WHERE p.resource_id = r.id AND p.name = ?) \
         )"
    );
    let mut params: Vec<&str> = Vec::with_capacity(link_properties.len() + 2);
    params.push(class);
    params.extend(link_properties.iter().map(String::as_str));
    params.push(property);
    let lacks: bool = conn
        .query_row(&sql, rusqlite::params_from_iter(&params), |row| row.get(0))
        .map_err(QueryError::query)?;
    Ok(lacks)
}

/// Return `true` when at least one resource of `class` has at least one
/// `representations` row — i.e. `class` is "image-like" for the purposes of
/// the `media="…"` boot validator.
pub(crate) fn class_has_representations(conn: &Connection, class: &str) -> Result<bool, QueryError> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS( \
                 SELECT 1 FROM representations rep \
                 JOIN resources r ON r.id = rep.resource_id \
                 WHERE r.class = ?1 \
             )",
            [class],
            |row| row.get(0),
        )
        .map_err(QueryError::query)?;
    Ok(exists)
}

// Tests
//
// Layer-2 integration tests for the query layer.
//
// Each test builds an in-memory SQLite database by applying the store's
// schema (`schema.sql`) and inserting hand-written fixture data, then asserts
// the behaviour of `get_resource`, `resource_exists`, and
// `properties_present_on_class_of`.
//
// These are unit tests inside the crate, not integration tests in a `tests/`
// directory, so they can call the `pub(crate)` functions directly. Those take
// a `&rusqlite::Connection`, which `ProjectReads` keeps off the read surface.

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use cpe_core::ir::{
        Calendar, DataArks, DatePrecision, GroupKey, IncomingGroupBy, IncomingGrouping, IncomingLayout, IncomingLink,
        IncomingLinks, IncomingLinksSpec, IncomingOrder, MemberParent, PropertyValue, RepresentationKind, Sort,
        SortDir, SortKey, Value, WhereClause, DEFAULT_INCOMING_LINKS_LIMIT, GROUPED_INCOMING_LIMIT_CEILING,
    };
    use cpe_core::lang::Lang;
    use rusqlite::Connection;

    use super::{
        build_where, class_exists, class_has_representations, class_of, compound_has_parts, compound_member_classes,
        compound_part_headers, count_resources, count_resources_sql, dangling_link_targets, data_arks,
        get_representations, get_resource, incoming_links, incoming_links_statement, link_target_classes_on_class,
        linking_resource_lacks_property, list_facet_values, list_nodes_for_property, list_resource_ids,
        malformed_data_ark_rows, malformed_property_rows, media_item_ids, media_thumb_uris,
        properties_present_on_class, properties_present_on_class_of, property_exists_anywhere,
        property_value_types_on_class, resource_exists, search_index_name,
    };

    // Schema DDL

    const SCHEMA_SQL: &str = include_str!("../schema.sql");

    // Fixture data

    /// All 8 value types covered; includes:
    /// - `link` with a resolvable target (`r2`) and a broken target (`missing`)
    /// - `list_value` with EN+DE labels in `list_node_labels`
    /// - multi-valued `text` with EN and DE rows (same resource+name, different seq)
    /// - `richtext` row with `value_lang IS NULL`
    /// - `boolean`, `integer`, `decimal`, `date` (with precision and range)
    const FIXTURE: &str = "
        -- Resources
        INSERT INTO resources (id, class) VALUES
            ('r1', 'Character'),
            ('r2', 'Location'),
            ('r3', 'Story');

        -- r2 has a name property (target label for the link from r1)
        INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
            ('r2', 'name', 0, 'text', 'Wonderland',   'en'),
            ('r2', 'name', 1, 'text', 'Wunderland',   'de');

        -- r3 has only a universal-lang name (value_lang IS NULL)
        INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
            ('r3', 'name', 0, 'text', 'Alice in Wonderland', NULL);

        -- r3 date with a curated display string, its language and a calendar
        INSERT INTO properties (resource_id, name, seq, value_type, value_lang,
                                date_start, date_end, date_precision, date_display, date_calendar) VALUES
            ('r3', 'pub_date', 0, 'date', 'de', '1491-07-28', '1491-07-28', 'day',
             '28. Juli 1491', 'julian');

        -- r3 date with none of the three (display / lang / calendar all NULL)
        INSERT INTO properties (resource_id, name, seq, value_type, date_start, date_end, date_precision) VALUES
            ('r3', 'reprint_date', 0, 'date', '1492', NULL, 'year');

        -- List setup for list_value
        INSERT INTO lists (id, name) VALUES ('l1', 'role');
        INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES ('n1', 'l1', NULL, 0);
        INSERT INTO list_node_labels (node_id, lang, label) VALUES
            ('n1', 'en', 'Protagonist'),
            ('n1', 'de', 'Protagonistin');

        -- r1 properties: one of each value type
        -- multi-valued text: EN and DE rows
        INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
            ('r1', 'name', 0, 'text', 'Alice', 'en'),
            ('r1', 'name', 1, 'text', 'Alice (DE)', 'de');

        -- richtext with value_lang IS NULL
        INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
            ('r1', 'description', 0, 'richtext', '**A curious girl.**', NULL);

        -- integer
        INSERT INTO properties (resource_id, name, seq, value_type, value_int) VALUES
            ('r1', 'chapter_count', 0, 'integer', 12);

        -- decimal
        INSERT INTO properties (resource_id, name, seq, value_type, value_real) VALUES
            ('r1', 'rating', 0, 'decimal', 4.5);

        -- boolean
        INSERT INTO properties (resource_id, name, seq, value_type, value_int) VALUES
            ('r1', 'is_protagonist', 0, 'boolean', 1);

        -- date (single, day precision)
        INSERT INTO properties (resource_id, name, seq, value_type, date_start, date_end, date_precision) VALUES
            ('r1', 'birth_date', 0, 'date', '1865-07-04', NULL, 'day');

        -- date (range, year precision)
        INSERT INTO properties (resource_id, name, seq, value_type, date_start, date_end, date_precision) VALUES
            ('r1', 'active_years', 0, 'date', '1860', '1870', 'year');

        -- link: resolvable target (r2) and broken target (missing)
        INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
            ('r1', 'location', 0, 'link', 'r2'),
            ('r1', 'other_link', 0, 'link', 'missing');

        -- list_value
        INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
            ('r1', 'role', 0, 'list_value', 'n1');
    ";

    // Helpers

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(FIXTURE).expect("fixture should load");
        conn
    }

    fn make_lang(code: &str) -> Lang {
        // Build a Lang via the public `resolve` API (resolve picks from a
        // declared list; we declare exactly the code we want as the only option).
        use cpe_core::ir::SiteConfig;
        use cpe_core::lang::resolve;
        let site = SiteConfig {
            title: "T".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec![code.to_owned()],
            default_language: code.to_owned(),
            nav: vec![],
            chrome: vec![],
            property_labels: vec![],
            class_labels: vec![],
            theme: cpe_core::theme::ThemeKnob::default(),
            date_precision_word: false,
            footer_attribution: None,
            not_found_browse: vec![],
            narrative_companion: Default::default(),
            narrative_side: Default::default(),
            text_align: Default::default(),
        };
        resolve(&[code.to_owned()], &site)
    }

    /// Build a `Lang` for `code` against a site that declares *all* `declared`
    /// languages (first is the default). This lets tests exercise paths where the
    /// resolved lang is a real declared language, but some data (e.g.
    /// `list_node_labels`) lacks an entry for it.
    fn make_lang_for(code: &str, declared: &[&str]) -> Lang {
        use cpe_core::ir::SiteConfig;
        use cpe_core::lang::resolve;
        assert!(!declared.is_empty(), "make_lang_for: declared languages must not be empty");
        let site = SiteConfig {
            title: "T".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: declared.iter().map(|s| s.to_string()).collect(),
            default_language: declared[0].to_owned(),
            nav: vec![],
            chrome: vec![],
            property_labels: vec![],
            class_labels: vec![],
            theme: cpe_core::theme::ThemeKnob::default(),
            date_precision_word: false,
            footer_attribution: None,
            not_found_browse: vec![],
            narrative_companion: Default::default(),
            narrative_side: Default::default(),
            text_align: Default::default(),
        };
        resolve(&[code.to_owned()], &site)
    }

    // Tests

    #[test]
    fn missing_id_returns_ok_none() {
        let conn = setup_db();
        let lang = make_lang("en");
        let result = get_resource(&conn, "does_not_exist", &lang).expect("should not error");
        assert!(result.is_none(), "missing id should return Ok(None)");
    }

    #[test]
    fn resource_exists_true_for_present_id() {
        let conn = setup_db();
        assert!(resource_exists(&conn, "r1").expect("should not error"), "r1 should exist");
    }

    #[test]
    fn resource_exists_false_for_absent_id() {
        let conn = setup_db();
        assert!(
            !resource_exists(&conn, "nope").expect("should not error"),
            "nope should not exist"
        );
    }

    #[test]
    fn properties_grouped_by_name_preserving_seq() {
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang)
            .expect("query should not error")
            .expect("r1 should exist");

        // `name` property should be grouped into one Property with two values.
        let name_prop = resource
            .properties
            .iter()
            .find(|p| p.name == "name")
            .expect("r1 should have a 'name' property");

        assert_eq!(name_prop.values.len(), 2, "multi-valued text: EN and DE rows should be grouped");
        // EN row (seq=0) comes first.
        assert_eq!(
            name_prop.values[0],
            PropertyValue::Text { text: "Alice".to_owned(), lang: Some("en".to_owned()) }
        );
        assert_eq!(
            name_prop.values[1],
            PropertyValue::Text { text: "Alice (DE)".to_owned(), lang: Some("de".to_owned()) }
        );
    }

    #[test]
    fn richtext_null_lang_handled() {
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang)
            .expect("query should not error")
            .expect("r1 should exist");

        let desc = resource
            .properties
            .iter()
            .find(|p| p.name == "description")
            .expect("r1 should have 'description'");

        assert_eq!(desc.values.len(), 1);
        assert_eq!(
            desc.values[0],
            PropertyValue::Richtext {
                text: "**A curious girl.**".to_owned(),
                lang: None, // value_lang IS NULL → None
            }
        );
    }

    #[test]
    fn integer_value() {
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource
            .properties
            .iter()
            .find(|p| p.name == "chapter_count")
            .expect("chapter_count exists");
        assert_eq!(p.values[0], PropertyValue::Integer(12));
    }

    #[test]
    fn decimal_value() {
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource.properties.iter().find(|p| p.name == "rating").expect("rating exists");
        assert_eq!(p.values[0], PropertyValue::Decimal(4.5));
    }

    #[test]
    fn boolean_value() {
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource
            .properties
            .iter()
            .find(|p| p.name == "is_protagonist")
            .expect("is_protagonist exists");
        assert_eq!(p.values[0], PropertyValue::Boolean(true));
    }

    #[test]
    fn date_single_day_precision() {
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource
            .properties
            .iter()
            .find(|p| p.name == "birth_date")
            .expect("birth_date exists");

        assert_eq!(
            p.values[0],
            PropertyValue::Date {
                start: "1865-07-04".to_owned(),
                end: None,
                precision: DatePrecision::Day,
                display: None,
                display_lang: None,
                calendar: None,
            }
        );
    }

    #[test]
    fn date_range_year_precision() {
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource
            .properties
            .iter()
            .find(|p| p.name == "active_years")
            .expect("active_years exists");

        assert_eq!(
            p.values[0],
            PropertyValue::Date {
                start: "1860".to_owned(),
                end: Some("1870".to_owned()),
                precision: DatePrecision::Year,
                display: None,
                display_lang: None,
                calendar: None,
            }
        );
    }

    fn r3_date(name: &str) -> PropertyValue {
        let conn = setup_db();
        let lang = make_lang("en");
        let mut resource = get_resource(&conn, "r3", &lang).expect("no error").expect("r3 exists");
        let idx = resource
            .properties
            .iter()
            .position(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} exists on r3"));
        resource.properties.swap_remove(idx).values.remove(0)
    }

    #[test]
    fn date_display_and_calendar_hydrate() {
        // The request language (en) differs from the display string's
        // language (de): hydration carries both verbatim and decides nothing.
        assert_eq!(
            r3_date("pub_date"),
            PropertyValue::Date {
                start: "1491-07-28".to_owned(),
                end: Some("1491-07-28".to_owned()),
                precision: DatePrecision::Day,
                display: Some("28. Juli 1491".to_owned()),
                display_lang: Some("de".to_owned()),
                calendar: Some(Calendar::Julian),
            }
        );
    }

    #[test]
    fn date_without_display_is_none() {
        assert_eq!(
            r3_date("reprint_date"),
            PropertyValue::Date {
                start: "1492".to_owned(),
                end: None,
                precision: DatePrecision::Year,
                display: None,
                display_lang: None,
                calendar: None,
            }
        );
    }

    #[test]
    fn date_calendar_gregorian_hydrates() {
        let conn = setup_db();
        conn.execute_batch(
            "INSERT INTO properties (resource_id, name, seq, value_type, date_start, date_precision, date_calendar)
                 VALUES ('r3', 'greg_date', 0, 'date', '1489', 'year', 'gregorian');",
        )
        .expect("extra fixture should load");
        let resource = get_resource(&conn, "r3", &make_lang("en"))
            .expect("no error")
            .expect("r3 exists");
        let p = resource
            .properties
            .iter()
            .find(|p| p.name == "greg_date")
            .expect("greg_date exists");
        assert!(
            matches!(p.values[0], PropertyValue::Date { calendar: Some(Calendar::Gregorian), .. }),
            "got {:?}",
            p.values[0]
        );
    }

    #[test]
    fn unknown_date_calendar_rejected_by_check() {
        let conn = setup_db();
        let err = conn
            .execute_batch(
                "INSERT INTO properties (resource_id, name, seq, value_type, date_start, date_calendar)
                     VALUES ('r3', 'bad_date', 0, 'date', '1489', 'hebrew');",
            )
            .expect_err("an unknown date_calendar must violate the CHECK");
        assert!(
            err.to_string().contains("CHECK"),
            "expected a CHECK constraint failure, got: {err}"
        );
    }

    #[test]
    fn link_resolved_to_requested_lang() {
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource
            .properties
            .iter()
            .find(|p| p.name == "location")
            .expect("location exists");

        assert_eq!(
            p.values[0],
            PropertyValue::Link {
                target_id: "r2".to_owned(),
                target_label: Some("Wonderland".to_owned()), // EN label
                target_class: Some("Location".to_owned()),
            }
        );
    }

    #[test]
    fn link_resolved_to_de_lang() {
        let conn = setup_db();
        let lang = make_lang("de");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource
            .properties
            .iter()
            .find(|p| p.name == "location")
            .expect("location exists");

        assert_eq!(
            p.values[0],
            PropertyValue::Link {
                target_id: "r2".to_owned(),
                target_label: Some("Wunderland".to_owned()), // DE label
                target_class: Some("Location".to_owned()),
            }
        );
    }

    #[test]
    fn link_broken_target_gives_none_label() {
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource
            .properties
            .iter()
            .find(|p| p.name == "other_link")
            .expect("other_link exists");

        assert_eq!(
            p.values[0],
            PropertyValue::Link {
                target_id: "missing".to_owned(),
                target_label: None, // broken target → no label
                target_class: None, // broken target → no class
            }
        );
    }

    #[test]
    fn link_target_with_universal_lang_name_resolves_for_unknown_lang() {
        // r3 has name with value_lang IS NULL (universal). Requesting "fr" (not in
        // any target labels for r3) should fall back to the universal NULL-lang row.
        let conn = setup_db();
        // r3 is not directly linked from r1, so we create a tiny extra fixture.
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('r4', 'Character');
             INSERT INTO properties (resource_id, name, seq, value_type, target_id)
                 VALUES ('r4', 'story_link', 0, 'link', 'r3');",
        )
        .expect("extra fixture should load");

        let lang = make_lang("fr");
        let resource = get_resource(&conn, "r4", &lang).expect("no error").expect("r4 exists");

        let p = resource
            .properties
            .iter()
            .find(|p| p.name == "story_link")
            .expect("story_link exists");

        assert_eq!(
            p.values[0],
            PropertyValue::Link {
                target_id: "r3".to_owned(),
                // r3's name has value_lang IS NULL → universal fallback → label present
                target_label: Some("Alice in Wonderland".to_owned()),
                target_class: Some("Story".to_owned()),
            }
        );
    }

    #[test]
    fn list_value_resolved_to_requested_lang_en() {
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource.properties.iter().find(|p| p.name == "role").expect("role exists");

        assert_eq!(
            p.values[0],
            PropertyValue::ListValue {
                node_id: "n1".to_owned(),
                label: Some("Protagonist".to_owned()),
            }
        );
    }

    #[test]
    fn list_value_resolved_to_requested_lang_de() {
        let conn = setup_db();
        let lang = make_lang("de");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource.properties.iter().find(|p| p.name == "role").expect("role exists");

        assert_eq!(
            p.values[0],
            PropertyValue::ListValue {
                node_id: "n1".to_owned(),
                label: Some("Protagonistin".to_owned()),
            }
        );
    }

    #[test]
    fn list_value_fallback_to_first_when_lang_absent() {
        // Scenario: the list node has a label only in EN (no DE row exists).
        // The resolved Lang is DE — a fully-declared project language. Because
        // no DE label is present, `get_resource` must fall back to the first-
        // present label (EN = "Protagonist").
        //
        // This is distinct from the `_lang_de` test above: there the DE label
        // EXISTS and is returned directly; here it is ABSENT so the fallback path
        // is exercised.
        let conn = setup_db();
        // Remove the DE label from n1 so only EN is present.
        conn.execute_batch("DELETE FROM list_node_labels WHERE node_id = 'n1' AND lang = 'de';")
            .expect("delete DE label");

        // Lang is resolved as "de" from a site that declares both "en" and "de".
        let lang = make_lang_for("de", &["en", "de"]);
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource.properties.iter().find(|p| p.name == "role").expect("role exists");

        // The DE label is absent; the engine falls back to the first-present
        // label (EN row, stored first in seq order) → "Protagonist".
        assert_eq!(
            p.values[0],
            PropertyValue::ListValue {
                node_id: "n1".to_owned(),
                label: Some("Protagonist".to_owned()),
            },
            "when requested lang has no label, should fall back to first-present label"
        );
    }

    #[test]
    fn properties_present_on_class_of_returns_union() {
        // r1 and r2/r3 are different classes; r1 is 'Character'.
        // properties_present_on_class_of("r1") should return all names across
        // all Character resources (just r1 here).
        let conn = setup_db();
        let names = properties_present_on_class_of(&conn, "r1").expect("no error");

        // All property names that appear on r1.
        let expected: HashSet<String> = [
            "name",
            "description",
            "chapter_count",
            "rating",
            "is_protagonist",
            "birth_date",
            "active_years",
            "location",
            "other_link",
            "role",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        assert_eq!(
            names, expected,
            "should return all distinct property names for the Character class"
        );
    }

    #[test]
    fn properties_present_on_class_of_unknown_id_returns_empty() {
        let conn = setup_db();
        let names = properties_present_on_class_of(&conn, "no_such_id").expect("no error");
        assert!(names.is_empty(), "unknown id should return empty set");
    }

    #[test]
    fn properties_present_on_class_of_covers_sibling_resources() {
        // Add a second Character resource with a different property set.
        // The union should include names from both.
        let conn = setup_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('r5', 'Character');
             INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang)
                 VALUES ('r5', 'nickname', 0, 'text', 'The Girl', 'en');",
        )
        .expect("extra fixture should load");

        let names = properties_present_on_class_of(&conn, "r1").expect("no error");
        assert!(
            names.contains("nickname"),
            "union should include 'nickname' from sibling Character r5"
        );
        assert!(names.contains("name"), "union should still include 'name' from r1");
    }

    #[test]
    fn resource_class_is_correct() {
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");
        assert_eq!(resource.class, "Character");
        assert_eq!(resource.id, "r1");
    }

    #[test]
    fn query_error_type_does_not_expose_rusqlite() {
        // `get_resource` is `pub(crate)`, so only an in-crate test calls it with
        // a `Connection`. This test exercises the error path by triggering a
        // query error on a schema-less connection.
        let conn = Connection::open_in_memory().expect("in-memory db");
        // No schema applied → querying `resources` should fail.
        let result = get_resource(&conn, "x", &make_lang("en"));
        assert!(result.is_err(), "query on schema-less db should error");
        // The error message should be accessible without naming rusqlite::Error
        // through any public surface.
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(!msg.is_empty(), "error message should be non-empty: {msg}");
    }

    // get_representations round-trip

    #[test]
    fn get_representations_reads_alt_and_attribution() {
        // Verify that the `alt` and `attribution` columns round-trip
        // correctly through `get_representations` when they are non-NULL.
        // Also exercises the `seq` column index (col 6).
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('img_res', 'Image');
             INSERT INTO representations
                 (resource_id, kind, uri, width, height, alt, attribution, seq)
             VALUES
                 ('img_res', 'image',
                  'https://iiif.example.org/0854/img001/full/640,/0/default.jpg',
                  640, 480,
                  'Alice falling down the rabbit hole',
                  'John Tenniel · Public Domain',
                  0);",
        )
        .expect("fixture should load");

        let reprs = get_representations(&conn, "img_res").expect("get_representations should not error");
        assert_eq!(reprs.len(), 1, "should have exactly one representation");

        let repr = &reprs[0];
        assert_eq!(repr.kind, RepresentationKind::Image);
        assert_eq!(repr.uri, "https://iiif.example.org/0854/img001/full/640,/0/default.jpg");
        assert_eq!(repr.width, Some(640));
        assert_eq!(repr.height, Some(480));
        assert_eq!(repr.alt, Some("Alice falling down the rabbit hole".to_owned()));
        assert_eq!(repr.attribution, Some("John Tenniel \u{00b7} Public Domain".to_owned()));
        assert_eq!(repr.seq, 0);
    }

    #[test]
    fn get_representations_null_alt_and_attribution_give_none() {
        // Verify NULL columns round-trip to `None` (the dormant-column
        // behaviour when alt/attribution are absent).
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('plain_res', 'Image');
             INSERT INTO representations
                 (resource_id, kind, uri, width, height, seq)
             VALUES ('plain_res', 'image', 'https://example.org/img.jpg', NULL, NULL, 0);",
        )
        .expect("fixture should load");

        let reprs = get_representations(&conn, "plain_res").expect("should not error");
        assert_eq!(reprs.len(), 1);
        let repr = &reprs[0];
        assert_eq!(repr.alt, None, "NULL alt should give None");
        assert_eq!(repr.attribution, None, "NULL attribution should give None");
        assert_eq!(repr.width, None);
        assert_eq!(repr.height, None);
    }

    // malformed_property_rows

    /// Build a minimal DB with schema but no data — used for clean-DB assertions.
    fn setup_clean_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn
    }

    #[test]
    fn malformed_property_rows_clean_db_returns_empty() {
        // A database with only well-formed rows (or no rows) returns empty vec.
        let conn = setup_db(); // setup_db() uses the full FIXTURE with valid rows
        let rows = malformed_property_rows(&conn).expect("should not error");
        assert!(
            rows.is_empty(),
            "well-formed fixture should produce no malformed rows, got: {rows:?}"
        );
    }

    #[test]
    fn malformed_property_rows_text_null_value_text() {
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
             -- text row with NULL value_text (violation)
             INSERT INTO properties (resource_id, name, seq, value_type)
                 VALUES ('r1', 'label', 0, 'text');",
        )
        .expect("fixture should load");

        let rows = malformed_property_rows(&conn).expect("no error");
        assert_eq!(rows.len(), 1, "one violating row: {rows:?}");
        assert_eq!(rows[0].0, "r1");
        assert_eq!(rows[0].1, "label");
        assert_eq!(rows[0].2, "text");
        assert_eq!(rows[0].3, "value_text");
    }

    #[test]
    fn malformed_property_rows_date_null_date_start() {
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
             -- date row with NULL date_start (violation)
             INSERT INTO properties (resource_id, name, seq, value_type)
                 VALUES ('r1', 'created', 0, 'date');",
        )
        .expect("fixture should load");

        let rows = malformed_property_rows(&conn).expect("no error");
        assert_eq!(rows.len(), 1, "one violating row: {rows:?}");
        assert_eq!(rows[0].2, "date");
        assert_eq!(rows[0].3, "date_start");
    }

    #[test]
    fn malformed_property_rows_link_null_target_id() {
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
             -- link row with NULL target_id (violation)
             INSERT INTO properties (resource_id, name, seq, value_type)
                 VALUES ('r1', 'related', 0, 'link');",
        )
        .expect("fixture should load");

        let rows = malformed_property_rows(&conn).expect("no error");
        assert_eq!(rows.len(), 1, "one violating row: {rows:?}");
        assert_eq!(rows[0].2, "link");
        assert_eq!(rows[0].3, "target_id");
    }

    #[test]
    fn malformed_property_rows_accumulates_all_violation_types() {
        // Three rows covering three different violation classes, ordered by
        // (resource_id, name, seq). Proves the function accumulates rather than
        // short-circuits.
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
             -- text row: NULL value_text
             INSERT INTO properties (resource_id, name, seq, value_type)
                 VALUES ('r1', 'label', 0, 'text');
             -- date row: NULL date_start
             INSERT INTO properties (resource_id, name, seq, value_type)
                 VALUES ('r1', 'when', 0, 'date');
             -- link row: NULL target_id
             INSERT INTO properties (resource_id, name, seq, value_type)
                 VALUES ('r1', 'ref', 0, 'link');",
        )
        .expect("fixture should load");

        let rows = malformed_property_rows(&conn).expect("no error");
        assert_eq!(rows.len(), 3, "all three violations returned: {rows:?}");

        // Ordered by (resource_id, name, seq): label < ref < when alphabetically
        let names: Vec<&str> = rows.iter().map(|r| r.1.as_str()).collect();
        assert_eq!(names, vec!["label", "ref", "when"]);

        // Verify the missing_column mapping for each.
        let missing: Vec<&str> = rows.iter().map(|r| r.3).collect();
        assert_eq!(missing, vec!["value_text", "target_id", "date_start"]);
    }

    #[test]
    fn malformed_property_rows_integer_null_value_int() {
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
             INSERT INTO properties (resource_id, name, seq, value_type)
                 VALUES ('r1', 'count', 0, 'integer');",
        )
        .expect("fixture should load");

        let rows = malformed_property_rows(&conn).expect("no error");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].2, "integer");
        assert_eq!(rows[0].3, "value_int");
    }

    #[test]
    fn malformed_property_rows_decimal_null_value_real() {
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
             INSERT INTO properties (resource_id, name, seq, value_type)
                 VALUES ('r1', 'score', 0, 'decimal');",
        )
        .expect("fixture should load");

        let rows = malformed_property_rows(&conn).expect("no error");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].2, "decimal");
        assert_eq!(rows[0].3, "value_real");
    }

    #[test]
    fn malformed_property_rows_boolean_null_value_int() {
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
             INSERT INTO properties (resource_id, name, seq, value_type)
                 VALUES ('r1', 'active', 0, 'boolean');",
        )
        .expect("fixture should load");

        let rows = malformed_property_rows(&conn).expect("no error");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].2, "boolean");
        assert_eq!(rows[0].3, "value_int");
    }

    #[test]
    fn malformed_property_rows_richtext_null_value_text() {
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
             INSERT INTO properties (resource_id, name, seq, value_type)
                 VALUES ('r1', 'body', 0, 'richtext');",
        )
        .expect("fixture should load");

        let rows = malformed_property_rows(&conn).expect("no error");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].2, "richtext");
        assert_eq!(rows[0].3, "value_text");
    }

    #[test]
    fn malformed_property_rows_list_value_null_target_id() {
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
             INSERT INTO properties (resource_id, name, seq, value_type)
                 VALUES ('r1', 'category', 0, 'list_value');",
        )
        .expect("fixture should load");

        let rows = malformed_property_rows(&conn).expect("no error");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].2, "list_value");
        assert_eq!(rows[0].3, "target_id");
    }

    #[test]
    fn malformed_property_rows_well_formed_rows_not_returned() {
        // date row with NULL date_precision should NOT appear (precision is
        // legitimately optional — only date_start being NULL is a violation).
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
             -- valid date row: date_start is set, date_precision is NULL (fine)
             INSERT INTO properties (resource_id, name, seq, value_type, date_start)
                 VALUES ('r1', 'created', 0, 'date', '2024-01-01');",
        )
        .expect("fixture should load");

        let rows = malformed_property_rows(&conn).expect("no error");
        assert!(rows.is_empty(), "NULL date_precision must NOT be a violation: {rows:?}");
    }

    // class_of

    #[test]
    fn class_of_returns_class_for_existing_resource() {
        let conn = setup_db();
        let result = class_of(&conn, "r1").expect("should not error");
        assert_eq!(result, Some("Character".to_owned()), "r1 has class='Character'");
    }

    #[test]
    fn class_of_returns_none_for_absent_id() {
        let conn = setup_db();
        let result = class_of(&conn, "no_such_id").expect("should not error");
        assert_eq!(result, None, "absent id should return Ok(None)");
    }

    // target_class resolution in get_resource

    #[test]
    fn link_populates_target_class_for_existing_target() {
        // r1 has a 'location' link pointing at r2 (class='Location').
        // get_resource should populate target_class=Some("Location").
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource
            .properties
            .iter()
            .find(|p| p.name == "location")
            .expect("location exists");

        match &p.values[0] {
            PropertyValue::Link { target_class, target_id, .. } => {
                assert_eq!(target_id, "r2");
                assert_eq!(
                    target_class.as_deref(),
                    Some("Location"),
                    "r2 has class='Location', should be reflected in target_class"
                );
            }
            other => panic!("expected Link, got {other:?}"),
        }
    }

    #[test]
    fn link_target_class_is_none_for_dangling_link() {
        // r1 has an 'other_link' pointing at 'missing' which does not exist.
        // target_class must be None (not an error).
        let conn = setup_db();
        let lang = make_lang("en");
        let resource = get_resource(&conn, "r1", &lang).expect("no error").expect("r1 exists");

        let p = resource
            .properties
            .iter()
            .find(|p| p.name == "other_link")
            .expect("other_link exists");

        match &p.values[0] {
            PropertyValue::Link { target_class, target_label, .. } => {
                assert_eq!(
                    target_class.as_deref(),
                    None,
                    "dangling link target should have target_class=None"
                );
                assert_eq!(
                    target_label.as_deref(),
                    None,
                    "dangling link target should also have target_label=None"
                );
            }
            other => panic!("expected Link, got {other:?}"),
        }
    }

    // dangling_link_targets

    #[test]
    fn dangling_link_targets_returns_only_absent_targets() {
        // The FIXTURE has r1 with two link properties:
        //   - 'location' → 'r2' (r2 exists in resources) — valid, not returned
        //   - 'other_link' → 'missing' (absent from resources) — dangling, returned
        let conn = setup_db();
        let pairs = dangling_link_targets(&conn).expect("should not error");

        // Only the dangling pair should appear.
        assert_eq!(pairs.len(), 1, "exactly one dangling link expected, got: {pairs:?}");
        assert_eq!(
            pairs[0],
            ("r1".to_owned(), "missing".to_owned()),
            "dangling pair should be (source='r1', target='missing')"
        );
    }

    #[test]
    fn dangling_link_targets_clean_db_returns_empty() {
        // A DB where all link targets exist should return an empty vec.
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('src', 'Thing'), ('tgt', 'Thing');
             INSERT INTO properties (resource_id, name, seq, value_type, target_id)
                 VALUES ('src', 'ref', 0, 'link', 'tgt');",
        )
        .expect("fixture should load");

        let pairs = dangling_link_targets(&conn).expect("should not error");
        assert!(
            pairs.is_empty(),
            "all link targets present — no dangling pairs expected, got: {pairs:?}"
        );
    }

    #[test]
    fn dangling_link_targets_multiple_dangling_ordered() {
        // Two resources each with a dangling link; results must be ordered
        // deterministically by (resource_id, target_id).
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('a', 'Thing'), ('b', 'Thing');
             INSERT INTO properties (resource_id, name, seq, value_type, target_id)
                 VALUES ('a', 'ref', 0, 'link', 'gone1'),
                        ('b', 'ref', 0, 'link', 'gone2');",
        )
        .expect("fixture should load");

        let pairs = dangling_link_targets(&conn).expect("should not error");
        assert_eq!(pairs.len(), 2, "two dangling pairs expected: {pairs:?}");
        // Ordered by (resource_id, target_id): 'a' < 'b'.
        assert_eq!(pairs[0], ("a".to_owned(), "gone1".to_owned()));
        assert_eq!(pairs[1], ("b".to_owned(), "gone2".to_owned()));
    }

    // ── list_resource_ids / count_resources / class_exists / property helpers ─

    /// Build a DB with a few Characters and Locations for listing tests.
    ///
    /// Characters: c1 (name EN=Alice, DE=Alice DE), c2 (name EN=Bob, DE=Bob DE),
    ///             c3 (name EN=Zara), c4 (no name at all — for NULL-sort-last).
    /// Locations: l1 (name EN=Wonderland).
    /// Each character also has `age` (integer) for range/in filter tests.
    fn setup_list_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "
            INSERT INTO resources (id, class) VALUES
                ('c1', 'Character'),
                ('c2', 'Character'),
                ('c3', 'Character'),
                ('c4', 'Character'),
                ('l1', 'Location');

            -- c1: Alice (EN+DE multi-valued name), age=10
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('c1', 'name', 0, 'text', 'Alice', 'en'),
                ('c1', 'name', 1, 'text', 'Alice DE', 'de');
            INSERT INTO properties (resource_id, name, seq, value_type, value_int) VALUES
                ('c1', 'age', 0, 'integer', 10);

            -- c2: Bob (EN+DE multi-valued name), age=25
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('c2', 'name', 0, 'text', 'Bob', 'en'),
                ('c2', 'name', 1, 'text', 'Bob DE', 'de');
            INSERT INTO properties (resource_id, name, seq, value_type, value_int) VALUES
                ('c2', 'age', 0, 'integer', 25);

            -- c3: Zara (EN only), age=5
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('c3', 'name', 0, 'text', 'Zara', 'en');
            INSERT INTO properties (resource_id, name, seq, value_type, value_int) VALUES
                ('c3', 'age', 0, 'integer', 5);

            -- c4: no name (for NULL-sort-last test), age=99
            INSERT INTO properties (resource_id, name, seq, value_type, value_int) VALUES
                ('c4', 'age', 0, 'integer', 99);

            -- l1: Wonderland
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('l1', 'name', 0, 'text', 'Wonderland', 'en');
            ",
        )
        .expect("list fixture should load");
        conn
    }

    // class_exists

    #[test]
    fn class_exists_true_for_present_class() {
        let conn = setup_list_db();
        assert!(class_exists(&conn, "Character").expect("no error"), "Character should exist");
    }

    #[test]
    fn class_exists_false_for_absent_class() {
        let conn = setup_list_db();
        assert!(
            !class_exists(&conn, "NoSuchClass").expect("no error"),
            "NoSuchClass should not exist"
        );
    }

    // property_exists_anywhere

    #[test]
    fn property_exists_anywhere_true_for_present_name() {
        let conn = setup_list_db();
        assert!(
            property_exists_anywhere(&conn, "name").expect("no error"),
            "name should exist anywhere"
        );
    }

    #[test]
    fn property_exists_anywhere_false_for_absent_name() {
        let conn = setup_list_db();
        assert!(
            !property_exists_anywhere(&conn, "no_such_prop").expect("no error"),
            "no_such_prop should not exist anywhere"
        );
    }

    // properties_present_on_class

    #[test]
    fn properties_present_on_class_returns_union() {
        let conn = setup_list_db();
        let names = properties_present_on_class(&conn, "Character").expect("no error");
        // Characters have 'name' (c1, c2, c3) and 'age' (c1, c2, c3, c4).
        assert!(names.contains("name"), "name should be present on Character");
        assert!(names.contains("age"), "age should be present on Character");
    }

    #[test]
    fn properties_present_on_class_empty_for_absent_class() {
        let conn = setup_list_db();
        let names = properties_present_on_class(&conn, "NoSuchClass").expect("no error");
        assert!(names.is_empty(), "absent class should return empty set");
    }

    #[test]
    fn properties_present_on_class_does_not_include_other_class_props() {
        let conn = setup_list_db();
        // Location has 'name'. Character has 'age'. Querying Location should NOT
        // include 'age'.
        let names = properties_present_on_class(&conn, "Location").expect("no error");
        assert!(names.contains("name"), "Location has 'name'");
        assert!(!names.contains("age"), "Location should NOT have 'age'");
    }

    // list_resource_ids — basic sort

    #[test]
    fn list_resource_ids_sort_asc_by_name() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let sorts = vec![Sort {
            key: SortKey::Property("name".to_owned()),
            direction: SortDir::Asc,
        }];
        let ids = list_resource_ids(&conn, "Character", &[], &sorts, 100, 0, &lang).expect("no error");
        // c4 has no name (NULL sort_val) → sorts last.
        // With lang=en: c1 has EN="Alice", c2 has EN="Bob", c3 has EN="Zara".
        // EN match is prioritised; order: Alice < Bob < Zara, c4 (NULL) last.
        assert_eq!(ids, vec!["c1", "c2", "c3", "c4"]);
    }

    #[test]
    fn list_resource_ids_sort_desc_by_name() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let sorts = vec![Sort {
            key: SortKey::Property("name".to_owned()),
            direction: SortDir::Desc,
        }];
        let ids = list_resource_ids(&conn, "Character", &[], &sorts, 100, 0, &lang).expect("no error");
        // Zara > Bob > Alice; c4 (NULL) last regardless of direction.
        assert_eq!(ids, vec!["c3", "c2", "c1", "c4"]);
    }

    // list_resource_ids — NULL sort last (both directions)

    #[test]
    fn list_resource_ids_null_sort_last_asc() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let sorts = vec![Sort {
            key: SortKey::Property("name".to_owned()),
            direction: SortDir::Asc,
        }];
        let ids = list_resource_ids(&conn, "Character", &[], &sorts, 100, 0, &lang).expect("no error");
        // c4 has no name → NULL sort_val → must be last.
        assert_eq!(ids.last().expect("non-empty"), "c4");
    }

    #[test]
    fn list_resource_ids_null_sort_last_desc() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let sorts = vec![Sort {
            key: SortKey::Property("name".to_owned()),
            direction: SortDir::Desc,
        }];
        let ids = list_resource_ids(&conn, "Character", &[], &sorts, 100, 0, &lang).expect("no error");
        // c4 has no name → NULL sort_val → must be last regardless of DESC.
        assert_eq!(ids.last().expect("non-empty"), "c4");
    }

    // ── list_resource_ids — multi-valued property: each id ONCE, correct order ─

    #[test]
    fn list_resource_ids_multi_valued_sort_no_duplicates() {
        // c1 has TWO name rows (EN=Alice, DE=Alice DE). Without GROUP BY+MIN,
        // a JOIN would yield two rows for c1. Verify each id appears exactly once.
        let conn = setup_list_db();
        let lang = make_lang("en");
        let sorts = vec![Sort {
            key: SortKey::Property("name".to_owned()),
            direction: SortDir::Asc,
        }];
        let ids = list_resource_ids(&conn, "Character", &[], &sorts, 100, 0, &lang).expect("no error");

        // No duplicates.
        let mut seen = std::collections::HashSet::new();
        for id in &ids {
            assert!(seen.insert(id.clone()), "duplicate id: {id}");
        }

        // With lang=en: EN values are Alice/Bob/Zara. EN row is preferred.
        // Alphabetical order: Alice < Bob < Zara; c4 (no name) last.
        assert_eq!(ids, vec!["c1", "c2", "c3", "c4"]);
    }

    #[test]
    fn list_resource_ids_multi_valued_sort_min_keyed_order() {
        // Explicitly verify the sort key is the EN value, not an arbitrary row pick.
        // c1: EN=Alice, DE=Alice DE → sort key = "Alice" (EN preferred)
        // c2: EN=Bob, DE=Bob DE    → sort key = "Bob"   (EN preferred)
        // DESC order should be Zara, Bob, Alice, (no-name last).
        let conn = setup_list_db();
        let lang = make_lang("en");
        let sorts = vec![Sort {
            key: SortKey::Property("name".to_owned()),
            direction: SortDir::Desc,
        }];
        let ids = list_resource_ids(&conn, "Character", &[], &sorts, 100, 0, &lang).expect("no error");
        assert_eq!(ids[0], "c3", "Zara should be first in DESC");
        assert_eq!(ids[1], "c2", "Bob should be second in DESC");
        assert_eq!(ids[2], "c1", "Alice should be third in DESC");
        assert_eq!(ids[3], "c4", "no-name should be last");
    }

    // ── list_resource_ids — where + sort on different multi-valued props → no dups ─

    #[test]
    fn list_resource_ids_where_and_sort_on_different_props_no_duplicates() {
        // Filter by age (single-valued), sort by name (multi-valued).
        // Ensures row multiplication can't come from the sort JOIN when a where
        // EXISTS is also applied.
        let conn = setup_list_db();
        let lang = make_lang("en");
        let wheres = vec![WhereClause::Range {
            property: "age".to_owned(),
            start: Some(Value::Integer(1)),
            end: Some(Value::Integer(100)),
        }];
        let sorts = vec![Sort {
            key: SortKey::Property("name".to_owned()),
            direction: SortDir::Asc,
        }];
        let ids = list_resource_ids(&conn, "Character", &wheres, &sorts, 100, 0, &lang).expect("no error");

        // All four characters have age in [1,100]. Check no duplicates.
        let mut seen = std::collections::HashSet::new();
        for id in &ids {
            assert!(seen.insert(id.clone()), "duplicate id: {id}");
        }
        assert_eq!(ids.len(), 4, "expected 4 characters, got: {ids:?}");
    }

    /// Guard against a future refactor that uses a JOIN (instead of EXISTS) for
    /// the `where` clause. A JOIN on a multi-valued `where` property multiplies
    /// rows — one per matching value row — so a resource with two `keyword` rows
    /// that both satisfy the filter would appear twice.
    ///
    /// Fixture: 3 Characters with a multi-valued `keyword` property (2 rows each).
    /// One also has a multi-valued `name` property (EN+DE). The `where` filters on
    /// `keyword`, the sort is on `name`. A JOIN-based `where` would produce 2 rows
    /// per resource per keyword match, multiplied again by the sort JOIN → each
    /// id would appear 4+ times. The EXISTS approach produces exactly 1 row each.
    #[test]
    fn list_resource_ids_multivalued_where_no_duplicates() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "
            INSERT INTO resources (id, class) VALUES
                ('mw1', 'MultiItem'),
                ('mw2', 'MultiItem'),
                ('mw3', 'MultiItem');

            -- mw1: keywords = ['fiction', 'classic'] (2 rows) + name EN+DE
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('mw1', 'keyword', 0, 'text', 'fiction',  NULL),
                ('mw1', 'keyword', 1, 'text', 'classic',  NULL),
                ('mw1', 'name',    0, 'text', 'Beta Item', 'en'),
                ('mw1', 'name',    1, 'text', 'Beta Ding', 'de');

            -- mw2: keywords = ['fiction', 'modern'] (2 rows) + name EN+DE
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('mw2', 'keyword', 0, 'text', 'fiction', NULL),
                ('mw2', 'keyword', 1, 'text', 'modern',  NULL),
                ('mw2', 'name',    0, 'text', 'Alpha Item', 'en'),
                ('mw2', 'name',    1, 'text', 'Alpha Ding', 'de');

            -- mw3: keywords = ['nonfiction', 'modern'] — does NOT match 'fiction'
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('mw3', 'keyword', 0, 'text', 'nonfiction', NULL),
                ('mw3', 'keyword', 1, 'text', 'modern',     NULL),
                ('mw3', 'name',    0, 'text', 'Gamma Item', 'en'),
                ('mw3', 'name',    1, 'text', 'Gamma Ding', 'de');
            ",
        )
        .expect("multi-valued where fixture should load");

        let lang = make_lang("en");
        // WHERE keyword = 'fiction' — matches mw1 and mw2 (both have a 'fiction' row).
        // mw3 does NOT have 'fiction' so must be excluded.
        // SORT by name ASC (also multi-valued: EN+DE rows per resource).
        // A JOIN-based WHERE would multiply mw1/mw2 (2 keyword rows × 2 name rows = 4 rows each).
        let wheres = vec![WhereClause::Equals {
            property: "keyword".to_owned(),
            value: Value::String("fiction".to_owned()),
        }];
        let sorts = vec![Sort {
            key: SortKey::Property("name".to_owned()),
            direction: SortDir::Asc,
        }];

        let ids = list_resource_ids(&conn, "MultiItem", &wheres, &sorts, 100, 0, &lang).expect("query should succeed");

        // Exactly 2 results (mw1 and mw2), not 4 or 8.
        assert_eq!(
            ids.len(),
            2,
            "WHERE on multi-valued property + multi-valued sort must produce exactly 2 results \
             (mw1 and mw2), not multiplied rows. Got: {ids:?}"
        );

        // Each id appears exactly once — no duplicates.
        let mut seen = std::collections::HashSet::new();
        for id in &ids {
            assert!(
                seen.insert(id.clone()),
                "duplicate id '{id}' — WHERE clause JOIN would cause this. Got: {ids:?}"
            );
        }

        // mw3 must be excluded (its keyword rows don't contain 'fiction').
        assert!(
            !ids.contains(&"mw3".to_owned()),
            "mw3 has no 'fiction' keyword and must be excluded. Got: {ids:?}"
        );

        // Sort order: Alpha Item < Beta Item → mw2 first, mw1 second.
        assert_eq!(
            ids,
            vec!["mw2", "mw1"],
            "sort by name ASC: Alpha Item < Beta Item → mw2 before mw1. Got: {ids:?}"
        );

        // Count function must also agree (not double-count the multi-valued rows).
        let count = count_resources(&conn, "MultiItem", &wheres).expect("count should succeed");
        assert_eq!(
            count, 2,
            "count_resources with multi-valued WHERE must return 2, not a multiplied count. Got: {count}"
        );
    }

    // list_resource_ids — where equals

    #[test]
    fn list_resource_ids_where_equals_string() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let wheres = vec![WhereClause::Equals {
            property: "name".to_owned(),
            value: Value::String("Alice".to_owned()),
        }];
        let ids = list_resource_ids(&conn, "Character", &wheres, &[], 100, 0, &lang).expect("no error");
        assert_eq!(ids, vec!["c1"]);
    }

    #[test]
    fn list_resource_ids_where_equals_integer() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let wheres = vec![WhereClause::Equals { property: "age".to_owned(), value: Value::Integer(25) }];
        let ids = list_resource_ids(&conn, "Character", &wheres, &[], 100, 0, &lang).expect("no error");
        assert_eq!(ids, vec!["c2"]);
    }

    // list_resource_ids — where in

    #[test]
    fn list_resource_ids_where_in() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let wheres = vec![WhereClause::In {
            property: "name".to_owned(),
            values: vec![Value::String("Alice".to_owned()), Value::String("Zara".to_owned())],
        }];
        let ids = list_resource_ids(&conn, "Character", &wheres, &[], 100, 0, &lang).expect("no error");
        // Order is by id (no sort specified); c1 and c3.
        let mut ids_sorted = ids.clone();
        ids_sorted.sort();
        assert_eq!(ids_sorted, vec!["c1", "c3"]);
    }

    // list_resource_ids — where range

    #[test]
    fn list_resource_ids_where_range_both_bounds() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let wheres = vec![WhereClause::Range {
            property: "age".to_owned(),
            start: Some(Value::Integer(10)),
            end: Some(Value::Integer(25)),
        }];
        let ids = list_resource_ids(&conn, "Character", &wheres, &[], 100, 0, &lang).expect("no error");
        // age=10 (c1) and age=25 (c2) — both in [10,25].
        let mut ids_sorted = ids.clone();
        ids_sorted.sort();
        assert_eq!(ids_sorted, vec!["c1", "c2"]);
    }

    #[test]
    fn list_resource_ids_where_range_lower_bound_only() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let wheres = vec![WhereClause::Range {
            property: "age".to_owned(),
            start: Some(Value::Integer(25)),
            end: None,
        }];
        let ids = list_resource_ids(&conn, "Character", &wheres, &[], 100, 0, &lang).expect("no error");
        // age >= 25: c2 (25) and c4 (99).
        let mut ids_sorted = ids.clone();
        ids_sorted.sort();
        assert_eq!(ids_sorted, vec!["c2", "c4"]);
    }

    #[test]
    fn list_resource_ids_where_range_upper_bound_only() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let wheres = vec![WhereClause::Range {
            property: "age".to_owned(),
            start: None,
            end: Some(Value::Integer(10)),
        }];
        let ids = list_resource_ids(&conn, "Character", &wheres, &[], 100, 0, &lang).expect("no error");
        // age <= 10: c1 (10) and c3 (5).
        let mut ids_sorted = ids.clone();
        ids_sorted.sort();
        assert_eq!(ids_sorted, vec!["c1", "c3"]);
    }

    // list_resource_ids — pagination

    #[test]
    fn list_resource_ids_pagination_limit_offset() {
        // 4 Characters sorted by id (no sort clause). Pages of 2.
        let conn = setup_list_db();
        let lang = make_lang("en");
        let page1 = list_resource_ids(&conn, "Character", &[], &[], 2, 0, &lang).expect("no error");
        let page2 = list_resource_ids(&conn, "Character", &[], &[], 2, 2, &lang).expect("no error");

        // Page 1 should have 2 results, page 2 should have the other 2.
        assert_eq!(page1.len(), 2, "page 1 should have 2 results: {page1:?}");
        assert_eq!(page2.len(), 2, "page 2 should have 2 results: {page2:?}");

        // Combined, they should cover all 4 ids exactly once.
        let mut all: Vec<String> = page1.into_iter().chain(page2).collect();
        all.sort();
        assert_eq!(all, vec!["c1", "c2", "c3", "c4"]);
    }

    #[test]
    fn list_resource_ids_offset_beyond_end_returns_empty() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let ids = list_resource_ids(&conn, "Character", &[], &[], 10, 100, &lang).expect("no error");
        assert!(ids.is_empty(), "offset beyond end should return empty: {ids:?}");
    }

    // list_resource_ids — empty result

    #[test]
    fn list_resource_ids_empty_for_absent_class() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let ids = list_resource_ids(&conn, "NoSuchClass", &[], &[], 100, 0, &lang).expect("no error");
        assert!(ids.is_empty(), "absent class should return empty: {ids:?}");
    }

    #[test]
    fn list_resource_ids_empty_when_where_matches_nothing() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let wheres = vec![WhereClause::Equals {
            property: "name".to_owned(),
            value: Value::String("DoesNotExist".to_owned()),
        }];
        let ids = list_resource_ids(&conn, "Character", &wheres, &[], 100, 0, &lang).expect("no error");
        assert!(ids.is_empty(), "non-matching where should return empty: {ids:?}");
    }

    // count_resources

    #[test]
    fn count_resources_matches_list_length_no_filter() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let count = count_resources(&conn, "Character", &[]).expect("no error");
        let ids = list_resource_ids(&conn, "Character", &[], &[], u32::MAX, 0, &lang).expect("no error");
        assert_eq!(count, ids.len(), "count should match unpaginated id list length");
    }

    #[test]
    fn count_resources_matches_list_length_with_filter() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let wheres = vec![WhereClause::Range {
            property: "age".to_owned(),
            start: Some(Value::Integer(10)),
            end: Some(Value::Integer(30)),
        }];
        let count = count_resources(&conn, "Character", &wheres).expect("no error");
        let ids = list_resource_ids(&conn, "Character", &wheres, &[], u32::MAX, 0, &lang).expect("no error");
        assert_eq!(count, ids.len(), "count with filter should match unpaginated id list length");
    }

    #[test]
    fn count_resources_zero_for_no_match() {
        let conn = setup_list_db();
        let wheres = vec![WhereClause::Equals {
            property: "name".to_owned(),
            value: Value::String("NoBody".to_owned()),
        }];
        let count = count_resources(&conn, "Character", &wheres).expect("no error");
        assert_eq!(count, 0, "no matching resources should return 0");
    }

    #[test]
    fn count_resources_zero_for_absent_class() {
        let conn = setup_list_db();
        let count = count_resources(&conn, "NoSuchClass", &[]).expect("no error");
        assert_eq!(count, 0);
    }

    // value_out_of_range error

    #[test]
    fn integer_value_out_of_i64_range_returns_error() {
        use super::QueryError;
        let conn = setup_list_db();
        let lang = make_lang("en");
        // i128::MAX is well outside i64 range.
        let wheres = vec![WhereClause::Equals { property: "age".to_owned(), value: Value::Integer(i128::MAX) }];
        let result = list_resource_ids(&conn, "Character", &wheres, &[], 10, 0, &lang);
        assert!(
            matches!(result, Err(QueryError::ValueOutOfRange(_))),
            "i128::MAX should produce ValueOutOfRange, got: {result:?}"
        );
    }

    // Numeric integer sort (regression: no CAST to TEXT)

    /// Resources with integer sort values including single- and double-digit
    /// numbers must sort in NUMERIC order (1, 2, 9, 10, 11, 12), NOT lexical
    /// order (1, 10, 11, 12, 2, 9), which a CAST(value_int AS TEXT) would
    /// produce.
    ///
    /// MIN(value_int) with NUMERIC affinity (no text cast) in the
    /// language-aware COALESCE — integer values sort correctly because
    /// SQLite applies NUMERIC affinity when comparing/ordering INTEGER columns.
    #[test]
    fn list_resource_ids_integer_sort_is_numeric_not_lexical() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "
            INSERT INTO resources (id, class) VALUES
                ('n1', 'Item'),
                ('n2', 'Item'),
                ('n3', 'Item'),
                ('n4', 'Item'),
                ('n5', 'Item'),
                ('n6', 'Item');
            -- chapter-number style: integers 1, 2, 9, 10, 11, 12
            INSERT INTO properties (resource_id, name, seq, value_type, value_int) VALUES
                ('n1', 'chapter-number', 0, 'integer', 1),
                ('n2', 'chapter-number', 0, 'integer', 2),
                ('n3', 'chapter-number', 0, 'integer', 9),
                ('n4', 'chapter-number', 0, 'integer', 10),
                ('n5', 'chapter-number', 0, 'integer', 11),
                ('n6', 'chapter-number', 0, 'integer', 12);
            ",
        )
        .expect("integer fixture should load");

        let lang = make_lang("en");
        let sorts = vec![Sort {
            key: SortKey::Property("chapter-number".to_owned()),
            direction: SortDir::Asc,
        }];
        let ids = list_resource_ids(&conn, "Item", &[], &sorts, 100, 0, &lang).expect("query should succeed");

        // Numeric order: 1, 2, 9, 10, 11, 12.
        // Lexical (incorrect) order would be: 1, 10, 11, 12, 2, 9.
        assert_eq!(
            ids,
            vec!["n1", "n2", "n3", "n4", "n5", "n6"],
            "integer sort must be numeric (1,2,9,10,11,12), not lexical (1,10,11,12,2,9). \
             Got: {ids:?}"
        );
    }

    // Case-insensitive text sort (COLLATE NOCASE)

    /// A one-class fixture whose `name` values are the given `(id, name)` pairs.
    fn name_sort_db(rows: &[(&str, &str)]) -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        for (id, name) in rows {
            conn.execute("INSERT INTO resources (id, class) VALUES (?1, 'Item')", [id])
                .expect("resource row");
            conn.execute(
                "INSERT INTO properties (resource_id, name, seq, value_type, value_text) \
                 VALUES (?1, 'name', 0, 'text', ?2)",
                [id, name],
            )
            .expect("name row");
        }
        conn
    }

    fn ids_by_name(conn: &Connection, direction: SortDir) -> Vec<String> {
        let sorts = vec![Sort { key: SortKey::Property("name".to_owned()), direction }];
        list_resource_ids(conn, "Item", &[], &sorts, 100, 0, &make_lang("en")).expect("query should succeed")
    }

    #[test]
    fn list_resource_ids_text_sort_is_case_insensitive() {
        let conn = name_sort_db(&[("x1", "b"), ("x2", "A"), ("x3", "C")]);
        assert_eq!(ids_by_name(&conn, SortDir::Asc), vec!["x2", "x1", "x3"]);
        assert_eq!(ids_by_name(&conn, SortDir::Desc), vec!["x3", "x1", "x2"]);
    }

    /// Names equal under case-folding tie-break by id, in both directions.
    #[test]
    fn list_resource_ids_case_folded_tie_breaks_by_id() {
        let conn = name_sort_db(&[("t2", "a"), ("t1", "A"), ("t3", "b")]);
        assert_eq!(ids_by_name(&conn, SortDir::Asc), vec!["t1", "t2", "t3"]);
        assert_eq!(ids_by_name(&conn, SortDir::Desc), vec!["t3", "t1", "t2"]);
    }

    /// NOCASE folds ASCII only: an umlaut-initial name still files after
    /// every ASCII letter (`Ä` is U+00C4, compared as UTF-8 bytes).
    #[test]
    fn list_resource_ids_nocase_is_ascii_only() {
        let conn = name_sort_db(&[("u1", "Ärger"), ("u2", "zebra"), ("u3", "Apfel")]);
        assert_eq!(ids_by_name(&conn, SortDir::Asc), vec!["u3", "u2", "u1"]);
    }

    // Language-aware text sort

    /// Resources with multi-language name properties must sort by the requested
    /// language's value, not by the minimum across all languages. This test
    /// verifies both the lang-specific sort and the fallback chain.
    ///
    /// Scenario: two resources with conflicting EN vs IT names.
    ///   - Resource X: EN="Zebra", IT="Alpaca"
    ///   - Resource Y: EN="Apple", IT="Zucca"
    ///
    /// When sorted by EN: Apple < Zebra → Y, X (Y first).
    /// When sorted by IT: Alpaca < Zucca → X, Y (X first).
    ///
    /// These are different orderings, proving the language-aware sort works.
    #[test]
    fn list_resource_ids_lang_aware_sort_differs_by_language() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "
            INSERT INTO resources (id, class) VALUES
                ('x', 'Animal'),
                ('y', 'Animal');
            -- Resource X: EN='Zebra', IT='Alpaca'
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('x', 'name', 0, 'text', 'Zebra',  'en'),
                ('x', 'name', 1, 'text', 'Alpaca', 'it');
            -- Resource Y: EN='Apple', IT='Zucca'
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('y', 'name', 0, 'text', 'Apple', 'en'),
                ('y', 'name', 1, 'text', 'Zucca', 'it');
            ",
        )
        .expect("lang-sort fixture should load");

        let sorts_asc = vec![Sort {
            key: SortKey::Property("name".to_owned()),
            direction: SortDir::Asc,
        }];

        // EN sort: Apple < Zebra → Y, X.
        let lang_en = make_lang("en");
        let ids_en =
            list_resource_ids(&conn, "Animal", &[], &sorts_asc, 100, 0, &lang_en).expect("query should succeed");
        assert_eq!(ids_en, vec!["y", "x"], "EN sort: Apple < Zebra → Y, X. Got: {ids_en:?}");

        // IT sort: Alpaca < Zucca → X, Y.
        let lang_it = make_lang("it");
        let ids_it =
            list_resource_ids(&conn, "Animal", &[], &sorts_asc, 100, 0, &lang_it).expect("query should succeed");
        assert_eq!(ids_it, vec!["x", "y"], "IT sort: Alpaca < Zucca → X, Y. Got: {ids_it:?}");

        // The two orderings must differ (this is the crux of the test).
        assert_ne!(ids_en, ids_it, "EN and IT orderings must differ for this fixture");
    }

    /// Universal (value_lang IS NULL) names sort correctly under any requested
    /// language — they are picked by the second COALESCE arm (lang IS NULL).
    #[test]
    fn list_resource_ids_lang_aware_sort_universal_fallback() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "
            INSERT INTO resources (id, class) VALUES
                ('u1', 'Thing'),
                ('u2', 'Thing'),
                ('u3', 'Thing');
            -- u1: universal name 'Beta' (no value_lang)
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('u1', 'name', 0, 'text', 'Beta', NULL);
            -- u2: universal name 'Alpha' (no value_lang)
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('u2', 'name', 0, 'text', 'Alpha', NULL);
            -- u3: no name at all (NULL sort key → sorts last)
            ",
        )
        .expect("universal-lang fixture should load");

        let sorts_asc = vec![Sort {
            key: SortKey::Property("name".to_owned()),
            direction: SortDir::Asc,
        }];

        // Requesting 'de' — neither resource has a 'de' value_lang row.
        // Falls back to universal (value_lang IS NULL) for both, so sort
        // is Alpha < Beta → u2, u1, u3 (u3 last for NULL sort key).
        let lang_de = make_lang("de");
        let ids = list_resource_ids(&conn, "Thing", &[], &sorts_asc, 100, 0, &lang_de).expect("query should succeed");
        assert_eq!(
            ids,
            vec!["u2", "u1", "u3"],
            "universal-lang fallback: Alpha < Beta → u2, u1, u3. Got: {ids:?}"
        );
    }

    // list_resource_ids — multi-sort (two sort clauses simultaneously)

    /// Guard the param-binding-order invariant for the multi-sort path.
    ///
    /// The SQL generated by `list_resource_ids` for N sort clauses places all
    /// lang params (CASE WHEN value_lang = ?) in the SELECT before all join-name
    /// params (AND name = ?) in the FROM clause. A regression here would bind the
    /// wrong string to each `?`, corrupting the sort key entirely.
    ///
    /// Fixture: three resources with a primary key (`rank`, integer) and a
    /// secondary key (`name`, text). The primary key ties for two of them, so the
    /// secondary sort is the tiebreak. The expected order is determined by:
    ///   1. rank ASC (primary): tied ranks sort by secondary.
    ///   2. name ASC (secondary): alphabetical within the tie group.
    ///
    /// Concretely:
    ///   - ms1: rank=1, name="Charlie" → rank=1 group, C comes after A
    ///   - ms2: rank=1, name="Alice"   → rank=1 group, A comes first
    ///   - ms3: rank=2, name="Bob"     → rank=2, after both rank=1 resources
    ///
    /// Expected order: ms2 (rank=1, name=Alice), ms1 (rank=1, name=Charlie), ms3 (rank=2).
    #[test]
    fn list_resource_ids_multi_sort_two_clauses_tiebreak() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "
            INSERT INTO resources (id, class) VALUES
                ('ms1', 'Item'),
                ('ms2', 'Item'),
                ('ms3', 'Item');

            -- ms1: rank=1 (ties with ms2), name='Charlie'
            INSERT INTO properties (resource_id, name, seq, value_type, value_int) VALUES
                ('ms1', 'rank', 0, 'integer', 1);
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('ms1', 'name', 0, 'text', 'Charlie', 'en');

            -- ms2: rank=1 (ties with ms1), name='Alice'
            INSERT INTO properties (resource_id, name, seq, value_type, value_int) VALUES
                ('ms2', 'rank', 0, 'integer', 1);
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('ms2', 'name', 0, 'text', 'Alice', 'en');

            -- ms3: rank=2 (no tie), name='Bob'
            INSERT INTO properties (resource_id, name, seq, value_type, value_int) VALUES
                ('ms3', 'rank', 0, 'integer', 2);
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('ms3', 'name', 0, 'text', 'Bob', 'en');
            ",
        )
        .expect("multi-sort fixture should load");

        let lang = make_lang("en");
        // Two sort clauses: rank ASC (primary), name ASC (secondary tiebreak).
        let sorts = vec![
            Sort {
                key: SortKey::Property("rank".to_owned()),
                direction: SortDir::Asc,
            },
            Sort {
                key: SortKey::Property("name".to_owned()),
                direction: SortDir::Asc,
            },
        ];

        let ids =
            list_resource_ids(&conn, "Item", &[], &sorts, 100, 0, &lang).expect("multi-sort query should succeed");

        // ms2 (rank=1, Alice) before ms1 (rank=1, Charlie) before ms3 (rank=2, Bob).
        assert_eq!(
            ids,
            vec!["ms2", "ms1", "ms3"],
            "multi-sort: rank ASC then name ASC tiebreak must yield \
             ms2 (r=1,Alice) < ms1 (r=1,Charlie) < ms3 (r=2,Bob). Got: {ids:?}"
        );
    }

    // list_resource_ids — SortKey::Class / SortKey::Id (field sorts)
    //
    // Every `list_resource_ids` call is already filtered to a single class via
    // its `class: &str` parameter (see the `SortKey` doc comment: `Class`/`Id`
    // are request-time-only). So within one call `r.class` is constant across
    // the whole result set — a `SortKey::Class` term never discriminates by
    // itself. These tests still exercise the real value: the fixture
    // (`setup_list_db`) has resources of >=2 classes (Character + Location) in
    // the same table, so the tests lock that the generated SQL (a bare
    // `r.class`/`r.id` column reference, no join, no bound param, no
    // `(sort_val IS NULL)` term) compiles and binds correctly, and that the
    // stable `id` tiebreak still applies.

    #[test]
    fn list_resource_ids_sort_by_class_asc() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let sorts = vec![Sort { key: SortKey::Class, direction: SortDir::Asc }];
        let ids = list_resource_ids(&conn, "Character", &[], &sorts, 100, 0, &lang).expect("no error");
        // class is constant (all "Character") within this call's result set →
        // ordering reduces to the stable id tiebreak.
        assert_eq!(ids, vec!["c1", "c2", "c3", "c4"]);
    }

    #[test]
    fn list_resource_ids_sort_by_class_desc() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let sorts = vec![Sort { key: SortKey::Class, direction: SortDir::Desc }];
        let ids = list_resource_ids(&conn, "Character", &[], &sorts, 100, 0, &lang).expect("no error");
        // class is constant, so DESC on a tied term is a no-op; the id
        // tiebreak (always ASC) still yields ascending id order.
        assert_eq!(ids, vec!["c1", "c2", "c3", "c4"]);
    }

    #[test]
    fn list_resource_ids_sort_by_id_asc() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let sorts = vec![Sort { key: SortKey::Id, direction: SortDir::Asc }];
        let ids = list_resource_ids(&conn, "Character", &[], &sorts, 100, 0, &lang).expect("no error");
        assert_eq!(ids, vec!["c1", "c2", "c3", "c4"]);
    }

    #[test]
    fn list_resource_ids_sort_by_id_desc() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let sorts = vec![Sort { key: SortKey::Id, direction: SortDir::Desc }];
        let ids = list_resource_ids(&conn, "Character", &[], &sorts, 100, 0, &lang).expect("no error");
        assert_eq!(ids, vec!["c4", "c3", "c2", "c1"]);
    }

    // list_resource_ids — mixed field + property sort (interleaving)

    /// Locks the param-binding-order invariant when a `SortKey::Class`/`Id`
    /// (zero placeholders) is mixed with a `SortKey::Property` (lang + name
    /// placeholders) in the same call. A regression that miscounts field-sort
    /// contributions would shift the lang/name params onto the wrong `?`,
    /// corrupting the property sort that follows.
    #[test]
    fn list_resource_ids_mixed_class_then_property_sort() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let sorts = vec![
            Sort { key: SortKey::Class, direction: SortDir::Asc },
            Sort {
                key: SortKey::Property("name".to_owned()),
                direction: SortDir::Asc,
            },
        ];
        let ids = list_resource_ids(&conn, "Character", &[], &sorts, 100, 0, &lang).expect("no error");
        // class ties (all "Character") → the name sort (EN: Alice/Bob/Zara)
        // decides; c4 (no name) sorts last, same as a name-only sort.
        assert_eq!(
            ids,
            vec!["c1", "c2", "c3", "c4"],
            "mixed [Class, Property(name)] sort must bind params correctly and \
             fall through to the name tiebreak. Got: {ids:?}"
        );
    }

    /// Same invariant with the field/property order reversed — the property
    /// sort's lang/name params must still land correctly even when a
    /// zero-placeholder field sort follows it (not just precedes it).
    #[test]
    fn list_resource_ids_mixed_property_then_id_sort() {
        let conn = setup_list_db();
        let lang = make_lang("en");
        let sorts = vec![
            Sort {
                key: SortKey::Property("name".to_owned()),
                direction: SortDir::Asc,
            },
            Sort { key: SortKey::Id, direction: SortDir::Desc },
        ];
        let ids = list_resource_ids(&conn, "Character", &[], &sorts, 100, 0, &lang).expect("no error");
        // name sort (EN: Alice/Bob/Zara) decides; c4 (no name) sorts last.
        // The trailing Id-Desc term never matters here (name has no ties),
        // but must not corrupt the property sort's param binding.
        assert_eq!(
            ids,
            vec!["c1", "c2", "c3", "c4"],
            "mixed [Property(name), Id] sort must bind params correctly. Got: {ids:?}"
        );
    }

    // compound_has_parts

    /// Minimal compound fixture: a Story parent with three Chapter children,
    /// each at a distinct seq. Shared by `compound_has_parts` and
    /// `compound_part_headers` tests below.
    fn setup_compound_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "
            INSERT INTO resources (id, class) VALUES
                ('ST_001', 'Story'),
                ('SCH_001', 'StoryChapter'),
                ('SCH_002', 'StoryChapter'),
                ('SCH_003', 'StoryChapter');

            -- Three members, seq 0, 1, 2 — distinct, exercises basic ordering.
            INSERT INTO compound_parts (parent_id, child_id, seq) VALUES
                ('ST_001', 'SCH_001', 0),
                ('ST_001', 'SCH_002', 1),
                ('ST_001', 'SCH_003', 2);
            ",
        )
        .expect("compound fixture should load");
        conn
    }

    #[test]
    fn compound_has_parts_true_for_parent_with_rows() {
        let conn = setup_compound_db();
        assert!(
            compound_has_parts(&conn, "ST_001").expect("no error"),
            "ST_001 has compound_parts rows — should return true"
        );
    }

    #[test]
    fn compound_has_parts_false_for_parent_with_no_rows() {
        let conn = setup_compound_db();
        // SCH_001 exists as a resource but has no children.
        assert!(
            !compound_has_parts(&conn, "SCH_001").expect("no error"),
            "SCH_001 has no compound_parts rows — should return false"
        );
    }

    #[test]
    fn compound_member_classes_literal_parent_and_all_parents() {
        let conn = setup_compound_db();
        // A second compound with members of a different class, so the
        // literal-parent and all-parents arms give different answers.
        conn.execute_batch(
            "
            INSERT INTO resources (id, class) VALUES ('BK_001', 'Book'), ('PG_001', 'Page');
            INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('BK_001', 'PG_001', 0);
            ",
        )
        .expect("extra compound fixture should load");

        assert_eq!(
            compound_member_classes(&conn, Some("ST_001")).expect("no error"),
            vec!["StoryChapter".to_owned()],
            "a literal parent yields only its own children's classes"
        );
        assert_eq!(
            compound_member_classes(&conn, None).expect("no error"),
            vec!["Page".to_owned(), "StoryChapter".to_owned()],
            "None yields every compound child's class, sorted and distinct"
        );
        assert!(
            compound_member_classes(&conn, Some("SCH_001")).expect("no error").is_empty(),
            "a parent with no compound_parts rows yields no classes"
        );
    }

    #[test]
    fn compound_has_parts_false_for_nonexistent_parent() {
        let conn = setup_compound_db();
        assert!(
            !compound_has_parts(&conn, "NO_SUCH_ID").expect("no error"),
            "nonexistent parent should return false (not an error)"
        );
    }

    // compound_part_headers

    /// `setup_compound_db` + a `name` property (EN) on each of the three
    /// members, plus a multi-valued (EN+DE) `name` on SCH_002 to exercise the
    /// no-row-duplication guarantee, and NO `name` at all on SCH_003 to
    /// exercise the id-fallback.
    fn setup_compound_headers_db() -> Connection {
        let conn = setup_compound_db();
        conn.execute_batch(
            "
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('SCH_001', 'name', 0, 'text', 'Down the Rabbit Hole', 'en'),
                ('SCH_002', 'name', 0, 'text', 'The Pool of Tears',     'en'),
                ('SCH_002', 'name', 1, 'text', 'Das Tränenmeer',       'de');
            -- SCH_003 intentionally has no 'name' property row at all.
            ",
        )
        .expect("headers fixture should load");
        conn
    }

    #[test]
    fn compound_part_headers_returns_members_in_seq_order() {
        let conn = setup_compound_headers_db();
        let lang = make_lang("en");
        let headers = compound_part_headers(&conn, "ST_001", &lang).expect("should not error");
        let ids: Vec<&str> = headers.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["SCH_001", "SCH_002", "SCH_003"],
            "headers must be returned in compound_parts seq order, got: {headers:?}"
        );
    }

    #[test]
    fn compound_part_headers_resolves_class_and_lang_aware_name() {
        let conn = setup_compound_headers_db();
        let lang_en = make_lang("en");
        let headers_en = compound_part_headers(&conn, "ST_001", &lang_en).expect("should not error");
        assert_eq!(headers_en[0].class, "StoryChapter");
        assert_eq!(headers_en[0].name, "Down the Rabbit Hole");
        assert_eq!(
            headers_en[1].name, "The Pool of Tears",
            "requested lang (en) must resolve over de: {:?}",
            headers_en[1]
        );

        let lang_de = make_lang_for("de", &["en", "de"]);
        let headers_de = compound_part_headers(&conn, "ST_001", &lang_de).expect("should not error");
        assert_eq!(
            headers_de[1].name, "Das Tränenmeer",
            "requested lang (de) must resolve when present: {:?}",
            headers_de[1]
        );
    }

    #[test]
    fn compound_part_headers_no_row_duplication_for_multi_valued_name() {
        let conn = setup_compound_headers_db();
        let lang = make_lang("en");
        let headers = compound_part_headers(&conn, "ST_001", &lang).expect("should not error");
        assert_eq!(
            headers.len(),
            3,
            "SCH_002's multi-valued (en+de) name must not multiply its row, got: {headers:?}"
        );
    }

    #[test]
    fn compound_part_headers_id_fallback_when_no_name_value() {
        let conn = setup_compound_headers_db();
        let lang = make_lang("en");
        let headers = compound_part_headers(&conn, "ST_001", &lang).expect("should not error");
        let sch_003 = headers.iter().find(|h| h.id == "SCH_003").expect("SCH_003 must be present");
        assert_eq!(
            sch_003.name, "SCH_003",
            "member with no 'name' property must fall back to its id, got: {sch_003:?}"
        );
    }

    #[test]
    fn compound_part_headers_empty_parent_returns_empty_vec() {
        let conn = setup_compound_headers_db();
        let lang = make_lang("en");
        let headers = compound_part_headers(&conn, "SCH_001", &lang).expect("should not error");
        assert!(headers.is_empty(), "parent with no compound_parts rows should return []");
    }

    // incoming_links
    //
    // Fixture: target strip T1 (and T2), compounds B1/B2 (class Book), Pages:
    // - P_a: B1 seq 2, name "zeta", strip-left → T1
    // - P_b: B1 seq 1, name "alpha", strip-left AND strip-right → T1
    // - P_c: B2 seq 0, name "alpha" (name tie with P_b), strip-left → T1
    // - P_d: no compound, NO name, strip-left → T1
    // - P_e: B2 seq 5 AND B1 seq 9 (multi-parent — first parent is B1, with B1's seq 9; independent
    //   MIN()s would wrongly pair B1 with 5), name "beta", strip-right → T1
    // - P_f: B2 seq 3, multi-valued name en "gamma" / de "Gamma DE", strip-left → T1
    // - P_g: list_value row named strip-left with target_id T1 (non-link)
    // - P_h: links T1 via other-prop (not in the property list)
    // - P_i: strip-left → T2 only
    // - N1 (class Note): strip-left → T1 (wrong class)

    fn setup_incoming_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "
            INSERT INTO resources (id, class) VALUES
                ('T1', 'BorderStrip'), ('T2', 'BorderStrip'),
                ('B1', 'Book'), ('B2', 'Book'),
                ('P_a', 'Page'), ('P_b', 'Page'), ('P_c', 'Page'), ('P_d', 'Page'),
                ('P_e', 'Page'), ('P_f', 'Page'), ('P_g', 'Page'), ('P_h', 'Page'),
                ('P_i', 'Page'), ('N1', 'Note');

            INSERT INTO compound_parts (parent_id, child_id, seq) VALUES
                ('B1', 'P_a', 2), ('B1', 'P_b', 1), ('B1', 'P_e', 9),
                ('B2', 'P_c', 0), ('B2', 'P_e', 5), ('B2', 'P_f', 3);

            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('P_a', 'name', 0, 'text', 'zeta',     NULL),
                ('P_b', 'name', 0, 'text', 'alpha',    NULL),
                ('P_c', 'name', 0, 'text', 'alpha',    NULL),
                ('P_e', 'name', 0, 'text', 'beta',     NULL),
                ('P_f', 'name', 0, 'text', 'gamma',    'en'),
                ('P_f', 'name', 1, 'text', 'Gamma DE', 'de'),
                ('N1',  'name', 0, 'text', 'a note',   NULL);

            INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
                ('P_a', 'strip-left',  0, 'link', 'T1'),
                ('P_b', 'strip-left',  0, 'link', 'T1'),
                ('P_b', 'strip-right', 0, 'link', 'T1'),
                ('P_c', 'strip-left',  0, 'link', 'T1'),
                ('P_d', 'strip-left',  0, 'link', 'T1'),
                ('P_e', 'strip-right', 0, 'link', 'T1'),
                ('P_f', 'strip-left',  0, 'link', 'T1'),
                ('P_g', 'strip-left',  0, 'list_value', 'T1'),
                ('P_h', 'other-prop',  0, 'link', 'T1'),
                ('P_i', 'strip-left',  0, 'link', 'T2'),
                ('N1',  'strip-left',  0, 'link', 'T1');
            ",
        )
        .expect("incoming fixture should load");
        conn
    }

    fn strip_props() -> Vec<String> {
        vec!["strip-left".to_owned(), "strip-right".to_owned()]
    }

    /// An ungrouped spec with none of the row options — the Incunabula
    /// `/strips/:id` shape.
    fn flat_spec(class: &str, properties: &[String], order: IncomingOrder, limit: u32) -> IncomingLinksSpec {
        IncomingLinksSpec {
            label: "caption".to_owned(),
            class: class.to_owned(),
            properties: properties.to_vec(),
            order,
            limit,
            grouping: None,
            row_label: None,
            row_aside: None,
            sort_property: None,
        }
    }

    /// A positional call shape for the flat-list tests; builds a
    /// [`flat_spec`] and runs the real `incoming_links`.
    fn incoming_links_flat(
        conn: &Connection,
        target_id: &str,
        class: &str,
        properties: &[String],
        order: IncomingOrder,
        lang: &Lang,
        limit: u32,
    ) -> Result<IncomingLinks, super::QueryError> {
        incoming_links(conn, &flat_spec(class, properties, order, limit), target_id, lang)
    }

    #[test]
    fn linking_resource_lacks_property_scopes_to_linking_resources() {
        let conn = setup_incoming_db();
        // P_d links through `strip-left` and has no `name`.
        assert!(linking_resource_lacks_property(&conn, "Page", &strip_props(), "name").expect("no error"));
        // Only P_b and P_e link through `strip-right`; both carry a `name`
        // (the name-less P_g/P_h link through other properties/types only).
        assert!(
            !linking_resource_lacks_property(&conn, "Page", &["strip-right".to_owned()], "name").expect("no error")
        );
        assert!(!linking_resource_lacks_property(&conn, "Page", &[], "name").expect("no error"));
    }

    /// `(id, via)` pairs, the shape every ordering assertion compares.
    fn id_via(links: &[IncomingLink]) -> Vec<(&str, &str)> {
        links.iter().map(|l| (l.id.as_str(), l.via.as_str())).collect()
    }

    #[test]
    fn incoming_links_one_row_per_source_and_property_with_via() {
        let conn = setup_incoming_db();
        let links = incoming_links_flat(
            &conn,
            "T1",
            "Page",
            &strip_props(),
            IncomingOrder::Id,
            &make_lang("en"),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("should not error")
        .rows;
        let p_b: Vec<&str> = links.iter().filter(|l| l.id == "P_b").map(|l| l.via.as_str()).collect();
        assert_eq!(
            p_b,
            vec!["strip-left", "strip-right"],
            "a source linking through two properties yields one row per property: {links:?}"
        );
        assert_eq!(links.len(), 7, "P_a..P_f (P_b twice), got: {links:?}");
    }

    #[test]
    fn incoming_links_zero_incoming_rows_is_empty_not_error() {
        let conn = setup_incoming_db();
        let result = incoming_links_flat(
            &conn,
            "B1",
            "Page",
            &strip_props(),
            IncomingOrder::Name,
            &make_lang("en"),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("should not error");
        assert!(result.rows.is_empty(), "got: {:?}", result.rows);
        assert_eq!(result.total, 0);
    }

    #[test]
    fn incoming_links_empty_properties_is_empty_without_querying() {
        let conn = setup_incoming_db();
        let result = incoming_links_flat(
            &conn,
            "T1",
            "Page",
            &[],
            IncomingOrder::Name,
            &make_lang("en"),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("an empty property list must not build `IN ()`");
        assert!(result.rows.is_empty(), "got: {:?}", result.rows);
        assert_eq!(result.total, 0);
    }

    #[test]
    fn incoming_links_compound_seq_groups_by_parent_non_members_last() {
        let conn = setup_incoming_db();
        let links = incoming_links_flat(
            &conn,
            "T1",
            "Page",
            &strip_props(),
            IncomingOrder::CompoundSeq,
            &make_lang("en"),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("should not error")
        .rows;
        assert_eq!(
            id_via(&links),
            vec![
                ("P_b", "strip-left"),
                ("P_b", "strip-right"),
                ("P_a", "strip-left"),
                ("P_e", "strip-right"),
                ("P_c", "strip-left"),
                ("P_f", "strip-left"),
                ("P_d", "strip-left"),
            ],
            "B1 members by seq, then B2 members by seq (no interleaving), then non-members"
        );
    }

    #[test]
    fn incoming_links_name_order_with_id_tie_break() {
        let conn = setup_incoming_db();
        let links = incoming_links_flat(
            &conn,
            "T1",
            "Page",
            &strip_props(),
            IncomingOrder::Name,
            &make_lang("en"),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("should not error")
        .rows;
        // NOCASE: the id-fallback "P_d" files as "p_d", between "gamma" and
        // "zeta", not first as its uppercase initial would under BINARY.
        assert_eq!(
            id_via(&links),
            vec![
                ("P_b", "strip-left"),
                ("P_b", "strip-right"),
                ("P_c", "strip-left"),
                ("P_e", "strip-right"),
                ("P_f", "strip-left"),
                ("P_d", "strip-left"),
                ("P_a", "strip-left"),
            ]
        );
    }

    #[test]
    fn incoming_links_id_order() {
        let conn = setup_incoming_db();
        let links = incoming_links_flat(
            &conn,
            "T1",
            "Page",
            &strip_props(),
            IncomingOrder::Id,
            &make_lang("en"),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("should not error")
        .rows;
        assert_eq!(
            id_via(&links),
            vec![
                ("P_a", "strip-left"),
                ("P_b", "strip-left"),
                ("P_b", "strip-right"),
                ("P_c", "strip-left"),
                ("P_d", "strip-left"),
                ("P_e", "strip-right"),
                ("P_f", "strip-left"),
            ]
        );
    }

    #[test]
    fn incoming_links_multi_parent_source_takes_first_parent_with_its_own_seq() {
        let conn = setup_incoming_db();
        let links = incoming_links_flat(
            &conn,
            "T1",
            "Page",
            &strip_props(),
            IncomingOrder::CompoundSeq,
            &make_lang("en"),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("should not error")
        .rows;
        let p_e = links.iter().find(|l| l.id == "P_e").expect("P_e present");
        assert_eq!(
            p_e.parent,
            Some(MemberParent { id: "B1".to_owned(), class: "Book".to_owned(), seq: 9 }),
            "lexicographically-first parent B1 with B1's seq (9), never B2's 5"
        );
    }

    #[test]
    fn incoming_links_parent_some_for_member_none_otherwise() {
        let conn = setup_incoming_db();
        let links = incoming_links_flat(
            &conn,
            "T1",
            "Page",
            &strip_props(),
            IncomingOrder::Id,
            &make_lang("en"),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("should not error")
        .rows;
        let p_c = links.iter().find(|l| l.id == "P_c").expect("P_c present");
        assert_eq!(
            p_c.parent,
            Some(MemberParent { id: "B2".to_owned(), class: "Book".to_owned(), seq: 0 })
        );
        let p_d = links.iter().find(|l| l.id == "P_d").expect("P_d present");
        assert_eq!(p_d.parent, None, "non-member has no parent: {p_d:?}");
        assert_eq!(p_d.class, "Page");
    }

    #[test]
    fn incoming_links_name_falls_back_to_id_and_multi_valued_name_is_one_row() {
        let conn = setup_incoming_db();
        let links = incoming_links_flat(
            &conn,
            "T1",
            "Page",
            &strip_props(),
            IncomingOrder::Id,
            &make_lang_for("de", &["en", "de"]),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("should not error")
        .rows;
        let p_d = links.iter().find(|l| l.id == "P_d").expect("P_d present");
        assert_eq!(p_d.name, "P_d", "no name value → id");
        let p_f: Vec<&IncomingLink> = links.iter().filter(|l| l.id == "P_f").collect();
        assert_eq!(p_f.len(), 1, "multi-valued name must not multiply rows");
        assert_eq!(p_f[0].name, "Gamma DE", "requested lang (de) resolves");
    }

    #[test]
    fn incoming_links_excludes_non_link_other_property_other_target_other_class() {
        let conn = setup_incoming_db();
        let links = incoming_links_flat(
            &conn,
            "T1",
            "Page",
            &strip_props(),
            IncomingOrder::Id,
            &make_lang("en"),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("should not error")
        .rows;
        for excluded in ["P_g", "P_h", "P_i", "N1"] {
            assert!(
                links.iter().all(|l| l.id != excluded),
                "{excluded} must be excluded, got: {links:?}"
            );
        }
        // The class filter is what excludes N1: asking for Note finds it.
        let notes = incoming_links_flat(
            &conn,
            "T1",
            "Note",
            &strip_props(),
            IncomingOrder::Id,
            &make_lang("en"),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("should not error")
        .rows;
        assert_eq!(id_via(&notes), vec![("N1", "strip-left")]);
    }

    #[test]
    fn incoming_links_limit_caps_rows_and_reports_total() {
        let conn = setup_incoming_db();
        // Unlimited baseline: T1's full match count in Id order.
        let unlimited = incoming_links_flat(
            &conn,
            "T1",
            "Page",
            &strip_props(),
            IncomingOrder::Id,
            &make_lang("en"),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("should not error");
        assert_eq!(unlimited.total, 7, "got: {:?}", unlimited.rows);

        let capped = incoming_links_flat(&conn, "T1", "Page", &strip_props(), IncomingOrder::Id, &make_lang("en"), 2)
            .expect("should not error");
        assert_eq!(capped.rows.len(), 2, "got: {:?}", capped.rows);
        assert_eq!(capped.total, 7);
        assert_eq!(
            id_via(&capped.rows),
            id_via(&unlimited.rows[..2]),
            "the capped rows must equal the first two of the unlimited ordering"
        );
    }

    #[test]
    fn incoming_links_limit_above_count_returns_all() {
        let conn = setup_incoming_db();
        let result = incoming_links_flat(&conn, "T1", "Page", &strip_props(), IncomingOrder::Id, &make_lang("en"), 50)
            .expect("should not error");
        assert_eq!(result.rows.len(), result.total, "got: {:?}", result.rows);
    }

    #[test]
    fn incoming_links_zero_rows_total_is_zero() {
        let conn = setup_incoming_db();
        let result = incoming_links_flat(
            &conn,
            "B1",
            "Page",
            &strip_props(),
            IncomingOrder::Name,
            &make_lang("en"),
            DEFAULT_INCOMING_LINKS_LIMIT,
        )
        .expect("should not error");
        assert!(result.rows.is_empty());
        assert_eq!(result.total, 0);
    }

    /// The plan of one `incoming_links` statement, one detail line per row.
    fn incoming_plan(conn: &Connection, spec: &IncomingLinksSpec) -> String {
        let stmt = incoming_links_statement(spec, "T1", &make_lang("en"));
        let sql = format!("EXPLAIN QUERY PLAN {}", stmt.sql);
        let mut prepared = conn.prepare(&sql).expect("EXPLAIN QUERY PLAN prepares");
        let plan: Vec<String> = prepared
            .query_map(rusqlite::params_from_iter(&stmt.params), |row| row.get::<_, String>(3))
            .expect("plan rows read")
            .collect::<Result<Vec<_>, _>>()
            .expect("plan rows are text");
        plan.join("\n")
    }

    /// Pins the plan: the link rows are seeked on `idx_properties_target` by
    /// `target_id` (the unary `+` keeps `idx_properties_name*` and
    /// `idx_resources_class` from driving), and `properties` is never scanned
    /// — neither in the flat shape nor in the grouped shape, whose optional
    /// scalar subqueries (`p`, `ln`, `lnl`) must each be a primary-key seek.
    /// Mirrors `target_in_where_uses_composite_target_index`.
    #[test]
    fn incoming_links_seeks_composite_target_index() {
        let conn = setup_incoming_db();
        let mut specs: Vec<IncomingLinksSpec> = [IncomingOrder::Name, IncomingOrder::Id, IncomingOrder::CompoundSeq]
            .into_iter()
            .map(|order| flat_spec("Page", &strip_props(), order, DEFAULT_INCOMING_LINKS_LIMIT))
            .collect();
        let mut grouped = flat_spec("Page", &strip_props(), IncomingOrder::Name, GROUPED_INCOMING_LIMIT_CEILING);
        grouped.grouping = Some(IncomingGrouping {
            by: IncomingGroupBy::Property("decade".to_owned()),
            layout: IncomingLayout::Cards,
            note_label: None,
        });
        grouped.row_label = Some("date".to_owned());
        grouped.row_aside = Some("source".to_owned());
        grouped.sort_property = Some("sort-key".to_owned());
        specs.push(grouped);

        for spec in &specs {
            let plan_text = incoming_plan(&conn, spec);
            let shape = (&spec.order, spec.grouping.is_some());
            assert!(
                plan_text.contains("SEARCH l USING INDEX idx_properties_target (target_id=?)"),
                "{shape:?}: the link rows must be seeked on idx_properties_target. \
                 Plan was:\n{plan_text}"
            );
            for scanned in ["SCAN l", "SCAN n", "SCAN p", "SCAN ln", "SCAN lnl"] {
                assert!(
                    !plan_text.contains(scanned),
                    "{shape:?}: `{scanned}` — properties/list tables must never be \
                     scanned. Plan was:\n{plan_text}"
                );
            }
        }
    }

    /// The Incunabula `/strips/:id` shape (ungrouped, no row options) must
    /// run LITERALLY the base statement: every optional lookup is emitted only
    /// when its spec field is `Some`.
    #[test]
    fn incoming_links_ungrouped_sql_is_unchanged() {
        const PRE_G3_NAME_ORDER: &str = "SELECT s.id, s.class, s.via, s.name, s.parent_id, \
            pr.class, cp.seq, COUNT(*) OVER () AS total FROM ( SELECT l.resource_id AS id, \
            r.class AS class, l.name AS via, COALESCE(MIN(CASE WHEN n.value_lang = ? THEN \
            n.value_text END), MIN(CASE WHEN n.value_lang IS NULL THEN n.value_text END), \
            MIN(n.value_text)) AS name, (SELECT cpf.parent_id FROM compound_parts cpf WHERE \
            cpf.child_id = l.resource_id ORDER BY cpf.parent_id LIMIT 1) AS parent_id FROM \
            properties l JOIN resources r ON r.id = l.resource_id LEFT JOIN properties n ON \
            n.resource_id = l.resource_id AND n.name = 'name' WHERE l.target_id = ? AND \
            l.value_type = 'link' AND +l.name IN (?, ?) AND +r.class = ? GROUP BY \
            l.resource_id, l.name, r.class ) s LEFT JOIN compound_parts cp ON cp.parent_id = \
            s.parent_id AND cp.child_id = s.id LEFT JOIN resources pr ON pr.id = s.parent_id \
            ORDER BY COALESCE(s.name, s.id) COLLATE NOCASE, s.id, s.via LIMIT ?";
        let lang = make_lang("en");
        let name_order = "COALESCE(s.name, s.id) COLLATE NOCASE, s.id, s.via";
        for (order, clause) in [
            (IncomingOrder::Name, name_order),
            (IncomingOrder::Id, "s.id, s.via"),
            (
                IncomingOrder::CompoundSeq,
                "(s.parent_id IS NULL), s.parent_id, cp.seq, s.id, s.via",
            ),
        ] {
            let spec = flat_spec("Page", &strip_props(), order, DEFAULT_INCOMING_LINKS_LIMIT);
            let stmt = incoming_links_statement(&spec, "T1", &lang);
            assert_eq!(stmt.sql, PRE_G3_NAME_ORDER.replace(name_order, clause), "{order:?}");
            assert_eq!(stmt.params.len(), 6, "lang, target, 2 names, class, limit");
        }
        // `group-by="via"` needs no SQL key either: still the same string.
        let mut via = flat_spec("Page", &strip_props(), IncomingOrder::Name, DEFAULT_INCOMING_LINKS_LIMIT);
        via.grouping = Some(IncomingGrouping {
            by: IncomingGroupBy::Via,
            layout: IncomingLayout::Columns,
            note_label: None,
        });
        assert_eq!(incoming_links_statement(&via, "T1", &lang).sql, PRE_G3_NAME_ORDER);
    }

    // incoming_links grouped rows
    //
    // Fixture: target Work W1; Performances link to it via `work`:
    // - Q1: decade D1840 (seq 1), genre "Oper", date "12.3.1841", sort-key "1841-03-12", source
    //   S_IB (de "IB", en "IB en"), note de "Notiz" / en "note"
    // - Q2: decade D1830 (seq 0), genre "oper" (case variant of Q1's), date "1.1.1835", sort-key
    //   "1835-01-01", source S_TGB (label en ONLY), note universal (NULL lang) "universal"
    // - Q3: decade multi-valued — seq 0 D1840, seq 1 D1830 (first value wins); genre "Ballett";
    //   sort-key "a-lower" (NOCASE check vs "B-UPPER"); note fr ONLY "seulement fr"
    // - Q4: no decade, no genre, no date, no sort-key, no source, no note
    // - Q5: decade D1830, sort-key "B-UPPER", no name (id fallback)
    // D1830 labels de "1830er" / en "1830s"; D1840 label de "1840er" only.

    fn setup_grouped_incoming_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "
            INSERT INTO resources (id, class) VALUES
                ('W1', 'Work'),
                ('Q1', 'Performance'), ('Q2', 'Performance'), ('Q3', 'Performance'),
                ('Q4', 'Performance'), ('Q5', 'Performance');

            INSERT INTO lists (id, name) VALUES ('decade', 'decade'), ('source', 'source');
            INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES
                ('D1830', 'decade', NULL, 0), ('D1840', 'decade', NULL, 1),
                ('S_IB', 'source', NULL, 0), ('S_TGB', 'source', NULL, 1);
            INSERT INTO list_node_labels (node_id, lang, label) VALUES
                ('D1830', 'de', '1830er'), ('D1830', 'en', '1830s'),
                ('D1840', 'de', '1840er'),
                ('S_IB', 'de', 'IB'), ('S_IB', 'en', 'IB en'),
                ('S_TGB', 'en', 'Tgb en');

            INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
                ('Q1', 'work', 0, 'link', 'W1'),
                ('Q2', 'work', 0, 'link', 'W1'),
                ('Q3', 'work', 0, 'link', 'W1'),
                ('Q4', 'work', 0, 'link', 'W1'),
                ('Q5', 'work', 0, 'link', 'W1'),
                ('Q1', 'decade', 0, 'list_value', 'D1840'),
                ('Q2', 'decade', 0, 'list_value', 'D1830'),
                ('Q3', 'decade', 0, 'list_value', 'D1840'),
                ('Q3', 'decade', 1, 'list_value', 'D1830'),
                ('Q5', 'decade', 0, 'list_value', 'D1830'),
                ('Q1', 'source', 0, 'list_value', 'S_IB'),
                ('Q2', 'source', 0, 'list_value', 'S_TGB');

            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('Q1', 'name', 0, 'text', 'q one', NULL),
                ('Q2', 'name', 0, 'text', 'q two', NULL),
                ('Q3', 'name', 0, 'text', 'q three', NULL),
                ('Q4', 'name', 0, 'text', 'q four', NULL),
                ('Q1', 'genre', 0, 'text', 'Oper', NULL),
                ('Q2', 'genre', 0, 'text', 'oper', NULL),
                ('Q3', 'genre', 0, 'text', 'Ballett', NULL),
                ('Q1', 'date', 0, 'text', '12.3.1841', NULL),
                ('Q2', 'date', 0, 'text', '1.1.1835', NULL),
                ('Q1', 'sort-key', 0, 'text', '1841-03-12', NULL),
                ('Q2', 'sort-key', 0, 'text', '1835-01-01', NULL),
                ('Q3', 'sort-key', 0, 'text', 'a-lower', NULL),
                ('Q5', 'sort-key', 0, 'text', 'B-UPPER', NULL),
                ('Q1', 'note', 0, 'text', 'Notiz', 'de'),
                ('Q1', 'note', 1, 'text', 'note', 'en'),
                ('Q2', 'note', 0, 'text', 'universal', NULL),
                ('Q3', 'note', 0, 'text', 'seulement fr', 'fr');
            ",
        )
        .expect("grouped incoming fixture should load");
        conn
    }

    /// A spec over the grouped fixture, `order="id"`, grouped by `by`.
    fn work_spec(by: Option<IncomingGroupBy>) -> IncomingLinksSpec {
        let mut spec = flat_spec(
            "Performance",
            &["work".to_owned()],
            IncomingOrder::Id,
            GROUPED_INCOMING_LIMIT_CEILING,
        );
        spec.grouping = by.map(|by| IncomingGrouping { by, layout: IncomingLayout::Cards, note_label: None });
        spec
    }

    fn run_work(spec: &IncomingLinksSpec, lang: &Lang) -> Vec<IncomingLink> {
        incoming_links(&setup_grouped_incoming_db(), spec, "W1", lang)
            .expect("should not error")
            .rows
    }

    fn row<'a>(links: &'a [IncomingLink], id: &str) -> &'a IncomingLink {
        links
            .iter()
            .find(|l| l.id == id)
            .unwrap_or_else(|| panic!("{id} present in {links:?}"))
    }

    #[test]
    fn incoming_links_list_valued_group_key() {
        let spec = work_spec(Some(IncomingGroupBy::Property("decade".to_owned())));
        let links = run_work(&spec, &make_lang_for("en", &["de", "en"]));
        assert_eq!(links.len(), 5, "one row per source: {links:?}");
        assert_eq!(
            row(&links, "Q2").group,
            Some(GroupKey::List { key: "D1830".to_owned(), label: "1830s".to_owned(), seq: 0 })
        );
        assert_eq!(
            row(&links, "Q1").group,
            Some(GroupKey::List { key: "D1840".to_owned(), label: "1840er".to_owned(), seq: 1 }),
            "no en label for D1840 → any label"
        );
    }

    #[test]
    fn incoming_links_text_valued_group_key_folds_case() {
        let spec = work_spec(Some(IncomingGroupBy::Property("genre".to_owned())));
        let links = run_work(&spec, &make_lang("en"));
        let oper = Some(GroupKey::Text { key: "Oper".to_owned() });
        assert_eq!(row(&links, "Q1").group, oper);
        assert_eq!(
            row(&links, "Q2").group,
            oper,
            "`oper` folds onto the first spelling met in row order (Q1's `Oper`)"
        );
        assert_eq!(row(&links, "Q3").group, Some(GroupKey::Text { key: "Ballett".to_owned() }));
    }

    #[test]
    fn incoming_links_missing_group_value_is_none_and_kept() {
        let spec = work_spec(Some(IncomingGroupBy::Property("decade".to_owned())));
        let links = run_work(&spec, &make_lang("en"));
        assert_eq!(
            row(&links, "Q4").group,
            None,
            "a source with no group value keeps its row, with no group"
        );
    }

    #[test]
    fn incoming_links_multi_valued_group_property_is_one_row_first_value() {
        let spec = work_spec(Some(IncomingGroupBy::Property("decade".to_owned())));
        let links = run_work(&spec, &make_lang("en"));
        let q3: Vec<&IncomingLink> = links.iter().filter(|l| l.id == "Q3").collect();
        assert_eq!(q3.len(), 1, "a multi-valued group property must not multiply rows");
        assert!(
            matches!(&q3[0].group, Some(GroupKey::List { key, .. }) if key == "D1840"),
            "the seq-0 value (D1840) wins, not the lower-list-seq D1830: {:?}",
            q3[0].group
        );
    }

    #[test]
    fn incoming_links_via_grouping_and_ungrouped_carry_no_group_key() {
        for spec in [work_spec(Some(IncomingGroupBy::Via)), work_spec(None)] {
            let links = run_work(&spec, &make_lang("en"));
            assert!(links.iter().all(|l| l.group.is_none()), "{links:?}");
            assert!(links.iter().all(|l| l.label.is_none() && l.aside.is_none()));
        }
    }

    #[test]
    fn incoming_links_row_label_falls_back_to_none() {
        let mut spec = work_spec(None);
        spec.row_label = Some("date".to_owned());
        let links = run_work(&spec, &make_lang("en"));
        assert_eq!(row(&links, "Q1").label.as_deref(), Some("12.3.1841"));
        assert_eq!(row(&links, "Q4").label, None, "no `date` → None (renderer uses name)");
        assert_eq!(row(&links, "Q4").name, "q four");
    }

    #[test]
    fn incoming_links_aside_text_language_fallback() {
        let mut spec = work_spec(None);
        spec.row_aside = Some("note".to_owned());
        let links = run_work(&spec, &make_lang_for("de", &["de", "en"]));
        assert_eq!(row(&links, "Q1").aside.as_deref(), Some("Notiz"), "requested lang");
        assert_eq!(row(&links, "Q2").aside.as_deref(), Some("universal"), "no de value → universal");
        assert_eq!(
            row(&links, "Q3").aside.as_deref(),
            Some("seulement fr"),
            "no de, no universal → any"
        );
        assert_eq!(row(&links, "Q4").aside, None);

        let links = run_work(&spec, &make_lang_for("en", &["de", "en"]));
        assert_eq!(row(&links, "Q1").aside.as_deref(), Some("note"));
    }

    #[test]
    fn incoming_links_aside_list_label_language_fallback() {
        let mut spec = work_spec(None);
        spec.row_aside = Some("source".to_owned());
        let links = run_work(&spec, &make_lang_for("de", &["de", "en"]));
        assert_eq!(row(&links, "Q1").aside.as_deref(), Some("IB"), "requested lang");
        assert_eq!(row(&links, "Q2").aside.as_deref(), Some("Tgb en"), "no de label → any label");
        assert_eq!(row(&links, "Q4").aside, None);
    }

    #[test]
    fn incoming_links_sort_property_nocase_missing_last_wins_over_order() {
        let mut spec = work_spec(None);
        spec.order = IncomingOrder::Name;
        spec.sort_property = Some("sort-key".to_owned());
        let links = run_work(&spec, &make_lang("en"));
        let ids: Vec<&str> = links.iter().map(|l| l.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["Q2", "Q1", "Q3", "Q5", "Q4"],
            "sort-key NOCASE (`a-lower` before `B-UPPER`), missing (Q4) last; \
             `order=name` is ignored"
        );
    }

    #[test]
    fn incoming_links_sort_property_ties_break_on_id() {
        let mut spec = work_spec(None);
        spec.sort_property = Some("date".to_owned());
        let links = run_work(&spec, &make_lang("en"));
        let ids: Vec<&str> = links.iter().map(|l| l.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["Q2", "Q1", "Q3", "Q4", "Q5"],
            "three missing dates sort last, among themselves by id"
        );
    }

    #[test]
    fn incoming_links_grouped_ceiling_is_bound_as_the_limit() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "
            INSERT INTO resources (id, class) VALUES ('W1', 'Work');
            INSERT INTO resources (id, class)
                WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < 60)
                SELECT printf('X%02d', i), 'Performance' FROM c;
            INSERT INTO properties (resource_id, name, seq, value_type, target_id)
                SELECT id, 'work', 0, 'link', 'W1' FROM resources WHERE class = 'Performance';
            ",
        )
        .expect("fan-in fixture should load");
        let lang = make_lang("en");

        // Grouped with no `limit=`: lowering resolved the ceiling, so all 60
        // rows (more than the ungrouped default of 50) come back.
        let grouped = work_spec(Some(IncomingGroupBy::Via));
        assert_eq!(grouped.limit, GROUPED_INCOMING_LIMIT_CEILING);
        let result = incoming_links(&conn, &grouped, "W1", &lang).expect("should not error");
        assert_eq!(result.rows.len(), 60);
        assert_eq!(result.total, 60);

        // An explicit `limit=50` on a grouped list is still honoured.
        let mut capped = grouped.clone();
        capped.limit = DEFAULT_INCOMING_LINKS_LIMIT;
        let result = incoming_links(&conn, &capped, "W1", &lang).expect("should not error");
        assert_eq!(result.rows.len(), 50);
        assert_eq!(result.total, 60, "the total stays uncapped");
    }

    // media_item_ids / media_thumb_uris
    //
    // Fixture: CH_001 (a Character) → three Image targets (I_001, I_002,
    // I_003) via the `image` link property, seq 2, 0, 1 (deliberately out of
    // id order so the ORDER BY seq assertion is meaningful). I_001 and I_002
    // each have one `kind='image'` Representation; I_003 has none (exercises
    // the "no image" not-an-error path for media_thumb_uris). CH_002 has no
    // `image` property rows at all (exercises the empty-Vec / absent-from-map
    // paths).

    fn setup_media_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "
            INSERT INTO resources (id, class) VALUES
                ('CH_001', 'Character'),
                ('CH_002', 'Character'),
                ('I_001', 'Image'),
                ('I_002', 'Image'),
                ('I_003', 'Image');

            -- Seq deliberately out of id order: 2, 0, 1.
            INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
                ('CH_001', 'image', 2, 'link', 'I_001'),
                ('CH_001', 'image', 0, 'link', 'I_002'),
                ('CH_001', 'image', 1, 'link', 'I_003');

            INSERT INTO representations (resource_id, kind, uri, width, height, seq) VALUES
                ('I_001', 'image', 'https://iiif.example.org/i001/full/max/0/default.jpg', NULL, NULL, 0),
                ('I_002', 'image', 'https://iiif.example.org/i002/full/max/0/default.jpg', 320, 480, 0);
            -- I_003 intentionally has NO representations row.
            ",
        )
        .expect("media fixture should load");
        conn
    }

    #[test]
    fn media_item_ids_orders_by_seqnum() {
        let conn = setup_media_db();
        let ids = media_item_ids(&conn, "CH_001", "image").expect("should not error");
        assert_eq!(
            ids,
            vec!["I_002", "I_003", "I_001"],
            "must be ordered by properties.seq (0, 1, 2), not id or insertion order, got: {ids:?}"
        );
    }

    #[test]
    fn media_item_ids_empty_for_resource_with_no_media_property() {
        let conn = setup_media_db();
        let ids = media_item_ids(&conn, "CH_002", "image").expect("should not error");
        assert!(
            ids.is_empty(),
            "a resource with no 'image' property rows must return [], got: {ids:?}"
        );
    }

    #[test]
    fn media_thumb_uris_batches_multiple_ids_in_one_query() {
        let conn = setup_media_db();
        // Both ids are passed in a single call — the SQL shape (one prepared
        // statement, an IN (...) list) means this never issues a query per id.
        let ids = vec!["CH_001".to_owned(), "CH_002".to_owned()];
        let out = media_thumb_uris(&conn, "image", &ids).expect("should not error");

        // CH_001's first (lowest-seq) media target is I_002 (seq 0), which
        // has a stored width of 320.
        let ch_001 = out.get("CH_001").expect("CH_001 should resolve a thumb");
        assert_eq!(
            ch_001.uri, "https://iiif.example.org/i002/full/max/0/default.jpg",
            "must resolve the lowest-seqnum media target's thumb URI, got: {out:?}"
        );
        assert_eq!(
            ch_001.width,
            Some(320),
            "must carry the target image's stored width, got: {out:?}"
        );
        assert_eq!(ch_001.height, Some(480), "and its stored height: {out:?}");
        // CH_002 has no media property rows at all — absent, not an error.
        assert!(
            !out.contains_key("CH_002"),
            "a resource with no media values must be absent from the map, got: {out:?}"
        );
    }

    #[test]
    fn media_thumb_uris_absent_when_first_target_has_no_image_representation() {
        let conn = setup_media_db();
        // Isolate: give CH_002 a single media value pointing at I_003, which
        // has no representations row.
        conn.execute_batch(
            "INSERT INTO properties (resource_id, name, seq, value_type, target_id) \
             VALUES ('CH_002', 'image', 0, 'link', 'I_003');",
        )
        .expect("fixture insert should succeed");
        let ids = vec!["CH_002".to_owned()];
        let out = media_thumb_uris(&conn, "image", &ids).expect("should not error");
        assert!(
            !out.contains_key("CH_002"),
            "a media target with no image Representation must not appear in the map, got: {out:?}"
        );
    }

    #[test]
    fn media_thumb_uris_empty_ids_slice_returns_empty_map() {
        let conn = setup_media_db();
        let out = media_thumb_uris(&conn, "image", &[]).expect("should not error");
        assert!(out.is_empty());
    }

    // Facet query tests
    //
    // Fixture for facet tests: two Character resources (r1 and r4) sharing a
    // `role` list_value property. r1 → n1 only. r4 → n1 and n2 (multi-valued,
    // exercises duplicate-safety). Also a second node n2 (EN-only) for
    // ordering/fallback coverage.
    //
    // Role list (l1): n1 (EN=Protagonist, DE=Protagonistin), n2 (EN=Antagonist,
    // no DE — exercises lang fallback). Two Character resources: r1→n1; r4→n1
    // and r4→n2 (multi-row). Counts: n1=2 (r1 and r4), n2=1 (r4 only).

    fn setup_facet_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        // Base fixture
        conn.execute_batch(FIXTURE).expect("base fixture should load");
        // Add second node and second Character
        conn.execute_batch(
            "
            -- n2: second list node (EN only — no DE, exercises lang fallback)
            INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES ('n2', 'l1', NULL, 1);
            INSERT INTO list_node_labels (node_id, lang, label) VALUES
                ('n2', 'en', 'Antagonist');

            -- r4: second Character with role=n1 (shared with r1) and role=n2 (multi-valued)
            INSERT INTO resources (id, class) VALUES ('r4', 'Character');
            INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
                ('r4', 'role', 0, 'list_value', 'n1'),
                ('r4', 'role', 1, 'list_value', 'n2');
            ",
        )
        .expect("facet extension fixture should load");
        conn
    }

    // TargetIn filtering

    /// `TargetIn` with a single node id filters to only resources having that node.
    #[test]
    fn target_in_single_id_filters_correctly() {
        let conn = setup_facet_db();
        let lang = make_lang("en");
        // Filter Character resources where role target_id IN ('n2').
        // Only r4 has n2.
        let wheres = vec![WhereClause::TargetIn {
            property: "role".to_owned(),
            target_ids: vec!["n2".to_owned()],
        }];
        let ids = list_resource_ids(&conn, "Character", &wheres, &[], 100, 0, &lang).expect("no error");
        assert_eq!(ids, vec!["r4"], "only r4 has role=n2");
    }

    /// `TargetIn` with multiple ids is an OR (union): resources matching ANY of the ids.
    #[test]
    fn target_in_multiple_ids_is_or_union() {
        let conn = setup_facet_db();
        let lang = make_lang("en");
        // Filter Character resources where role target_id IN ('n1', 'n2').
        // r1 has n1, r4 has n1 and n2 → both should appear.
        let wheres = vec![WhereClause::TargetIn {
            property: "role".to_owned(),
            target_ids: vec!["n1".to_owned(), "n2".to_owned()],
        }];
        let mut ids = list_resource_ids(&conn, "Character", &wheres, &[], 100, 0, &lang).expect("no error");
        ids.sort(); // stable for assertion
        assert_eq!(ids, vec!["r1", "r4"], "both r1 and r4 match n1 OR n2");
    }

    /// `TargetIn` is duplicate-safe: r4 has TWO role rows (n1 and n2) but must
    /// appear exactly once when filtering on n1 (or n2, or both).
    #[test]
    fn target_in_multi_valued_resource_counted_once() {
        let conn = setup_facet_db();
        let lang = make_lang("en");
        // r4 has two role rows — filtering on n1 must not duplicate r4.
        let wheres = vec![WhereClause::TargetIn {
            property: "role".to_owned(),
            target_ids: vec!["n1".to_owned()],
        }];
        let ids = list_resource_ids(&conn, "Character", &wheres, &[], 100, 0, &lang).expect("no error");
        // Both r1 and r4 have n1; each must appear exactly once.
        let count_r4 = ids.iter().filter(|id| id.as_str() == "r4").count();
        assert_eq!(count_r4, 1, "r4 has two role rows but must appear exactly once: {ids:?}");
        assert_eq!(ids.len(), 2, "r1 and r4 match n1: {ids:?}");
    }

    /// `TargetIn` with an empty list emits `AND 0` (no match).
    /// This is the defensive dead branch — `query::page` guards against this.
    #[test]
    fn target_in_empty_ids_matches_nothing() {
        let conn = setup_facet_db();
        let lang = make_lang("en");
        let wheres = vec![WhereClause::TargetIn { property: "role".to_owned(), target_ids: vec![] }];
        let ids = list_resource_ids(&conn, "Character", &wheres, &[], 100, 0, &lang).expect("no error");
        assert!(ids.is_empty(), "empty TargetIn must match nothing (AND 0 branch): {ids:?}");
    }

    // WhereClause::Search / search_index_name

    /// Query-plan regression guard for the cost of facet narrowing.
    ///
    /// The *deterministic* half of that guard: `hdm_performances_timing`
    /// (`dsp-incubator cpe/bin/tests/smoke.rs`) measures wall-clock and therefore stays
    /// `#[ignore]`d, but the shape of the plan is a hard fact that does not
    /// vary with the machine, so it can and does gate CI.
    ///
    /// `build_where`'s `TargetIn` arm emits a CORRELATED subquery, re-evaluated
    /// once per candidate row. It is only affordable if SQLite can satisfy
    /// `pW.target_id = ? AND pW.resource_id = r.id` with a single index seek.
    /// With a single-column `idx_properties_target ON properties(target_id)` it
    /// cannot: it seeks `target_id` and then SCANS that target's whole
    /// bucket looking for `resource_id`, making a narrowed browse cost
    /// ~3-4 ms per matched row (HdM `?source=ib`: 15.6 s).
    ///
    /// The assertion is on the plan text, not on a duration, and it runs
    /// against the statement `count_resources` really issues (via the shared
    /// `count_resources_sql`). The tiny fixture DB is fine here: with no
    /// `ANALYZE` stats SQLite plans from its built-in estimates, which do not
    /// depend on row count, so the chosen index is the same one it chooses on
    /// HdM's 140k-row `properties` table.
    #[test]
    fn target_in_where_uses_composite_target_index() {
        let conn = setup_facet_db();
        let wheres = vec![WhereClause::TargetIn {
            property: "role".to_owned(),
            target_ids: vec!["n1".to_owned()],
        }];
        let (fragment, _params) = build_where(&wheres).expect("TargetIn fragment builds");
        let sql = format!("EXPLAIN QUERY PLAN {}", count_resources_sql(&fragment));

        // The statement carries three `?`s (class, property name, target id);
        // EXPLAIN QUERY PLAN still requires them bound, though their values
        // cannot change the chosen index (no ANALYZE stats exist to consult).
        let mut stmt = conn.prepare(&sql).expect("EXPLAIN QUERY PLAN prepares");
        let plan: Vec<String> = stmt
            .query_map(["Character", "role", "n1"], |row| row.get::<_, String>(3))
            .expect("plan rows read")
            .collect::<Result<Vec<_>, _>>()
            .expect("plan rows are text");
        let plan_text = plan.join("\n");

        assert!(
            plan_text.contains("idx_properties_target (target_id=? AND resource_id=?)"),
            "the TargetIn EXISTS subquery must seek idx_properties_target on BOTH \
             target_id and resource_id — a target_id-only seek re-scans the whole \
             bucket for every candidate row (the 15.6 s G11 defect). Plan was:\n{plan_text}"
        );
        assert!(
            !plan_text.contains("SCAN pW"),
            "the TargetIn EXISTS subquery must never SCAN the properties table. \
             Plan was:\n{plan_text}"
        );
    }

    /// `build_where` lowers a `Search` clause to the documented
    /// `r.id IN (SELECT … MATCH ?)` fragment, binding the tokenized match
    /// string (not the raw query) as the sole `?` param.
    #[test]
    fn build_where_search_clause_shape_and_bound_param() {
        let wheres = vec![WhereClause::Search {
            table: "search_thing_name".to_owned(),
            query: "rab".to_owned(),
        }];
        let (fragment, params) = build_where(&wheres).expect("no error");
        assert_eq!(
            fragment,
            " AND r.id IN (SELECT resource_id FROM search_thing_name \
             WHERE search_thing_name MATCH ?)"
        );
        assert_eq!(params, vec![rusqlite::types::Value::Text("\"rab\"*".to_owned())]);
    }

    /// A `Search` clause whose query tokenizes to zero terms emits no
    /// fragment at all (never a malformed/empty MATCH).
    #[test]
    fn build_where_search_clause_inactive_query_emits_no_fragment() {
        let wheres = vec![WhereClause::Search {
            table: "search_thing_name".to_owned(),
            query: "   ".to_owned(),
        }];
        let (fragment, params) = build_where(&wheres).expect("no error");
        assert_eq!(fragment, "", "whitespace-only query must emit no clause");
        assert!(params.is_empty());
    }

    #[test]
    fn search_index_name_deterministic_regardless_of_input_order() {
        let a = search_index_name("Thing", &["description".to_owned(), "name".to_owned()]);
        let b = search_index_name("Thing", &["name".to_owned(), "description".to_owned()]);
        assert_eq!(a, b, "input order must not affect the derived table name");
    }

    #[test]
    fn search_index_name_sanitizes_non_identifier_chars() {
        let name = search_index_name("My-Class", &["full-text".to_owned()]);
        assert!(
            !name.contains('-'),
            "a hyphen from class/search names must not survive into the identifier: {name:?}"
        );
        assert_eq!(name, "search_My_Class_full_text");
    }

    // list_facet_values

    /// `list_facet_values` returns (node_id, label, count) for each distinct node
    /// in the `role` property across Character resources.
    /// n1 count=2 (r1 and r4), n2 count=1 (r4 only). ORDER BY cnt DESC, label, target_id.
    #[test]
    fn list_facet_values_returns_correct_counts_and_order() {
        let conn = setup_facet_db();
        let lang = make_lang("en");
        let rows =
            list_facet_values(&conn, "Character", "role", &[], &lang).expect("list_facet_values should not error");

        // n1 appears on 2 resources → count=2; n2 on 1 → count=1.
        // ORDER BY cnt DESC: n1 first, n2 second.
        assert_eq!(rows.len(), 2, "should have two option rows: {rows:?}");

        let (node_id_0, label_0, count_0) = &rows[0];
        assert_eq!(node_id_0, "n1", "first row should be n1 (higher count)");
        assert_eq!(label_0.as_deref(), Some("Protagonist"), "n1 EN label should be Protagonist");
        assert_eq!(*count_0, 2, "n1 count should be 2 (r1 and r4)");

        let (node_id_1, label_1, count_1) = &rows[1];
        assert_eq!(node_id_1, "n2", "second row should be n2 (lower count)");
        assert_eq!(label_1.as_deref(), Some("Antagonist"), "n2 EN label should be Antagonist");
        assert_eq!(*count_1, 1, "n2 count should be 1 (r4 only)");
    }

    /// A lane vocabulary where count order and `seq` order DISAGREE: `seq`-0
    /// `lo` has the fewest values (1), `seq`-2 `hi` the most (3), `seq`-1
    /// `mid` 2. A node no Book points at (`unused`) and a Book value pointing
    /// at a node absent from `list_nodes` (`ghost`) are both left out.
    fn setup_lane_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
        conn.execute_batch(
            "
            INSERT INTO lists (id, name) VALUES ('office', 'office');
            INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES
                ('hi', 'office', NULL, 2),
                ('lo', 'office', NULL, 0),
                ('mid', 'office', NULL, 1),
                ('unused', 'office', NULL, 3);
            INSERT INTO list_node_labels (node_id, lang, label) VALUES
                ('lo', 'de', 'Niedrig'),
                ('lo', 'en', 'Low'),
                ('mid', 'de', 'Mitte'),
                ('hi', 'de', 'Hoch'),
                ('unused', 'en', 'Unused');
            INSERT INTO resources (id, class) VALUES
                ('b1', 'Book'), ('b2', 'Book'), ('b3', 'Book'),
                ('b4', 'Book'), ('b5', 'Book'), ('b6', 'Book'), ('b7', 'Book'),
                ('x1', 'Other');
            INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
                ('b1', 'office', 0, 'list_value', 'hi'),
                ('b2', 'office', 0, 'list_value', 'hi'),
                ('b3', 'office', 0, 'list_value', 'hi'),
                ('b4', 'office', 0, 'list_value', 'mid'),
                ('b5', 'office', 0, 'list_value', 'mid'),
                ('b6', 'office', 0, 'list_value', 'lo'),
                ('b7', 'office', 0, 'list_value', 'ghost'),
                ('x1', 'office', 0, 'list_value', 'unused');
            ",
        )
        .expect("lane fixture should load");
        conn
    }

    #[test]
    fn list_nodes_for_property_follows_seq_not_count() {
        let conn = setup_lane_db();
        let nodes = list_nodes_for_property(&conn, "Book", "office", &make_lang("de"))
            .expect("list_nodes_for_property should not error");
        let ids: Vec<&str> = nodes.iter().map(|n| n.node_id.as_str()).collect();
        assert_eq!(ids, ["lo", "mid", "hi"], "seq order, not count order");
        // The facet query orders the same vocabulary by count instead (its
        // tail — `lo` and the label-less `ghost`, both count 1 — is not
        // pinned here).
        let facet: Vec<String> = list_facet_values(&conn, "Book", "office", &[], &make_lang("de"))
            .expect("list_facet_values should not error")
            .into_iter()
            .map(|(id, _, _)| id)
            .collect();
        assert_eq!(facet[..2], ["hi", "mid"]);
    }

    #[test]
    fn list_nodes_for_property_picks_the_requested_label_then_any() {
        let conn = setup_lane_db();
        let de = list_nodes_for_property(&conn, "Book", "office", &make_lang("de")).expect("no error");
        let labels: Vec<Option<&str>> = de.iter().map(|n| n.label.as_deref()).collect();
        assert_eq!(labels, [Some("Niedrig"), Some("Mitte"), Some("Hoch")]);
        let en = list_nodes_for_property(&conn, "Book", "office", &make_lang("en")).expect("no error");
        let labels: Vec<Option<&str>> = en.iter().map(|n| n.label.as_deref()).collect();
        // `lo` has an EN label; `mid`/`hi` are DE-only and fall back to it.
        assert_eq!(labels, [Some("Low"), Some("Mitte"), Some("Hoch")]);
    }

    #[test]
    fn list_nodes_for_property_of_an_absent_property_is_empty() {
        let conn = setup_lane_db();
        let nodes = list_nodes_for_property(&conn, "Book", "nope", &make_lang("en")).expect("no error");
        assert!(nodes.is_empty());
    }

    /// `list_facet_values` with `lang="de"` resolves DE label for n1
    /// (`Protagonistin`) and falls back to EN (`Antagonist`) for n2 (no DE row).
    #[test]
    fn list_facet_values_lang_fallback_for_missing_translation() {
        let conn = setup_facet_db();
        let lang = make_lang_for("de", &["en", "de"]);
        let rows =
            list_facet_values(&conn, "Character", "role", &[], &lang).expect("list_facet_values should not error");

        assert_eq!(rows.len(), 2, "should have two option rows: {rows:?}");

        // n1 has a DE label → should get "Protagonistin".
        let (node_id_0, label_0, _) = &rows[0];
        assert_eq!(node_id_0, "n1");
        assert_eq!(
            label_0.as_deref(),
            Some("Protagonistin"),
            "n1 should get DE label Protagonistin"
        );

        // n2 has only an EN label → fallback to MIN(lnl.label) = "Antagonist".
        let (node_id_1, label_1, _) = &rows[1];
        assert_eq!(node_id_1, "n2");
        assert_eq!(
            label_1.as_deref(),
            Some("Antagonist"),
            "n2 should fall back to EN label Antagonist when DE is absent"
        );
    }

    /// `list_facet_values` respects `base_wheres` when filtering the count.
    /// With a TargetIn base_where on n1 only, n2 should not appear.
    #[test]
    fn list_facet_values_respects_base_wheres() {
        let conn = setup_facet_db();
        let lang = make_lang("en");
        // Base where: only resources that have role=n1 (excludes resources
        // with only n2 — though in our fixture r4 has both, so it still appears).
        // To actually restrict: filter by a different property (non-matching).
        // Use a TargetIn that only matches r1 (which has no n2 role) to verify
        // the count for n1 drops to 1.
        //
        // Alternatively, test with a restrictive where that excludes r4.
        // r1 has name='Alice' (en); r4 has no name property.
        // Use an Equals where that only matches r1.
        let base_wheres = vec![WhereClause::Equals {
            property: "name".to_owned(),
            value: Value::String("Alice".to_owned()),
        }];
        let rows = list_facet_values(&conn, "Character", "role", &base_wheres, &lang)
            .expect("list_facet_values with base_wheres should not error");

        // Only r1 matches the base_where (r4 has no name='Alice').
        // r1 has only n1 → only n1 should appear, count=1.
        assert_eq!(rows.len(), 1, "base_where filters to r1 only → only n1 should appear: {rows:?}");
        let (node_id, _, count) = &rows[0];
        assert_eq!(node_id, "n1");
        assert_eq!(*count, 1, "only r1 matches → count=1");
    }

    /// `list_facet_values` returns an empty vec for a property absent on the class.
    #[test]
    fn list_facet_values_absent_property_returns_empty() {
        let conn = setup_facet_db();
        let lang = make_lang("en");
        let rows = list_facet_values(&conn, "Character", "no_such_property", &[], &lang).expect("no error");
        assert!(rows.is_empty(), "absent property should return empty vec: {rows:?}");
    }

    // property_value_types_on_class

    /// `property_value_types_on_class` returns `{"list_value"}` for the `role`
    /// property on the `Character` class (all rows are `list_value` type).
    #[test]
    fn property_value_types_on_class_list_value_for_role() {
        let conn = setup_facet_db();
        let types = property_value_types_on_class(&conn, "Character", "role")
            .expect("property_value_types_on_class should not error");
        assert_eq!(
            types,
            HashSet::from(["list_value".to_owned()]),
            "role property on Character should have type {{list_value}}: {types:?}"
        );
    }

    /// `property_value_types_on_class` returns the correct type for a non-list property.
    #[test]
    fn property_value_types_on_class_text_for_name() {
        let conn = setup_facet_db();
        let types = property_value_types_on_class(&conn, "Character", "name").expect("no error");
        assert_eq!(
            types,
            HashSet::from(["text".to_owned()]),
            "name property on Character should have type {{text}}: {types:?}"
        );
    }

    /// `property_value_types_on_class` returns an empty set for a property absent
    /// on the given class (but which may exist on other classes).
    #[test]
    fn property_value_types_on_class_empty_for_absent_property() {
        let conn = setup_facet_db();
        // 'name' exists on Location (r2) but we ask about it on Story class.
        // r3 (Story) has a name property, so let's use 'no_such_prop'.
        let types = property_value_types_on_class(&conn, "Character", "no_such_prop").expect("no error");
        assert!(types.is_empty(), "absent property should return empty set: {types:?}");
    }

    /// `property_value_types_on_class` returns empty when the class itself is absent.
    #[test]
    fn property_value_types_on_class_empty_for_absent_class() {
        let conn = setup_facet_db();
        let types = property_value_types_on_class(&conn, "NoSuchClass", "role").expect("no error");
        assert!(types.is_empty(), "absent class should return empty set: {types:?}");
    }

    // link_target_classes_on_class / class_has_representations (embedded-media)

    #[test]
    fn link_target_classes_on_class_returns_resolvable_target_classes() {
        // r1 (Character) has 'location' link -> r2 (Location) and 'other_link'
        // -> a dangling target (no resolvable class contributed).
        let conn = setup_db();
        let classes = link_target_classes_on_class(&conn, "Character", "location").expect("no error");
        assert_eq!(classes, HashSet::from(["Location".to_owned()]));
    }

    #[test]
    fn link_target_classes_on_class_empty_for_dangling_only_link() {
        let conn = setup_db();
        let classes = link_target_classes_on_class(&conn, "Character", "other_link").expect("no error");
        assert!(
            classes.is_empty(),
            "a link whose only target is dangling should contribute no class: {classes:?}"
        );
    }

    #[test]
    fn link_target_classes_on_class_empty_for_absent_property() {
        let conn = setup_db();
        let classes = link_target_classes_on_class(&conn, "Character", "no_such_prop").expect("no error");
        assert!(classes.is_empty());
    }

    #[test]
    fn class_has_representations_false_when_no_representations_rows() {
        let conn = setup_db();
        assert!(!class_has_representations(&conn, "Location").expect("no error"));
    }

    #[test]
    fn class_has_representations_true_when_a_resource_carries_one() {
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class) VALUES ('img1', 'Image');
             INSERT INTO representations (resource_id, kind, uri, seq)
                 VALUES ('img1', 'image', 'https://example.org/full/max/0/default.jpg', 0);",
        )
        .expect("fixture should load");
        assert!(class_has_representations(&conn, "Image").expect("no error"));
        assert!(!class_has_representations(&conn, "Character").expect("no error"));
    }

    // Annotations
    // dsp-incubator cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md

    mod annotations {
        use std::collections::BTreeSet;

        use cpe_core::ir::{AnnotationShape, MemberParent};
        use rusqlite::Connection;

        use super::super::{
            annotated_members, annotation_crops, annotation_links_by_id, annotation_partners, annotations_on,
            link_targets, links_targeting, malformed_annotation_rows, page_plates, property_texts, resource_links,
            showcase_annotation_row, EntryTarget,
        };
        use super::{make_lang, SCHEMA_SQL};

        /// Ids sort lexically AGAINST area order on `pg-1` (`z` .25, `b` and
        /// `c` an exact .125 tie, `a` .01), and every table is inserted out of
        /// order, so a missing `ORDER BY` or tie-break fails. `pg-2` is a
        /// member of two compounds; `bk-a` < `bk-c` names its parent.
        const FIXTURE: &str = "
            INSERT INTO resources (id, class) VALUES
                ('bk-c', 'Book'), ('bk-a', 'Book'), ('bk-b', 'Book'),
                ('pg-2', 'Page'), ('pg-1', 'Page'), ('pg-3', 'Page'), ('pg-4', 'Page'),
                ('pg-9', 'Page');
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('bk-a', 'name', 0, 'text', 'Buch A', 'de'),
                ('bk-a', 'name', 1, 'text', 'Book A', 'en'),
                ('bk-b', 'name', 0, 'text', 'Liber B', NULL),
                ('pg-1', 'name', 0, 'text', 'a1v', NULL),
                ('pg-2', 'name', 0, 'text', 'a2r', NULL),
                ('pg-9', 'name', 0, 'text', 'z9', NULL);
            INSERT INTO compound_parts (parent_id, child_id, seq) VALUES
                ('bk-c', 'pg-2', 5),
                ('bk-a', 'pg-3', 3), ('bk-a', 'pg-1', 1), ('bk-a', 'pg-2', 2), ('bk-a', 'pg-4', 4),
                ('bk-b', 'pg-9', 1);
            INSERT INTO annotations (id, resource_id, shape, x, y, w, h, points, color, remark, remark_lang) VALUES
                ('c', 'pg-1', 'circle',    0.5,  0.5, 0.5,  0.25, NULL, '#3333ff', 'Kreis', 'de'),
                ('a', 'pg-1', 'rectangle', 0.1,  0.1, 0.1,  0.1,  NULL, '#ff3333', 'Klein', 'de'),
                ('z', 'pg-1', 'rectangle', 0.0,  0.0, 0.5,  0.5,  NULL, '#ff3333', 'Gross', NULL),
                ('b', 'pg-1', 'polygon',   0.25, 0.0, 0.25, 0.5,  '0.25,0 0.5,0.5 0.25,0.5', NULL, NULL, NULL),
                ('k', 'pg-9', 'rectangle', 0.1,  0.1, 0.2,  0.2,  NULL, '#ff3333', NULL, NULL),
                ('j', 'pg-9', 'rectangle', 0.0,  0.0, 0.5,  0.5,  NULL, '#ff3333', NULL, NULL),
                ('m', 'pg-2', 'rectangle', 0.0,  0.0, 0.5,  0.5,  NULL, '#ff3333', NULL, NULL);
            INSERT INTO annotation_links (id, name, name_lang, comment, comment_lang) VALUES
                ('L3', 'identischer Holzschnitt', 'de', 'duplicate of L1', 'en'),
                ('L1', 'identischer Holzschnitt', 'de', NULL, NULL),
                ('L2', 'one target', NULL, NULL, NULL),
                ('L4', 'self', NULL, NULL, NULL),
                ('L5', 'page level', NULL, 'Ganze Seite', 'de'),
                ('L6', 'im selben Band gebunden', 'de', NULL, NULL),
                ('L7', 'book and annotation', NULL, NULL, NULL),
                ('L8', 'page only', NULL, NULL, NULL);
            INSERT INTO annotation_link_targets (link_id, seq, annotation_id, resource_id) VALUES
                ('L3', 1, 'a', NULL), ('L3', 0, 'k', NULL),
                ('L1', 1, 'k', NULL), ('L1', 0, 'a', NULL),
                ('L2', 0, 'a', NULL),
                ('L4', 2, 'm', NULL), ('L4', 1, 'a', NULL), ('L4', 0, 'a', NULL),
                ('L5', 1, 'z', NULL), ('L5', 0, NULL, 'pg-3'),
                ('L6', 2, NULL, 'bk-c'), ('L6', 0, NULL, 'bk-a'), ('L6', 1, NULL, 'bk-b'),
                ('L7', 0, NULL, 'bk-a'), ('L7', 1, 'm', NULL),
                ('L8', 0, NULL, 'pg-2');
        ";

        fn setup() -> Connection {
            let conn = Connection::open_in_memory().expect("in-memory db should open");
            conn.pragma_update(None, "foreign_keys", true).expect("pragma should apply");
            conn.execute_batch(SCHEMA_SQL).expect("schema should apply");
            conn.execute_batch(FIXTURE).expect("fixture should load");
            conn
        }

        #[test]
        fn annotations_on_orders_larger_box_first_ties_by_id() {
            let conn = setup();
            let got: Vec<(u32, String)> = annotations_on(&conn, "pg-1")
                .expect("query should succeed")
                .into_iter()
                .map(|(n, a)| (n, a.id))
                .collect();
            let want = [(1, "z"), (2, "b"), (3, "c"), (4, "a")].map(|(n, id)| (n, id.to_owned()));
            assert_eq!(got, want);
        }

        #[test]
        fn partner_number_is_the_partner_pages_register_number() {
            let conn = setup();
            let lang = make_lang("en");
            for (entry, page) in [("a", "pg-9"), ("z", "pg-1"), ("m", "pg-1")] {
                let register = annotations_on(&conn, page).expect("ok");
                let partners = annotation_partners(&conn, EntryTarget::Annotation(entry), &lang).expect("ok");
                for p in partners.iter().filter(|p| p.resource_id == page) {
                    let id = p.annotation_id.as_deref().expect("annotation partner");
                    let (n, _) = register.iter().find(|(_, a)| a.id == id).expect("partner on its page");
                    assert_eq!(p.number, Some(*n), "{entry} → {id}");
                }
            }
        }

        #[test]
        fn malformed_polygon_points_are_a_query_error_not_a_panic() {
            let conn = setup();
            conn.execute_batch(
                "INSERT INTO annotations (id, resource_id, shape, x, y, w, h, points) \
                 VALUES ('bad', 'pg-4', 'polygon', 0.1, 0.1, 0.2, 0.2, 'garbage');",
            )
            .expect("insert should succeed");
            assert!(annotations_on(&conn, "pg-4").is_err());
        }

        #[test]
        fn annotations_on_reads_shape_box_points_and_remark() {
            let conn = setup();
            let rows = annotations_on(&conn, "pg-1").expect("query should succeed");
            let rows: Vec<_> = rows.into_iter().map(|(_, a)| a).collect();
            let b = rows.iter().find(|a| a.id == "b").expect("b present");
            assert_eq!(b.shape, AnnotationShape::Polygon);
            assert_eq!(b.points, [(0.25, 0.0), (0.5, 0.5), (0.25, 0.5)]);
            assert_eq!((b.x, b.y, b.w, b.h), (0.25, 0.0, 0.25, 0.5));
            assert_eq!(b.color, None);
            let c = rows.iter().find(|a| a.id == "c").expect("c present");
            assert_eq!(c.shape, AnnotationShape::Circle);
            assert!(c.points.is_empty(), "only a polygon carries points");
            assert_eq!(c.color.as_deref(), Some("#3333ff"));
            assert_eq!(c.remark.as_deref(), Some("Kreis"));
            assert_eq!(c.remark_lang.as_deref(), Some("de"));
            assert_eq!(c.resource_id, "pg-1");
        }

        #[test]
        fn annotations_on_unannotated_resource_is_empty() {
            let conn = setup();
            assert!(annotations_on(&conn, "pg-4").expect("ok").is_empty());
        }

        #[test]
        fn partners_skip_self_and_one_target_links_and_keep_duplicate_links() {
            let conn = setup();
            let partners = annotation_partners(&conn, EntryTarget::Annotation("a"), &make_lang("en"))
                .expect("query should succeed");
            let got: Vec<(&str, Option<&str>)> = partners
                .iter()
                .map(|p| (p.link_id.as_str(), p.annotation_id.as_deref()))
                .collect();
            // L2 (one target) yields nothing; L4's two `a` targets are the
            // entry itself; L1 and L3 both join a↔k and both come through.
            assert_eq!(got, [("L1", Some("k")), ("L3", Some("k")), ("L4", Some("m"))]);
        }

        #[test]
        fn annotation_partner_carries_its_number_page_and_parent() {
            let conn = setup();
            let partners = annotation_partners(&conn, EntryTarget::Annotation("a"), &make_lang("en"))
                .expect("query should succeed");
            let k = &partners[0];
            // `k` is the smaller of pg-9's two annotations.
            assert_eq!(k.number, Some(2));
            assert_eq!(k.link_name, "identischer Holzschnitt");
            assert_eq!(k.link_name_lang.as_deref(), Some("de"));
            assert_eq!(k.resource_id, "pg-9");
            assert_eq!(k.resource_class, "Page");
            assert_eq!(k.resource_label, "z9");
            assert_eq!(
                k.parent,
                Some(MemberParent { id: "bk-b".to_owned(), class: "Book".to_owned(), seq: 1 })
            );
            assert_eq!(k.parent_title.as_deref(), Some("Liber B"));
            assert!(k.first_member, "pg-9 is bk-b's only member");
            // `m`'s page sits in bk-a (seq 2) and bk-c (seq 5): the first parent wins.
            let m = &partners[2];
            assert_eq!(m.number, Some(1));
            assert_eq!(m.parent.as_ref().map(|p| (p.id.as_str(), p.seq)), Some(("bk-a", 2)));
            assert!(!m.first_member, "pg-1 comes before pg-2 in bk-a");
        }

        #[test]
        fn first_member_breaks_a_seq_tie_by_child_id() {
            let conn = setup();
            conn.execute_batch(
                "INSERT INTO resources (id, class) VALUES ('bk-t', 'Book'), ('pg-0', 'Page');
                 INSERT INTO compound_parts (parent_id, child_id, seq) VALUES
                     ('bk-t', 'pg-9', 1), ('bk-t', 'pg-0', 1);
                 DELETE FROM compound_parts WHERE parent_id = 'bk-b';",
            )
            .expect("fixture should load");
            let partners = annotation_partners(&conn, EntryTarget::Annotation("a"), &make_lang("en"))
                .expect("query should succeed");
            assert_eq!(partners[0].parent.as_ref().map(|p| p.id.as_str()), Some("bk-t"));
            assert!(!partners[0].first_member, "pg-0 sorts before pg-9 at seq 1");
        }

        #[test]
        fn resource_target_is_a_partner_and_has_annotation_partners() {
            let conn = setup();
            let of_z = annotation_partners(&conn, EntryTarget::Annotation("z"), &make_lang("de"))
                .expect("query should succeed");
            assert_eq!(of_z.len(), 1);
            let page = &of_z[0];
            assert_eq!(page.link_id, "L5");
            assert_eq!(page.annotation_id, None);
            assert_eq!(page.number, None);
            assert_eq!(page.resource_id, "pg-3");
            assert_eq!(page.resource_label, "pg-3", "no name falls back to the id");
            assert_eq!(page.parent.as_ref().map(|p| p.seq), Some(3));
            assert!(!page.first_member);
            assert_eq!(page.parent_title.as_deref(), Some("Buch A"));
            let en = annotation_partners(&conn, EntryTarget::Annotation("z"), &make_lang("en"))
                .expect("query should succeed");
            assert_eq!(en[0].parent_title.as_deref(), Some("Book A"));

            let of_page = annotation_partners(&conn, EntryTarget::Resource("pg-3"), &make_lang("en"))
                .expect("query should succeed");
            let got: Vec<(&str, Option<&str>, Option<u32>, &str)> = of_page
                .iter()
                .map(|p| {
                    (
                        p.link_id.as_str(),
                        p.annotation_id.as_deref(),
                        p.number,
                        p.resource_label.as_str(),
                    )
                })
                .collect();
            assert_eq!(got, [("L5", Some("z"), Some(1), "a1v")]);
            assert!(of_page[0].first_member, "pg-1 is bk-a's first member");
        }

        /// `pg-1` links two images (`img-y` at seq 0 with two representations,
        /// `img-x` at seq 1); `pg-9`'s image has no stored size; `pg-2` has
        /// none. Ids sort against `seq` so a missing `ORDER BY` fails.
        const CROP_IMAGES: &str = "
            INSERT INTO resources (id, class) VALUES
                ('img-x', 'Image'), ('img-y', 'Image'), ('img-z', 'Image');
            INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
                ('pg-1', 'image', 1, 'link', 'img-x'),
                ('pg-1', 'image', 0, 'link', 'img-y'),
                ('pg-9', 'image', 0, 'link', 'img-z');
            INSERT INTO representations (resource_id, kind, uri, width, height, seq) VALUES
                ('img-x', 'image', 'https://iiif.example/x.jp2/full/max/0/default.jpg', 10, 20, 0),
                ('img-y', 'image', 'https://iiif.example/y2.jp2/full/max/0/default.jpg', 30, 40, 1),
                ('img-y', 'image', 'https://iiif.example/y1.jp2/full/max/0/default.jpg', 3564, 5304, 0),
                ('img-z', 'image', 'https://iiif.example/z.jp2/full/max/0/default.jpg', NULL, NULL, 0);
        ";

        fn crop_ids(ids: &[&str]) -> Vec<String> {
            ids.iter().map(|id| (*id).to_owned()).collect()
        }

        #[test]
        fn annotation_crops_keep_the_requested_order_and_omit_unknown_ids() {
            let conn = setup();
            conn.execute_batch(CROP_IMAGES).expect("fixture should load");
            let got: Vec<String> =
                annotation_crops(&conn, &crop_ids(&["m", "nope", "b", "a", "m"]), "image", &make_lang("en"))
                    .expect("query should succeed")
                    .into_iter()
                    .map(|c| c.annotation.id)
                    .collect();
            assert_eq!(got, ["m", "b", "a", "m"]);
            assert!(annotation_crops(&conn, &[], "image", &make_lang("en")).expect("ok").is_empty());
        }

        #[test]
        fn annotation_crop_carries_box_page_parent_and_first_image() {
            let conn = setup();
            conn.execute_batch(CROP_IMAGES).expect("fixture should load");
            let crops = annotation_crops(&conn, &crop_ids(&["b", "k"]), "image", &make_lang("de"))
                .expect("query should succeed");
            let b = &crops[0];
            assert_eq!(b.annotation.shape, AnnotationShape::Polygon);
            assert_eq!(
                (b.annotation.x, b.annotation.y, b.annotation.w, b.annotation.h),
                (0.25, 0.0, 0.25, 0.5)
            );
            assert_eq!(b.annotation.resource_id, "pg-1");
            assert_eq!(b.resource_class, "Page");
            assert_eq!(b.resource_label, "a1v");
            assert_eq!(
                b.parent,
                Some(MemberParent { id: "bk-a".to_owned(), class: "Book".to_owned(), seq: 1 })
            );
            assert_eq!(b.parent_title.as_deref(), Some("Buch A"));
            let image = b.image.as_ref().expect("pg-1 has an image");
            assert_eq!(image.uri, "https://iiif.example/y1.jp2/full/max/0/default.jpg");
            assert_eq!((image.width, image.height), (Some(3564), Some(5304)));
            let k = &crops[1];
            let image = k.image.as_ref().expect("pg-9 has an image");
            assert_eq!((image.width, image.height), (None, None));
        }

        #[test]
        fn annotation_crop_without_that_media_property_has_no_image() {
            let conn = setup();
            conn.execute_batch(CROP_IMAGES).expect("fixture should load");
            let crops = annotation_crops(&conn, &crop_ids(&["m", "a"]), "image", &make_lang("en"))
                .expect("query should succeed");
            assert_eq!(crops[0].image, None, "pg-2 links no image");
            let other =
                annotation_crops(&conn, &crop_ids(&["a"]), "scan", &make_lang("en")).expect("query should succeed");
            assert_eq!(other[0].image, None, "pg-1 has no `scan` property");
            assert_eq!(other[0].resource_label, "a1v");
        }

        #[test]
        fn page_plates_keep_the_requested_order_and_omit_unknown_ids() {
            let conn = setup();
            conn.execute_batch(CROP_IMAGES).expect("fixture should load");
            let got: Vec<String> =
                page_plates(&conn, &crop_ids(&["pg-9", "nope", "pg-1", "pg-9"]), "image", &make_lang("en"))
                    .expect("query should succeed")
                    .into_iter()
                    .map(|p| p.resource_id)
                    .collect();
            assert_eq!(got, ["pg-9", "pg-1", "pg-9"]);
            assert!(page_plates(&conn, &[], "image", &make_lang("en")).expect("ok").is_empty());
        }

        #[test]
        fn page_plate_carries_label_parent_and_first_image() {
            let conn = setup();
            conn.execute_batch(CROP_IMAGES).expect("fixture should load");
            let plates = page_plates(&conn, &crop_ids(&["pg-2", "pg-1", "pg-3"]), "image", &make_lang("en"))
                .expect("query should succeed");
            // pg-2 sits in bk-a (seq 2) and bk-c (seq 5): the first parent wins.
            let pg2 = &plates[0];
            assert_eq!(pg2.resource_class, "Page");
            assert_eq!(pg2.resource_label, "a2r");
            assert_eq!(pg2.parent.as_ref().map(|p| (p.id.as_str(), p.seq)), Some(("bk-a", 2)));
            assert_eq!(pg2.parent_title.as_deref(), Some("Book A"));
            assert_eq!(pg2.image, None, "pg-2 links no image");
            let pg1 = &plates[1];
            let image = pg1.image.as_ref().expect("pg-1 has an image");
            assert_eq!(image.uri, "https://iiif.example/y1.jp2/full/max/0/default.jpg");
            assert_eq!((image.width, image.height), (Some(3564), Some(5304)));
            assert_eq!(plates[2].resource_label, "pg-3", "no name falls back to the id");
        }

        #[test]
        fn annotation_links_by_id_keep_the_requested_order_and_omit_unknown_ids() {
            let conn = setup();
            let links = annotation_links_by_id(&conn, &crop_ids(&["L5", "nope", "L1"])).expect("query should succeed");
            let got: Vec<(&str, Option<&str>)> = links.iter().map(|l| (l.id.as_str(), l.comment.as_deref())).collect();
            assert_eq!(got, [("L5", Some("Ganze Seite")), ("L1", None)]);
            assert_eq!(links[0].comment_lang.as_deref(), Some("de"));
            assert_eq!(links[1].name, "identischer Holzschnitt");
        }

        #[test]
        fn showcase_annotation_row_reads_resource_remark_and_image_size() {
            let conn = setup();
            conn.execute_batch(CROP_IMAGES).expect("fixture should load");
            let c = showcase_annotation_row(&conn, "c", "image").expect("ok").expect("c exists");
            assert_eq!(c.resource_id, "pg-1");
            assert_eq!(c.remark.as_deref(), Some("Kreis"));
            assert_eq!((c.width, c.height), (Some(3564), Some(5304)));
            assert_eq!(c.uri.as_deref(), Some("https://iiif.example/y1.jp2/full/max/0/default.jpg"));
            let m = showcase_annotation_row(&conn, "m", "image").expect("ok").expect("m exists");
            assert_eq!((m.width, m.height), (None, None), "pg-2 links no image");
            assert_eq!(m.uri, None);
            assert_eq!(showcase_annotation_row(&conn, "nope", "image").expect("ok"), None);
        }

        #[test]
        fn property_texts_lists_every_language_in_seq_order() {
            let conn = setup();
            conn.execute_batch(
                "INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                     ('pg-1', 'description', 1, 'text', 'zweite', 'de'),
                     ('pg-1', 'description', 0, 'text', 'first', 'en');",
            )
            .expect("fixture should load");
            assert_eq!(property_texts(&conn, "pg-1", "description").expect("ok"), ["first", "zweite"]);
            assert!(property_texts(&conn, "pg-2", "description").expect("ok").is_empty());
        }

        #[test]
        fn link_targets_are_in_seq_order_and_none_for_an_unknown_link() {
            let conn = setup();
            let l5 = link_targets(&conn, "L5").expect("ok").expect("L5 exists");
            let got: Vec<(Option<&str>, Option<&str>)> = l5
                .iter()
                .map(|t| (t.resource_id.as_deref(), t.annotation_id.as_deref()))
                .collect();
            assert_eq!(got, [(Some("pg-3"), None), (None, Some("z"))]);
            assert_eq!(link_targets(&conn, "nope").expect("ok"), None);
        }

        #[test]
        fn links_targeting_lists_page_level_links_by_id() {
            let conn = setup();
            let ids =
                |id| -> Vec<String> { links_targeting(&conn, id).expect("ok").into_iter().map(|l| l.id).collect() };
            assert_eq!(ids("pg-3"), ["L5"]);
            assert_eq!(ids("pg-2"), ["L8"]);
            assert_eq!(ids("bk-a"), ["L6", "L7"]);
            assert!(ids("pg-1").is_empty(), "annotation targets are not page-level");
            let l5 = links_targeting(&conn, "pg-3").expect("ok").remove(0);
            assert_eq!(l5.comment.as_deref(), Some("Ganze Seite"));
            assert_eq!(l5.comment_lang.as_deref(), Some("de"));
        }

        #[test]
        fn annotated_members_counts_annotations_and_page_level_links() {
            let conn = setup();
            let expected: BTreeSet<String> = ["pg-1", "pg-2", "pg-3"].map(str::to_owned).into();
            assert_eq!(annotated_members(&conn, "bk-a").expect("ok"), expected);
            let b: BTreeSet<String> = ["pg-9".to_owned()].into();
            assert_eq!(annotated_members(&conn, "bk-b").expect("ok"), b);
        }

        #[test]
        fn resource_links_lists_other_resource_targets_only() {
            let conn = setup();
            let got: Vec<(String, String, String)> = resource_links(&conn, "bk-b", &make_lang("en"))
                .expect("ok")
                .into_iter()
                .map(|l| (l.link_id, l.resource_id, l.resource_label))
                .collect();
            let want = [("L6", "bk-a", "Book A"), ("L6", "bk-c", "bk-c")]
                .map(|(l, r, n)| (l.to_owned(), r.to_owned(), n.to_owned()));
            assert_eq!(got, want);
            // bk-a's L7 also targets the annotation `m`, which is not a
            // resource-level partner.
            let of_a: Vec<(String, String)> = resource_links(&conn, "bk-a", &make_lang("en"))
                .expect("ok")
                .into_iter()
                .map(|l| (l.link_id, l.resource_id))
                .collect();
            let want_a = [("L6", "bk-b"), ("L6", "bk-c")].map(|(l, r)| (l.to_owned(), r.to_owned()));
            assert_eq!(of_a, want_a);
        }

        #[test]
        fn malformed_annotation_rows_returns_every_row_and_targetless_links() {
            let conn = setup();
            conn.execute_batch("INSERT INTO annotation_links (id, name) VALUES ('L0', 'no target'), ('LZ', 'none');")
                .expect("insert should succeed");
            let rows = malformed_annotation_rows(&conn).expect("query should succeed");
            let ids: Vec<&str> = rows.annotations.iter().map(|a| a.id.as_str()).collect();
            assert_eq!(ids, ["a", "b", "c", "j", "k", "m", "z"]);
            let b = &rows.annotations[1];
            assert_eq!(b.shape, "polygon");
            assert_eq!(b.points.as_deref(), Some("0.25,0 0.5,0.5 0.25,0.5"));
            assert_eq!(rows.empty_links, ["L0", "LZ"]);
        }
    }

    // Data ARKs

    #[test]
    fn data_arks_reads_both_tables_apart_and_omits_null_rows() {
        let conn = setup_clean_db();
        // `x` is both a resource id and an annotation id, with different ARKs.
        conn.execute_batch(
            "INSERT INTO resources (id, class, ark) VALUES
                 ('x', 'Page', 'ark-of-resource-x'),
                 ('y', 'Page', NULL),
                 ('img', 'Image', NULL),
                 ('z', 'Book', 'ark-of-resource-z');
             INSERT INTO annotations (id, resource_id, shape, x, y, w, h, ark) VALUES
                 ('x', 'y', 'rectangle', 0.1, 0.1, 0.2, 0.2, 'ark-of-annotation-x'),
                 ('n', 'y', 'rectangle', 0.1, 0.1, 0.2, 0.2, NULL);",
        )
        .expect("fixture should load");
        let arks = data_arks(&conn).expect("query should succeed");
        assert_eq!(
            arks,
            DataArks::from_stored(
                std::collections::HashMap::from([
                    ("x".to_owned(), "ark-of-resource-x".to_owned()),
                    ("z".to_owned(), "ark-of-resource-z".to_owned()),
                ]),
                std::collections::HashMap::from([("x".to_owned(), "ark-of-annotation-x".to_owned())]),
            ),
            "NULL rows are omitted"
        );
        assert_eq!(arks.resource("x"), Some("ark-of-resource-x"));
        assert_eq!(arks.resource("z"), Some("ark-of-resource-z"));
        assert_eq!(arks.resource("y"), None);
        assert_eq!(arks.annotation("x"), Some("ark-of-annotation-x"));
        assert_eq!(arks.annotation("n"), None);
        assert_eq!(arks.annotation("z"), None, "a resource id is not an annotation id");
    }

    #[test]
    fn malformed_data_ark_rows_returns_every_stored_ark_unjudged_in_order() {
        let conn = setup_clean_db();
        conn.execute_batch(
            "INSERT INTO resources (id, class, ark) VALUES
                 ('r2', 'Page', ''),
                 ('r1', 'Page', 'not an ark'),
                 ('r3', 'Image', NULL);
             INSERT INTO annotations (id, resource_id, shape, x, y, w, h, ark) VALUES
                 ('a1', 'r1', 'rectangle', 0.1, 0.1, 0.2, 0.2, 'http://x/');",
        )
        .expect("fixture should load");
        let rows: Vec<(&str, String, String)> = malformed_data_ark_rows(&conn)
            .expect("query should succeed")
            .into_iter()
            .map(|r| (r.table, r.id, r.ark))
            .collect();
        let want = [
            ("resources", "r1", "not an ark"),
            ("resources", "r2", ""),
            ("annotations", "a1", "http://x/"),
        ]
        .map(|(t, i, a)| (t, i.to_owned(), a.to_owned()));
        assert_eq!(rows, want, "'' is a stored value, NULL is not");
    }

    #[test]
    fn data_arks_is_empty_without_any_ark() {
        let conn = setup_clean_db();
        conn.execute_batch("INSERT INTO resources (id, class) VALUES ('r1', 'Thing');")
            .expect("fixture should load");
        assert_eq!(data_arks(&conn).expect("query should succeed"), DataArks::default());
    }
}
