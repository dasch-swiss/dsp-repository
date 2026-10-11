//! The store's composite reads over a project's connection: every lock a
//! page request takes on it is taken here.
//!
//! Lock rule: each read locks with `ProjectStore::lock`, keeps the guard in
//! the narrowest scope that runs its queries, and drops it before hydrating
//! ids (`get_resource` re-locks per id) and before returning. A guard is
//! never held across hydration or an `.await`, and never returned. The
//! `Mutex` is non-reentrant, so a read never calls another read while it
//! holds the guard. `ProjectStore` owns the `Mutex`; [`ProjectStore::reads`]
//! is the way to a `ProjectReads`. The locks taken outside this module are
//! one per fact (`LiveProjectFacts`) and one per load step (`ProjectStore`).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use cpe_core::ir::{
    AnnotationCrop, AnnotationEntry, AnnotationLink, AnnotationPartner, CompoundPage, Facet, FacetGroup, FacetOption,
    IncomingLinks, IncomingLinksSpec, LaneNode, MediaThumb, MemberHeader, PagePlate, Pagination, PropertyLabel,
    Resource, ResourceLink, ResourceListPage, Sort, WhereClause,
};
use cpe_core::lang::Lang;

use crate::db::ProjectStore;
use crate::query::{self, QueryError};

/// Resolve a raw `?page=` value to a 0-based position in `headers`.
///
/// The raw value is percent-/`+`-decoded once (`util::decode_query_value` —
/// the host's `query_pairs` never decodes, and a typed pagenum such as
/// `a1r; Titelblatt` arrives as `a1r%3B+Titelblatt`) and trimmed; an empty
/// result counts as absent (`None`). Tiers, first hit wins:
///
/// 1. exact member **id**;
/// 2. (a) exact display **name**, case-SENSITIVE — first member in `seq` order (pagenums repeat
///    within a book, and case-folded ids mean `A1v` and `a1v` are distinct members: a typed `A1v`
///    must reach `A1v`), then (b) exact name, case-INsensitive — first in `seq` order;
/// 3. a **unique** case-insensitive prefix of the name. "Unique" is by distinct (case-folded) name,
///    not by member: a prefix whose every hit carries the same name is tier 2b spelled short, so it
///    resolves to the first such member; a prefix spanning two different names (`a1` over
///    `a1r`/`a1v`) is ambiguous → `None`.
///
/// `None` ⇒ the caller falls back to `?member=`. Pure and total.
pub fn resolve_page_param(headers: &[MemberHeader], raw: &str) -> Option<usize> {
    let decoded = cpe_core::util::decode_query_value(raw);
    let needle = decoded.trim();
    if needle.is_empty() {
        return None;
    }
    // Tier 1 — exact id.
    if let Some(pos) = headers.iter().position(|h| h.id == needle) {
        return Some(pos);
    }
    // Tier 2a — exact name, case-sensitive, first in seq.
    if let Some(pos) = headers.iter().position(|h| h.name == needle) {
        return Some(pos);
    }
    // Tier 2b — exact name, case-insensitive, first in seq.
    let folded = needle.to_lowercase();
    let folded_names: Vec<String> = headers.iter().map(|h| h.name.to_lowercase()).collect();
    if let Some(pos) = folded_names.iter().position(|n| *n == folded) {
        return Some(pos);
    }
    // Tier 3 — unique (by distinct name) case-insensitive prefix.
    let mut hits = folded_names.iter().enumerate().filter(|(_, n)| n.starts_with(&folded));
    let (first_pos, first_name) = hits.next()?;
    if hits.any(|(_, n)| n != first_name) {
        return None;
    }
    Some(first_pos)
}

/// The composite reads over one project's connection. Borrows the store;
/// built per call (`ProjectStore::reads`), it holds no state of its own.
pub struct ProjectReads<'a> {
    store: &'a ProjectStore,
}

impl<'a> ProjectReads<'a> {
    pub(crate) fn new(store: &'a ProjectStore) -> Self {
        Self { store }
    }

    /// Fetch a `Resource` by id from this project's in-memory database.
    ///
    /// Locks `conn` internally for the duration of the query and releases it
    /// before returning. Never holds the guard across `.await`.
    ///
    /// Returns `Ok(None)` for a missing resource (caller maps to 404).
    pub fn get_resource(&self, id: &str, lang: &Lang) -> Result<Option<Resource>, QueryError> {
        let conn = self.store.lock();
        query::get_resource(&conn, id, lang)
    }

