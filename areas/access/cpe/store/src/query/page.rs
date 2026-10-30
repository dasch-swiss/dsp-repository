//! The store's page builder: one [`PageData`] per page request, assembled
//! from [`ProjectReads`].
//!
//! [`page_data`] walks `page.body` and fetches each slot's data (the
//! per-slot [`ComponentData`]), rolls the slots' fetch outcomes up into the
//! page's [`PageStatus`], and then runs the side-data lookups
//! `RenderContext` borrows, only those the request's scope needs: which scope
//! runs which lookup is the table on [`PageScope`].
//!
//! A fragment scope whose status is [`PageStatus::Error`] runs no side
//! lookup: its host answers a bare 500 without reading any.
//!
//! Pure assembly: every read goes through a `ProjectReads` method, one after
//! another, and this module never takes the connection itself (the reads
//! module owns that protocol). The request values come through
//! [`PageParams`](cpe_core::view_data::PageParams), so the host keeps every query-string reader and
//! decode point.

use std::collections::{HashMap, HashSet};

use cpe_core::ir::{
    any_facet_selected, Component, CropImage, EditionBook, EditionCardsPayload, FacetGroup, IncomingLinks, MediaThumb,
    MemberHeader, Page, Pagination, PropertyLabel, RepresentationKind, Resource, ResourceLink, ShowcaseCase,
    ShowcaseRemark, Sort, SortDir, SortKey, StripRowPayload, TimelinePayload, Value, WhereClause, WoodcutCasesPayload,
};
use cpe_core::lang::Lang;
use cpe_core::util::is_active_search_query;
use cpe_core::view_data::{ComponentData, FetchOutcome, PageData, PageRequest, PageScope, PageStatus};

use crate::query::reads::ProjectReads;
use crate::query::search_index_name;

/// Build the page data for one request: the per-slot data, the page's status
/// and the side data the request's scope asks for (see the module doc).
///
/// `media_properties` is the project's class → media-property map and
/// `property_labels` its `site.property_labels` (facet group labels).
///
/// The side lookups run after the 404 downgrade, on the downgraded
/// components, in the order `media_items`, `media_thumbs`,
/// `compound_members`, `compound_parents`, `incoming_links`,
/// `resource_links`. Each degrades on its own: a failed lookup is logged and
/// leaves its field empty (or `None`), never changing the status.
pub fn page_data(
    reads: &ProjectReads<'_>,
    media_properties: &HashMap<String, String>,
    property_labels: &[PropertyLabel],
    page: &Page,
    request: &PageRequest<'_>,
    lang: &Lang,
) -> PageData {
    let (components, status) = build_components(reads, property_labels, page, request, lang);

    // Which lookups run: (thumbs, record, links), the columns of the scope
    // table. "record" is media items, compound members and parents, and
    // incoming links. A fragment that errored runs none.
    let ok = status != PageStatus::Error;
    let (thumbs, record, links) = match request.scope {
        PageScope::Page => (true, true, true),
        PageScope::CompoundFragment => (ok, ok, false),
        PageScope::ListFragment => (ok, false, false),
    };

    let media_items = if record {
        media_items_for_page(reads, media_properties, &components, lang)
    } else {
        Vec::new()
    };
    let media_thumbs = if thumbs {
        media_thumbs_for_page(reads, page, &components)
    } else {
        HashMap::new()
    };
    let compound_members = if record {
        compound_members_for_page(reads, &components, lang)
    } else {
        Vec::new()
    };
    let compound_parents = if record {
        compound_parents_for_page(reads, page, &components, request.route_params, lang)
    } else {
        HashMap::new()
    };
    let incoming_links = if record {
        incoming_links_for_page(reads, page, &components, lang)
    } else {
        None
    };
    let resource_links = if links {
        resource_links_for_page(reads, page, &components, lang)
    } else {
        Vec::new()
    };

    PageData {
        components,
        status,
        media_items,
        media_thumbs,
        compound_members,
        compound_parents,
        incoming_links,
        resource_links,
    }
}

