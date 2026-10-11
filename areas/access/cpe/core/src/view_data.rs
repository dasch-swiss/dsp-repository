//! The per-request view data: what the store builds for one page request and
//! page assembly reads to render it.
//!
//! Plain data over `ir`, in `cpe-core`. `ComponentData` is the per-slot
//! carrier, `PageData` the whole bundle (the slots, the page's status and the
//! side data `RenderContext` borrows), and `PageRequest`/`PageParams` what the
//! store needs to know about the request. This module imports only `ir` (and
//! `std`): nothing here renders, queries or parses a request.

use std::collections::HashMap;

use crate::ir::{
    CompoundPage, EditionCardsPayload, FacetGroup, IncomingLinks, MediaThumb, MemberHeader, OrderOption, Resource,
    ResourceLink, ResourceListPage, Sort, StripRowPayload, TimelinePayload, WoodcutCasesPayload,
};

// ── ComponentData — unified per-slot carrier ──────────────────────────────────

/// The unified per-slot data carrier for `render_page`.
///
/// Aligned 1:1 with `page.body`; each variant carries the data for the
/// component at that position. Built by the store (`query::page`);
/// `render_page` zips `page.body` against a `&[ComponentData]` slice so each
/// `render_component` call receives exactly its own slot.
///
/// Variant semantics:
/// - `Detail(Some(r))` — resource found and hydrated.
/// - `Detail(None)` — resource not found → `render_not_found` body.
/// - `List(page)` — paginated list; an empty list (`total == 0`) is valid.
/// - `Compound(page)` — compound viewer page; an empty compound is valid.
/// - `Facets(groups)` — facet-panel groups built by `ProjectReads::facet_options`; an empty `Vec`
///   means no facets are configured or all produced no options. Rendered by the `FacetPanel`
///   component arm; ignored by `compose_page_title` and `first_resolved_resource` (no resource
///   title).
/// - `Timeline(payload)` — a `timeline`'s resources plus its lane vocabulary in `seq` order; an
///   empty `resources` list is valid.
/// - `WoodcutCases(payload)` — the annotations and links a `woodcut-cases` block names; an unknown
///   id is simply absent.
/// - `EditionCards(payload)` / `StripRow(payload)` — the books of an `edition-cards` block, the
///   strips of a `strip-row`.
/// - `Other` — non-data component (narrative, …); no fetch was needed.
#[derive(Debug, PartialEq)]
pub enum ComponentData {
    /// A resource-detail slot. `None` = resource-not-found → not-found body.
    Detail(Option<Resource>),
    /// A resource-list slot. Always carries a page (empty list = `total == 0`).
    List(ResourceListPage),
    /// A compound-viewer slot. Always carries a page (empty = `total == 0`).
    Compound(CompoundPage),
    /// A facet-panel slot. Carries the hydrated option groups for the renderer.
    /// Empty vec = no facets configured (or all produced no options).
    Facets(Vec<FacetGroup>),
    /// A timeline slot. Always carries the class's resources (possibly none)
    /// and the lane vocabulary.
    Timeline(TimelinePayload),
    /// A `woodcut-cases` slot: the crops and links its cases name.
    WoodcutCases(WoodcutCasesPayload),
    /// An `edition-cards` slot: its books with their cover images.
    EditionCards(EditionCardsPayload),
    /// A `strip-row` slot: its strips, their tiles and the class's total.
    StripRow(StripRowPayload),
    /// A non-data component (narrative, page, …). No fetch; no payload.
    Other,
}

// ── Outcome and status ────────────────────────────────────────────────────────

/// The outcome of one data-backed slot's fetch.
///
/// The store records one per fetching slot; [`PageStatus::from_outcomes`]
/// rolls them up into the page's status.
#[derive(Debug, PartialEq, Eq)]
pub enum FetchOutcome {
    /// The slot's data loaded.
    Found,
    /// The slot names a resource the project does not have (a
    /// `resource-detail` slot).
    NotFound,
    /// The slot's data could not be loaded: a query failed, or a route
    /// parameter the slot references is missing.
    Error,
}

/// The outcome of a whole page request, which the host maps to its response
/// status (200, 404, 500).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageStatus {
    /// Every data-backed slot was found, or the page has none.
    Ok,
    /// At least one slot's resource does not exist, and none failed.
    NotFound,
    /// At least one slot's fetch failed.
    Error,
}

impl PageStatus {
    /// Roll per-slot fetch outcomes up into the page's status.
    ///
    /// Precedence (highest wins):
    /// 1. Any `Error` → [`PageStatus::Error`].
    /// 2. Any `NotFound` → [`PageStatus::NotFound`].
    /// 3. All `Found` (or an empty slice) → [`PageStatus::Ok`].
    ///
    /// An empty slice means the page has no data-backed components and is
    /// always `Ok`.
    pub fn from_outcomes(outcomes: &[FetchOutcome]) -> Self {
        if outcomes.contains(&FetchOutcome::Error) {
            return PageStatus::Error;
        }
        if outcomes.contains(&FetchOutcome::NotFound) {
            return PageStatus::NotFound;
        }
        PageStatus::Ok
    }
}

// ── The request ───────────────────────────────────────────────────────────────