    /// Fetch a paginated page of resources of `class`, filtered by `wheres`,
    /// sorted by `sorts`, for the requested `page` (1-based).
    ///
    /// Pagination behaviour:
    /// - `Pagination::Limit(n)`: returns at most `n` items starting from the first, ignores `page`,
    ///   sets `effective_page = 1`. `total` is the count of hydrated items (no extra COUNT query).
    /// - `Pagination::PerPage(n)`: queries `count_resources` first, then clamps `page` to `[1,
    ///   last_page]` where `last_page = max(1, ceil(total / n))`. `total` is the COUNT result.
    ///
    /// Lock protocol: the `conn` mutex is locked once for the COUNT (PerPage
    /// only) and the id list query, then dropped. Each resource is hydrated
    /// via `self.get_resource` (which re-locks-and-drops independently).
    /// The mutex is never held across the hydration loop.
    ///
    /// An id returned by `list_resource_ids` that resolves to `None` via
    /// `get_resource` indicates an invariant violation (the id was in the DB
    /// when listed but vanished before hydration — impossible with a
    /// read-only in-memory DB). The implementation panics with `expect` on
    /// this path rather than silently skipping, because the invariant is
    /// guaranteed by our own logic.
    pub fn list_resources(
        &self,
        class: &str,
        wheres: &[WhereClause],
        sorts: &[Sort],
        pagination: Pagination,
        page: u32,
        lang: &Lang,
    ) -> Result<ResourceListPage, QueryError> {
        match pagination {
            Pagination::Limit(n) => {
                let ids = {
                    let conn = self.store.lock();
                    query::list_resource_ids(&conn, class, wheres, sorts, n, 0, lang)?
                };

                let items = self.hydrate_ids(&ids, lang)?;
                let total = items.len();
                Ok(ResourceListPage { items, total, page: 1, pagination, unfiltered_total: None })
            }
            Pagination::PerPage(n) => {
                let (total, ids) = {
                    let conn = self.store.lock();
                    let total = query::count_resources(&conn, class, wheres)?;
                    // Compute last page: at least 1 even when the result set is empty.
                    let last_page = last_page_for(total, n);
                    // Clamp the requested page; page=0 is treated as page=1.
                    let effective_page = page.max(1).min(last_page);
                    // Saturating multiply: n and effective_page are u32; the
                    // product is the byte offset into a u32-indexed list so it
                    // comfortably fits in u64, but we keep u32 here — both are
                    // at most ~4 billion in practice. Use saturating_mul so an
                    // absurd product can't overflow to a wrong offset.
                    let offset = (effective_page - 1).saturating_mul(n);
                    let ids = query::list_resource_ids(&conn, class, wheres, sorts, n, offset, lang)?;
                    (total, ids)
                    // guard dropped here
                };

                let items = self.hydrate_ids(&ids, lang)?;
                // Re-derive effective_page from the returned (total, n) so the
                // struct is consistent even if the caller passed page=0.
                let last_page = last_page_for(total, n);
                let effective_page = page.max(1).min(last_page);
                Ok(ResourceListPage {
                    items,
                    total,
                    page: effective_page,
                    pagination,
                    unfiltered_total: None,
                })
            }
        }
    }

    /// Count resources of `class` matching `wheres`, with no pagination or
    /// hydration.
    ///
    /// Locks `conn`, delegates to `query::count_resources`, then drops the
    /// guard before returning. A thin wrapper like the other reads here —
    /// keeps `rusqlite::Connection` opaque outside the store.
    ///
    /// Used by the page builder (`query::page`) to compute
    /// `ResourceListPage.unfiltered_total` (the "N of M" header) and a strip
    /// row's total: for a list, called with its static config `wheres`
    /// only (never the request-time facet `TargetIn` clauses) after
    /// `list_resources` has already run with the full merged `wheres`.
    pub fn count_resources(&self, class: &str, wheres: &[WhereClause]) -> Result<usize, QueryError> {
        let conn = self.store.lock();
        query::count_resources(&conn, class, wheres)
    }