/// Build the unified `ComponentData` carrier for a page using a two-pass walk.
///
/// **Why two passes:** a `FacetPanel` may appear before its target `ResourceList`
/// in `page.body` (config order is free). Both components need cross-component
/// data: the list needs the panel's active filters, the panel needs the list's
/// class. A single forward pass cannot satisfy both orderings.
///
/// **Pass 1** (cheap pre-scan): build three maps from `page.body`:
/// - `list_meta`: `list-id → (class, config-wheres)` from named `ResourceList` components.
/// - `facet_wheres`: `list-id → Vec<WhereClause::TargetIn>` derived from each `FacetPanel`'s active
///   query params.
/// - `search_meta`: `list-id → (searches, decoded ?q= value)` from each `SearchBar`'s `target` —
///   `searches` lives on the `SearchBar`, not the `ResourceList`, so pass 2 can't recover it
///   otherwise (analogous to `facet_wheres`). The `q` value is [`PageParams::search`]'s (the host
///   percent- AND `+`-decodes it — `q` is the first genuinely free-text query value, so unlike
///   every other query value it cannot skip decoding), and recorded even when empty/inactive;
///   presence of an entry (not its activity) is what tells pass 2 "a search-bar targets this list
///   at all".
///
/// **Pass 2** (emit): walk `page.body` again, building `ComponentData` per slot:
/// - `ResourceList` queries with the config `wheres` merged with any `facet_wheres` accumulated for
///   its id, plus a `WhereClause::Search` when `search_meta` has an entry for its id AND the
///   recorded query is active (`is_active_search_query`, the same tokenizer-backed test
///   `fts_match_expr` uses — never raw-string non-emptiness).
/// - `FacetPanel` looks up its target list's `(class, base_wheres)` from `list_meta` and calls
///   `reads.facet_options` for each facet.
/// - All other components unchanged.
///
/// Positionally aligned with `page.body`. `request.route_params` supplies
/// values for `$param` references (empty for static pages; the route's
/// `:param` map for parameterised pages).
///
/// `request.params` supplies the request's query values:
/// - `ResourceList`: the browse page (`list_page`) and the sort (`sorts`).
/// - `CompoundViewer`: `member` and the raw `?page=` (`reader_page`; `page` wins when it resolves —
///   see `ProjectReads::compound_parts`).
/// - `FacetPanel`: each facet's selected values (`facet_values`).
/// - `SearchBar`: the decoded `?q=` (`search`).
///
/// Returns `(components, status)` where `status` is the highest severity seen
/// across all fetches (`Error` > `NotFound` > `Found`/`Other`), via
/// `PageStatus::from_outcomes`. Empty compound (`total == 0`) is a normal
/// `Ok`. `Detail(None)` (resource not found) yields `NotFound`. A facet panel
/// render with DB error yields `Error` (like a list error).
///
/// [`PageParams::search`]: cpe_core::view_data::PageParams::search
fn build_components(
    reads: &ProjectReads<'_>,
    property_labels: &[PropertyLabel],
    page: &Page,
    request: &PageRequest<'_>,
    lang: &Lang,
) -> (Vec<ComponentData>, PageStatus) {
    let route_params = request.route_params;
    let params = request.params;

    // Pass 1: build list_meta and facet_wheres maps

    // list_meta: list id → (class, config wheres), borrowed from page body.
    // Lifetime: borrows from `page` (which borrows from `Project`'s config).
    let mut list_meta: HashMap<&str, (&str, &[WhereClause])> = HashMap::new();
    // facet_wheres: list id → accumulated TargetIn clauses from active query params.
    let mut facet_wheres: HashMap<String, Vec<WhereClause>> = HashMap::new();
    // search_meta: list-id (the search-bar's `target`) → (searches, raw q).
    // Read in pass 1 because `SearchBar` carries `searches` (not `ResourceList`),
    // and a SearchBar may appear BEFORE its target list in `page.body` (the
    // search row renders above the list) — pass 2's ResourceList arm needs
    // this pre-scanned, exactly like `facet_wheres`.
    let mut search_meta: HashMap<&str, (&[String], String)> = HashMap::new();

    for component in &page.body {
        match component {
            Component::ResourceList { id: Some(list_id), class, wheres, .. } => {
                list_meta.insert(list_id.as_str(), (class.as_str(), wheres.as_slice()));
            }
            Component::FacetPanel { target, facets } => {
                for facet in facets {
                    let ids = params.facet_values(&facet.property);
                    // Caller contract: only push a TargetIn clause when
                    // the id list is non-empty. An empty TargetIn would emit `AND 0`
                    // (never-match), which is a defensive dead branch not an intended path.
                    if !ids.is_empty() {
                        facet_wheres
                            .entry(target.clone())
                            .or_default()
                            .push(WhereClause::TargetIn { property: facet.property.clone(), target_ids: ids });
                    }
                }
            }
            Component::SearchBar { target, searches, .. } => {
                // Always record the raw query, even "" — pass 2 needs to know
                // whether a search-bar targets this list at all (to tell "no
                // search-bar on this list" apart from "search-bar present but
                // query inactive"), not just whether the query is active.
                // Multiple search-bars targeting the same list would last-write-
                // win here, mirroring `list_meta`'s duplicate-id behavior — a
                // defensive, shouldn't-happen case boot validation already
                // prevents (`DuplicateResourceListId`).
                let q = params.search();
                search_meta.insert(target.as_str(), (searches.as_slice(), q));
            }
            _ => {}
        }
    }

    // Pass 1.5: pre-resolve every FacetPanel's groups
    //
    // Computed ONCE, before pass 2, so the `ResourceList` arm below can gate
    // `has_active_facets` on the SAME resolved-selection data the
    // `FacetPanel` arm renders (`any_facet_selected`) — never on
    // `facet_wheres` non-emptiness (a hand-crafted bogus value like
    // `?role=NOPE` produces a non-empty `TargetIn` clause without selecting
    // any REAL option). A page has at
    // most one `FacetPanel` (`DuplicateSingletonComponent`), so `target`
    // collisions are not a concern in practice, but `HashMap` handles a
    // last-write-win degrade gracefully regardless.
    let mut facet_groups_by_target: HashMap<&str, Vec<FacetGroup>> = HashMap::new();
    let mut facet_panel_errored: HashSet<&str> = HashSet::new();
    for component in &page.body {
        let Component::FacetPanel { target, facets } = component else {
            continue;
        };
        // Boot invariant: FacetPanel target resolves to a same-page
        // resource-list id (validated by UnresolvedTarget at boot time).
        let (class, base_wheres) = list_meta.get(target.as_str()).expect(
            "boot invariant: FacetPanel target resolves to a same-page \
             resource-list id (UnresolvedTarget) and that ResourceList \
             carries id: Some(...) (also enforced by UnresolvedTarget boot validation)",
        );

        let mut groups: Vec<FacetGroup> = Vec::with_capacity(facets.len());
        for facet in facets {
            let active_ids = params.facet_values(&facet.property);
            match reads.facet_options(class, facet, base_wheres, &active_ids, property_labels, lang) {
                Ok(group) => groups.push(group),
                Err(e) => {
                    tracing::error!(
                        class,
                        property = facet.property.as_str(),
                        error = %e,
                        "database error computing facet options for FacetPanel"
                    );
                    facet_panel_errored.insert(target.as_str());
                    break;
                }
            }
        }
        if !facet_panel_errored.contains(target.as_str()) {
            facet_groups_by_target.insert(target.as_str(), groups);
        }
    }

    // Pass 2: emit ComponentData per body slot

    let mut components: Vec<ComponentData> = Vec::with_capacity(page.body.len());
    let mut outcomes: Vec<FetchOutcome> = Vec::new();

    for component in &page.body {
        match component {
            // ResourceDetail
            Component::ResourceDetail { id, .. } => {
                let resource_id = match id {
                    None => {
                        // Missing id: validation forbids this on a valid
                        // config; treat defensively as not-found rather than panic.
                        components.push(ComponentData::Detail(None));
                        outcomes.push(FetchOutcome::NotFound);
                        continue;
                    }
                    Some(raw_id) if raw_id.starts_with('$') => {
                        let param_name = &raw_id[1..];
                        match route_params.get(param_name) {
                            Some(v) => v.clone(),
                            None => {
                                tracing::error!(
                                    param = param_name,
                                    "route param referenced in resource-detail id not found in extracted path params"
                                );
                                components.push(ComponentData::Detail(None));
                                outcomes.push(FetchOutcome::Error);
                                continue;
                            }
                        }
                    }
                    Some(literal) => literal.clone(),
                };

                match reads.get_resource(&resource_id, lang) {
                    Ok(Some(resource)) => {
                        components.push(ComponentData::Detail(Some(resource)));
                        outcomes.push(FetchOutcome::Found);
                    }
                    Ok(None) => {
                        components.push(ComponentData::Detail(None));
                        outcomes.push(FetchOutcome::NotFound);
                    }
                    Err(e) => {
                        tracing::error!(
                            resource_id = resource_id.as_str(),
                            error = %e,
                            "database error fetching resource for ResourceDetail"
                        );
                        components.push(ComponentData::Detail(None));
                        outcomes.push(FetchOutcome::Error);
                    }
                }
            }

            // ResourceList
            // gloss/chips are render-only config (consumed by render_resource_list,
            // not the data query), so they are not needed to build ComponentData.
            Component::ResourceList {
                id,
                class,
                wheres,
                sorts,
                pagination,
                order_options,
                order_name_property,
                ..
            } => {
                // Merge config wheres with any facet-narrowing TargetIn clauses.
                // Only named lists (id=Some) can be facet targets; id-less lists
                // never have facet_wheres entries and always use the config wheres alone.
                let mut merged: Vec<WhereClause> = wheres.clone();
                if let Some(list_id) = id {
                    if let Some(fw) = facet_wheres.get(list_id.as_str()) {
                        merged.extend(fw.iter().cloned());
                    }
                }
                // `has_active_facets` gates whether to compute `unfiltered_total`
                // below (config wheres only) — deliberately NOT derived from
                // `facet_wheres` non-emptiness (a hand-crafted bogus value like
                // `?role=NOPE` produces a non-empty `TargetIn` clause that
                // matches zero real rows) but from the SAME
                // `any_facet_selected` predicate the pre-resolved
                // `facet_groups_by_target` (pass 1.5) and the narrowed-to-zero
                // rendering surface both use, so copy and count can never
                // disagree.
                let has_active_facets = id
                    .as_deref()
                    .and_then(|list_id| facet_groups_by_target.get(list_id))
                    .map(|groups| any_facet_selected(groups))
                    .unwrap_or(false);

                // Merge in the search-bar's narrowing clause, if this list has an
                // active search query targeting it (Component::SearchBar.target ==
                // this list's id). "Active" is decided by the same tokenizer as
                // `fts_match_expr` (via `is_active_search_query`), never raw-string
                // non-emptiness — a whitespace-/punctuation-only `?q=` must behave
                // like no search at all.
                let mut has_active_search = false;
                if let Some(list_id) = id {
                    if let Some((searches, q)) = search_meta.get(list_id.as_str()) {
                        if is_active_search_query(q) {
                            has_active_search = true;
                            merged.push(WhereClause::Search {
                                table: search_index_name(class, searches),
                                query: q.clone(),
                            });
                        }
                    }
                }

                // For Limit pagination, always pass page 1 (list_resources
                // ignores it for Limit, but we keep the call site consistent).
                let requested_page = match pagination {
                    Pagination::Limit(_) => 1,
                    Pagination::PerPage(_) => params.list_page(),
                };

                // `?sort=` runtime override — see `PageParams::sorts`.
                let sorts_to_use = params.sorts(sorts, order_options, order_name_property.as_deref());

                match reads.list_resources(class, &merged, &sorts_to_use, *pagination, requested_page, lang) {
                    Ok(mut list_page) => {
                        // "N of M" header data: only meaningful when facets or an
                        // active search actually narrowed this list. Computed post-hoc
                        // against the ORIGINAL config `wheres` (not `merged`, which also
                        // carries the facet TargetIn clauses and/or the search clause) —
                        // `list_resources` already ran above with the narrowed set, so
                        // this is one extra COUNT query.
                        if has_active_facets || has_active_search {
                            match reads.count_resources(class, wheres) {
                                Ok(unfiltered) => {
                                    list_page.unfiltered_total = Some(unfiltered);
                                }
                                Err(e) => {
                                    tracing::error!(
                                        class = class.as_str(),
                                        error = %e,
                                        "database error computing unfiltered count for ResourceList component"
                                    );
                                    components.push(ComponentData::Other);
                                    outcomes.push(FetchOutcome::Error);
                                    continue;
                                }
                            }
                        }
                        components.push(ComponentData::List(list_page));
                        outcomes.push(FetchOutcome::Found);
                    }
                    Err(e) => {
                        tracing::error!(
                            class = class.as_str(),
                            error = %e,
                            "database error fetching resource list for ResourceList component"
                        );
                        components.push(ComponentData::Other);
                        outcomes.push(FetchOutcome::Error);
                    }
                }
            }

            // CompoundViewer
            Component::CompoundViewer { source, media, annotations, .. } => {
                // Resolve source: literal id or `$param` reference.
                let source_id = if let Some(param_name) = source.strip_prefix('$') {
                    match route_params.get(param_name) {
                        Some(v) => v.clone(),
                        None => {
                            tracing::error!(
                                param = param_name,
                                "route param referenced in compound-viewer source not found in extracted path params"
                            );
                            components.push(ComponentData::Other);
                            outcomes.push(FetchOutcome::Error);
                            continue;
                        }
                    }
                } else {
                    source.clone()
                };

                // `?member=` as a 1-based u32 (fallible; clamps done inside
                // ProjectReads::compound_parts). Default 1 on all failure cases.
                let member = params.member();
                // `?page=<page-id|name|prefix>` — handed over RAW: `ProjectReads::compound_parts`
                // decodes it exactly once and, when it resolves, it beats
                // `member`. This is the single parse point.
                let page_param = params.reader_page();

                // `media="…"` rides on the
                // compound payload: `compound_parts` hydrates the member's
                // scan(s) into `member_media` in the same call, with no
                // second load; so does the annotation register when the
                // reader has `annotations=#true`.
                match reads.compound_parts(&source_id, member, page_param, media.as_deref(), *annotations, lang) {
                    Ok(compound_page) => {
                        // Empty compound (total == 0) is a normal 200 — not 404.
                        components.push(ComponentData::Compound(compound_page));
                        outcomes.push(FetchOutcome::Found);
                    }
                    Err(e) => {
                        tracing::error!(
                            source_id = source_id.as_str(),
                            error = %e,
                            "database error fetching compound parts for CompoundViewer"
                        );
                        components.push(ComponentData::Other);
                        outcomes.push(FetchOutcome::Error);
                    }
                }
            }

            // FacetPanel
            // Groups (and any DB-error state) were already resolved once in
            // pass 1.5 above — this arm only emits the ComponentData, it
            // never re-queries `ProjectReads::facet_options`.
            Component::FacetPanel { target, .. } => {
                if facet_panel_errored.contains(target.as_str()) {
                    // A DB error on any facet degrades the entire panel to Other + 500.
                    components.push(ComponentData::Other);
                    outcomes.push(FetchOutcome::Error);
                } else {
                    // Success: push Facets data; no FetchOutcome (panel render is not
                    // a 404-able fetch — same as Narrative/Other arms).
                    // Clone, not remove: the ResourceList arm reads the same map for
                    // `has_active_facets` and may come after the panel.
                    let groups = facet_groups_by_target.get(target.as_str()).cloned().unwrap_or_default();
                    components.push(ComponentData::Facets(groups));
                }
            }

            // Timeline
            // Every resource of the class, sorted by the date property, plus
            // the lane vocabulary in `seq` order. No config wheres, no facets,
            // no `?sort=`/`?page=` — the timeline is not a browse surface.
            Component::Timeline { class, date_property, lane_property, .. } => {
                let sorts = [Sort {
                    key: SortKey::Property(date_property.clone()),
                    direction: SortDir::Asc,
                }];
                // `TIMELINE_HYDRATION_LIMIT` is a guard against an unbounded
                // hydration, not a design choice: the timeline draws every
                // resource of its class, and Incunabula has 19 Books. If a
                // class ever has more resources than the limit, this
                // truncates the timeline without failing; the `tracing::warn!`
                // below is the signal. Note it also fires for a class of
                // exactly the limit (`items.len() == TIMELINE_HYDRATION_LIMIT`),
                // which is a false-positive-shaped warning, not proof of an
                // actual truncation.
                const TIMELINE_HYDRATION_LIMIT: u32 = 500;
                let resources =
                    reads.list_resources(class, &[], &sorts, Pagination::Limit(TIMELINE_HYDRATION_LIMIT), 1, lang);
                let lanes = reads.list_nodes_for_property(class, lane_property, lang);
                match resources.and_then(|resources| lanes.map(|lanes| (resources, lanes))) {
                    Ok((resources, lanes)) => {
                        // Widen the limit rather than narrow `.len()`
                        // (no narrowing `as`).
                        if resources.items.len()
                            == usize::try_from(TIMELINE_HYDRATION_LIMIT).expect("u32 constant always fits in usize")
                        {
                            tracing::warn!(
                                class = class.as_str(),
                                limit = TIMELINE_HYDRATION_LIMIT,
                                "timeline: resource count hit the hydration limit; the timeline may be truncated"
                            );
                        }
                        components.push(ComponentData::Timeline(TimelinePayload { resources, lanes }));
                        outcomes.push(FetchOutcome::Found);
                    }
                    Err(e) => {
                        tracing::error!(
                            class = class.as_str(),
                            error = %e,
                            "database error fetching resources or lanes for Timeline component"
                        );
                        components.push(ComponentData::Other);
                        outcomes.push(FetchOutcome::Error);
                    }
                }
            }

            // Woodcut cases
            // Every crop and detail annotation the cases name (with its page,
            // book and image), every link they name (remarks, page cases),
            // and every page a page case names (with its book and image).
            Component::WoodcutCases { media, cases, .. } => {
                let mut annotation_ids: Vec<String> = Vec::new();
                let mut link_ids: Vec<String> = Vec::new();
                let mut page_ids: Vec<String> = Vec::new();
                for case in cases {
                    match case {
                        ShowcaseCase::Crops { crops, detail, .. } => {
                            annotation_ids.extend(crops.iter().chain(detail.iter()).map(|c| c.annotation.clone()));
                        }
                        ShowcaseCase::Pages { link, pages, .. } => {
                            link_ids.push(link.clone());
                            page_ids.extend(pages.iter().cloned());
                        }
                    }
                    if let Some(ShowcaseRemark::Link(link)) = case.remark() {
                        link_ids.push(link.clone());
                    }
                }
                let fetched = reads.annotation_crops(&annotation_ids, media, lang).and_then(|crops| {
                    let links = reads.annotation_links_by_id(&link_ids)?;
                    let pages = reads.page_plates(&page_ids, media, lang)?;
                    Ok(WoodcutCasesPayload { crops, links, pages })
                });
                match fetched {
                    Ok(payload) => {
                        components.push(ComponentData::WoodcutCases(payload));
                        outcomes.push(FetchOutcome::Found);
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "database error fetching crops, links or pages for WoodcutCases component"
                        );
                        components.push(ComponentData::Other);
                        outcomes.push(FetchOutcome::Error);
                    }
                }
            }

            // Edition cards
            // Each card's book that exists, in card order, with the first
            // image representation of its media property's first target.
            Component::EditionCards { media, books, .. } => {
                let fetched: Result<Vec<_>, _> = books
                    .iter()
                    .filter_map(|card| {
                        let resource = match reads.get_resource(&card.book, lang) {
                            Ok(resource) => resource?,
                            Err(e) => return Some(Err(e)),
                        };
                        let cover = reads.media_items(&card.book, media, lang).map(|images| {
                            images
                                .iter()
                                .flat_map(|image| &image.representations)
                                .find(|r| r.kind == RepresentationKind::Image)
                                .map(|r| CropImage { uri: r.uri.clone(), width: r.width, height: r.height })
                        });
                        Some(cover.map(|cover| EditionBook { resource, cover }))
                    })
                    .collect();
                match fetched {
                    Ok(books) => {
                        components.push(ComponentData::EditionCards(EditionCardsPayload { books }));
                        outcomes.push(FetchOutcome::Found);
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "database error fetching books or covers for EditionCards component"
                        );
                        components.push(ComponentData::Other);
                        outcomes.push(FetchOutcome::Error);
                    }
                }
            }

            // Strip row
            // The class's resources used at least `min_uses` times, most-used
            // first (ties by id, the sort's own tie-break), capped at
            // `limit`; their tile images; and the class's unfiltered count,
            // the `{n}` of the slot's "all N" link.
            Component::StripRow { class, media, uses_property, min_uses, limit } => {
                let wheres = [WhereClause::Range {
                    property: uses_property.clone(),
                    start: Some(Value::Integer(i128::from(*min_uses))),
                    end: None,
                }];
                let sorts = [Sort {
                    key: SortKey::Property(uses_property.clone()),
                    direction: SortDir::Desc,
                }];
                let fetched = reads
                    .list_resources(class, &wheres, &sorts, Pagination::Limit(*limit), 1, lang)
                    .and_then(|page| {
                        let ids: Vec<String> = page.items.iter().map(|r| r.id.clone()).collect();
                        let thumbs = reads.media_thumbs(media, &ids)?;
                        let class_total = reads.count_resources(class, &[])?;
                        Ok(StripRowPayload { strips: page.items, thumbs, class_total })
                    });
                match fetched {
                    Ok(payload) => {
                        components.push(ComponentData::StripRow(payload));
                        outcomes.push(FetchOutcome::Found);
                    }
                    Err(e) => {
                        tracing::error!(
                            class = class.as_str(),
                            error = %e,
                            "database error fetching strips or tiles for StripRow component"
                        );
                        components.push(ComponentData::Other);
                        outcomes.push(FetchOutcome::Error);
                    }
                }
            }

            // Non-data components
            // Listed, not a `_` wildcard: a new `Component` variant fails to
            // compile here until it gets a fetch arm or joins this list.
            Component::Narrative { .. } | Component::SearchBar { .. } => {
                // They render from config/content; no fetch needed.
                components.push(ComponentData::Other);
            }
        }
    }

    let status = PageStatus::from_outcomes(&outcomes);

    // A page that 404s (its `ResourceDetail` slot resolved to `None`) must
    // not also render a sibling `CompoundViewer`'s empty-compound body —
    // `compound_parts` on the same unresolved source id legitimately
    // returns `total == 0`, which `render_component`'s `CompoundViewer` arm
    // renders as "this compound has no members", a second, contradictory
    // empty state underneath the real 404. Generic (no project literals):
    // any `Compound` payload is downgraded to `Other` (renders nothing —
    // `render_component`'s `_ => html!{}` fallback) whenever the page's
    // overall status is already `NotFound`.
    if status == PageStatus::NotFound {
        for c in &mut components {
            if matches!(c, ComponentData::Compound(_)) {
                *c = ComponentData::Other;
            }
        }
    }

    (components, status)
}