/// Which response the page data is for, and so which side data the store
/// looks up: it runs only the lookups the scope needs.
///
/// | Side data | `Page` | `CompoundFragment` | `ListFragment` |
/// |---|---|---|---|
/// | `media_items` | yes | yes | empty |
/// | `media_thumbs` | yes | yes | yes |
/// | `compound_members` | yes | yes | empty |
/// | `compound_parents` | yes | yes | empty |
/// | `incoming_links` | yes | yes | `None` |
/// | `resource_links` | yes | empty | empty |
///
/// The two fragment scopes run no side lookup at all when the page's status
/// is [`PageStatus::Error`]; `Page` still runs them, since a full page renders
/// with its error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageScope {
    /// A full page render.
    Page,
    /// The compound viewer's `_compound` fragment (forward member paging).
    CompoundFragment,
    /// A list refinement fragment: `_facets`, `_sort` or `_search`.
    ListFragment,
}

/// The request values page data depends on.
///
/// Declared here and implemented in `dsp-incubator cpe/bin`, so every
/// query-string reader and decode point stays there: the store asks for
/// a value at the point it needs it and never parses the query string.
///
/// `Sync`, so a `PageRequest` (which holds `&dyn PageParams`) is `Send` and
/// an async handler holding one across an `.await` stays `Send`.
pub trait PageParams: Sync {
    /// `?page=` read as a browse page number (1-based).
    fn list_page(&self) -> u32;
    /// `?member=`, the compound viewer's 1-based member index.
    fn member(&self) -> u32;
    /// `?page=` raw, undecoded: the store decodes it once when it resolves a
    /// compound member by id, name or prefix.
    fn reader_page(&self) -> Option<&str>;
    /// The sort order for a list: a `?sort=` override when it names a
    /// built-in or one of `options`, else `config`. `name_property` is the
    /// list's `order-name-property`, which the built-in name rows order by.
    fn sorts(&self, config: &[Sort], options: &[OrderOption], name_property: Option<&str>) -> Vec<Sort>;
    /// The selected values of the facet over `property`, deduplicated and
    /// capped.
    fn facet_values(&self, property: &str) -> Vec<String>;
    /// `?q=`, decoded; empty when absent.
    fn search(&self) -> String;
}

/// One page request, as the store needs it.
pub struct PageRequest<'a> {
    /// The route's `:param` values by name.
    pub route_params: &'a HashMap<String, String>,
    /// The request's query values.
    pub params: &'a dyn PageParams,
    /// Which response this is for (see [`PageScope`]).
    pub scope: PageScope,
}

// ── The bundle ────────────────────────────────────────────────────────────────

/// Everything one page request renders from: the per-slot data, the page's
/// status and the side data `RenderContext` borrows.
///
/// A side field the request's [`PageScope`] does not ask for is empty (or
/// `None`).
pub struct PageData {
    /// One entry per `page.body` component, aligned 1:1 with it. On
    /// [`PageStatus::NotFound`] every compound slot is
    /// [`ComponentData::Other`].
    pub components: Vec<ComponentData>,
    /// The page's status, rolled up from the slots' fetch outcomes.
    pub status: PageStatus,
    /// The `resource-detail` resource's linked images, in order, for the
    /// record viewer; empty when none resolved or its class has no media
    /// property.
    pub media_items: Vec<Resource>,
    /// Row thumbnails by resource id for the first `resource-list` with a
    /// `media` property.
    pub media_thumbs: HashMap<String, MediaThumb>,
    /// The `resource-detail` resource's own compound member headers.
    pub compound_members: Vec<MemberHeader>,
    /// The parent resource of each compound that resolved with no members,
    /// keyed by source id.
    pub compound_parents: HashMap<String, Resource>,
    /// The `resource-detail` resource's `incoming-links` rows. `None` when the
    /// record declares none, did not resolve, or the lookup failed; never `Some(empty)` on a
    /// failure.
    pub incoming_links: Option<IncomingLinks>,
    /// The `resource-detail` resource's annotation-link partners, looked up
    /// only when it declares a `group links-label=`.
    pub resource_links: Vec<ResourceLink>,
}

#[cfg(test)]
mod tests {
    use super::{FetchOutcome, PageStatus};

    #[test]
    fn from_outcomes_empty_slice_is_ok() {
        assert_eq!(PageStatus::from_outcomes(&[]), PageStatus::Ok);
    }

    #[test]
    fn from_outcomes_all_found_is_ok() {
        let outcomes = [FetchOutcome::Found, FetchOutcome::Found];
        assert_eq!(PageStatus::from_outcomes(&outcomes), PageStatus::Ok);
    }

    #[test]
    fn from_outcomes_any_not_found_is_not_found() {
        let outcomes = [FetchOutcome::Found, FetchOutcome::NotFound];
        assert_eq!(PageStatus::from_outcomes(&outcomes), PageStatus::NotFound);
        assert_eq!(PageStatus::from_outcomes(&[FetchOutcome::NotFound]), PageStatus::NotFound);
    }

    #[test]
    fn from_outcomes_any_error_is_error() {
        let outcomes = [FetchOutcome::Found, FetchOutcome::Error];
        assert_eq!(PageStatus::from_outcomes(&outcomes), PageStatus::Error);
        assert_eq!(PageStatus::from_outcomes(&[FetchOutcome::Error]), PageStatus::Error);
    }

    #[test]
    fn from_outcomes_error_takes_precedence_over_not_found() {
        let outcomes = [FetchOutcome::NotFound, FetchOutcome::Error];
        assert_eq!(PageStatus::from_outcomes(&outcomes), PageStatus::Error);
    }
}