    /// Fetch a single-member view of a compound Resource.
    ///
    /// Returns a `CompoundPage` with the hydrated member at the requested
    /// (1-based) `index`, plus a lightweight `MemberHeader` for every member
    /// (`CompoundPage.members`, used by the CV-2 contents rail). Mirrors
    /// `list_resources` for the lock protocol: the `conn` guard is dropped
    /// inside a block — after fetching `compound_part_headers` once — before
    /// any per-member hydration call. The member id list is derived from the
    /// fetched headers (`headers.iter().map(|h| h.id.clone())`) rather than a
    /// second `query::compound_parts` call, so only one query runs under the
    /// lock.
    ///
    /// - `total == 0`: the compound has no members (empty/non-compound source). Returns
    ///   `CompoundPage { member: None, index: 1, total: 0, members: vec![], member_media: vec![]
    ///   }`.
    /// - `total >= 1`: `index` is clamped to `1..=total` (out-of-range or zero → nearest boundary).
    ///   The member at the clamped position is hydrated via `get_resource`; the clamped 1-based
    ///   index is stored in `CompoundPage.index`; `members.len() == total`.
    ///
    /// `page` is the RAW (still percent-/`+`-encoded) `?page=` value, if any.
    /// It is resolved against the
    /// fetched headers by [`resolve_page_param`] — no extra SQL — and, when
    /// it resolves, **beats** `index` (the `?member=N` value). An absent,
    /// empty, unresolvable or ambiguous `page` falls back to `index` (so a
    /// request carrying both an unresolvable `page` and a valid `member`
    /// keeps the `member`), which itself defaults to 1 at the call site.
    ///
    /// `media_property` is the `compound-viewer media="…"` attribute.
    /// When `Some`, `CompoundPage.member_media`
    /// is filled with the hydrated member's linked Images via
    /// [`ProjectReads::media_items`] (Seqnum order, host→image link per dsp-incubator
    /// cpe/docs/adr/0008-images-are-embedded-only-resources.md); when `None`, `member_media`
    /// stays empty and no extra query runs.
    ///
    /// `annotations` is the `compound-viewer annotations=` attribute: when
    /// `true`, `CompoundPage.annotations` holds the shown member's register
    /// ([`ProjectReads::page_annotations`]) and `annotated_members` the compound's
    /// annotated members ([`ProjectReads::annotated_members`]); when `false`, both
    /// stay empty and neither query runs. A failure of either query fails the
    /// whole call (the reader answers 500), unlike the Apparat's resource
    /// links, which degrade to empty: the annotation rows are boot-validated
    /// (`MalformedAnnotation`), so a failure here is a broken store, not data.
    pub fn compound_parts(
        &self,
        parent_id: &str,
        index: u32,
        page: Option<&str>,
        media_property: Option<&str>,
        annotations: bool,
        lang: &Lang,
    ) -> Result<CompoundPage, QueryError> {
        let headers = {
            let conn = self.store.lock();
            query::compound_part_headers(&conn, parent_id, lang)?
            // guard dropped here
        };

        if headers.is_empty() {
            // No first page to name, so a stale `?page=` sets no status.
            return Ok(CompoundPage {
                member: None,
                index: 1,
                total: 0,
                members: vec![],
                member_media: vec![],
                stale_page: None,
                annotations: vec![],
                annotated_members: BTreeSet::new(),
            });
        }

        let total = headers.len();
        let ids: Vec<String> = headers.iter().map(|h| h.id.clone()).collect();
        // Clamp in usize space: convert the 1-based u32 index → usize (saturate on
        // overflow), clamp to [1, total], then subtract 1 for the 0-based slice.
        // Never use `as` for numeric narrowing.
        // `?page=` wins over `?member=` when it resolves;
        // otherwise fall back to the clamped `?member=` position.
        let resolved = page.and_then(|raw| resolve_page_param(&headers, raw));
        // A non-empty `?page=` that names no page (unknown or ambiguous) is
        // kept, decoded, so the reader can say so; the
        // page shown is still the `?member=` fallback.
        let stale_page = match (page, resolved) {
            (Some(raw), None) => {
                let decoded = cpe_core::util::decode_query_value(raw);
                (!decoded.trim().is_empty()).then_some(decoded)
            }
            _ => None,
        };
        let pos = resolved.unwrap_or_else(|| usize::try_from(index).unwrap_or(usize::MAX).clamp(1, total) - 1);

        let member = self.get_resource(&ids[pos], lang)?.expect(
            "compound_part_headers returned an id absent from resources \
             — unreachable: read-only in-memory DB has no concurrent writes",
        );

        // Convert the 0-based pos back to a 1-based u32.
        // pos + 1 ≤ total which is bounded by usize; for any realistic compound
        // this fits in u32. Use try_from with expect naming the invariant.
        let effective_index =
            u32::try_from(pos + 1).expect("pos+1 ≤ total which is bounded by compound size — fits in u32");

        let member_media = match media_property {
            Some(property) => self.media_items(&member.id, property, lang)?,
            None => vec![],
        };

        let (register, annotated_members) = if annotations {
            (self.page_annotations(&member.id, lang)?, self.annotated_members(parent_id)?)
        } else {
            (vec![], BTreeSet::new())
        };

        Ok(CompoundPage {
            member: Some(member),
            index: effective_index,
            total,
            members: headers,
            member_media,
            stale_page,
            annotations: register,
            annotated_members,
        })
    }