/// Resolve the pre-hydrated media items for the page's (at most one, boot-
/// enforced `DuplicateSingletonComponent`) `ResourceDetail` slot, for
/// threading through `RenderContext.media_items`.
/// Returns an empty `Vec` when the page has no resolved resource-detail
/// resource, or the resource's class has no declared media property.
///
/// Errors from `ProjectReads::media_items` are logged and treated as "no
/// media" — a media-fetch failure must not turn an otherwise-successful
/// resource-detail render into a 500.
fn media_items_for_page(
    reads: &ProjectReads<'_>,
    media_properties: &HashMap<String, String>,
    components: &[ComponentData],
    lang: &Lang,
) -> Vec<Resource> {
    let Some(resource) = components.iter().find_map(|c| match c {
        ComponentData::Detail(Some(r)) => Some(r),
        _ => None,
    }) else {
        return Vec::new();
    };
    let Some(property) = media_properties.get(&resource.class) else {
        return Vec::new();
    };
    match reads.media_items(&resource.id, property, lang) {
        Ok(items) => items,
        Err(e) => {
            tracing::error!(
                resource_id = resource.id.as_str(),
                property = property.as_str(),
                error = %e,
                "database error fetching media items for ResourceDetail"
            );
            Vec::new()
        }
    }
}