    /// Lightweight member headers for `parent_id`, without hydrating any
    /// member's full `Resource`.
    /// Unlike `compound_parts` (built for the compound-viewer's own
    /// paging, which always hydrates the member at an index), a
    /// `resource-detail` page only needs to know WHETHER its own resource is a
    /// compound parent and, if so, list its members by id/class/name — it
    /// never pages into one from this call site.
    ///
    /// Returns an empty `Vec` when `parent_id` has no `compound_parts` rows
    /// (the common case for every non-compound host — Alice's Characters,
    /// Locations, and StoryChapters are never compound parents) — the
    /// renderer's contract is to omit `.rv-contents` entirely in that case.
    pub(crate) fn compound_member_headers(
        &self,
        parent_id: &str,
        lang: &Lang,
    ) -> Result<Vec<MemberHeader>, QueryError> {
        let conn = self.store.lock();
        query::compound_part_headers(&conn, parent_id, lang)
    }

    /// Every `spec.class` resource that links TO `target_id` through one of
    /// `spec.properties` — the reverse resolution behind a `resource-detail`'s
    /// `incoming-links` child.
    ///
    /// Locks `conn`, runs `query::incoming_links`, and drops the guard on
    /// return. An empty `properties` list or a target nothing links to both
    /// return `IncomingLinks { rows: vec![], total: 0 }`. `spec.limit` caps
    /// `rows`; `total` is always the uncapped match count. The render-only
    /// spec fields (`label`, `grouping.layout`, `grouping.note_label`) are
    /// not read here.
    pub(crate) fn incoming_links(
        &self,
        spec: &IncomingLinksSpec,
        target_id: &str,
        lang: &Lang,
    ) -> Result<IncomingLinks, QueryError> {
        let conn = self.store.lock();
        query::incoming_links(&conn, spec, target_id, lang)
    }

    /// The annotation register of `resource_id` (dsp-incubator
    /// cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md): its annotations
    /// as numbered entries (larger box first, ties by id), then one
    /// unnumbered entry per link that targets the resource itself, in
    /// link-id order. Each entry carries its partners, ungrouped and
    /// undeduplicated (see [`AnnotationPartner`]). Empty for a resource with
    /// neither.
    pub fn page_annotations(&self, resource_id: &str, lang: &Lang) -> Result<Vec<AnnotationEntry>, QueryError> {
        let conn = self.store.lock();
        let mut entries = Vec::new();
        for (number, annotation) in query::annotations_on(&conn, resource_id)? {
            let partners = query::annotation_partners(&conn, query::EntryTarget::Annotation(&annotation.id), lang)?;
            entries.push(AnnotationEntry::Annotation { number, annotation, partners });
        }
        let links = query::links_targeting(&conn, resource_id)?;
        if links.is_empty() {
            return Ok(entries);
        }
        let mut by_link: BTreeMap<String, Vec<AnnotationPartner>> = BTreeMap::new();
        for partner in query::annotation_partners(&conn, query::EntryTarget::Resource(resource_id), lang)? {
            by_link.entry(partner.link_id.clone()).or_default().push(partner);
        }
        entries.extend(
            links
                .into_iter()
                .map(|link| AnnotationEntry::Link { partners: by_link.remove(&link.id).unwrap_or_default(), link }),
        );
        Ok(entries)
    }

    /// The members of compound `parent_id` that have an annotation register
    /// (at least one annotation, or a link targeting the member itself) —
    /// the set the reader's signposts mark. One query per compound.
    pub fn annotated_members(&self, parent_id: &str) -> Result<BTreeSet<String>, QueryError> {
        let conn = self.store.lock();
        query::annotated_members(&conn, parent_id)
    }

    /// The resource-level link partners of `resource_id`: the other resources
    /// that share an annotation link with it, ordered by link id then target
    /// order (see [`ResourceLink`]).
    pub fn resource_links(&self, resource_id: &str, lang: &Lang) -> Result<Vec<ResourceLink>, QueryError> {
        let conn = self.store.lock();
        query::resource_links(&conn, resource_id, lang)
    }

    /// The annotations `ids` with what a figure cut from each one's resource
    /// image needs (see [`AnnotationCrop`]), in the order of `ids`; an
    /// unknown id is omitted. `media_property` names the annotated
    /// resource's `link` property to its image.
    pub(crate) fn annotation_crops(
        &self,
        ids: &[String],
        media_property: &str,
        lang: &Lang,
    ) -> Result<Vec<AnnotationCrop>, QueryError> {
        let conn = self.store.lock();
        query::annotation_crops(&conn, ids, media_property, lang)
    }

    /// The resources `ids` with what a whole-page figure needs (see
    /// [`PagePlate`]), in the order of `ids`; an unknown id is omitted.
    /// `media_property` names each resource's `link` property to its image.
    pub(crate) fn page_plates(
        &self,
        ids: &[String],
        media_property: &str,
        lang: &Lang,
    ) -> Result<Vec<PagePlate>, QueryError> {
        let conn = self.store.lock();
        query::page_plates(&conn, ids, media_property, lang)
    }

    /// The annotation links `ids`, in the order of `ids`; an unknown id is
    /// omitted.
    pub(crate) fn annotation_links_by_id(&self, ids: &[String]) -> Result<Vec<AnnotationLink>, QueryError> {
        let conn = self.store.lock();
        query::annotation_links_by_id(&conn, ids)
    }

    /// Compute facet option groups for a single facet on a given class.
    ///
    /// Locks `conn` briefly to run `query::list_facet_values`, drops the guard
    /// before building the `FacetGroup` struct (never holds the guard across
    /// the build — mirrors the lock discipline in `list_resources` /
    /// `compound_parts`).
    ///
    /// `base_wheres` are the resource-list config `where` clauses (static
    /// filters) that apply to the baseline population. The counts are computed
    /// against this baseline only — NOT affected by `active_ids` (counts are
    /// static per design; they never vanish when an option is selected).
    ///
    /// `active_ids` is post-processing only: it sets `FacetOption::selected`
    /// (membership test) in Rust. It is NOT a SQL parameter and must NOT be
    /// passed into `list_facet_values`.
    ///
    /// Returns one `FacetGroup` with:
    /// - `label` = the declared `labels { property "…" { … } }` translation for `lang` when
    ///   `property_labels` (the project's `site.property_labels`) has one for `facet.property`,
    ///   else the humanized property name (the language-independent fallback).
    /// - `kind` = `facet.kind`.
    /// - `options` = one `FacetOption` per returned `(node_id, label_opt, count)`, with
    ///   `label_opt.unwrap_or_else(|| node_id.clone())` as the orphan-node fallback.
    pub fn facet_options(
        &self,
        class: &str,
        facet: &Facet,
        base_wheres: &[WhereClause],
        active_ids: &[String],
        property_labels: &[PropertyLabel],
        lang: &Lang,
    ) -> Result<FacetGroup, QueryError> {
        let rows = {
            let conn = self.store.lock();
            query::list_facet_values(&conn, class, &facet.property, base_wheres, lang)?
            // guard dropped here
        };

        let options = rows
            .into_iter()
            .map(|(node_id, label_opt, count)| {
                let label = label_opt.unwrap_or_else(|| node_id.clone());
                let selected = active_ids.iter().any(|a| a == &node_id);
                FacetOption { value: node_id, label, count, selected }
            })
            .collect();

        let label = cpe_core::labels::resolve_property_label(property_labels, &facet.property, lang);

        Ok(FacetGroup {
            property: facet.property.clone(),
            label,
            kind: facet.kind,
            options,
        })
    }