/// Resolve the page's (at most one) `resource-detail` resource's own compound
/// member headers, for threading through `RenderContext.compound_members`.
/// Mirrors
/// `media_items_for_page`'s shape exactly. Returns an empty `Vec` when the
/// page has no resolved resource-detail resource, or that resource has no
/// `compound_parts` rows of its own (the common case — none of Alice's
/// Characters/Locations/StoryChapters are compound parents).
///
/// Errors from `ProjectReads::compound_member_headers` are logged and treated
/// as "no members" — a lookup failure must not turn an otherwise-successful
/// resource-detail render into a 500.
fn compound_members_for_page(reads: &ProjectReads<'_>, components: &[ComponentData], lang: &Lang) -> Vec<MemberHeader> {
    let Some(resource) = components.iter().find_map(|c| match c {
        ComponentData::Detail(Some(r)) => Some(r),
        _ => None,
    }) else {
        return Vec::new();
    };
    match reads.compound_member_headers(&resource.id, lang) {
        Ok(headers) => headers,
        Err(e) => {
            tracing::error!(
                resource_id = resource.id.as_str(),
                error = %e,
                "database error fetching compound member headers for ResourceDetail"
            );
            Vec::new()
        }
    }
}

/// Resolve the incoming-links rows for the page's (at most one)
/// `resource-detail` resource, for threading through
/// `RenderContext.incoming_links`. Mirrors `compound_members_for_page`.
///
/// `None` when the page's `resource-detail` declares no `incoming-links`
/// child, or its resource did not resolve. On a query error the failure is
/// logged and `None` returned — never a 500, and never `Some(empty)`: a
/// lookup failure must not render the "nothing links here" sentence.
pub fn incoming_links_for_page(
    reads: &ProjectReads<'_>,
    page: &Page,
    components: &[ComponentData],
    lang: &Lang,
) -> Option<IncomingLinks> {
    let spec = page.body.iter().find_map(|c| match c {
        Component::ResourceDetail { incoming_links: Some(spec), .. } => Some(spec),
        _ => None,
    })?;
    let resource = components.iter().find_map(|c| match c {
        ComponentData::Detail(Some(r)) => Some(r),
        _ => None,
    })?;
    match reads.incoming_links(spec, &resource.id, lang) {
        Ok(rows) => Some(rows),
        Err(e) => {
            tracing::error!(
                resource_id = resource.id.as_str(),
                source_class = spec.class.as_str(),
                error = %e,
                "database error fetching incoming links for ResourceDetail"
            );
            None
        }
    }
}

/// Resolve the resource-level annotation-link partners of the page's (at
/// most one) `resource-detail` resource, for
/// threading through `RenderContext.resource_links`.
///
/// Empty unless that `resource-detail` declares a `group links-label=`, so a
/// project without one never runs the query. On a query error the failure is
/// logged and the row simply does not render — never a 500. This is the
/// opposite of the reader's annotation queries inside
/// `ProjectReads::compound_parts`, which fail the request: the Apparat row is
/// a secondary property row, the register is the reader's own content.
fn resource_links_for_page(
    reads: &ProjectReads<'_>,
    page: &Page,
    components: &[ComponentData],
    lang: &Lang,
) -> Vec<ResourceLink> {
    let wants_links = page.body.iter().any(|c| {
        matches!(c, Component::ResourceDetail { groups, .. }
            if groups.iter().any(|g| g.links_label.is_some()))
    });
    let resource = components.iter().find_map(|c| match c {
        ComponentData::Detail(Some(r)) => Some(r),
        _ => None,
    });
    let Some(resource) = resource.filter(|_| wants_links) else {
        return Vec::new();
    };
    reads.resource_links(&resource.id, lang).unwrap_or_else(|e| {
        tracing::error!(
            resource_id = resource.id.as_str(),
            error = %e,
            "database error fetching resource links for ResourceDetail"
        );
        Vec::new()
    })
}