    /// The list nodes `class`'s `property` values point at, in vocabulary
    /// (`list_nodes.seq`) order, each labelled in `lang` (else any language).
    ///
    /// The lane vocabulary of a `timeline`. Not
    /// [`ProjectReads::facet_options`]: that one orders by value count. Locks
    /// `conn` for the one query and drops the guard before returning.
    pub(crate) fn list_nodes_for_property(
        &self,
        class: &str,
        property: &str,
        lang: &Lang,
    ) -> Result<Vec<LaneNode>, QueryError> {
        let conn = self.store.lock();
        query::list_nodes_for_property(&conn, class, property, lang)
    }

    /// Hydrate a slice of resource ids into `Resource` values.
    ///
    /// An id returned by `list_resource_ids` that resolves to `None` via
    /// `get_resource` is an invariant violation: the read-only in-memory DB
    /// has no concurrent writers, so an id present at list time must still
    /// exist at hydration time. We assert this with `expect` — a panic here
    /// names the invariant and is the correct outcome.
    fn hydrate_ids(&self, ids: &[String], lang: &Lang) -> Result<Vec<Resource>, QueryError> {
        let mut items = Vec::with_capacity(ids.len());
        for id in ids {
            let resource = self.get_resource(id, lang)?.expect(
                "list_resource_ids returned an id absent from resources \
                 — unreachable: read-only in-memory DB has no concurrent writes",
            );
            items.push(resource);
        }
        Ok(items)
    }

    /// Fetch the ordered, hydrated Image resources a host resource points at
    /// via its `link`-typed `property`.
    ///
    /// Mirrors `compound_parts`'s two-phase lock protocol exactly: `conn` is
    /// locked once for `query::media_item_ids`, the guard is dropped, then
    /// each id is hydrated via `self.get_resource` (which re-locks-and-drops
    /// independently) — the mutex is never held across the hydration loop.
    ///
    /// Returns an empty `Vec` when `resource_id` has no values for `property`
    /// (not an error).
    pub fn media_items(&self, resource_id: &str, property: &str, lang: &Lang) -> Result<Vec<Resource>, QueryError> {
        let ids = {
            let conn = self.store.lock();
            query::media_item_ids(&conn, resource_id, property)?
            // guard dropped here
        };

        self.hydrate_ids(&ids, lang)
    }

    /// Batch-fetch the first (lowest-seqnum) media target's representation
    /// URI for each id in `ids`, keyed by id.
    ///
    /// Used to render browse-row thumbnails for a class whose media property
    /// is `class_media_property` — one query for the whole page's rows, not
    /// N+1. Locks `conn`, delegates to `query::media_thumb_uris`, drops the
    /// guard before returning (no post-hydration needed — the result is raw
    /// URIs, not full `Resource`s). Ids that resolve to no image are simply
    /// absent from the returned map (not an error).
    pub(crate) fn media_thumbs(
        &self,
        class_media_property: &str,
        ids: &[String],
    ) -> Result<HashMap<String, MediaThumb>, QueryError> {
        let conn = self.store.lock();
        query::media_thumb_uris(&conn, class_media_property, ids)
    }
}

// ── Pagination helpers ────────────────────────────────────────────────────────

/// Compute the last (highest) valid page number for `total` items with `per_page`
/// items per page.
///
/// Returns `1` when `total == 0` (there is always at least one page, even an
/// empty one — this prevents `clamp(1, 0)` which would panic).
/// Uses integer ceiling division: `ceil(total / per_page)`.
///
/// Extracted as a standalone pure function so it can be unit-tested without a DB.
pub fn last_page_for(total: usize, per_page: u32) -> u32 {
    if total == 0 || per_page == 0 {
        return 1;
    }
    // Ceiling division: (total + per_page - 1) / per_page, saturating to u32::MAX.
    let per = usize::try_from(per_page).unwrap_or(usize::MAX);
    let pages = total.div_ceil(per);
    // Cast: `pages` ≤ `total` (which is a usize). For any realistic dataset this
    // fits in u32; saturate rather than truncate silently.
    pages.try_into().unwrap_or(u32::MAX)
}