/// Resolve the id → raw representation-URI map for the page's `resource-list`
/// browse-row thumbnails, mirroring
/// `media_items_for_page`'s shape.
///
/// Finds the first `ResourceList` component with a declared `media` property
/// and its corresponding `ComponentData::List` slot, collects the resolved
/// page's item ids, and batch-fetches their thumb URIs via
/// `ProjectReads::media_thumbs` — one query for the whole page, not N+1.
/// Returns an empty map when the page has no `resource-list`, the list's
/// `media` attribute is absent, or the list has no items.
///
/// Errors from `ProjectReads::media_thumbs` are logged and treated as "no
/// thumbnails" — a media-fetch failure must not turn an otherwise-successful
/// resource-list render into a 500.
fn media_thumbs_for_page(
    reads: &ProjectReads<'_>,
    page: &Page,
    components: &[ComponentData],
) -> HashMap<String, MediaThumb> {
    for (c, d) in page.body.iter().zip(components.iter()) {
        if let (Component::ResourceList { media: Some(property), .. }, ComponentData::List(list_page)) = (c, d) {
            let ids: Vec<String> = list_page.items.iter().map(|r| r.id.clone()).collect();
            if ids.is_empty() {
                return HashMap::new();
            }
            return match reads.media_thumbs(property, &ids) {
                Ok(map) => map,
                Err(e) => {
                    tracing::error!(
                        property = property.as_str(),
                        error = %e,
                        "database error fetching media thumbs for ResourceList"
                    );
                    HashMap::new()
                }
            };
        }
    }
    HashMap::new()
}

/// Resolve the compound-parent Resource for any `CompoundViewer` component
/// that resolved to `total == 0`,
/// keyed by the compound's resolved source id — mirrors
/// `media_thumbs_for_page`'s shape.
///
/// `build_components` resolves the compound source id internally (literal
/// or `$param`) and returns no ids in its `ComponentData::Compound` payload,
/// so this helper re-resolves the source id itself — the SAME
/// literal/`$param` resolution `build_components`' own `CompoundViewer` arm
/// uses. Only populated when a compound genuinely resolved to zero parts — a
/// populated compound (`total >= 1`) never needs its parent fetched
/// separately (the member IS the payload).
///
/// Errors from `ProjectReads::get_resource` are logged and treated as "no
/// parent" (the renderer degrades to the sentence-alone branch) — a lookup
/// failure must not turn an otherwise-successful empty-compound render into
/// a 500. A source id that resolves to `Ok(None)` (genuinely no such
/// Resource) is the ordinary "parent missing" degrade path, not an error —
/// logged at neither level.
fn compound_parents_for_page(
    reads: &ProjectReads<'_>,
    page: &Page,
    components: &[ComponentData],
    route_params: &HashMap<String, String>,
    lang: &Lang,
) -> HashMap<String, Resource> {
    let mut parents = HashMap::new();
    for (c, d) in page.body.iter().zip(components.iter()) {
        let (Component::CompoundViewer { source, .. }, ComponentData::Compound(compound_page)) = (c, d) else {
            continue;
        };
        if compound_page.total != 0 {
            continue;
        }
        // Resolve source: literal id or `$param` reference (mirrors
        // `build_components`' CompoundViewer arm).
        let source_id: String = if let Some(param_name) = source.strip_prefix('$') {
            match route_params.get(param_name) {
                Some(v) => v.clone(),
                None => continue,
            }
        } else {
            source.clone()
        };
        match reads.get_resource(&source_id, lang) {
            Ok(Some(resource)) => {
                parents.insert(source_id, resource);
            }
            Ok(None) => {
                // Parent id genuinely doesn't resolve — the renderer degrades
                // to the sentence alone; not an error condition.
            }
            Err(e) => {
                tracing::error!(
                    source_id = source_id.as_str(),
                    error = %e,
                    "database error fetching compound parent for CompoundViewer"
                );
            }
        }
    }
    parents
}
