//! CPE's router: the overview of the projects, the shared static files, each
//! project's assets and one explicit route per configured page, closed by a
//! fallback.
//!
//! ## Routing design
//!
//! The routes are the paths below the mount: a host nests this router at the
//! base, and a handler sees the request path without it. What a handler
//! writes into its answer (a page's links, a `Location`, a cookie's `Path`)
//! is built from the base (`cpe_core::url`), and no handler writes the request
//! path there in any other way.
//!
//! Every page is registered under its full path (`/<project id><route>`), so
//! an unknown path under a project matches no route and reaches the fallback.
//! [`build`] always sets that fallback. It renders the project's not-found
//! page under a loaded project's id, answers 503 under the id of a project
//! that did not load, and renders CPE's own not-found page otherwise. It never
//! redirects and never puts the request path into the response; see
//! `fallback_handler`.
//!
//! ## Route-param syntax boundary (axum 0.8 / matchit 0.8)
//!
//! axum 0.8 requires `{name}` and `{*path}` syntax; the old `:name`/`*path`
//! panics at startup. CPE's config language and IR keep `:name` syntax
//! (`docs/src/cpe/config-language.md`). The boundary translation is applied
//! only where a route string is handed to `app.route(…)`, via
//! `axum_route(...)`.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Path, Request};
use axum::http::{header, HeaderValue, StatusCode, Uri};
use axum::response::sse::Event;
use axum::response::{IntoResponse, Redirect, Response, Sse};
use axum::routing::get;
use axum::Router;
use cpe_core::theme::Appearance;
use cpe_core::url::{Base, ROOT};
use datastar::prelude::{ExecuteScript, PatchElements, PatchSignals};
use futures_util::stream;

use crate::assets::AssetInventory;
use crate::request::{query_pairs, RequestedAppearance, RequestedLang};
use crate::{static_files, unavailable};

// Route-param syntax translation (CPE config syntax → axum 0.8 matchit)

/// Translate a CPE config-language route path to the syntax required by axum 0.8
/// (matchit 0.8): `:name` → `{name}` and a trailing `*path` wildcard → `{*path}`.
///
/// CPE's config files and IR use `:name` syntax; this boundary translation is
/// applied **only** where the route string is handed to `app.route(…)`. The
/// config language, the IR and the param-extractor keep `:name`.
///
/// Paths with no param segments pass through unchanged.
/// See the unit tests in the `tests` module for examples.
pub(crate) fn axum_route(route: &str) -> String {
    let mut out = String::with_capacity(route.len() + 4);
    for segment in route.split('/') {
        out.push('/');
        if let Some(name) = segment.strip_prefix(':') {
            // Named parameter: `:name` → `{name}`
            out.push('{');
            out.push_str(name);
            out.push('}');
        } else if let Some(name) = segment.strip_prefix('*') {
            // Wildcard catch-all: `*path` → `{*path}`
            out.push('{');
            out.push('*');
            out.push_str(name);
            out.push('}');
        } else {
            out.push_str(segment);
        }
    }
    // strip the leading extra '/' added before the first (empty) segment from split
    out[1..].to_owned()
}

// Status-code mapping

/// Map a page's [`cpe_core::view_data::PageStatus`] to the response's HTTP status:
/// `Ok` → 200, `NotFound` → 404, `Error` → 500.
fn status_code(status: cpe_core::view_data::PageStatus) -> StatusCode {
    match status {
        cpe_core::view_data::PageStatus::Ok => StatusCode::OK,
        cpe_core::view_data::PageStatus::NotFound => StatusCode::NOT_FOUND,
        cpe_core::view_data::PageStatus::Error => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Build CPE's router over the loaded projects.
///
/// `unavailable` holds the ids of the project folders that did not load: no
/// route is registered for them, and the fallback answers their prefix 503.
/// `assets` is every loaded project's asset inventory. Each handler closure
/// gets a clone of `origin_cfg` and resolves its own request's public origin
/// from it, so one process can answer under several hosts.
///
/// `base` is where a host mounts the router. No route holds it: the route
/// paths are the layout of `cpe_core::url` at the root, and `base` goes into
/// what the handlers answer.
pub(crate) fn build(
    projects: &[Arc<cpe_web::project::Project>],
    unavailable: &[String],
    assets: &AssetInventory,
    origin_cfg: crate::request::OriginConfig,
    base: &Base,
) -> Router {
    let served = Arc::new(projects.to_vec());
    // The shared static files for a page without a project (the overview,
    // CPE's own not-found). The viewer's files sit in a directory of the same
    // table, as `document_head` asks for them.
    let mut app = static_routes(Router::new(), &ROOT.static_dir()).route(
        &ROOT.home(),
        get(site_index_handler(Arc::clone(&served), origin_cfg.clone(), base.clone())),
    );
    for project in projects {
        let routes = ROOT.project(project.id());
        // The same files once more under the project, where its pages load
        // them from, and the project's own files (a cover image). Both
        // segments start with `_`, which the first segment of a page's route
        // never does (`DiagnosticKind::ReservedRoute`), so neither can
        // collide with a page.
        app = static_routes(app, &routes.static_dir());
        let project_assets = assets.clone();
        let project_id = project.id().to_owned();
        app = app.route(
            &axum_route(&routes.asset("*path")),
            get(move |Path(path): Path<String>| std::future::ready(project_assets.serve(&project_id, &path))),
        );
        for (idx, page) in project.config().pages.iter().enumerate() {
            // Build the full path for this page.
            let home = routes.home();
            let p = if page.route == "/" {
                home.trim_end_matches('/').to_owned()
            } else {
                routes.page(&page.route)
            };

            if page.params.is_empty() {
                // Static page: no path parameters — use the simple handler.
                app = app.route(
                    &axum_route(&p),
                    get(make_handler(Arc::clone(project), idx, origin_cfg.clone(), base.clone())),
                );

                // For the root page, also register the trailing-slash form.
                // axum 0.8 does not automatically redirect or match `/<id>/`
                // when only `/<id>` is registered, so both must be explicit.
                if page.route == "/" {
                    app = app.route(
                        &axum_route(&home),
                        get(make_handler(Arc::clone(project), idx, origin_cfg.clone(), base.clone())),
                    );
                }

                // Register a DataStar SSE facet fragment route for every static page
                // whose body contains a FacetPanel component. The fragment endpoint
                // lives at `{page-path}/_facets` — see
                // `make_facet_fragment_handler`'s doc comment for the exact event set it
                // returns: four `datastar-patch-elements` events in one SSE stream — panel
                // (#facets-<list-id>), filterbar (#filters-<list-id>), list
                // (#<list-id>), crumb line (#crumb-<list-id>) — plus a
                // `datastar-patch-signals` $copied reset. The filterbar is its own patch
                // region so the active-filter chip row never goes stale on a JS facet
                // interaction.
                let has_facet_panel = page
                    .body
                    .iter()
                    .any(|c| matches!(c, cpe_core::ir::Component::FacetPanel { .. }));
                if has_facet_panel {
                    let facets_path =
                        format!("{}/{}", p, cpe_components::component::facet_panel::FACET_FRAGMENT_SEGMENT);
                    app = app.route(
                        &axum_route(&facets_path),
                        get(make_facet_fragment_handler(
                            Arc::clone(project),
                            idx,
                            origin_cfg.clone(),
                            base.clone(),
                        )),
                    );
                }

                // Register a DataStar SSE sort fragment route for every static page
                // whose body contains a ResourceList component. The fragment endpoint
                // lives at `{page-path}/_sort` — see `make_sort_fragment_handler`'s doc
                // comment for the exact event set it returns; sort never touches
                // facets/filters (contrast with `_facets`'s three-region morph).
                let has_resource_list = page
                    .body
                    .iter()
                    .any(|c| matches!(c, cpe_core::ir::Component::ResourceList { .. }));
                if has_resource_list {
                    let sort_path =
                        format!("{}/{}", p, cpe_components::component::resource_list::SORT_FRAGMENT_SEGMENT);
                    app = app.route(
                        &axum_route(&sort_path),
                        get(make_sort_fragment_handler(
                            Arc::clone(project),
                            idx,
                            origin_cfg.clone(),
                            base.clone(),
                        )),
                    );
                }

                // Register a DataStar SSE search fragment route for every static
                // page whose body contains a SearchBar component. The fragment
                // endpoint lives at `{page-path}/_search` — see
                // `make_search_fragment_handler`'s doc comment for the exact event set it
                // returns: THREE `datastar-patch-elements` events when the list has a
                // facet-panel with at least one non-empty group (filterbar
                // `#filters-<list-id>` + list `#<list-id>` + crumb line
                // `#crumb-<list-id>`) or TWO otherwise (list + crumb), plus a
                // `datastar-patch-signals` $copied reset — the search row itself
                // (`#q`/`.search-row`) is in no target selector.
                let has_search_bar = page.body.iter().any(|c| matches!(c, cpe_core::ir::Component::SearchBar { .. }));
                if has_search_bar {
                    let search_path =
                        format!("{}/{}", p, cpe_components::component::search_bar::SEARCH_FRAGMENT_SEGMENT);
                    app = app.route(
                        &axum_route(&search_path),
                        get(make_search_fragment_handler(
                            Arc::clone(project),
                            idx,
                            origin_cfg.clone(),
                            base.clone(),
                        )),
                    );
                }

                // No fragment route for the record viewer: every `resource-detail`
                // with media renders `record--viewer`, whose pager pages client-side
                // via the `$plate` signal + `goToPage`, never `@get`
                // (`dsp-incubator cpe/docs/adr/0007-fragment-swaps-are-transient.md`).

                // Register a DataStar SSE compound-viewer fragment route for
                // every static page whose body contains a CompoundViewer
                // component. A static-source compound-viewer never needs a
                // route param, so `make_compound_fragment_handler` (no `Path`
                // extractor) is correct here, mirroring the static-page
                // facet/sort/search registrations above.
                let has_compound_viewer = page
                    .body
                    .iter()
                    .any(|c| matches!(c, cpe_core::ir::Component::CompoundViewer { .. }));
                if has_compound_viewer {
                    let compound_path = format!(
                        "{}/{}",
                        p,
                        cpe_components::component::compound_viewer::COMPOUND_FRAGMENT_SEGMENT
                    );
                    app = app.route(
                        &axum_route(&compound_path),
                        get(make_compound_fragment_handler(
                            Arc::clone(project),
                            idx,
                            origin_cfg.clone(),
                            base.clone(),
                        )),
                    );
                }
            } else {
                // Parameterised page (e.g. `/characters/:id`): translate the
                // CPE `:name` param syntax to axum 0.8's `{name}` syntax before
                // registering. The config/IR are not changed.
                app = app.route(
                    &axum_route(&p),
                    get(make_parameterized_handler(
                        Arc::clone(project),
                        idx,
                        origin_cfg.clone(),
                        base.clone(),
                    )),
                );

                // No record-viewer fragment route here either (see the note in
                // the static-page branch above).

                // Register a DataStar SSE facet fragment route for every parameterised
                // page whose body contains a FacetPanel component. Symmetrical with the
                // static-page branch.
                let has_facet_panel_param = page
                    .body
                    .iter()
                    .any(|c| matches!(c, cpe_core::ir::Component::FacetPanel { .. }));
                if has_facet_panel_param {
                    let facets_path =
                        format!("{}/{}", p, cpe_components::component::facet_panel::FACET_FRAGMENT_SEGMENT);
                    app = app.route(
                        &axum_route(&facets_path),
                        get(make_facet_fragment_handler(
                            Arc::clone(project),
                            idx,
                            origin_cfg.clone(),
                            base.clone(),
                        )),
                    );
                }

                // Register a DataStar SSE sort fragment route for every parameterised
                // page whose body contains a ResourceList component. Symmetrical with
                // the static-page branch.
                let has_resource_list_param = page
                    .body
                    .iter()
                    .any(|c| matches!(c, cpe_core::ir::Component::ResourceList { .. }));
                if has_resource_list_param {
                    let sort_path =
                        format!("{}/{}", p, cpe_components::component::resource_list::SORT_FRAGMENT_SEGMENT);
                    app = app.route(
                        &axum_route(&sort_path),
                        get(make_sort_fragment_handler(
                            Arc::clone(project),
                            idx,
                            origin_cfg.clone(),
                            base.clone(),
                        )),
                    );
                }

                // Register a DataStar SSE search fragment route for every
                // parameterised page whose body contains a SearchBar component.
                // Symmetrical with the static-page branch.
                let has_search_bar_param =
                    page.body.iter().any(|c| matches!(c, cpe_core::ir::Component::SearchBar { .. }));
                if has_search_bar_param {
                    let search_path =
                        format!("{}/{}", p, cpe_components::component::search_bar::SEARCH_FRAGMENT_SEGMENT);
                    app = app.route(
                        &axum_route(&search_path),
                        get(make_search_fragment_handler(
                            Arc::clone(project),
                            idx,
                            origin_cfg.clone(),
                            base.clone(),
                        )),
                    );
                }

                // Register a DataStar SSE compound-viewer fragment route for
                // every parameterised page whose body contains a
                // CompoundViewer component. Unlike the static-page branch
                // above, this registration needs REAL `Path` extraction
                // (`make_compound_fragment_handler_param`, following
                // `make_parameterized_handler`'s pattern) — NOT
                // `make_facet_fragment_handler`'s param-less shape: a
                // route-param compound page's `compound-viewer source="$id"`
                // (Incunabula's book reader, `/incunabula/books/:id`) needs
                // the real `:id` value to resolve at all.
                let has_compound_viewer_param = page
                    .body
                    .iter()
                    .any(|c| matches!(c, cpe_core::ir::Component::CompoundViewer { .. }));
                if has_compound_viewer_param {
                    let compound_path = format!(
                        "{}/{}",
                        p,
                        cpe_components::component::compound_viewer::COMPOUND_FRAGMENT_SEGMENT
                    );
                    app = app.route(
                        &axum_route(&compound_path),
                        get(make_compound_fragment_handler_param(
                            Arc::clone(project),
                            idx,
                            origin_cfg.clone(),
                            base.clone(),
                        )),
                    );
                }
            }
        }
    }

    // A router without a fallback of its own inherits the outer router's when
    // it is nested, which would hand CPE's unknown paths to another capability.
    app.fallback(fallback_handler(served, unavailable.into(), origin_cfg, base.clone()))
}

/// Serve the static table, and the viewer's files in its `image-viewer`
/// directory, from `dir`.
fn static_routes(app: Router, dir: &cpe_core::url::StaticDir) -> Router {
    app.route(
        &axum_route(&dir.file("*path")),
        get(|Path(path): Path<String>| std::future::ready(static_files::serve(&path))),
    )
    .route(&axum_route(&dir.file("image-viewer/*path")), get(serve_viewer_asset))
}

/// The overview of the projects, at the router's root: one teaser per loaded
/// project. A project that did not load has none, as its prefix answers 503.
///
/// `?appearance=` sets the cookie and redirects to the page without it, as
/// on a project's page; no other query key is read or removed. The page is
/// in one of the built-in languages: there is no project to declare any.
fn site_index_handler(
    projects: Arc<Vec<Arc<cpe_web::project::Project>>>,
    origin_cfg: crate::request::OriginConfig,
    base: Base,
) -> impl Fn(Request) -> std::future::Ready<Response> + Clone + Send + 'static {
    move |req: Request| {
        let requested = req
            .extensions()
            .get::<RequestedLang>()
            .cloned()
            .unwrap_or(RequestedLang { prefs: Vec::new(), from_query: false });
        let requested_appearance = req
            .extensions()
            .get::<RequestedAppearance>()
            .cloned()
            .unwrap_or(RequestedAppearance { value: None, from_query: false });
        if let Some(redirect) = redirect_without_keys(req.uri(), &["appearance"], &requested_appearance, &base) {
            return std::future::ready(redirect);
        }

        let lang = cpe_core::lang::resolve_builtin(&requested.prefs);
        let teasers: Vec<_> = projects.iter().map(|project| project.teaser(&lang)).collect();
        let markup = cpe_web::page::render_site_index(
            &base,
            &teasers,
            &lang,
            requested_appearance.value.unwrap_or_default(),
            &origin_cfg.resolve(req.headers()),
        );
        let mut resp = (StatusCode::OK, markup).into_response();
        attach_lang_cookie(&mut resp, &requested, lang.as_str(), &base);
        std::future::ready(resp)
    }
}

/// Router-wide fallback for any path that matched no route registered above.
///
/// **Project resolution** is byte-level, undecoded, first-path-segment
/// equality against the known project ids:
/// - Strip exactly one leading `/`.
/// - Take up to the next `/` (or the rest of the string).
/// - Test the resulting slice for EXACT STRING EQUALITY against a project id — no percent-decoding,
///   no case-folding, no normalization, no prefix matching.
///
/// This is why `//evil.example/x` resolves to the EMPTY segment, never
/// `evil.example`: stripping one leading `/` from `//evil.example/x` leaves
/// `/evil.example/x`, and taking up to the next `/` of THAT stops at index 0
/// (empty). The extracted segment is used ONLY in the equality tests
/// above — it is never echoed into a response.
///
/// - The id of a project that did not load → 503 ([`unavailable::response`]): the project exists
///   and is not served, which is not a missing page.
/// - A loaded project's id → `cpe_web::page::render_not_found_project`: the project's own 404, full
///   project chrome and its `theme {}`.
/// - No match → `cpe_web::page::render_not_found_site`: CPE's own 404, product-default knob,
///   built-in language resolution.
///
/// **Security-load-bearing: this handler performs NO redirect and sets NO
/// cookie.** It must NEVER call `override_redirect` or
/// `redirect_without_keys` — both echo the request path into a `Location` header, which under a
/// catch-all fallback is an open redirect via any `//attacker.example/...`-shaped path
/// (browsers treat a scheme-relative `Location` as a redirect to that host).
/// It renders the body directly and never returns early with anything but
/// the 503 above.
///
/// **`current_path` for the chrome is pinned**, never the unmatched request
/// path, and no query string is carried over: the project's 404 passes
/// the project's home (`ProjectUrls::home`) with `current_query: None`, and
/// CPE's own takes no path at all. The failed path therefore appears nowhere in the
/// response — not in the visible copy, not in the lang-switcher or
/// appearance-toggle hrefs `shell_header` builds from
/// `current_path`/`current_query` either. Consequence: the language switcher
/// of a project's 404 navigates to the project home rather than re-rendering
/// the same 404.
///
/// Reads `RequestedLang`/`RequestedAppearance` from request extensions
/// defensively (defaulting to empty prefs / no appearance) — `activation`
/// layers `resolve_request_prefs` over the whole router including this
/// fallback, but router-only tests build a bare `Router` without it,
/// mirroring every other handler in this file.
///
/// O(1), no DB access, no body extractor — a fallback answers all HTTP
/// methods on an unmatched path (a POST to `/nope` renders this page too);
/// every matched route keeps its own method/405 behaviour.
fn fallback_handler(
    projects: Arc<Vec<Arc<cpe_web::project::Project>>>,
    unavailable: Arc<[String]>,
    origin_cfg: crate::request::OriginConfig,
    base: Base,
) -> impl Fn(Request) -> std::future::Ready<Response> + Clone + Send + 'static {
    move |req: Request| {
        let public_origin = origin_cfg.resolve(req.headers());
        let path = req.uri().path();
        // Byte-level, undecoded extraction — see the doc comment above for
        // the `//evil.example/x` → "" (empty) worked example.
        let segment = path.strip_prefix('/').unwrap_or(path);
        let segment = match segment.find('/') {
            Some(idx) => &segment[..idx],
            None => segment,
        };

        // An empty segment (the `//evil.example/x` sentinel) is no project's
        // id; the guard below and this one keep that explicit.
        if !segment.is_empty() && unavailable.iter().any(|id| id == segment) {
            return std::future::ready(unavailable::response());
        }

        let requested = req
            .extensions()
            .get::<RequestedLang>()
            .cloned()
            .unwrap_or(RequestedLang { prefs: Vec::new(), from_query: false });
        let requested_appearance = req
            .extensions()
            .get::<RequestedAppearance>()
            .cloned()
            .unwrap_or(RequestedAppearance { value: None, from_query: false });
        let appearance = requested_appearance.value.unwrap_or_default();

        // Guard: `segment` is empty for the `//evil.example/x` sentinel (see
        // the doc comment above) — it must never match a loaded project id.
        // `Vec::iter().find()` on an empty needle would only match a project
        // id that is itself the empty string, which no loaded project has,
        // but the explicit guard makes the invariant load-bearing rather than
        // incidental.
        let matched_project = if segment.is_empty() {
            None
        } else {
            projects.iter().find(|p| p.id() == segment)
        };

        let markup = match matched_project {
            Some(project) => {
                // The raw first segment matched a loaded project id.
                let site = &project.config().site;
                let lang = cpe_core::lang::resolve(&requested.prefs, site);
                let home_path = base.project(project.id()).home();
                let ctx = cpe_components::component::context::RenderContext {
                    base: &base,
                    lang,
                    project_id: project.id(),
                    current_path: &home_path,
                    current_query: None,
                    route_params: &[],
                    appearance,
                    selected_image: 0,
                    strip_open: true,
                    link_routes: &HashMap::new(),
                    media_properties: &HashMap::new(),
                    media_items: &[],
                    media_thumbs: &HashMap::new(),
                    // Intentionally empty: `render_not_found_project`
                    // never consumes `compound_members`/`compound_parents`/
                    // `class_list_routes` — if that changes, thread the
                    // project's real values here (e.g.
                    // `project.class_list_routes()`) instead.
                    compound_members: &[],
                    compound_parents: &HashMap::new(),
                    class_list_routes: &HashMap::new(),
                    public_origin: &public_origin,
                    incoming_links: None,
                    resource_links: &[],
                };
                cpe_web::page::render_not_found_project(site, &ctx)
            }
            None => {
                // No loaded project id matches the raw first segment.
                let lang = cpe_core::lang::resolve_builtin(&requested.prefs);
                cpe_web::page::render_not_found_site(&base, &lang, appearance)
            }
        };

        std::future::ready((StatusCode::NOT_FOUND, markup).into_response())
    }
}

/// Parse a named query parameter as a 1-based `u32`.
///
/// Returns the parsed value when present and `>= 1`; defaults to `1` in all
/// failure cases: absent, empty, non-numeric, zero, or overflow.
/// This function is **total** — it never panics.
fn parse_u32_param(query: Option<&str>, name: &str) -> u32 {
    query_pairs(query)
        .find(|(k, _)| *k == name)
        .and_then(|(_, v)| v.parse::<u32>().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(1)
}

/// Parse the `page=` query parameter as a 1-based `u32` page number.
///
/// Returns the parsed value when present and `>= 1`; defaults to `1` in all
/// failure cases: absent, empty, non-numeric, or zero. On a numeric value
/// that overflows `u32` (`ParseIntError::kind() == IntErrorKind::PosOverflow`)
/// this SATURATES to `u32::MAX` instead of falling back to `1` — the
/// pagination arithmetic behind `Project::page_data` clamps `page` to the
/// last page *before* multiplying, so an absurdly large `page` still lands
/// on the last page like any other out-of-range value, and
/// `canonical_page_redirect` can canonicalise it to that effective page in
/// one hop. Parses itself rather than delegating to `parse_u32_param`, which
/// maps overflow to `1`, same as every other failure case.
fn parse_page_param(query: Option<&str>) -> u32 {
    let raw = query_pairs(query).find(|(k, _)| *k == "page").map(|(_, v)| v);
    match raw {
        None => 1,
        Some(v) => match v.parse::<u32>() {
            Ok(n) if n >= 1 => n,
            Ok(_) => 1,
            Err(e) if e.kind() == &std::num::IntErrorKind::PosOverflow => u32::MAX,
            Err(_) => 1,
        },
    }
}

/// Return the (first) raw value for the `page=` query parameter, borrowed
/// from `query` (no allocation) — empty value included.
///
/// Uses the SAME occurrence semantics as `parse_u32_param`: the first `page`
/// pair wins, regardless of emptiness. A bare `?page` (no `=`) yields
/// `Some("")`. `None` iff no `page` key is present at all. Deliberately
/// **not** `query_param_one`, which skips empty values — that would make
/// `?page=&page=2` non-idempotent under `canonical_page_redirect`.
fn raw_page_param(query: Option<&str>) -> Option<&str> {
    query_pairs(query).find(|(k, _)| *k == "page").map(|(_, v)| v)
}

/// Rebuild `query`'s pairs, dropping every `page` pair (`page=…`, `page=`,
/// bare `page`) and appending exactly one canonical `page=<page>` at the end.
/// Every other pair is kept, in order, re-emitted as `k=v` (a bare key
/// normalises to `k=`); empty-key pairs (`?&page=…`) are skipped. Both of
/// those normalisations make repeated application idempotent.
///
/// `canonical_page_query(None, 2)` → `"page=2"`.
///
/// Deliberately NOT built on `cpe-components`' `override_query_param`:
/// that helper rewrites each matching pair in place
/// (`?page=&page=2` → `page=1&page=1`), which is loop-safe but not
/// canonical — a different semantics than this needs.
fn canonical_page_query(query: Option<&str>, page: u32) -> String {
    let mut kept: Vec<String> = query_pairs(query)
        .filter(|(k, _)| !k.is_empty() && *k != "page")
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    kept.push(format!("page={page}"));
    kept.join("&")
}

/// Guard-return for the two HTML page handlers: when a **page** request's
/// raw `page` param is present and does not match the effective page after
/// clamping, produce a `303 See Other` to the same path with exactly one
/// canonical `page=<effective>` and every other pair preserved in order.
///
/// `None` (no redirect) when:
/// - `status != 200` (a 404-ing sibling component must not be masked as a 303 → 404 — the visitor
///   should see the real 404, not a redirect to it);
/// - the page has no `List` component with `Pagination::PerPage(_)`;
/// - `raw_page_param(query)` is `None` (no `page` param at all — never redirect just because a
///   request omitted it).
///
/// Otherwise, `raw != effective.to_string()` is the redirect condition
/// (covers out of range, `0`, `abc`, `03`, `+3`, empty — all canonicalise in
/// one hop; `raw := effective` is a fixed point, so this never loops).
///
/// Called as a guard-return right AFTER `Project::page_data` (not before)
/// because it needs the effective (clamped) `page` from the hydrated
/// `ResourceListPage`. The one extra count+hydrate (and the page's side
/// lookups) on an out-of-range request is the accepted cost of that order.
/// `Location` is always a relative `path?query`, never built from
/// `Host`/origin. `path` is the request's public path, the base included.
fn canonical_page_redirect(
    path: &str,
    query: Option<&str>,
    status: StatusCode,
    components: &[cpe_core::view_data::ComponentData],
) -> Option<Response> {
    if status != StatusCode::OK {
        return None;
    }
    let list_page = components.iter().find_map(|c| match c {
        cpe_core::view_data::ComponentData::List(p) if matches!(p.pagination, cpe_core::ir::Pagination::PerPage(_)) => {
            Some(p)
        }
        _ => None,
    })?;
    let raw = raw_page_param(query)?;
    if raw == list_page.page.to_string() {
        return None;
    }
    let location = format!("{path}?{}", canonical_page_query(query, list_page.page));
    Some(Redirect::to(&location).into_response())
}

/// Return the (first) raw value for a single-value query-param key, borrowed
/// from `query` (no allocation).
///
/// `?sort=name` with `key="sort"` → `Some("name")`. Absent key, or a `key=`
/// with no value, → `None`. If the key is repeated, the first **non-empty**
/// value wins — empty values are skipped (`?sort=&sort=name` → `Some("name")`).
/// This differs from `parse_u32_param`, which inspects the first occurrence
/// regardless of emptiness (`?page=&page=2` fails to parse and defaults to 1).
///
/// This function is **total** — it never panics.
fn query_param_one<'q>(query: Option<&'q str>, key: &str) -> Option<&'q str> {
    query_pairs(query).find(|(k, v)| *k == key && !v.is_empty()).map(|(_, v)| v)
}

/// Read and decode the search-bar's `?q=` value: the free-text search query.
///
/// **Why `q` alone is decoded:** `query_pairs`/`query_param_one`/
/// `query_param_all` (`request.rs`, this file) split a raw query string on
/// `&` then `=` and hand back byte slices verbatim — no percent-decoding, no
/// `+`-means-space handling. Every OTHER query value read here (facet/
/// list-node ids, the `?sort=` closed vocabulary, language codes, numeric
/// `page`/`member` indices) is a closed-vocabulary identifier that never
/// legitimately contains a space or a reserved URL character. `q` is free
/// text: a search term routinely contains a space (`"quantum cat"`), and
/// BOTH of `search-bar`'s two input paths produce escaped bytes for that
/// space — the zero-JS `<form method=get>` fallback submits
/// `application/x-www-form-urlencoded` (space → `+`), and the DataStar
/// debounced-live path's `encodeURIComponent` escapes it as `%20` — so `q`
/// needs BOTH percent-decoding and `+`→space handling. Call THIS everywhere
/// `q` is read, never raw `query_param_one(current_query, "q")`, so decoding
/// is never forgotten at one call site and not another.
///
/// Returns `""` (never panics) when the key is absent or decoding fails.
fn read_search_query_param(query: Option<&str>) -> String {
    let raw = query_param_one(query, "q").unwrap_or("");
    // `application/x-www-form-urlencoded` value decoding: a literal `+`
    // means a space (applied BEFORE `%XX` percent-decoding — raw
    // percent-decoding alone does not treat `+` specially).
    let plus_decoded = raw.replace('+', " ");
    percent_encoding::percent_decode_str(&plus_decoded)
        .decode_utf8()
        .map(|cow| cow.into_owned())
        .unwrap_or_else(|_| raw.to_owned())
}

/// Resolve the effective sort criteria for a `ResourceList` request, in
/// three tiers: (1) the raw `?sort=` value parsed via `Sort::parse_wire` (the
/// built-ins); else (2) the value matched against the list's declared
/// `order-by` keys (`order_options` — the option's
/// `OrderOption::sort`); else (3) `config_sorts`. Tier 3 also covers an
/// absent param, `"default"`, and any unrecognized token — the documented
/// silent "As curated" fallback.
///
/// `name_property` is the list's `order-name-property`: when `Some`, the
/// built-in `name`/`name-desc` rows order by that property instead of `name`.
/// The wire tokens themselves never change.
///
/// Extracted as a standalone pure function so the override/fallback logic is
/// unit-testable without a full router/DB setup.
fn resolve_sorts(
    current_query: Option<&str>,
    config_sorts: &[cpe_core::ir::Sort],
    order_options: &[cpe_core::ir::OrderOption],
    name_property: Option<&str>,
) -> Vec<cpe_core::ir::Sort> {
    let Some(raw) = query_param_one(current_query, "sort") else {
        return config_sorts.to_vec();
    };
    if let Some(mut s) = cpe_core::ir::Sort::parse_wire(raw) {
        if let (Some(property), "name" | "name-desc") = (name_property, raw) {
            s.key = cpe_core::ir::SortKey::Property(property.to_owned());
        }
        return vec![s];
    }
    match order_options.iter().find(|o| o.key == raw) {
        Some(option) => vec![option.sort()],
        None => config_sorts.to_vec(),
    }
}

/// Return **all** values for a repeated query-param key.
///
/// `?role=R_01&role=R_02` with `key="role"` → `["R_01", "R_02"]`.
/// `?page=1` with `key="role"` → `[]`.
/// `None` query → `[]`.
///
/// Values are returned in the order they appear in the query string.
/// This function is **total** — it never panics.
pub(crate) fn query_param_all(query: Option<&str>, key: &str) -> Vec<String> {
    query_pairs(query)
        .filter(|(k, v)| *k == key && !v.is_empty())
        .map(|(_, v)| v.to_owned())
        .collect()
}

/// Cap on the number of distinct values a single facet query-param key may
/// contribute to a `TargetIn` where-clause.
pub(crate) const MAX_FACET_VALUES: usize = 256;

/// Return the deduplicated, capped set of values for a repeated facet
/// query-param key, in first-occurrence order.
///
/// Built on top of [`query_param_all`]: duplicates are removed BEFORE the
/// [`MAX_FACET_VALUES`] cap is applied, so a repeated value can never
/// consume a slot in the capped list — `"role=R&role=R&role=R&..."` (however
/// many times repeated) always yields `["R"]`, never an empty or truncated
/// result starved by its own duplicates.
pub(crate) fn facet_param_values(query: Option<&str>, key: &str) -> Vec<String> {
    let all = query_param_all(query, key);
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut deduped: Vec<String> = Vec::with_capacity(all.len());
    for value in &all {
        if seen.insert(value.as_str()) {
            deduped.push(value.clone());
        }
    }
    deduped.truncate(MAX_FACET_VALUES);
    deduped
}

/// The request's query values as the store asks for them
/// ([`cpe_core::view_data::PageParams`], read by `Project::page_data`).
///
/// Each method calls the reader that owns that value, so the decode
/// points stay here: `?q=` is decoded only by `read_search_query_param`, and
/// a compound reader's `?page=` reaches `Project::page_data` raw (it is decoded once,
/// when it resolves the member).
struct RequestParams<'q>(Option<&'q str>);

impl cpe_core::view_data::PageParams for RequestParams<'_> {
    fn list_page(&self) -> u32 {
        parse_page_param(self.0)
    }

    fn member(&self) -> u32 {
        parse_u32_param(self.0, "member")
    }

    fn reader_page(&self) -> Option<&str> {
        query_param_one(self.0, "page")
    }

    fn sorts(
        &self,
        config: &[cpe_core::ir::Sort],
        options: &[cpe_core::ir::OrderOption],
        name_property: Option<&str>,
    ) -> Vec<cpe_core::ir::Sort> {
        resolve_sorts(self.0, config, options, name_property)
    }

    fn facet_values(&self, property: &str) -> Vec<String> {
        facet_param_values(self.0, property)
    }

    fn search(&self) -> String {
        read_search_query_param(self.0)
    }
}

/// Attach a `Set-Cookie: lang=<resolved>` header when the preference came from
/// `?lang=`.
///
/// The cookie value is always the resolved language code (validated before
/// this call), never the raw query-string value. This prevents cookie
/// poisoning with unvalidated input.
///
/// Its `Path` is the base (`Base::cookie_path`), so the cookie reaches every
/// project of CPE and nothing else on the host.
fn attach_lang_cookie(resp: &mut Response, requested: &RequestedLang, cookie_value: &str, base: &Base) {
    if requested.from_query {
        let cookie = format!("lang={cookie_value}; Path={}; SameSite=Lax", base.cookie_path());
        let hv = HeaderValue::from_str(&cookie)
            .expect("Set-Cookie value built from a validated language code and base has no control chars");
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
}

/// Manage the `appearance` cookie when the preference came from `?appearance=`.
///
/// The cookie only ever holds the non-default `dark` state:
/// - `Some(Dark)` → `Set-Cookie: appearance=dark`.
/// - `Some(Light)` → a clearing cookie (`Max-Age=0`); light is the default, so there is no
///   `appearance=light` value to persist.
/// - `None` (unknown/unparseable value) → no cookie at all.
///
/// The cookie value is a fixed literal, never the raw query string. Its
/// `Path` is the base, as the language cookie's.
fn attach_appearance_cookie(resp: &mut Response, requested: &RequestedAppearance, base: &Base) {
    if requested.from_query {
        let path = base.cookie_path();
        let cookie = match requested.value {
            Some(Appearance::Dark) => format!("appearance=dark; Path={path}; SameSite=Lax"),
            // Light is the default; clear any persisted dark choice.
            Some(Appearance::Light) => format!("appearance=; Path={path}; Max-Age=0; SameSite=Lax"),
            None => return,
        };
        let hv = HeaderValue::from_str(&cookie)
            .expect("Set-Cookie for appearance has a validated base and no control chars");
        // Append (not insert) so a lang cookie is not displaced.
        resp.headers_mut().append(header::SET_COOKIE, hv);
    }
}

/// Query keys `override_redirect` strips with a 303. `appearance` is live
/// (it sets the appearance cookie); `accent`/`background`/`typeface`/`reset`
/// are retired keys, stripped so an old inbound link carrying one never
/// reaches an href the page emits (pager, language, facet links preserve the
/// query).
const STRIPPED_QUERY_KEYS: &[&str] = &["appearance", "accent", "background", "typeface", "reset"];

/// If the query carries any of `keys` (bare or `key=value`), return a 303 to
/// the SAME path with every such pair dropped and the rest kept in order, and
/// attach the appearance cookie (a no-op unless `?appearance=` parsed).
/// Returns `None` when no pair matches. The `Location` is the base and
/// `uri.path()`, so a caller must be a matched route, never the catch-all
/// fallback (whose path may be `//host/…`, which at a root mount would be
/// written as it is). The overview's route matches `/` alone.
fn redirect_without_keys(uri: &Uri, keys: &[&str], appearance: &RequestedAppearance, base: &Base) -> Option<Response> {
    let pairs: Vec<&str> = uri.query().unwrap_or("").split('&').filter(|kv| !kv.is_empty()).collect();
    let kept: Vec<&str> = pairs
        .iter()
        .copied()
        .filter(|kv| !keys.contains(&kv.split('=').next().unwrap_or(kv)))
        .collect();
    if kept.len() == pairs.len() {
        return None;
    }
    let path = base.request_path(uri.path());
    let location = if kept.is_empty() {
        path
    } else {
        format!("{path}?{}", kept.join("&"))
    };
    // Redirect::to yields 303 See Other (a GET on the clean URL).
    let mut resp = Redirect::to(&location).into_response();
    attach_appearance_cookie(&mut resp, appearance, base);
    Some(resp)
}

/// Project pages: any [`STRIPPED_QUERY_KEYS`] key → 303 to the clean URL,
/// setting only the appearance cookie. A guard-return at the top of each
/// project handler, before any rendering.
fn override_redirect(uri: &Uri, appearance: &RequestedAppearance, base: &Base) -> Option<Response> {
    redirect_without_keys(uri, STRIPPED_QUERY_KEYS, appearance, base)
}

/// Create a handler closure for a specific project page.
///
/// Captures `project` and `page_idx` by value. The handler:
/// 1. Reads `RequestedLang` from request extensions (default: empty prefs, for router-only tests
///    that do not layer the middleware).
/// 2. Resolves the language against the project's declared languages.
/// 3. Builds the page's data with one `Project::page_data` call (`PageScope::Page`); route params
///    are empty for static pages — validation (`UndeclaredRouteParam`) forbids `$param` ids on
///    static routes, so this is a safe invariant.
/// 4. Calls `render_page` to produce the full HTML document.
/// 5. If `from_query` is set, attaches a `Set-Cookie` response header using the resolved language
///    code (never the raw `?lang=` value).
fn make_handler(
    project: Arc<cpe_web::project::Project>,
    page_idx: usize,
    origin_cfg: crate::request::OriginConfig,
    base: Base,
) -> impl Fn(Request) -> std::future::Ready<Response> + Clone + Send + 'static {
    move |req: Request| {
        let public_origin = origin_cfg.resolve(req.headers());
        let path = base.request_path(req.uri().path());
        let requested = req
            .extensions()
            .get::<RequestedLang>()
            .cloned()
            .unwrap_or(RequestedLang { prefs: Vec::new(), from_query: false });
        let requested_appearance = req
            .extensions()
            .get::<RequestedAppearance>()
            .cloned()
            .unwrap_or(RequestedAppearance { value: None, from_query: false });

        let page = project
            .config()
            .pages
            .get(page_idx)
            .expect("page index from boot-built router is valid; pages Vec is immutable after boot");

        let lang = cpe_core::lang::resolve(&requested.prefs, &project.config().site);
        let cookie_value = lang.as_str().to_owned();

        // `?appearance=` or a retired knob key: 303 to the clean URL (setting
        // only the appearance cookie). Guard-return before rendering, so the
        // page re-renders from the cookie on the redirected request.
        if let Some(redirect) = override_redirect(req.uri(), &requested_appearance, &base) {
            return std::future::ready(redirect);
        }

        let query_str = req.uri().query().map(ToOwned::to_owned);
        let selected_image = crate::request::query_img(req.uri().query());
        let strip_open = crate::request::query_strip(req.uri().query());

        // Static pages have no path params; $param ids on static routes are
        // forbidden by validation (`UndeclaredRouteParam`), so an empty map is
        // always correct.
        let route_params = HashMap::new();
        let params = RequestParams(query_str.as_deref());
        let request = cpe_core::view_data::PageRequest {
            route_params: &route_params,
            params: &params,
            scope: cpe_core::view_data::PageScope::Page,
        };
        let cpe_core::view_data::PageData {
            components,
            status,
            media_items,
            media_thumbs,
            compound_members,
            compound_parents,
            incoming_links,
            resource_links,
        } = project.page_data(page, &request, &lang);
        let http_status = status_code(status);
        // Canonical page 303 — guard-return, HTML page handler only.
        if let Some(redirect) = canonical_page_redirect(&path, query_str.as_deref(), http_status, &components) {
            return std::future::ready(redirect);
        }
        let ctx = cpe_components::component::context::RenderContext {
            base: &base,
            lang,
            project_id: project.id(),
            current_path: &path,
            current_query: query_str.as_deref(),
            route_params: &[],
            appearance: requested_appearance.value.unwrap_or_default(),
            selected_image,
            strip_open,
            link_routes: project.class_routes(),
            media_properties: project.media_properties(),
            media_items: &media_items,
            media_thumbs: &media_thumbs,
            compound_members: &compound_members,
            compound_parents: &compound_parents,
            class_list_routes: project.class_list_routes(),
            public_origin: &public_origin,
            incoming_links: incoming_links.as_ref(),
            resource_links: &resource_links,
        };
        let markup = cpe_web::page::render_page(
            &project.config().site,
            page,
            project.narrative(),
            &ctx,
            &components,
            project.data_arks(),
        );

        // Overrides are never `from_query` here (that path 303-redirects above),
        // so only the lang cookie is set on a rendered response.
        let mut resp = (http_status, markup).into_response();
        attach_lang_cookie(&mut resp, &requested, &cookie_value, &base);

        std::future::ready(resp)
    }
}

/// Create a handler closure for a page that has path parameters (e.g. `/characters/:id`).
///
/// Extends `make_handler`'s logic with:
/// 1. Axum `Path<HashMap<String,String>>` extraction for the captured params.
/// 2. `route_params` populated in `RenderContext` from the extracted map.
/// 3. One `Project::page_data` call (`PageScope::Page`), which resolves both literal ids and
///    `$param` references.
/// 4. HTTP status mapping: `Ok(Some)` → 200; `Ok(None)` → 404 (page still rendered with the
///    not-found body inside chrome); `Err(QueryError)` → 500 (logged).
fn make_parameterized_handler(
    project: Arc<cpe_web::project::Project>,
    page_idx: usize,
    origin_cfg: crate::request::OriginConfig,
    base: Base,
) -> impl Fn(Path<HashMap<String, String>>, Request) -> std::future::Ready<Response> + Clone + Send + 'static {
    move |Path(path_params): Path<HashMap<String, String>>, req: Request| {
        let public_origin = origin_cfg.resolve(req.headers());
        let path = base.request_path(req.uri().path());
        let requested = req
            .extensions()
            .get::<RequestedLang>()
            .cloned()
            .unwrap_or(RequestedLang { prefs: Vec::new(), from_query: false });
        let requested_appearance = req
            .extensions()
            .get::<RequestedAppearance>()
            .cloned()
            .unwrap_or(RequestedAppearance { value: None, from_query: false });

        let page = project
            .config()
            .pages
            .get(page_idx)
            .expect("page index from boot-built router is valid; pages Vec is immutable after boot");

        let lang = cpe_core::lang::resolve(&requested.prefs, &project.config().site);
        let cookie_value = lang.as_str().to_owned();

        // Build route_params from the extracted Path map, preserving the page's
        // declared param order for deterministic downstream use.
        let route_params: Vec<(String, String)> = path_params.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // Any stripped key → 303 to the clean URL (guard-return).
        if let Some(redirect) = override_redirect(req.uri(), &requested_appearance, &base) {
            return std::future::ready(redirect);
        }

        let query_str = req.uri().query().map(ToOwned::to_owned);
        let selected_image = crate::request::query_img(req.uri().query());
        let strip_open = crate::request::query_strip(req.uri().query());

        let params = RequestParams(query_str.as_deref());
        let request = cpe_core::view_data::PageRequest {
            route_params: &path_params,
            params: &params,
            scope: cpe_core::view_data::PageScope::Page,
        };
        let cpe_core::view_data::PageData {
            components,
            status,
            media_items,
            media_thumbs,
            compound_members,
            compound_parents,
            incoming_links,
            resource_links,
        } = project.page_data(page, &request, &lang);
        let http_status = status_code(status);
        // Canonical page 303 — guard-return, HTML page handler only.
        if let Some(redirect) = canonical_page_redirect(&path, query_str.as_deref(), http_status, &components) {
            return std::future::ready(redirect);
        }
        let ctx = cpe_components::component::context::RenderContext {
            base: &base,
            lang,
            project_id: project.id(),
            current_path: &path,
            current_query: query_str.as_deref(),
            route_params: &route_params,
            appearance: requested_appearance.value.unwrap_or_default(),
            selected_image,
            strip_open,
            link_routes: project.class_routes(),
            media_properties: project.media_properties(),
            media_items: &media_items,
            media_thumbs: &media_thumbs,
            compound_members: &compound_members,
            compound_parents: &compound_parents,
            class_list_routes: project.class_list_routes(),
            public_origin: &public_origin,
            incoming_links: incoming_links.as_ref(),
            resource_links: &resource_links,
        };
        let markup = cpe_web::page::render_page(
            &project.config().site,
            page,
            project.narrative(),
            &ctx,
            &components,
            project.data_arks(),
        );

        // Overrides are never `from_query` here (that path 303-redirects above).
        let mut resp = (http_status, markup).into_response();
        attach_lang_cookie(&mut resp, &requested, &cookie_value, &base);

        std::future::ready(resp)
    }
}

/// Strip the DataStar `datastar=<json>` marker from a raw request query string.
///
/// DataStar appends a `datastar=<json>` key-value pair to every `@get(…)` request; each
/// fragment handler removes it before re-deriving `current_query`, so hrefs/params rendered
/// back into the swapped-in fragment don't accumulate the marker on the next round-trip.
///
/// Splits on `&`, drops empty pairs and the pair whose key (the part before the first `=`,
/// or the whole pair if there is no `=`) is exactly `"datastar"`, then rejoins the remaining
/// pairs with `&` — same separator, same order. Returns `None` iff `raw_query` is `None`;
/// returns `Some("")` if nothing is left after filtering (callers that want `Option<&str>`
/// with empty-string collapsed to `None` do that themselves, e.g. via
/// `.as_deref().filter(|s| !s.is_empty())`, matching every call site).
fn strip_datastar_marker(raw_query: Option<&str>) -> Option<String> {
    raw_query.map(|q| {
        let kept: Vec<&str> = q
            .split('&')
            .filter(|kv| !kv.is_empty())
            .filter(|kv| {
                let key = kv.split('=').next().unwrap_or(kv);
                key != "datastar"
            })
            .collect();
        kept.join("&")
    })
}

/// Compute the parent-page path by stripping the trailing `/<segment>` fragment suffix
/// (e.g. `/_facets`) from the request path.
///
/// Every DataStar fragment handler needs the *page's* path (not the fragment endpoint's) so
/// hrefs it emits stay page-relative instead of accumulating a repeated `/_<segment>/_<segment>`
/// suffix. A fragment's route is only ever reached as `<page-path>/<segment>`, so a path without
/// the suffix means a caller passed the wrong segment: a debug build panics on it, and a release
/// build returns `full_path` unchanged.
fn strip_fragment_suffix<'a>(full_path: &'a str, segment: &str) -> &'a str {
    let page = full_path.strip_suffix(segment).and_then(|page| page.strip_suffix('/'));
    debug_assert!(
        page.is_some(),
        "{full_path:?} does not end with the fragment segment /{segment}"
    );
    page.unwrap_or(full_path)
}

/// Build the "throwaway" [`cpe_components::component::context::RenderContext`] the crumb-patch
/// helper below needs — one shared by all three fragment handlers
/// (`_facets`/`_sort`/`_search`). A
/// crumb-line re-render only ever reads `base`/`lang`/`project_id`/`current_path`/
/// `current_query`/`link_routes`/`media_properties`/
/// `class_list_routes`/`public_origin` off the context — every resource/
/// media/facet-selection field is irrelevant to composing a crumb line, so
/// this helper hardcodes them to their empty/default values rather than
/// making every call site plumb a dozen fields it will never use.
fn crumb_render_ctx<'a>(
    req: &CrumbPatchRequest<'a>,
    empty_compound_parents: &'a HashMap<String, cpe_core::ir::Resource>,
    empty_media_thumbs: &'a HashMap<String, cpe_core::ir::MediaThumb>,
) -> cpe_components::component::context::RenderContext<'a> {
    let project = req.project;
    cpe_components::component::context::RenderContext {
        base: req.base,
        lang: req.lang.clone(),
        project_id: project.id(),
        current_path: req.current_path,
        current_query: req.current_query,
        route_params: &[],
        appearance: cpe_core::theme::Appearance::default(),
        selected_image: 0,
        strip_open: true,
        link_routes: project.class_routes(),
        media_properties: project.media_properties(),
        media_items: &[],
        media_thumbs: empty_media_thumbs,
        compound_members: &[],
        compound_parents: empty_compound_parents,
        class_list_routes: project.class_list_routes(),
        public_origin: req.public_origin,
        incoming_links: None,
        resource_links: &[],
    }
}

/// The shared per-request fields the crumb-patch helper needs — bundled
/// into one struct so the builder fn below (and its three call sites)
/// doesn't carry six near-identical positional arguments (clippy's
/// `too_many_arguments`).
struct CrumbPatchRequest<'a> {
    project: &'a cpe_web::project::Project,
    page: &'a cpe_core::ir::Page,
    lang: &'a cpe_core::lang::Lang,
    current_path: &'a str,
    current_query: Option<&'a str>,
    public_origin: &'a str,
    base: &'a Base,
}

/// Build the crumb-patch SSE `Event` for the BROWSE crumb-line variant —
/// shared by the `_facets`/`_sort`/`_search` fragment handlers.
/// Recomposes the crumb line via the SAME
/// `cpe_web::page::render_browse_breadcrumb_for_page` helper the full-page
/// render calls — the caller must never re-derive the crumb inline. Returns
/// `None` when the page has no id-bearing `ResourceList` (the helper's own
/// contract); every caller here is only ever registered for a page that
/// does have one, but the branch degrades gracefully rather than panicking
/// if that invariant is ever violated.
fn browse_crumb_patch_event(
    req: &CrumbPatchRequest<'_>,
    comp_data: &[cpe_core::view_data::ComponentData],
    list_id: &str,
) -> Option<Event> {
    let empty_compound_parents: HashMap<String, cpe_core::ir::Resource> = HashMap::new();
    let empty_media_thumbs: HashMap<String, cpe_core::ir::MediaThumb> = HashMap::new();
    let crumb_ctx = crumb_render_ctx(req, &empty_compound_parents, &empty_media_thumbs);
    let crumb_markup =
        cpe_web::page::render_browse_breadcrumb_for_page(req.page, &crumb_ctx, comp_data, &req.project.config().site)?;
    let crumb_id = cpe_components::component::breadcrumb::crumb_element_id(&format!("share-{list_id}"));
    let crumb_selector = format!("#{crumb_id}");
    Some(PatchElements::new(crumb_markup.into_string()).selector(&crumb_selector).into())
}

/// Build the `datastar-patch-signals` SSE `Event` that resets the
/// page-global `$copied` "Copied" receipt to `false`.
///
/// `$copied` is a DataStar signal, not an element — it survives ANY
/// element patch untouched (re-applying `data-signals="{copied:false}"`
/// on freshly patched crumb markup does NOT reset an already-set
/// signal, and `data-class="{'is-copied': $copied}"` just re-applies the
/// stale `true` to the new markup). Every refinement fragment response
/// (`_facets`/`_sort`/`_search`) must emit this event
/// alongside its `PatchElements` events so a refinement landing mid-receipt
/// clears the "Copied" state — the copied link is no longer this view.
/// One shared fn so all three handlers can't drift on the signal name or
/// the JSON shape.
///
/// `_dataCopied` is the dataset panel's receipt (`cpe-components`'
/// `data_links::DATA_SIGNALS`), reset the same way; on a page without a panel
/// the key is created and never read.
fn copied_signal_reset_event() -> Event {
    PatchSignals::new("{copied: false, _dataCopied: ''}").into()
}

/// Create a handler closure for a DataStar SSE fragment for the
/// compound-viewer member pager, for a STATIC page (no path params): a
/// URL-writing `pushState` swap for FORWARD clicks only (Back/Forward is a
/// real page load and never reaches this handler). Mirrors
/// `make_facet_fragment_handler`'s overall shape (arg shape,
/// `strip_datastar_marker`/`strip_fragment_suffix` use) — the route-param
/// sibling below is the one exception (real `Path` extraction,
/// `make_parameterized_handler`'s pattern), since a static-source
/// compound-viewer never needs one.
///
/// This handler is registered at
/// `/{project}/{page}/{COMPOUND_FRAGMENT_SEGMENT}` for every STATIC page
/// whose body contains a `CompoundViewer` component. It renders the viewer +
/// crumb + header controls (and, on an annotated reader, the
/// `#cv-annotations` register) via
/// `cpe_web::page::render_compound_fragment`, resets the annotation signals
/// (annotated readers only), `$copied` and (on a `layout="pages"` swap) the
/// viewer's page-global signals, and pushes the `pushState` +
/// `window.__cpeReaderUrl` update + title/focus `ExecuteScript` — see
/// [`compound_fragment_response`]'s own doc comment for the full event shape
/// and ordering.
fn make_compound_fragment_handler(
    project: Arc<cpe_web::project::Project>,
    page_idx: usize,
    origin_cfg: crate::request::OriginConfig,
    base: Base,
) -> impl Fn(Request) -> std::future::Ready<Response> + Clone + Send + 'static {
    move |req: Request| {
        std::future::ready(compound_fragment_response(
            &project,
            page_idx,
            &origin_cfg,
            &base,
            &req,
            &HashMap::new(),
        ))
    }
}

/// The route-param sibling of [`make_compound_fragment_handler`] — extracts a
/// real `Path<HashMap<String, String>>` (`make_parameterized_handler`'s
/// pattern), because a route-param compound page's `compound-viewer
/// source="$id"` needs the real `:id` value to resolve at all (Incunabula's
/// book reader, `/incunabula/books/:id`).
/// `make_facet_fragment_handler`'s param-less shape (`&HashMap::new()`) is
/// fine for `_facets`/`_sort`/`_search` — no facet/sort/search page needs a
/// route param to resolve — but wrong here.
fn make_compound_fragment_handler_param(
    project: Arc<cpe_web::project::Project>,
    page_idx: usize,
    origin_cfg: crate::request::OriginConfig,
    base: Base,
) -> impl Fn(Path<HashMap<String, String>>, Request) -> std::future::Ready<Response> + Clone + Send + 'static {
    move |Path(path_params): Path<HashMap<String, String>>, req: Request| {
        std::future::ready(compound_fragment_response(
            &project,
            page_idx,
            &origin_cfg,
            &base,
            &req,
            &path_params,
        ))
    }
}

/// Shared body for both compound-fragment handlers above — the only
/// difference between a static and a route-param compound page is whether
/// `route_params` is empty or populated from a real `Path` extraction.
///
/// SSE must NOT 303: same rationale as the facet/sort/search handlers —
/// `override_redirect` never runs here (only `datastar` is stripped from
/// `current_query`), and a redirect would break the SSE response;
/// `canonical_page_redirect` is never called from any fragment
/// handler, this one included.
///
/// This handler is reached ONLY by a JS-enhanced FORWARD click (a pager
/// step, a rail link, or a page-picker entry). Back/Forward is a real page
/// load: it never calls `@get` and never reaches this handler; see
/// `cpe-components`' `data-on:popstate__window` doc comment
/// (`component::compound_viewer`) for the `location.reload()` mechanism.
/// There is therefore no `pushState`-vs-`replaceState` discrimination to
/// make here — every response always `pushState`s.
///
/// Event order (mirroring how
/// `_facets`/`_sort`/`_search` order their own events): `PatchElements` for
/// `#compound-viewer`, then an optional `PatchElements` for `#crumb-<id>`,
/// then a `PatchElements` for `#header-controls` (every swap, so the
/// language and appearance hrefs follow the new `?page=`; the appearance is
/// the request's `RequestedAppearance`, as on a full load),
/// then, on an `annotations=#true` reader only, a `PatchElements` for
/// `#cv-annotations` followed by a `PatchSignals` resetting the annotation
/// signals (`cpe_components::component::annotation::annotation_reset_signals_json`), then an
/// optional `PatchSignals` resetting the `<image-viewer>` unit's
/// page-global signals (`layout="pages"` swaps only), then
/// `copied_signal_reset_event()` (every swap), then one `ExecuteScript`
/// combining `pushState` + the `window.__cpeReaderUrl` update + title +
/// focus-restore.
fn compound_fragment_response(
    project: &Arc<cpe_web::project::Project>,
    page_idx: usize,
    origin_cfg: &crate::request::OriginConfig,
    base: &Base,
    req: &Request,
    route_params: &HashMap<String, String>,
) -> Response {
    let public_origin = origin_cfg.resolve(req.headers());
    let requested = req
        .extensions()
        .get::<RequestedLang>()
        .cloned()
        .unwrap_or(RequestedLang { prefs: Vec::new(), from_query: false });
    // The re-sent header toggle reads the appearance, so it must be the
    // request's own (query, else cookie), exactly as a full load resolves it.
    let appearance = req
        .extensions()
        .get::<RequestedAppearance>()
        .and_then(|a| a.value)
        .unwrap_or_default();

    let page = project
        .config()
        .pages
        .get(page_idx)
        .expect("page index from boot-built router is valid; pages Vec is immutable after boot");

    let lang = cpe_core::lang::resolve(&requested.prefs, &project.config().site);

    // Strip the `datastar` marker (mirrors the facet/sort/search handlers)
    // so hrefs rendered back into the swapped-in fragment don't accumulate it.
    let raw_query = req.uri().query();
    let filtered_query: Option<String> = strip_datastar_marker(raw_query);
    let current_query: Option<&str> = filtered_query.as_deref().filter(|s| !s.is_empty());

    // Compute current_path: strip the trailing `/_compound` suffix so
    // emitted hrefs remain page-relative (avoid `…/_compound/_compound`).
    let full_path = base.request_path(req.uri().path());
    let current_path = strip_fragment_suffix(
        &full_path,
        cpe_components::component::compound_viewer::COMPOUND_FRAGMENT_SEGMENT,
    );

    // An erroring fragment answers a bare 500; the store then runs no side
    // lookup. The fragment never renders the Apparat's resource links.
    let params = RequestParams(current_query);
    let request = cpe_core::view_data::PageRequest {
        route_params,
        params: &params,
        scope: cpe_core::view_data::PageScope::CompoundFragment,
    };
    let cpe_core::view_data::PageData {
        components: comp_data,
        status,
        media_items,
        media_thumbs,
        compound_members,
        compound_parents,
        incoming_links,
        ..
    } = project.page_data(page, &request, &lang);
    if status == cpe_core::view_data::PageStatus::Error {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let route_params_vec: Vec<(String, String)> = route_params.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

    let ctx = cpe_components::component::context::RenderContext {
        base,
        lang: lang.clone(),
        project_id: project.id(),
        current_path,
        current_query,
        route_params: &route_params_vec,
        appearance,
        selected_image: 0,
        strip_open: true,
        link_routes: project.class_routes(),
        media_properties: project.media_properties(),
        media_items: &media_items,
        media_thumbs: &media_thumbs,
        compound_members: &compound_members,
        compound_parents: &compound_parents,
        class_list_routes: project.class_list_routes(),
        public_origin: &public_origin,
        incoming_links: incoming_links.as_ref(),
        resource_links: &[],
    };

    // `render_compound_fragment` internally re-derives the viewer
    // markup, crumb line, title, and canonical URL via the SAME calls
    // `render_page` makes for this page — so this handler cannot drift from
    // the full-page render on any of the four.
    let Some(output) = cpe_web::page::render_compound_fragment(
        &project.config().site,
        page,
        project.narrative(),
        &ctx,
        &comp_data,
        project.data_arks(),
    ) else {
        // No CompoundViewer component resolved data for this page — should
        // not happen (registration guard), but degrade to a bare non-200
        // status rather than panic, mirroring the other fragment handlers'
        // "boot invariant violated" defensive branches (bare status, empty
        // body — the actual convention those handlers use, not a full HTML
        // page morphed into an SSE target).
        tracing::error!(
            page = page.route.as_str(),
            "_compound fragment handler reached for a page with no resolved \
             CompoundViewer data — registration guard drift"
        );
        return StatusCode::NOT_FOUND.into_response();
    };

    let viewer_event: Event = PatchElements::new(output.viewer.into_string())
        .selector("#compound-viewer")
        .into();

    let mut events = vec![Ok::<Event, Infallible>(viewer_event)];

    // The crumb patch is an `Option` (mirroring the facet/sort/search
    // handlers' own defensive crumb degrade): `render_compound_fragment`
    // returns `None` for a compound-viewer page with no sibling
    // `ResourceDetail`/`ResourceList`/`Narrative` component to derive a
    // crumb line from — uncommon (every real project pairs one), but not a
    // hard requirement.
    if let (Some(crumb), Some(crumb_id)) = (output.crumb, output.crumb_id) {
        let crumb_selector = format!("#{crumb_id}");
        let crumb_event: Event = PatchElements::new(crumb.into_string()).selector(&crumb_selector).into();
        events.push(Ok::<Event, Infallible>(crumb_event));
    }

    // The header controls, on every swap: their language and appearance hrefs
    // must link to the page the address bar is about to show.
    let header_selector = format!("#{}", cpe_core::markup_contract::HEADER_CONTROLS_ID);
    let header_event: Event = PatchElements::new(output.header_controls.into_string())
        .selector(&header_selector)
        .into();
    events.push(Ok::<Event, Infallible>(header_event));

    // The annotation register's target, a second reader patch target outside
    // `#compound-viewer` (`annotations=#true` only; the empty target on a page
    // without a register). Before the signal resets, so a reset of the
    // register's own signals lands on its final DOM.
    if let Some(annotations) = output.annotations {
        let selector = format!("#{}", cpe_components::component::annotation::ANNOTATIONS_TARGET_ID);
        let annotations_event: Event = PatchElements::new(annotations.into_string()).selector(&selector).into();
        events.push(Ok::<Event, Infallible>(annotations_event));
        // Hover, pin and arrival belong to the page just replaced; the
        // toggle (`_annoShow`) is never reset, so it survives the turn. After
        // the viewer patch, so the swapped-in stage's effect re-runs with it.
        let reset = PatchSignals::new(cpe_components::component::annotation::annotation_reset_signals_json());
        events.push(Ok::<Event, Infallible>(reset.into()));
    }

    // Reset the `<image-viewer>` unit's TWO page-global signal
    // groups (`.rv` and the separate `.rv__canvas` zoom-state group — see
    // `component::media::render_viewer_unit`/`render_viewer_canvas` for the
    // exact declared initial values this mirrors) on a `layout="pages"`
    // swap only. Re-declaring `data-signals` on the freshly patched markup
    // does NOT reset an already-changed signal client-side — without this event a
    // visitor who opened Adjust/Details or zoomed in before paging keeps
    // stale drawer/zoom/scale state after the swap. `layout="contents"` has
    // no viewer unit and skips this event entirely. Both signal groups have
    // disjoint top-level keys, so one `PatchSignals` event covers both.
    let is_pages_layout = page.body.iter().any(|c| {
        matches!(
            c,
            cpe_core::ir::Component::CompoundViewer { layout: cpe_core::ir::CompoundLayout::Pages, .. }
        )
    });
    if is_pages_layout {
        // The reset literal is composed by
        // `cpe_components::component::media::viewer_reset_signals_json` (mirrors
        // `component::media`'s own `.rv`/`.rv__canvas` `data-signals`
        // declarations), not hand-written here, so the two can never drift
        // on key set.
        let reset = PatchSignals::new(cpe_components::component::media::viewer_reset_signals_json());
        events.push(Ok::<Event, Infallible>(reset.into()));
    }

    // Also on EVERY swap (both layouts): reset the share-view `$copied`
    // signal, exactly like the other three fragment handlers do alongside
    // their own crumb patch: any swap that patches the crumb resets `copied`.
    events.push(Ok::<Event, Infallible>(copied_signal_reset_event()));

    // pushState + the window.__cpeReaderUrl update + title + focus script.
    // This handler is reached ONLY by a forward click (see the fn doc comment
    // above), so there is no popstate-vs-forward-click discrimination to make:
    // every response always pushes state, unconditionally.
    //
    // The canonical URL is anchor-free at the source (`share_href_for_reader`
    // and every other reader href/action builder): no call-site stripping is
    // needed.
    let canonical_url = output.canonical_url.as_str();
    // `pushState` must receive a RELATIVE (origin-free) URL, never the
    // absolute `canonical_url`: per the WHATWG spec it throws
    // `SecurityError` if the given URL's origin differs from the document's
    // real origin, a plausible failure mode given `public_origin` is itself
    // derived per-request from headers. A relative URL is
    // always same-origin by construction. `canonical_url` was built from
    // this SAME `public_origin` (via `ctx.public_origin`), so stripping it
    // back off is a guaranteed-matching prefix strip, not a best-effort one.
    let relative_url = canonical_url
        .strip_prefix(public_origin.trim_end_matches('/'))
        .unwrap_or(canonical_url);
    let escaped_relative = cpe_core::util::js_string_escape(relative_url);
    // Both the URL and the title are escaped via `js_string_escape` —
    // titles are free-form prose (an apostrophe, in principle a
    // `</script>` substring, or a line-terminator character would each
    // break the single-quoted JS literal / the surrounding `<script>` tag
    // otherwise).
    let escaped_title = cpe_core::util::js_string_escape(&output.title);

    // `window.__cpeReaderUrl` is the plain, one-way "URL the reader
    // currently shows" value the popstate listener compares against
    // (`cpe_components::component::compound_viewer`'s `READER_URL_INIT_SCRIPT`/
    // `POPSTATE_RELOAD_EXPR`, NOT a DataStar signal — a fragment swap never
    // re-runs the inline init script that seeds it on a normal page load,
    // so this write, right alongside `pushState`, is the only thing that
    // keeps it in sync with the address bar after a JS-enhanced click).
    let script = format!(
        "history.pushState(null, '', '{escaped_relative}'); \
         window.__cpeReaderUrl = '{escaped_relative}'; \
         document.title = '{escaped_title}'; \
         document.getElementById('compound-viewer')?.focus({{ preventScroll: true }});"
    );
    let script_event: Event = ExecuteScript::new(script).into();
    events.push(Ok::<Event, Infallible>(script_event));

    let sse_stream = stream::iter(events);
    Sse::new(sse_stream).into_response()
}

/// Everything `cpe_web::page::SearchFragmentSurface::resolve` needs for one
/// `resource-list`, scanned out of `page.body` + its resolved `ComponentData`.
///
/// The three fragment handlers (`_facets`/`_sort`/`_search`) all call
/// `render_resource_list`/`render_active_filters` DIRECTLY — unlike the
/// initial page render (`cpe_web::page::render_page` → `render_body`), which
/// resolves this internally — so each needs this same lookup. Kept as one
/// shared helper rather than three copies.
struct SearchWiring<'a> {
    /// The list's `Component::ResourceList.class` (empty string if somehow
    /// not found — defensive; every handler that calls this already knows
    /// `list_id` names a real `ResourceList` in `page.body`).
    class: &'a str,
    /// The sibling facet-panel's RESOLVED `FacetGroup` data, or `None` when
    /// this page has no facet-panel targeting `list_id` (or its data didn't
    /// resolve this request) — the SAME source `render_active_filters`'s own
    /// "Clear all" uses, never `Component::FacetPanel.facets` (config).
    facet_groups: Option<&'a [cpe_core::ir::FacetGroup]>,
    /// `Some(scope)` iff a `Component::SearchBar` targets `list_id` at all
    /// (`scope` itself may be `None` — undeclared `scope` attr). `None` means
    /// no search-bar targets this list — the caller must not attempt to
    /// build a `SearchFragmentSurface` in that case.
    search_bar_scope: Option<Option<&'a str>>,
}

/// Scan `page.body` (zipped with its resolved `comp_data`) for `list_id`'s
/// class, its sibling facet-panel's resolved data, and whether a search-bar
/// targets it at all.
fn resolve_search_wiring<'a>(
    page: &'a cpe_core::ir::Page,
    comp_data: &'a [cpe_core::view_data::ComponentData],
    list_id: &str,
) -> SearchWiring<'a> {
    let mut class: &str = "";
    let mut facet_groups: Option<&[cpe_core::ir::FacetGroup]> = None;
    let mut search_bar_scope: Option<Option<&str>> = None;

    for (c, d) in page.body.iter().zip(comp_data.iter()) {
        match c {
            cpe_core::ir::Component::ResourceList { id: Some(id), class: cls, .. } if id.as_str() == list_id => {
                class = cls.as_str();
            }
            cpe_core::ir::Component::FacetPanel { target, .. } if target == list_id => {
                if let cpe_core::view_data::ComponentData::Facets(groups) = d {
                    facet_groups = Some(groups.as_slice());
                }
            }
            cpe_core::ir::Component::SearchBar { target, scope, .. } if target == list_id => {
                search_bar_scope = Some(scope.as_deref());
            }
            _ => {}
        }
    }

    SearchWiring { class, facet_groups, search_bar_scope }
}

/// Build the `Option<cpe_web::page::SearchFragmentSurface>` for `list_id`, or
/// `None` when NEITHER a search-bar targets it NOR any facet is actually
/// active. A page with a facet-panel but no search-bar can narrow to zero
/// too, so the surface does not depend on a search-bar. `filters_active` is
/// [`cpe_core::ir::any_facet_selected`] on `wiring.facet_groups` — the SAME
/// predicate that gates `unfiltered_total` in the store's page builder
/// (`query::page`), so copy and count can never disagree.
///
/// **`?q=` is never read when no search-bar targets `list_id`** —
/// `read_search_query_param` is only called inside the `Some(scope)` arm; the
/// `None` arm passes an empty `q` without touching the query string's `q` key
/// at all (mirrors `cpe_web::page`'s `build_narrowed_owned`).
///
/// The caller then passes `.as_ref().and_then(|s| s.search_surface())`/
/// `.active_query()`/`.narrowed_surface()` to `ResourceListCfg`/
/// `render_active_filters` exactly like `cpe_web::page`'s `render_body` does
/// internally.
fn build_search_fragment_surface(
    project: &cpe_web::project::Project,
    page: &cpe_core::ir::Page,
    comp_data: &[cpe_core::view_data::ComponentData],
    list_id: &str,
    current_path: &str,
    current_query: Option<&str>,
    lang: &cpe_core::lang::Lang,
) -> Option<cpe_web::page::SearchFragmentSurface> {
    let wiring = resolve_search_wiring(page, comp_data, list_id);
    let filters_active = wiring.facet_groups.map(cpe_core::ir::any_facet_selected).unwrap_or(false);
    let (scope, q) = match wiring.search_bar_scope {
        Some(scope) => (scope, read_search_query_param(current_query)),
        None => {
            if !filters_active {
                return None;
            }
            (None, String::new())
        }
    };
    Some(cpe_web::page::SearchFragmentSurface::resolve(
        &project.config().site,
        scope,
        wiring.class,
        q,
        current_path,
        current_query,
        wiring.facet_groups,
        lang,
    ))
}

/// Create a handler closure for a DataStar SSE fragment for the facet panel and its target list.
///
/// This handler is registered at `/{project}/{page}/{FACET_FRAGMENT_SEGMENT}` for every
/// page (static **and** parameterised) whose body contains a `FacetPanel` component. When
/// the DataStar `@get(…)` fires on a facet toggle click, this handler:
///
/// 1. Strips the `datastar` key from the query string (DataStar appends it on `@get` requests;
///    removing it prevents the re-rendered toggle hrefs from accumulating it on the next click —
///    matches the compound handler).
/// 2. Calls `Project::page_data` with the stripped query so both `Facets(..)` and the narrowed
///    `List(..)` are built with the correct active facet state.
/// 3. Renders the `FacetPanel` slot via `render_facet_panel`, the same slot's active-filter chip
///    row via `render_active_filters`, and the target `ResourceList` slot via
///    `render_resource_list`, using the same cfg-building logic as the page handler (the shared
///    `ResourceListCfg::with_links` and direct `FacetPanelCfg` construction —
///    `render_active_filters` reuses the same `FacetPanelCfg` as the panel).
/// 4. Emits **four** `PatchElements` events in one SSE stream — panel, filterbar, list (any order
///    is safe; this order mirrors `render_body`'s composition order) and the recomposed crumb line.
///    Panel selector: `#facets-<list-id>`; filterbar selector: `#filters-<list-id>`; list selector:
///    `#<list-id>`; crumb selector: `#crumb-<list-id>`. The filterbar is its own patch region —
///    without it the chip row would go stale on a JS facet interaction (a removed chip would
///    linger; Clear-all lives inside the row it must clear). The crumb event is an `Option` (a
///    defensive degrade when the page has no id-bearing resource-list), so this is "up to four".
///    `_sort` still morphs only the list + crumb — sort never changes facets.
/// 5. Emits, in addition, ONE `PatchSignals` `$copied` reset event (`copied_signal_reset_event` — a
///    distinct SSE event type, not counted towards the `datastar-patch-elements` total above), so a
///    refinement landing mid-receipt clears the "Copied" state: five SSE events in all.
///
/// SSE must NOT 303: the `override_redirect` guard is deliberately absent — only `datastar`
/// is stripped from `current_query`. A redirect would break the SSE response.
///
/// This handler takes only `Request` (no `Path` extractor). For a parameterised faceted page,
/// the route path ends with `/{segment}` after any path params, so the same
/// no-Path-extractor approach works: path params are NOT in the
/// `_facets` URL (they are part of the registered route prefix).
///
/// ## Shared cfg-builder contract
///
/// Both the page handler (via `render_component` in `cpe-web`) and this fragment handler
/// build `FacetPanelCfg` and `ResourceListCfg` from the same inputs. `FacetPanelCfg` has
/// all-`pub` fields, so this crate constructs it directly. `ResourceListCfg` exposes
/// `ResourceListCfg::with_links` (a factory that hides the `pub(crate)` `LinkResolver`)
/// so this crate can build it without naming the crate-internal type.
fn make_facet_fragment_handler(
    project: Arc<cpe_web::project::Project>,
    page_idx: usize,
    origin_cfg: crate::request::OriginConfig,
    base: Base,
) -> impl Fn(Request) -> std::future::Ready<Response> + Clone + Send + 'static {
    move |req: Request| {
        let public_origin = origin_cfg.resolve(req.headers());
        let requested = req
            .extensions()
            .get::<RequestedLang>()
            .cloned()
            .unwrap_or(RequestedLang { prefs: Vec::new(), from_query: false });

        let page = project
            .config()
            .pages
            .get(page_idx)
            .expect("page index from boot-built router is valid; pages Vec is immutable after boot");

        let lang = cpe_core::lang::resolve(&requested.prefs, &project.config().site);

        // Build current_query from the request query, stripping the `datastar` key.
        // DataStar appends a `datastar=<json>` marker on @get requests; removing it
        // prevents the swapped-in hrefs from accumulating it (mirrors the compound handler).
        // The stripped query still carries the facet params (?role=R_01, etc.) so
        // FacetPanelCfg.current_query is correct for regenerating toggle hrefs.
        let raw_query = req.uri().query();
        let filtered_query: Option<String> = strip_datastar_marker(raw_query);
        let current_query: Option<&str> = filtered_query.as_deref().filter(|s| !s.is_empty());

        // Compute current_path: strip the trailing `/_facets` suffix so emitted
        // toggle hrefs remain page-relative (avoid `…/_facets/_facets`).
        let full_path = base.request_path(req.uri().path());
        let current_path =
            strip_fragment_suffix(&full_path, cpe_components::component::facet_panel::FACET_FRAGMENT_SEGMENT);

        // Resolve component data (static page — empty path params).
        // Project::page_data reads facet params and merges them into the list query;
        // the stripped current_query flows in so no `datastar=` appears in the results.
        let params = RequestParams(current_query);
        let route_params = HashMap::new();
        let request = cpe_core::view_data::PageRequest {
            route_params: &route_params,
            params: &params,
            scope: cpe_core::view_data::PageScope::ListFragment,
        };
        let cpe_core::view_data::PageData { components: comp_data, status, media_thumbs, .. } =
            project.page_data(page, &request, &lang);
        if status == cpe_core::view_data::PageStatus::Error {
            return std::future::ready(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }

        // Find the first FacetPanel component in page.body and locate its target.
        // Boot invariant: if a FacetPanel is configured, its target is a same-page
        // ResourceList with an id (UnresolvedTarget validation hard-fails otherwise).
        // NOTE: handles the first FacetPanel on the page only; a multi-panel page
        // would need a loop here.
        let list_id: &str = match page.body.iter().find_map(|c| {
            if let cpe_core::ir::Component::FacetPanel { target, .. } = c {
                Some(target.as_str())
            } else {
                None
            }
        }) {
            Some(id) => id,
            None => {
                // No FacetPanel in this page — should not happen (registration guard),
                // but return 404 defensively rather than panic. Log it: if it ever
                // fires, the registration guard and this handler have drifted.
                tracing::error!(
                    page = page.route.as_str(),
                    "_facets fragment handler reached for a page with no FacetPanel \
                     — registration guard drift"
                );
                return std::future::ready(StatusCode::NOT_FOUND.into_response());
            }
        };

        // Extract the Facets data and the List data from the component slice.
        // Zip page.body against comp_data to find the matching slots.
        let mut facet_groups: Option<&[cpe_core::ir::FacetGroup]> = None;
        let mut list_page: Option<&cpe_core::ir::ResourceListPage> = None;
        // Annotated-row config, captured from the target list so the
        // swapped-in fragment renders the same gloss + chips as the full page.
        let mut list_gloss: Option<&cpe_core::ir::Gloss> = None;
        let mut list_chips: &[cpe_core::ir::ChipSpec] = &[];
        let mut list_order_name_label: Option<&str> = None;
        let mut list_order_name_desc_label: Option<&str> = None;
        let mut list_order_default_label: Option<&str> = None;
        let mut list_order_options: &[cpe_core::ir::OrderOption] = &[];
        let mut list_grid = false;
        let mut list_tile_ratio = None;
        // The list's raw class name — threaded through `with_links` so
        // `render_surf_head` can resolve the eyebrow independent of `list.items`.
        let mut list_class: &str = "";

        for (c, d) in page.body.iter().zip(comp_data.iter()) {
            match (c, d) {
                (cpe_core::ir::Component::FacetPanel { .. }, cpe_core::view_data::ComponentData::Facets(groups)) => {
                    if facet_groups.is_none() {
                        facet_groups = Some(groups.as_slice());
                    }
                }
                (
                    cpe_core::ir::Component::ResourceList {
                        id: Some(id),
                        class,
                        gloss,
                        chips,
                        order_name_label,
                        order_name_desc_label,
                        order_default_label,
                        order_options,
                        grid,
                        tile_ratio,
                        ..
                    },
                    cpe_core::view_data::ComponentData::List(lp),
                ) => {
                    if id.as_str() == list_id && list_page.is_none() {
                        list_page = Some(lp);
                        list_gloss = gloss.as_ref();
                        list_chips = chips.as_slice();
                        list_order_name_label = order_name_label.as_deref();
                        list_order_name_desc_label = order_name_desc_label.as_deref();
                        list_order_default_label = order_default_label.as_deref();
                        list_order_options = order_options.as_slice();
                        list_grid = *grid;
                        list_tile_ratio = *tile_ratio;
                        list_class = class.as_str();
                    }
                }
                _ => {}
            }
        }

        let (groups, lp) = match (facet_groups, list_page) {
            (Some(g), Some(l)) => (g, l),
            _ => {
                // Data not found — return 500 (DB error or unexpected state).
                tracing::error!(
                    list_id,
                    "facet fragment handler: expected Facets + List slots not found in \
                     component data (boot invariant violated or DB error in Project::page_data)"
                );
                return std::future::ready(StatusCode::INTERNAL_SERVER_ERROR.into_response());
            }
        };

        // Build the FacetPanelCfg — all fields are pub, no factory needed.
        let panel_cfg = cpe_components::component::facet_panel::FacetPanelCfg { list_id, current_path, current_query };

        // Resolve the search-fragment surface — `None` when this list has no
        // search-bar at all. A facet toggle can happen while a search is active,
        // so this handler's swapped-in filterbar/list must stay in sync
        // with the active `?q=`, exactly like the initial page render.
        let search_data =
            build_search_fragment_surface(&project, page, &comp_data, list_id, current_path, current_query, &lang);
        let search_surface = search_data.as_ref().and_then(|s| s.search_surface());
        let active_query = search_data.as_ref().and_then(|s| s.active_query());
        // The narrowed-to-zero surface — `Some` iff an active search
        // query and/or an active facet is in effect, regardless of whether
        // this page has a search-bar at all.
        let narrowed_surface = search_data.as_ref().and_then(|s| s.narrowed_surface());

        // Resolve the chrome-id ordering-label overrides (id-only,
        // never literal text) — the same seam render_list_component uses
        // for a full-page/list render.
        let (order_name_label, order_name_desc_label, order_default_label) =
            cpe_core::labels::resolve_order_name_labels(
                &project.config().site,
                list_order_name_label,
                list_order_name_desc_label,
                list_order_default_label,
                &lang,
            );
        // The list's declared `order-by` rows, via
        // the same `pub` resolver `render_list_component` calls.
        let order_rows = cpe_core::labels::resolve_order_options(&project.config().site, list_order_options, &lang);

        // Build the ResourceListCfg via the shared factory (hides pub(crate) LinkResolver).
        let resolved_gloss = list_gloss.map(|g| cpe_core::labels::resolve_gloss(&project.config().site, g, &lang));
        let list_cfg = cpe_components::component::resource_list::ResourceListCfg::with_links(
            base.project(project.id()),
            project.class_routes(),
            current_path,
            current_query,
            Some(list_id),
            resolved_gloss.as_ref(),
            list_chips,
            list_class,
            &project.config().site.class_labels,
            search_surface,
            narrowed_surface,
            &media_thumbs,
            list_grid,
            list_tile_ratio,
            order_name_label,
            order_name_desc_label,
            order_default_label,
            &order_rows,
        );

        // Render panel + filterbar + list fragments. The filterbar reuses the
        // same panel_cfg (same list_id/current_path/current_query) — it is a
        // second view of the same `groups` data render_facet_panel renders.
        let panel_html =
            cpe_components::component::facet_panel::render_facet_panel(groups, &panel_cfg, &lang).into_string();
        let filterbar_html =
            cpe_components::component::facet_panel::render_active_filters(groups, &panel_cfg, &lang, active_query)
                .into_string();
        let list_html =
            cpe_components::component::resource_list::render_resource_list(lp, &list_cfg, &lang).into_string();

        // Build three PatchElements events: panel, filterbar, list.
        let panel_selector = format!("#facets-{list_id}");
        let filterbar_selector = format!("#filters-{list_id}");
        let list_selector = format!("#{list_id}");

        let panel_event: Event = PatchElements::new(panel_html).selector(&panel_selector).into();
        let filterbar_event: Event = PatchElements::new(filterbar_html).selector(&filterbar_selector).into();
        let list_event: Event = PatchElements::new(list_html).selector(&list_selector).into();

        let mut events = vec![
            Ok::<Event, Infallible>(panel_event),
            Ok::<Event, Infallible>(filterbar_event),
            Ok::<Event, Infallible>(list_event),
        ];

        // Fourth patch target: the crumb-line
        // recomposed for the new facet state. Built via the SAME
        // `render_browse_breadcrumb_for_page` helper `render_page`'s
        // initial render calls — never re-derive the crumb here.
        let crumb_req = CrumbPatchRequest {
            project: &project,
            page,
            lang: &lang,
            current_path,
            current_query,
            public_origin: &public_origin,
            base: &base,
        };
        if let Some(crumb_event) = browse_crumb_patch_event(&crumb_req, &comp_data, list_id) {
            events.push(Ok::<Event, Infallible>(crumb_event));
        }

        // Reset the page-global $copied "Copied" receipt — a refinement landing
        // mid-receipt clears it since the copied link is no longer this view.
        events.push(Ok::<Event, Infallible>(copied_signal_reset_event()));

        // Emit all five events in one SSE stream (panel, filterbar, list, crumb, copied-reset).
        let sse_stream = stream::iter(events);
        std::future::ready(Sse::new(sse_stream).into_response())
    }
}

/// Create a handler closure for a DataStar SSE fragment that re-renders only
/// the target `resource-list` after a sort change.
///
/// This handler is registered at `/{project}/{page}/{SORT_FRAGMENT_SEGMENT}` for
/// every page (static **and** parameterised) whose body contains a
/// `ResourceList` component. When a row in the ordering control's
/// `<details>` panel is submitted, this handler:
///
/// 1. Strips the `datastar` key from the query string (mirrors the facet and compound handlers).
/// 2. Calls `Project::page_data` with the stripped query — `?sort=` is already honoured there via
///    `resolve_sorts`, so the returned `List(..)` reflects the new sort order.
/// 3. Renders the first named (`id: Some(..)`) `ResourceList` slot via `render_resource_list` and
///    emits its `PatchElements` event targeting `#<list-id>`, plus a crumb-line `PatchElements`
///    event (present whenever the page has an id-bearing resource-list, which every
///    `_sort`-registered page does — the `Option` is a defensive degrade, not a text-diff check)
///    and a `PatchSignals` copied-reset event — up to three events total (list, crumb,
///    copied-reset). Facets/filters are untouched — sort never changes them (contrast with
///    `_facets`'s three-region morph).
///
/// SSE must NOT 303: same rationale as the facet and compound handlers —
/// `override_redirect` never runs here (only `datastar` is stripped from
/// `current_query`), and a redirect would break the SSE response. For the
/// same reason, `canonical_page_redirect` is never called from
/// this handler — the canonical-page 303 fires only from the two HTML page
/// handlers, never from any fragment/SSE handler, this one included.
///
/// A page whose only `ResourceList` has no `id` (no facet panel, no sort JS
/// enhancement target) reaches this route only via a stray/malformed request;
/// it 404s defensively rather than panicking — mirrors the facet handler's
/// "no FacetPanel" defensive branch.
fn make_sort_fragment_handler(
    project: Arc<cpe_web::project::Project>,
    page_idx: usize,
    origin_cfg: crate::request::OriginConfig,
    base: Base,
) -> impl Fn(Request) -> std::future::Ready<Response> + Clone + Send + 'static {
    move |req: Request| {
        let public_origin = origin_cfg.resolve(req.headers());
        let requested = req
            .extensions()
            .get::<RequestedLang>()
            .cloned()
            .unwrap_or(RequestedLang { prefs: Vec::new(), from_query: false });

        let page = project
            .config()
            .pages
            .get(page_idx)
            .expect("page index from boot-built router is valid; pages Vec is immutable after boot");

        let lang = cpe_core::lang::resolve(&requested.prefs, &project.config().site);

        // Build current_query from the request query, stripping the `datastar` key.
        // DataStar appends a `datastar=<json>` marker on @get requests; removing it
        // prevents the re-rendered list's hrefs from accumulating it (mirrors the
        // facet and compound handlers).
        let raw_query = req.uri().query();
        let filtered_query: Option<String> = strip_datastar_marker(raw_query);
        let current_query: Option<&str> = filtered_query.as_deref().filter(|s| !s.is_empty());

        // Compute current_path: strip the trailing `/_sort` suffix so the
        // rendered list's pager/sort-control hrefs remain page-relative (avoid
        // `…/_sort/_sort`).
        let full_path = base.request_path(req.uri().path());
        let current_path =
            strip_fragment_suffix(&full_path, cpe_components::component::resource_list::SORT_FRAGMENT_SEGMENT);

        // Resolve component data (static page — empty path params; a
        // parameterised list page would need its Path params threaded here,
        // mirroring `make_compound_fragment_handler_param`). `?sort=` is honoured
        // inside `Project::page_data` via `resolve_sorts`.
        let params = RequestParams(current_query);
        let route_params = HashMap::new();
        let request = cpe_core::view_data::PageRequest {
            route_params: &route_params,
            params: &params,
            scope: cpe_core::view_data::PageScope::ListFragment,
        };
        let cpe_core::view_data::PageData { components: comp_data, status, media_thumbs, .. } =
            project.page_data(page, &request, &lang);
        if status == cpe_core::view_data::PageStatus::Error {
            return std::future::ready(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }

        // Find the first named ResourceList (id: Some(..)) and its rendered data.
        let mut list_id: Option<&str> = None;
        let mut list_page: Option<&cpe_core::ir::ResourceListPage> = None;
        // Annotated-row config, captured so the swapped list keeps its
        // gloss + chips after a sort change.
        let mut list_gloss: Option<&cpe_core::ir::Gloss> = None;
        let mut list_chips: &[cpe_core::ir::ChipSpec] = &[];
        let mut list_order_name_label: Option<&str> = None;
        let mut list_order_name_desc_label: Option<&str> = None;
        let mut list_order_default_label: Option<&str> = None;
        let mut list_order_options: &[cpe_core::ir::OrderOption] = &[];
        let mut list_grid = false;
        let mut list_tile_ratio = None;
        // The list's raw class name.
        let mut list_class: &str = "";
        for (c, d) in page.body.iter().zip(comp_data.iter()) {
            if let (
                cpe_core::ir::Component::ResourceList {
                    id: Some(id),
                    class,
                    gloss,
                    chips,
                    order_name_label,
                    order_name_desc_label,
                    order_default_label,
                    order_options,
                    grid,
                    tile_ratio,
                    ..
                },
                cpe_core::view_data::ComponentData::List(lp),
            ) = (c, d)
            {
                list_id = Some(id.as_str());
                list_page = Some(lp);
                list_gloss = gloss.as_ref();
                list_chips = chips.as_slice();
                list_order_name_label = order_name_label.as_deref();
                list_order_name_desc_label = order_name_desc_label.as_deref();
                list_order_default_label = order_default_label.as_deref();
                list_order_options = order_options.as_slice();
                list_grid = *grid;
                list_tile_ratio = *tile_ratio;
                list_class = class.as_str();
                break;
            }
        }

        let (list_id, lp) = match (list_id, list_page) {
            (Some(id), Some(lp)) => (id, lp),
            _ => {
                tracing::error!(
                    page = page.route.as_str(),
                    "_sort fragment handler reached for a page with no named \
                     ResourceList — registration guard drift, or the list has no id"
                );
                return std::future::ready(StatusCode::NOT_FOUND.into_response());
            }
        };

        // Resolve the search-fragment surface — `None`
        // when this list has no search-bar at all. A sort change can happen
        // while a search is active, so the re-sorted list must keep showing
        // "N of M"/the scope note/the no-match state, exactly like the
        // initial page render. `_sort` never touches the filterbar (sort
        // never changes facets/the query) — only `ResourceListCfg.search`
        // is needed here, unlike `_facets`'/`_search`'s `active_query` too.
        let search_data =
            build_search_fragment_surface(&project, page, &comp_data, list_id, current_path, current_query, &lang);
        let search_surface = search_data.as_ref().and_then(|s| s.search_surface());
        // `_sort` never touches facets/the query, but a sort change CAN land
        // on an already-narrowed-to-zero list — the re-sorted list must keep
        // showing the narrowed-to-zero body, not degrade to the
        // genuinely-empty-class message.
        let narrowed_surface = search_data.as_ref().and_then(|s| s.narrowed_surface());

        // Resolve the chrome-id ordering-label overrides (id-only,
        // never literal text) — the same seam render_list_component uses
        // for a full-page/list render.
        let (order_name_label, order_name_desc_label, order_default_label) =
            cpe_core::labels::resolve_order_name_labels(
                &project.config().site,
                list_order_name_label,
                list_order_name_desc_label,
                list_order_default_label,
                &lang,
            );
        // The list's declared `order-by` rows, via
        // the same `pub` resolver `render_list_component` calls.
        let order_rows = cpe_core::labels::resolve_order_options(&project.config().site, list_order_options, &lang);

        // Build the ResourceListCfg via the shared factory (hides the pub(crate)
        // LinkResolver) — same approach as the facet fragment handler.
        let resolved_gloss = list_gloss.map(|g| cpe_core::labels::resolve_gloss(&project.config().site, g, &lang));
        let list_cfg = cpe_components::component::resource_list::ResourceListCfg::with_links(
            base.project(project.id()),
            project.class_routes(),
            current_path,
            current_query,
            Some(list_id),
            resolved_gloss.as_ref(),
            list_chips,
            list_class,
            &project.config().site.class_labels,
            search_surface,
            narrowed_surface,
            &media_thumbs,
            list_grid,
            list_tile_ratio,
            order_name_label,
            order_name_desc_label,
            order_default_label,
            &order_rows,
        );

        // Render the list fragment.
        let list_html =
            cpe_components::component::resource_list::render_resource_list(lp, &list_cfg, &lang).into_string();

        // Build the single PatchElements event targeting #<list-id>.
        let selector = format!("#{list_id}");
        let patch = PatchElements::new(list_html).selector(&selector);
        let list_event: Event = patch.into();

        let mut events = vec![Ok::<Event, Infallible>(list_event)];

        // Second patch target: the crumb-line
        // recomposed for the new sort state (the href's facet/query/page
        // params don't change, but the sort change is real refinement
        // state — recompose via the SAME `render_browse_breadcrumb_for_page`
        // helper `render_page`'s initial render calls).
        let crumb_req = CrumbPatchRequest {
            project: &project,
            page,
            lang: &lang,
            current_path,
            current_query,
            public_origin: &public_origin,
            base: &base,
        };
        if let Some(crumb_event) = browse_crumb_patch_event(&crumb_req, &comp_data, list_id) {
            events.push(Ok::<Event, Infallible>(crumb_event));
        }

        // Reset the page-global $copied "Copied" receipt — a refinement landing
        // mid-receipt clears it since the copied link is no longer this view.
        events.push(Ok::<Event, Infallible>(copied_signal_reset_event()));

        // Emit all three events (list, crumb, copied-reset) in one SSE stream.
        let sse_stream = stream::iter(events);
        std::future::ready(Sse::new(sse_stream).into_response())
    }
}

/// Create a handler closure for a DataStar SSE fragment that re-renders the
/// filterbar (when the page has a facet-panel) and the target `resource-list`
/// after a live search keystroke or the search row's `×` clear.
///
/// This handler is registered at `/{project}/{page}/{SEARCH_FRAGMENT_SEGMENT}`
/// for every page (static **and** parameterised) whose body contains a
/// `SearchBar` component. When the search input's debounced
/// `data-on:input`/the `×` clear button fires, this handler:
///
/// 1. Strips the `datastar` key from the query string (mirrors the facet, sort and compound
///    handlers).
/// 2. Calls `Project::page_data` with the stripped query — `?q=` is honoured there, so the returned
///    `List(..)` reflects the narrowed/unnarrowed set.
/// 3. Resolves the search-fragment surface via `build_search_fragment_surface` (the SAME
///    `SearchFragmentSurface` seam `_facets`/`_sort` share) — populates `ResourceListCfg.search`
///    only when `?q=` is active, and the filterbar's query-chip text the same way.
/// 4. Emits **three** `PatchElements` events (`#filters-<list-id>` + `#<list-id>` + the recomposed
///    crumb line `#crumb-<list-id>`) when this list also has a facet-panel resolving to at least
///    one non-empty group; **two** (`#<list-id>` + the crumb line) when it does not — either no
///    `FacetPanel` at all or one that resolves to zero groups (pinned by
///    `search_fragment_zero_groups_facet_panel_returns_one_event`). Neither layout renders a
///    filterbar element to patch. The crumb event is an `Option` (a defensive degrade when no
///    id-bearing resource-list resolves), so those are upper bounds. The search row itself
///    (`#q`/`.search-row`) is in NEITHER selector — it sits outside the swapped region by design,
///    so the caret/selection is never disturbed. Emitted in addition is ONE `PatchSignals`
///    `$copied` reset event (`copied_signal_reset_event` — a distinct SSE event type, not counted
///    towards the `datastar-patch-elements` totals above), so a search landing mid-receipt clears
///    the "Copied" state: four SSE events on a faceted page, three on a non-faceted one.
/// 5. The facet-panel itself (`#facets-<list-id>`) is never patched here — facet counts stay static
///    regardless of the active search, so only `_facets` (a real facet toggle) ever re-renders it.
///
/// SSE must NOT 303: same rationale as the other fragment handlers —
/// `override_redirect` never runs here (only `datastar` is stripped from
/// `current_query`), and a redirect would break the SSE response.
fn make_search_fragment_handler(
    project: Arc<cpe_web::project::Project>,
    page_idx: usize,
    origin_cfg: crate::request::OriginConfig,
    base: Base,
) -> impl Fn(Request) -> std::future::Ready<Response> + Clone + Send + 'static {
    move |req: Request| {
        let public_origin = origin_cfg.resolve(req.headers());
        let requested = req
            .extensions()
            .get::<RequestedLang>()
            .cloned()
            .unwrap_or(RequestedLang { prefs: Vec::new(), from_query: false });

        let page = project
            .config()
            .pages
            .get(page_idx)
            .expect("page index from boot-built router is valid; pages Vec is immutable after boot");

        let lang = cpe_core::lang::resolve(&requested.prefs, &project.config().site);

        // Build current_query from the request query, stripping the `datastar` key.
        let raw_query = req.uri().query();
        let filtered_query: Option<String> = strip_datastar_marker(raw_query);
        let current_query: Option<&str> = filtered_query.as_deref().filter(|s| !s.is_empty());

        // Compute current_path: strip the trailing `/_search` suffix so emitted
        // hrefs remain page-relative (avoid `…/_search/_search`).
        let full_path = base.request_path(req.uri().path());
        let current_path =
            strip_fragment_suffix(&full_path, cpe_components::component::search_bar::SEARCH_FRAGMENT_SEGMENT);

        // Resolve component data (static page — empty path params; a
        // parameterised search page would need its Path params threaded here,
        // mirroring `make_compound_fragment_handler_param`). `?q=` is
        // honoured inside `Project::page_data`.
        let params = RequestParams(current_query);
        let route_params = HashMap::new();
        let request = cpe_core::view_data::PageRequest {
            route_params: &route_params,
            params: &params,
            scope: cpe_core::view_data::PageScope::ListFragment,
        };
        let cpe_core::view_data::PageData { components: comp_data, status, media_thumbs, .. } =
            project.page_data(page, &request, &lang);
        if status == cpe_core::view_data::PageStatus::Error {
            return std::future::ready(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }

        // Find the first SearchBar component in page.body and locate its target.
        // Boot invariant: if a SearchBar is configured, its target is a same-page
        // ResourceList with an id (UnresolvedTarget validation hard-fails otherwise).
        // NOTE: handles the first SearchBar on the page only; a multi-search-bar
        // page would need a loop here.
        let list_id: &str = match page.body.iter().find_map(|c| {
            if let cpe_core::ir::Component::SearchBar { target, .. } = c {
                Some(target.as_str())
            } else {
                None
            }
        }) {
            Some(id) => id,
            None => {
                tracing::error!(
                    page = page.route.as_str(),
                    "_search fragment handler reached for a page with no SearchBar \
                     — registration guard drift"
                );
                return std::future::ready(StatusCode::NOT_FOUND.into_response());
            }
        };

        // Extract the target List data (+ gloss/chips) and, if this page
        // has a FacetPanel bound to the same list, its resolved Facets data —
        // `with_facets`/the 1-vs-2-event decision below both key off whether
        // `facet_groups` resolved to `Some`.
        let mut list_page: Option<&cpe_core::ir::ResourceListPage> = None;
        let mut list_gloss: Option<&cpe_core::ir::Gloss> = None;
        let mut list_chips: &[cpe_core::ir::ChipSpec] = &[];
        let mut list_order_name_label: Option<&str> = None;
        let mut list_order_name_desc_label: Option<&str> = None;
        let mut list_order_default_label: Option<&str> = None;
        let mut list_order_options: &[cpe_core::ir::OrderOption] = &[];
        let mut list_grid = false;
        let mut list_tile_ratio = None;
        let mut facet_groups: Option<&[cpe_core::ir::FacetGroup]> = None;
        // The list's raw class name.
        let mut list_class: &str = "";

        for (c, d) in page.body.iter().zip(comp_data.iter()) {
            match (c, d) {
                (
                    cpe_core::ir::Component::ResourceList {
                        id: Some(id),
                        class,
                        gloss,
                        chips,
                        order_name_label,
                        order_name_desc_label,
                        order_default_label,
                        order_options,
                        grid,
                        tile_ratio,
                        ..
                    },
                    cpe_core::view_data::ComponentData::List(lp),
                ) => {
                    if id.as_str() == list_id && list_page.is_none() {
                        list_page = Some(lp);
                        list_gloss = gloss.as_ref();
                        list_chips = chips.as_slice();
                        list_order_name_label = order_name_label.as_deref();
                        list_order_name_desc_label = order_name_desc_label.as_deref();
                        list_order_default_label = order_default_label.as_deref();
                        list_order_options = order_options.as_slice();
                        list_grid = *grid;
                        list_tile_ratio = *tile_ratio;
                        list_class = class.as_str();
                    }
                }
                (
                    cpe_core::ir::Component::FacetPanel { target, .. },
                    cpe_core::view_data::ComponentData::Facets(groups),
                ) if target == list_id => {
                    facet_groups = Some(groups.as_slice());
                }
                _ => {}
            }
        }

        let lp = match list_page {
            Some(lp) => lp,
            None => {
                tracing::error!(
                    list_id,
                    "search fragment handler: expected List slot not found in component \
                     data (boot invariant violated or DB error in Project::page_data)"
                );
                return std::future::ready(StatusCode::INTERNAL_SERVER_ERROR.into_response());
            }
        };

        // Resolve the search-fragment surface (the SAME seam `_facets`/`_sort`
        // share) — `Some` here is guaranteed (this handler only exists because
        // a SearchBar targets `list_id`), but reuse the shared helper anyway
        // so all three fragment handlers can never compute this differently.
        let search_data =
            build_search_fragment_surface(&project, page, &comp_data, list_id, current_path, current_query, &lang);
        let search_surface = search_data.as_ref().and_then(|s| s.search_surface());
        let active_query = search_data.as_ref().and_then(|s| s.active_query());
        let narrowed_surface = search_data.as_ref().and_then(|s| s.narrowed_surface());

        // Resolve the chrome-id ordering-label overrides (id-only,
        // never literal text) — the same seam render_list_component uses
        // for a full-page/list render.
        let (order_name_label, order_name_desc_label, order_default_label) =
            cpe_core::labels::resolve_order_name_labels(
                &project.config().site,
                list_order_name_label,
                list_order_name_desc_label,
                list_order_default_label,
                &lang,
            );
        // The list's declared `order-by` rows, via
        // the same `pub` resolver `render_list_component` calls.
        let order_rows = cpe_core::labels::resolve_order_options(&project.config().site, list_order_options, &lang);

        // Build the ResourceListCfg via the shared factory (hides pub(crate) LinkResolver).
        let resolved_gloss = list_gloss.map(|g| cpe_core::labels::resolve_gloss(&project.config().site, g, &lang));
        let list_cfg = cpe_components::component::resource_list::ResourceListCfg::with_links(
            base.project(project.id()),
            project.class_routes(),
            current_path,
            current_query,
            Some(list_id),
            resolved_gloss.as_ref(),
            list_chips,
            list_class,
            &project.config().site.class_labels,
            search_surface,
            narrowed_surface,
            &media_thumbs,
            list_grid,
            list_tile_ratio,
            order_name_label,
            order_name_desc_label,
            order_default_label,
            &order_rows,
        );
        let list_html =
            cpe_components::component::resource_list::render_resource_list(lp, &list_cfg, &lang).into_string();
        let list_selector = format!("#{list_id}");
        let list_event: Event = PatchElements::new(list_html).selector(&list_selector).into();

        // Faceted page: also re-render the filterbar (the query chip lives
        // there) and emit a second event. Non-faceted page: the list event
        // alone (no `.filterbar` element exists in the DOM to patch).
        //
        // Keyed on NON-EMPTY resolved groups, not mere `Some`-ness — a
        // `FacetPanel` that resolves to zero groups renders no rail AND no
        // `.filterbar` (see `render_body`'s layout switch in `cpe_web::page`),
        // so patching `#filters-{list_id}` here would target a `<div>` the
        // non-faceted layout never emits.
        let mut events: Vec<Result<Event, Infallible>> = match facet_groups.filter(|g| !g.is_empty()) {
            Some(groups) => {
                let panel_cfg =
                    cpe_components::component::facet_panel::FacetPanelCfg { list_id, current_path, current_query };
                let filterbar_html = cpe_components::component::facet_panel::render_active_filters(
                    groups,
                    &panel_cfg,
                    &lang,
                    active_query,
                )
                .into_string();
                let filterbar_selector = format!("#filters-{list_id}");
                let filterbar_event: Event = PatchElements::new(filterbar_html).selector(&filterbar_selector).into();
                vec![Ok(filterbar_event), Ok(list_event)]
            }
            None => vec![Ok(list_event)],
        };

        // One more patch target: the crumb-line recomposed for the new
        // search state. Built via the SAME
        // `render_browse_breadcrumb_for_page` helper `render_page`'s
        // initial render calls — never re-derive the crumb here.
        let crumb_req = CrumbPatchRequest {
            project: &project,
            page,
            lang: &lang,
            current_path,
            current_query,
            public_origin: &public_origin,
            base: &base,
        };
        if let Some(crumb_event) = browse_crumb_patch_event(&crumb_req, &comp_data, list_id) {
            events.push(Ok::<Event, Infallible>(crumb_event));
        }

        // Reset the page-global $copied "Copied" receipt — a refinement landing
        // mid-receipt clears it since the copied link is no longer this view.
        events.push(Ok::<Event, Infallible>(copied_signal_reset_event()));

        let sse_stream = stream::iter(events);
        std::future::ready(Sse::new(sse_stream).into_response())
    }
}

/// Serve an embedded `cpe-image-viewer` asset (the module, vendored
/// OpenSeadragon, control sprites). `path` is the wildcard-captured suffix
/// from `static_routes`' `image-viewer/*path`; `cpe_image_viewer::asset` is traversal-proof
/// (rejects empty/dot/`..` segments), so the untrusted request tail can be
/// forwarded straight in without a separate check.
async fn serve_viewer_asset(Path(path): Path<String>) -> Response {
    match cpe_image_viewer::asset(&path) {
        Some(a) => (
            [
                (header::CONTENT_TYPE, a.content_type),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            a.bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub(crate) fn content_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("js") => "text/javascript",
        Some("css") => "text/css; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("woff2") => "font/woff2",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

// Test-facing names
//
// The router's tests call these names. Production code reads `PageData`
// through `Project::page_data`, so each one only delegates to that path.

#[cfg(test)]
use cpe_core::view_data::FetchOutcome;

#[cfg(test)]
fn status_for_outcomes(outcomes: &[FetchOutcome]) -> StatusCode {
    status_code(cpe_core::view_data::PageStatus::from_outcomes(outcomes))
}

/// The page's components and status. `ListFragment` asks for the fewest side
/// lookups; neither result depends on the scope.
#[cfg(test)]
fn build_component_data(
    project: &cpe_web::project::Project,
    page: &cpe_core::ir::Page,
    route_params: &HashMap<String, String>,
    current_query: Option<&str>,
    lang: &cpe_core::lang::Lang,
) -> (Vec<cpe_core::view_data::ComponentData>, StatusCode) {
    let params = RequestParams(current_query);
    let request = cpe_core::view_data::PageRequest {
        route_params,
        params: &params,
        scope: cpe_core::view_data::PageScope::ListFragment,
    };
    let data = project.page_data(page, &request, lang);
    (data.components, status_code(data.status))
}

#[cfg(test)]
fn resolve_incoming_links(
    project: &cpe_web::project::Project,
    page: &cpe_core::ir::Page,
    components: &[cpe_core::view_data::ComponentData],
    lang: &cpe_core::lang::Lang,
) -> Option<cpe_core::ir::IncomingLinks> {
    project.page_incoming_links(page, components, lang)
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use axum::middleware::from_fn;
    use tower::ServiceExt;

    use super::*;
    use crate::discovery::discover_sql_folders;
    use crate::request::resolve_request_prefs;

    /// The folder of fixture projects (`alpha`, `gamma`, `delta` to `theta`).
    fn discovery_fixture() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery")
    }

    // Unit tests for status_for_outcomes

    #[test]
    fn status_for_outcomes_empty_slice_is_200() {
        assert_eq!(status_for_outcomes(&[]), StatusCode::OK);
    }

    #[test]
    fn status_for_outcomes_all_found_is_200() {
        let outcomes = [FetchOutcome::Found, FetchOutcome::Found];
        assert_eq!(status_for_outcomes(&outcomes), StatusCode::OK);
    }

    #[test]
    fn status_for_outcomes_any_not_found_is_404() {
        let outcomes = [FetchOutcome::Found, FetchOutcome::NotFound];
        assert_eq!(status_for_outcomes(&outcomes), StatusCode::NOT_FOUND);
    }

    #[test]
    fn status_for_outcomes_single_not_found_is_404() {
        assert_eq!(status_for_outcomes(&[FetchOutcome::NotFound]), StatusCode::NOT_FOUND);
    }

    #[test]
    fn status_for_outcomes_any_error_is_500() {
        let outcomes = [FetchOutcome::Found, FetchOutcome::Error];
        assert_eq!(status_for_outcomes(&outcomes), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn status_for_outcomes_error_takes_precedence_over_not_found() {
        // Error (500) beats NotFound (404): the presence of a DB failure
        // dominates over a missing resource.
        let outcomes = [FetchOutcome::NotFound, FetchOutcome::Error];
        assert_eq!(
            status_for_outcomes(&outcomes),
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error must take precedence over NotFound"
        );
    }

    #[test]
    fn status_for_outcomes_single_error_is_500() {
        assert_eq!(status_for_outcomes(&[FetchOutcome::Error]), StatusCode::INTERNAL_SERVER_ERROR);
    }

    fn load_fixture_projects() -> Vec<Arc<cpe_web::project::Project>> {
        discover_sql_folders(&discovery_fixture())
            .iter()
            .map(|d| {
                let (c, dt) = d.sources().expect("fixture sources are readable");
                let narrative_files = d.narrative_files();
                let asset_files = d.asset_files();
                // Discovery fixtures have no narrative/ dir, so narrative_contents
                // returns Ok([]) and we pass &[] to Project::load.
                let ctx = cpe_core::validate::context::ValidationContext {
                    project_id: &d.id,
                    reserved_ids: &["_static"],
                    narrative_files: &narrative_files,
                    narrative_files_complete: false,
                    asset_files: &asset_files,
                };
                Arc::new(cpe_web::project::Project::load(&ctx, &c, &dt, &[]).expect("fixture project loads"))
            })
            .collect()
    }

    #[test]
    fn content_type_known_and_unknown() {
        assert_eq!(content_type("foo.js"), "text/javascript");
        assert_eq!(content_type("foo.unknown"), "application/octet-stream");
    }

    #[test]
    fn content_type_webp() {
        assert_eq!(content_type("cover.webp"), "image/webp");
    }

    // The overview of the projects, at `/`

    /// The answer to `uri` with `headers`, from the router over `projects` at
    /// `base`, with the request preferences resolved as `activation` layers
    /// them.
    async fn site_index(
        projects: &[Arc<cpe_web::project::Project>],
        base: &Base,
        uri: &str,
        headers: &[(header::HeaderName, &str)],
    ) -> (StatusCode, axum::http::HeaderMap, String) {
        let router = build(
            projects,
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            base,
        )
        .layer(from_fn(resolve_request_prefs));
        let mut request = Request::builder().uri(uri);
        for (name, value) in headers {
            request = request.header(name, *value);
        }
        let response = router
            .oneshot(request.body(Body::empty()).expect("hand-crafted test request is well-formed"))
            .await
            .expect("oneshot against an in-process router never fails");
        let (parts, body) = response.into_parts();
        let body = axum::body::to_bytes(body, usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        (parts.status, parts.headers, String::from_utf8_lossy(&body).into_owned())
    }

    /// The overview's body at a root mount, in English.
    async fn site_index_body(projects: &[Arc<cpe_web::project::Project>]) -> String {
        let (status, _, body) = site_index(projects, &ROOT, "/", &[]).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }

    fn cpe_base() -> Base {
        Base::parse("/cpe").expect("/cpe is a base")
    }

    #[tokio::test]
    async fn site_index_cookie_appearance_dark_emits_data_appearance() {
        let (status, _, body) = site_index(&[], &ROOT, "/", &[(header::COOKIE, "appearance=dark")]).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("data-appearance=\"dark\""), "{body}");
        assert!(body.contains("href=\"/?appearance=light\""), "the toggle offers light: {body}");
        assert!(!site_index_body(&[]).await.contains("data-appearance"), "light is the default");
    }

    #[tokio::test]
    async fn site_index_emits_theme_and_direction_attrs() {
        let body = site_index_body(&[]).await;
        for attr in [
            "data-accent=\"ochre\"",
            "data-bg=\"tint\"",
            "data-pick=\"newsreader\"",
            "data-direction=\"edition\"",
        ] {
            assert!(body.contains(attr), "{attr}: {body}");
        }
    }

    /// The brand mark is a wordmark and no heading, so the page's one `<h1>`
    /// is the hidden one.
    #[tokio::test]
    async fn site_index_has_exactly_one_visually_hidden_h1() {
        let body = site_index_body(&load_fixture_projects()).await;
        assert_eq!(body.matches("<h1").count(), 1, "{body}");
        assert!(body.contains(r#"<h1 class="visually-hidden">"#), "{body}");
    }

    #[tokio::test]
    async fn site_index_has_skip_link_targeting_main() {
        let body = site_index_body(&[]).await;
        let after_body = body.split_once("<body>").expect("body present").1;
        assert!(
            after_body.starts_with(r##"<a class="skip-link" href="#main">"##),
            "the skip link is the first child of <body>: {body}"
        );
        assert!(body.contains(r#"<main class="shell__main" id="main" tabindex="-1">"#), "{body}");
    }

    /// The toggle's click: the cookie carries the choice, and the redirect
    /// leaves no `?appearance=` in the address.
    #[tokio::test]
    async fn site_index_query_appearance_dark_redirects_clean_and_sets_cookie() {
        for (base, location, cookie) in [
            (ROOT.clone(), "/", "appearance=dark; Path=/; SameSite=Lax"),
            (cpe_base(), "/cpe/", "appearance=dark; Path=/cpe; SameSite=Lax"),
        ] {
            let (status, headers, _) = site_index(&[], &base, "/?appearance=dark", &[]).await;
            assert_eq!(status, StatusCode::SEE_OTHER);
            assert_eq!(headers[header::LOCATION], location);
            assert_eq!(headers[header::SET_COOKIE], cookie);
        }
    }

    /// Light is the default, so choosing it clears the cookie.
    #[tokio::test]
    async fn site_index_query_appearance_light_redirects_clean_and_clears_cookie() {
        let (status, headers, _) = site_index(&[], &cpe_base(), "/?appearance=light", &[]).await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        assert_eq!(headers[header::LOCATION], "/cpe/");
        assert_eq!(headers[header::SET_COOKIE], "appearance=; Path=/cpe; Max-Age=0; SameSite=Lax");
    }

    /// Only `appearance` is removed: any other key is kept by the redirect
    /// and ignored by the page.
    #[tokio::test]
    async fn site_index_keeps_every_other_query_key() {
        let (status, headers, _) = site_index(&[], &cpe_base(), "/?lang=de&appearance=dark&accent=red", &[]).await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        assert_eq!(headers[header::LOCATION], "/cpe/?lang=de&accent=red");

        let (status, headers, body) = site_index(&[], &cpe_base(), "/?page=2&accent=red", &[]).await;
        assert_eq!(status, StatusCode::OK);
        assert!(!headers.contains_key(header::LOCATION) && !headers.contains_key(header::SET_COOKIE));
        assert!(!body.contains("page=2") && !body.contains("accent=red"), "{body}");
    }

    #[tokio::test]
    async fn site_index_renders_empty_state() {
        let body = site_index_body(&[]).await;
        assert!(body.contains("No projects are being presented yet."), "{body}");
        assert!(
            body.contains("resource-list__empty"),
            "the message has the empty row's treatment: {body}"
        );
        // The heading and the count stand above an empty list too.
        assert!(body.contains("index-list-head") && body.contains("0 projects"), "{body}");
        assert!(body.contains("class=\"shell\""), "{body}");
        assert!(!body.contains("project-list") && !body.contains("class=\"teaser\""), "{body}");
    }

    /// The degenerate crumb line: `CPE`, current and unlinked, with the share
    /// control, whose link is the origin and the base.
    #[tokio::test]
    async fn site_index_has_degenerate_breadcrumb_with_share_control() {
        let body = site_index_body(&[]).await;
        assert!(body.contains("class=\"breadcrumb\""), "{body}");
        assert_eq!(body.matches("breadcrumb__item").count(), 1, "{body}");
        assert!(body.contains("crumb-share"), "{body}");
        assert!(body.contains("href=\"http://localhost:8080/\""), "{body}");

        let (_, _, under_base) = site_index(&[], &cpe_base(), "/", &[]).await;
        assert!(under_base.contains("href=\"http://localhost:8080/cpe/\""), "{under_base}");
    }

    #[tokio::test]
    async fn site_index_lists_known_projects() {
        let projects = load_fixture_projects();
        let body = site_index_body(&projects).await;
        for (id, title) in [("alpha", "Alpha Fixture"), ("gamma", "Gamma Fixture")] {
            assert!(body.contains(&format!("href=\"/{id}/\"")), "{id}: {body}");
            assert!(body.contains(title), "{title}: {body}");
        }
        assert_eq!(body.matches("class=\"teaser\"").count(), projects.len(), "{body}");
        assert!(body.contains(&format!("{} projects", projects.len())), "{body}");
        // In the order the projects were given in, which is by id.
        let at = |id: &str| body.find(&format!("href=\"/{id}/\"")).unwrap_or(usize::MAX);
        assert!(at("alpha") < at("delta") && at("delta") < at("gamma"), "{body}");
        assert!(body.contains("tokens.css") && body.contains("shell.css"), "{body}");
    }

    /// Every URL the page writes has the base in front, once.
    #[tokio::test]
    async fn site_index_writes_every_url_under_the_base() {
        let (status, _, body) = site_index(&load_fixture_projects(), &cpe_base(), "/", &[]).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains(r#"<a class="teaser" data-accent="green" href="/cpe/alpha/">"#),
            "{body}"
        );
        assert!(body.contains(r#"<a class="brand" href="/cpe/""#), "{body}");
        assert!(body.contains(r#"href="/cpe/?appearance=dark""#), "{body}");
        assert!(body.contains(r#"href="/cpe/_static/cpe/tokens.css""#), "{body}");
        assert!(body.contains(r#"src="/cpe/_static/cpe/dasch-logo-colour.svg""#), "{body}");
        assert!(!body.contains("/cpe/cpe"), "{body}");
        for path in body.split('"').filter(|text| text.starts_with('/') && !text.starts_with("//")) {
            assert!(path.starts_with("/cpe/"), "{path} leaves the base");
        }
    }

    /// `alpha` declares `theme { accent "green" }`: its teaser carries the
    /// project's accent, and the page its own.
    #[tokio::test]
    async fn site_index_teaser_renders_project_accent_data_attr() {
        let body = site_index_body(&load_fixture_projects()).await;
        assert!(
            body.contains(r#"<a class="teaser" data-accent="green" href="/alpha/">"#),
            "{body}"
        );
        assert!(
            body.contains("<html lang=\"en\" data-direction=\"edition\" data-accent=\"ochre\""),
            "{body}"
        );
    }

    /// `alpha` declares `cover-image "assets/cover.webp"`, one of its own
    /// files.
    #[tokio::test]
    async fn site_index_teaser_with_cover_image_renders_img() {
        let body = site_index_body(&load_fixture_projects()).await;
        assert!(
            body.contains(r#"<img class="teaser__img" src="/alpha/_assets/assets/cover.webp" alt="">"#),
            "{body}"
        );
        assert!(body.contains("Alpha Credit"), "{body}");

        let (_, _, under_base) = site_index(&load_fixture_projects(), &cpe_base(), "/", &[]).await;
        assert!(
            under_base.contains(r#"src="/cpe/alpha/_assets/assets/cover.webp""#),
            "{under_base}"
        );
    }

    /// `gamma` declares no cover and no credit. The credit line is there all
    /// the same, empty, so that titles align across a row.
    #[tokio::test]
    async fn site_index_teaser_without_cover_image_renders_ghost() {
        let projects = load_fixture_projects();
        let gamma: Vec<_> = projects.into_iter().filter(|project| project.id() == "gamma").collect();
        let body = site_index_body(&gamma).await;
        assert!(
            body.contains(r#"<div class="teaser__ghost" aria-hidden="true"></div>"#),
            "{body}"
        );
        assert!(!body.contains("teaser__img"), "{body}");
        assert!(body.contains(r#"<p class="teaser__credit"></p>"#), "{body}");
    }

    #[tokio::test]
    async fn site_index_teaser_footer_shows_enter_cta_and_path() {
        let body = site_index_body(&load_fixture_projects()).await;
        let enter = cpe_components::chrome::ChromeMsg::SiteIndexEnterCta.text(&cpe_core::lang::resolve_builtin(&[]));
        assert!(
            body.contains(&format!(
                r#"<span class="teaser__enter">{enter} <span aria-hidden="true">→</span></span>"#
            )),
            "{body}"
        );
        assert!(body.contains(r#"<span class="teaser__path">/alpha/</span>"#), "{body}");

        let (_, _, under_base) = site_index(&load_fixture_projects(), &cpe_base(), "/", &[]).await;
        assert!(
            under_base.contains(r#"<span class="teaser__path">/cpe/alpha/</span>"#),
            "the path shown is the link's: {under_base}"
        );
    }

    /// `alpha` declares `eyebrow "Test Eyebrow"` and `gamma` none: its
    /// eyebrow line is there, empty.
    #[tokio::test]
    async fn site_index_teaser_eyebrow_renders_when_configured() {
        let projects = load_fixture_projects();
        let only = |id: &str| -> Vec<_> { projects.iter().filter(|project| project.id() == id).cloned().collect() };
        let alpha = site_index_body(&only("alpha")).await;
        assert!(
            alpha.contains(r#"<p class="eyebrow teaser__eyebrow">Test Eyebrow</p>"#),
            "{alpha}"
        );
        let gamma = site_index_body(&only("gamma")).await;
        assert!(gamma.contains(r#"<p class="eyebrow teaser__eyebrow"></p>"#), "{gamma}");
    }

    /// `delta` declares `image-credit "delta-credit"`, the id of a chrome
    /// string: the teaser shows its translation and never the id.
    #[tokio::test]
    async fn site_index_teaser_shows_localized_credit_via_chrome_id() {
        let projects = load_fixture_projects();
        let english = site_index_body(&projects).await;
        assert!(
            english.contains("Delta EN Credit") && !english.contains("delta-credit"),
            "{english}"
        );

        let (_, _, german) = site_index(&projects, &ROOT, "/?lang=de", &[]).await;
        assert!(
            german.contains("Delta DE Credit") && !german.contains("delta-credit"),
            "{german}"
        );
    }

    /// `delta` declares `en` and `de`, and its chrome strings have no other
    /// translation. French and Italian are built in: the page is in them and
    /// `delta`'s teaser in English.
    #[tokio::test]
    async fn site_index_teaser_never_leaks_chrome_id_for_undeclared_language() {
        let projects = load_fixture_projects();
        for lang in ["fr", "it"] {
            let (status, _, body) = site_index(&projects, &ROOT, &format!("/?lang={lang}"), &[]).await;
            assert_eq!(status, StatusCode::OK);
            assert!(body.contains(&format!("<html lang=\"{lang}\"")), "{lang}: {body}");
            assert!(!body.contains("delta-credit"), "{lang}: {body}");
            assert!(body.contains("Delta EN Credit"), "{lang}: {body}");
        }
    }

    #[tokio::test]
    async fn site_index_lang_query_de_localizes_and_sets_cookie() {
        let (status, headers, body) = site_index(&load_fixture_projects(), &cpe_base(), "/?lang=de", &[]).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[header::SET_COOKIE], "lang=de; Path=/cpe; SameSite=Lax");
        assert!(body.contains("<html lang=\"de\""), "{body}");
        for german in ["Zum Inhalt springen", "<h2>Projekte</h2>", "Öffnen "] {
            assert!(body.contains(german), "{german}: {body}");
        }
    }

    #[tokio::test]
    async fn site_index_lang_query_fr_localizes_empty_state() {
        let (status, _, body) = site_index(&[], &ROOT, "/?lang=fr", &[]).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("<html lang=\"fr\""), "{body}");
        assert!(body.contains("Aucun projet n'est encore présenté."), "{body}");
        assert!(body.contains("Aller au contenu"), "{body}");
    }

    /// The cookie holds the language the page is in, never the query's value.
    #[tokio::test]
    async fn site_index_lang_query_unknown_falls_back_to_en() {
        let (status, headers, body) = site_index(&[], &ROOT, "/?lang=zz", &[]).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("<html lang=\"en\""), "{body}");
        assert_eq!(headers[header::SET_COOKIE], "lang=en; Path=/; SameSite=Lax");
    }

    #[tokio::test]
    async fn site_index_accept_language_fr_localizes() {
        let (status, headers, body) = site_index(&[], &ROOT, "/", &[(header::ACCEPT_LANGUAGE, "fr")]).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("<html lang=\"fr\""), "{body}");
        assert!(body.contains("Aucun projet n'est encore présenté."), "{body}");
        assert!(!headers.contains_key(header::SET_COOKIE), "only `?lang=` sets the cookie");
    }

    #[tokio::test]
    async fn site_index_cookie_lang_de_localizes() {
        let (status, _, body) = site_index(&load_fixture_projects(), &ROOT, "/", &[(header::COOKIE, "lang=de")]).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("<html lang=\"de\""), "{body}");
        assert!(
            body.contains("Zum Inhalt springen") && body.contains("<h2>Projekte</h2>"),
            "{body}"
        );
    }

    #[tokio::test]
    async fn known_project_root_with_trailing_slash() {
        let projects = load_fixture_projects();
        let router = build(
            &projects,
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/alpha/")
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails");
        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        let body = String::from_utf8(body_bytes.into()).expect("test fixture bodies are valid UTF-8");
        // The fixture config has no description and no chrome/narrative,
        // so we check for the site title and the rendered page structure.
        assert!(body.contains("Alpha Fixture"), "expected site title 'Alpha Fixture' in: {body}");
        assert!(
            !body.contains("Phase 3 wires the real router"),
            "old placeholder text must not appear; got: {body}"
        );
    }

    #[tokio::test]
    async fn known_project_root_no_trailing_slash() {
        let projects = load_fixture_projects();
        let router = build(
            &projects,
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/alpha")
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails");
        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        let body = String::from_utf8(body_bytes.into()).expect("test fixture bodies are valid UTF-8");
        assert!(body.contains("Alpha Fixture"), "expected site title 'Alpha Fixture' in: {body}");
        assert!(
            !body.contains("Phase 3 wires the real router"),
            "old placeholder text must not appear; got: {body}"
        );
    }

    /// Unknown nested path under a known project → 404 via the router
    /// fallback (the project's not-found page). `?lang=en` forces English — the
    /// alpha fixture declares `languages "de" "en"` (de first, hence
    /// de-default); the middleware must be layered for the fallback to see the
    /// query-derived `RequestedLang`.
    #[tokio::test]
    async fn unknown_nested_path_returns_404() {
        let projects = load_fixture_projects();
        let router = build(
            &projects,
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        )
        .layer(from_fn(resolve_request_prefs));
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/alpha/some/sub/path?lang=en")
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails");
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "unknown nested path under a known project must 404 via the router fallback"
        );
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        let body = String::from_utf8(body_bytes.into()).expect("test fixture bodies are valid UTF-8");
        // The project's not-found page (`N1` in the messages): mono code +
        // heading, project chrome (brand shows the project title), no
        // breadcrumb, no search box.
        assert!(
            body.contains("404 — not found"),
            "N1 must render the mono nf.code mark, got: {body}"
        );
        assert!(
            body.contains("This page does not exist."),
            "N1 must render nf.path_title, got: {body}"
        );
        assert!(
            body.contains("Alpha Fixture"),
            "N1 must render the resolving project's own chrome, got: {body}"
        );
        assert!(!body.contains("class=\"breadcrumb"), "N1 must have no breadcrumb, got: {body}");
        assert!(!body.contains("search-row"), "N1 must have no search box, got: {body}");
        // No nav item may carry aria-current — the visitor is nowhere in the project.
        let nav_start = body.find("class=\"header__nav\"").expect("nav present");
        let nav_end = body[nav_start..].find("</nav>").expect("nav closing tag present");
        let nav_markup = &body[nav_start..nav_start + nav_end];
        assert!(
            !nav_markup.contains("aria-current"),
            "no nav item may be aria-current on N1, got nav: {nav_markup}"
        );
    }

    #[tokio::test]
    async fn unknown_project_returns_404() {
        let projects = load_fixture_projects();
        let router = build(
            &projects,
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/nonexistent/")
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        let body = String::from_utf8(body_bytes.into()).expect("test fixture bodies are valid UTF-8");
        // CPE's own not-found page (`N2` in the messages): mono code + heading,
        // bare chrome, single "All projects" exit to the overview at "/", index
        // footer.
        assert!(
            body.contains("404 — not found"),
            "N2 must render the mono nf.code mark, got: {body}"
        );
        assert!(
            body.contains("This address does not name a project."),
            "N2 must render nf.site_title, got: {body}"
        );
        assert!(
            body.contains("All projects"),
            "N2's one exit must be the all-projects link, got: {body}"
        );
        assert!(
            body.contains("<a href=\"/\"><span>All projects</span>"),
            "N2's all-projects link must point at the site index, got: {body}"
        );
        assert!(
            body.contains("footer-index__rights"),
            "N2 must keep the index footer, got: {body}"
        );
        assert!(
            !body.contains("Or browse"),
            "N2 must have no 'or browse' line (no project pages to offer), got: {body}"
        );
    }

    /// `/nope` (no known project prefix) renders bare chrome (CPE's own not-found page).
    #[tokio::test]
    async fn fallback_unknown_top_level_path_uses_site_chrome() {
        let delta = load_delta_project();
        let router = build(
            &[delta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/nope")
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        let body = String::from_utf8(body_bytes.into()).expect("test fixture bodies are valid UTF-8");
        assert!(
            body.contains("This address does not name a project."),
            "N2 must render, got: {body}"
        );
        assert!(
            !body.contains("Delta Fixture"),
            "N2 must not carry the project's own chrome, got: {body}"
        );
    }

    /// A POST to an unmatched path also renders the 404 body — the fallback
    /// answers every HTTP method (matched routes keep their own 405 behaviour).
    #[tokio::test]
    async fn fallback_answers_non_get_methods() {
        let projects = load_fixture_projects();
        let router = build(
            &projects,
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/nope")
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// Security invariant: the fallback must never redirect. A path shaped
    /// like `//evil.example/x` combined with `?appearance=dark` (which on
    /// every OTHER route triggers `override_redirect`'s 303-with-`Location`
    /// dance) must still render the 404 body directly, with no `Location`
    /// header — a `Location` here, under a catch-all fallback, would be an
    /// open redirect to `evil.example`.
    #[tokio::test]
    async fn fallback_never_redirects_on_scheme_relative_looking_path() {
        let projects = load_fixture_projects();
        // Layer `resolve_request_prefs` so `?appearance=dark` actually
        // populates `RequestedAppearance` in extensions — without this layer
        // the fallback's defensive `.unwrap_or(...)` default would make the
        // no-redirect assertion pass trivially even if `override_redirect`
        // were wrongly wired in.
        let router = build(
            &projects,
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        )
        .layer(from_fn(resolve_request_prefs));
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("//evil.example/x?appearance=dark&accent=red")
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(
            response.headers().get(header::LOCATION).is_none(),
            "the fallback must never set a Location header (open-redirect guard)"
        );
        assert!(
            response.headers().get(header::SET_COOKIE).is_none(),
            "the fallback must never set a cookie"
        );
    }

    /// Project-id matching is raw string equality — case is NOT folded.
    /// `DELTA` (uppercased) must not match the loaded `delta` project id, so
    /// this renders CPE's own not-found page (bare chrome), not the project's.
    #[tokio::test]
    async fn fallback_project_id_match_does_not_fold_case() {
        let delta = load_delta_project();
        let router = build(
            &[delta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/DELTA/nope")
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        let body = String::from_utf8(body_bytes.into()).expect("test fixture bodies are valid UTF-8");
        // CPE's not-found marker (see `unknown_project_returns_404`).
        assert!(
            body.contains("This address does not name a project."),
            "case-mismatched segment must render N2 (site chrome), got: {body}"
        );
    }

    /// Project-id matching is raw, undecoded byte equality — percent-encoded
    /// segments are NOT decoded before the comparison. `%64elta`
    /// must not match `delta`, so this renders CPE's own not-found page.
    #[tokio::test]
    async fn fallback_project_id_match_does_not_percent_decode() {
        let delta = load_delta_project();
        let router = build(
            &[delta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/%64elta/nope")
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        let body = String::from_utf8(body_bytes.into()).expect("test fixture bodies are valid UTF-8");
        // CPE's not-found marker (see `unknown_project_returns_404`).
        assert!(
            body.contains("This address does not name a project."),
            "percent-encoded segment must render N2 (site chrome), got: {body}"
        );
    }

    /// A matched route keeps its own method behaviour — the fallback only
    /// answers UNMATCHED paths (`fallback_answers_non_get_methods` above).
    /// POSTing to the (GET-only) `/delta` list route
    /// must 405, not fall through to the 404 fallback.
    #[tokio::test]
    async fn matched_route_keeps_method_behaviour_on_post() {
        let delta = load_delta_project();
        let router = build(
            &[delta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/delta")
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails");
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    /// No-reflection invariant: a marker segment in the failed request path
    /// must appear nowhere in the response — not the body, not any header
    /// (in particular not in a lang-switcher/appearance-toggle href, which
    /// would happen if `current_path` were NOT pinned to the project
    /// home, or to `/` outside a project).
    #[tokio::test]
    async fn fallback_never_reflects_the_failed_path() {
        let delta = load_delta_project();
        let router = build(
            &[delta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/delta/MARKER_SEGMENT_XYZ")
                    .body(Body::empty())
                    .expect("hand-crafted test request is well-formed"),
            )
            .await
            .expect("oneshot against an in-process router never fails");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(
            !response
                .headers()
                .values()
                .any(|v| v.to_str().unwrap_or("").contains("MARKER_SEGMENT_XYZ")),
            "the failed path must not appear in any response header"
        );
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        let body = String::from_utf8(body_bytes.into()).expect("test fixture bodies are valid UTF-8");
        assert!(
            !body.contains("MARKER_SEGMENT_XYZ"),
            "the failed path must not appear in the body, got: {body}"
        );
    }

    // Parameterised route tests

    /// Load the delta fixture project, which has a resource-detail page at
    /// `/characters/:id` with two known characters (alice, white-rabbit).
    fn load_delta_project() -> Arc<cpe_web::project::Project> {
        let discovered = discover_sql_folders(&discovery_fixture());
        let delta = discovered
            .iter()
            .find(|d| d.id == "delta")
            .expect("delta fixture must be present in discovery fixtures");
        let (config_src, data_sql) = delta.sources().expect("delta sources readable");
        let narrative_files = delta.narrative_files();
        let asset_files = delta.asset_files();
        let ctx = cpe_core::validate::context::ValidationContext {
            project_id: &delta.id,
            reserved_ids: &["_static"],
            narrative_files: &narrative_files,
            narrative_files_complete: false,
            asset_files: &asset_files,
        };
        Arc::new(cpe_web::project::Project::load(&ctx, &config_src, &data_sql, &[]).expect("delta project loads"))
    }

    /// GET `/delta/characters/alice` → 200 + rendered resource (known property).
    #[tokio::test]
    async fn parameterized_known_id_returns_200_and_resource() {
        let delta = load_delta_project();
        let router = build(
            &[delta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/delta/characters/alice")
                    .body(Body::empty())
                    .expect("well-formed request"),
            )
            .await
            .expect("oneshot succeeds");
        assert_eq!(response.status(), StatusCode::OK, "known resource id must return 200");
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");
        assert!(
            body.contains("Curiouser and curiouser!"),
            "response must contain Alice's known English quote, got: (truncated) {}",
            &body[..body.len().min(600)]
        );
        assert!(
            !body.contains("(component not yet implemented)"),
            "resource-detail must not show placeholder text; got: (truncated) {}",
            &body[..body.len().min(600)]
        );
    }

    /// GET `/delta/characters/alice?lang=de` → 200 + German property values.
    #[tokio::test]
    async fn parameterized_lang_de_returns_german_values() {
        let delta = load_delta_project();
        let router = build(
            &[delta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        )
        .layer(from_fn(resolve_request_prefs));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/delta/characters/alice?lang=de")
                    .body(Body::empty())
                    .expect("well-formed request"),
            )
            .await
            .expect("oneshot succeeds");
        assert_eq!(response.status(), StatusCode::OK, "?lang=de must return 200");
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");
        assert!(
            body.contains("Immer merkwürdiger!"),
            "response must contain Alice's German quote, got: (truncated) {}",
            &body[..body.len().min(600)]
        );
        assert!(
            body.contains("<html lang=\"de\""),
            "response must have <html lang=\"de\">, got: (truncated) {}",
            &body[..body.len().min(400)]
        );
    }

    // Static-route literal-id resource-detail tests

    /// Load the epsilon fixture project, which has a static `/featured` page with
    /// a literal-id `resource-detail id="res-01"`.
    fn load_epsilon_project() -> Arc<cpe_web::project::Project> {
        let discovered = discover_sql_folders(&discovery_fixture());
        let epsilon = discovered
            .iter()
            .find(|d| d.id == "epsilon")
            .expect("epsilon fixture must be present in discovery fixtures");
        let (config_src, data_sql) = epsilon.sources().expect("epsilon sources readable");
        let narrative_files = epsilon.narrative_files();
        let asset_files = epsilon.asset_files();
        let ctx = cpe_core::validate::context::ValidationContext {
            project_id: &epsilon.id,
            reserved_ids: &["_static"],
            narrative_files: &narrative_files,
            narrative_files_complete: false,
            asset_files: &asset_files,
        };
        Arc::new(cpe_web::project::Project::load(&ctx, &config_src, &data_sql, &[]).expect("epsilon project loads"))
    }

    /// Load the zeta fixture project: a single search-only (non-faceted)
    /// `resource-list id="things" class="Thing"` with a `search-bar
    /// target="things" searches="name"` on `/things`. Used by the
    /// search-query-injection tests.
    fn load_zeta_project() -> Arc<cpe_web::project::Project> {
        let discovered = discover_sql_folders(&discovery_fixture());
        let zeta = discovered
            .iter()
            .find(|d| d.id == "zeta")
            .expect("zeta fixture must be present in discovery fixtures");
        let (config_src, data_sql) = zeta.sources().expect("zeta sources readable");
        let narrative_files = zeta.narrative_files();
        let asset_files = zeta.asset_files();
        let ctx = cpe_core::validate::context::ValidationContext {
            project_id: &zeta.id,
            reserved_ids: &["_static"],
            narrative_files: &narrative_files,
            narrative_files_complete: false,
            asset_files: &asset_files,
        };
        Arc::new(cpe_web::project::Project::load(&ctx, &config_src, &data_sql, &[]).expect("zeta project loads"))
    }

    /// Load the eta fixture project: `resource-list id="widgets" class="Widget"`
    /// with BOTH a `facet-panel` (kind="list" on "category") AND a `search-bar
    /// target="widgets" searches="name"` on `/widgets` — the faceted
    /// counterpart to `zeta` (search-only, no facet-panel), used by the
    /// `_search` fragment-handler router tests.
    fn load_eta_project() -> Arc<cpe_web::project::Project> {
        let discovered = discover_sql_folders(&discovery_fixture());
        let eta = discovered
            .iter()
            .find(|d| d.id == "eta")
            .expect("eta fixture must be present in discovery fixtures");
        let (config_src, data_sql) = eta.sources().expect("eta sources readable");
        let narrative_files = eta.narrative_files();
        let asset_files = eta.asset_files();
        let ctx = cpe_core::validate::context::ValidationContext {
            project_id: &eta.id,
            reserved_ids: &["_static"],
            narrative_files: &narrative_files,
            narrative_files_complete: false,
            asset_files: &asset_files,
        };
        Arc::new(cpe_web::project::Project::load(&ctx, &config_src, &data_sql, &[]).expect("eta project loads"))
    }

    /// Load the theta fixture project: `resource-list id="widgets" class="Widget"`
    /// with BOTH a `facet-panel target="widgets"` declaring ZERO `facet`
    /// children (so its resolved groups are always `Some([])`) AND a
    /// `search-bar target="widgets" searches="name"` — the zero-groups
    /// counterpart to `eta` (real, non-empty facet group), used by the
    /// `_search` fragment-handler zero-groups test.
    fn load_theta_project() -> Arc<cpe_web::project::Project> {
        let discovered = discover_sql_folders(&discovery_fixture());
        let theta = discovered
            .iter()
            .find(|d| d.id == "theta")
            .expect("theta fixture must be present in discovery fixtures");
        let (config_src, data_sql) = theta.sources().expect("theta sources readable");
        let narrative_files = theta.narrative_files();
        let asset_files = theta.asset_files();
        let ctx = cpe_core::validate::context::ValidationContext {
            project_id: &theta.id,
            reserved_ids: &["_static"],
            narrative_files: &narrative_files,
            narrative_files_complete: false,
            asset_files: &asset_files,
        };
        Arc::new(cpe_web::project::Project::load(&ctx, &config_src, &data_sql, &[]).expect("theta project loads"))
    }

    /// GET `/epsilon/featured` → 200 + literal-id resource data rendered.
    ///
    /// A static route with a literal-id `resource-detail` must resolve and
    /// render the resource, not the not-found body.
    #[tokio::test]
    async fn static_literal_id_resource_detail_renders_resource() {
        let epsilon = load_epsilon_project();
        let router = build(
            &[epsilon],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/epsilon/featured")
                    .body(Body::empty())
                    .expect("well-formed request"),
            )
            .await
            .expect("oneshot succeeds");
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "static literal-id resource-detail page must return 200"
        );
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");
        assert!(
            body.contains("The Featured Resource"),
            "static literal-id resource-detail must render the resource name, got: (truncated) {}",
            &body[..body.len().min(600)]
        );
        assert!(
            body.contains("A static literal-id quote"),
            "static literal-id resource-detail must render the resource quote, got: (truncated) {}",
            &body[..body.len().min(600)]
        );
        assert!(
            !body.contains("could not be found"),
            "static literal-id resource-detail must NOT render the not-found body, got: (truncated) {}",
            &body[..body.len().min(600)]
        );
    }

    /// GET `/delta/characters/unknown-id` → 404 + not-found body.
    #[tokio::test]
    async fn parameterized_unknown_id_returns_404_and_not_found_body() {
        let delta = load_delta_project();
        let router = build(
            &[delta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/delta/characters/unknown-id")
                    .body(Body::empty())
                    .expect("well-formed request"),
            )
            .await
            .expect("oneshot succeeds");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "unknown resource id must return 404");
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");
        // The not-found body must be rendered inside chrome (not a bare 404).
        // The delta fixture's `resource-detail` declares no `class`, so the
        // class-less fallback copy applies (`nf.id_title_fallback` — no id_msg
        // paragraph, no browse-all exit; see `render_not_found`'s doc comment) —
        // the not-found breadcrumb's current-item text ("Not found") is the
        // most reliable marker regardless of class presence.
        assert!(
            body.contains("class=\"notfound")
                && (body.contains("There is nothing at this address.")
                    || body.contains("Unter dieser Adresse gibt es nichts.")),
            "404 response must contain the N3 not-found anatomy, got: (truncated) {}",
            &body[..body.len().min(600)]
        );
        assert!(
            body.contains("Not found") || body.contains("Nicht gefunden"),
            "404 response must contain the not-found breadcrumb current item, got: \
             (truncated) {}",
            &body[..body.len().min(600)]
        );
        // Chrome must still be present (page rendered even on 404).
        assert!(
            body.contains("Delta Fixture"),
            "404 response must still include site chrome (title), got: (truncated) {}",
            &body[..body.len().min(400)]
        );
    }

    // override_redirect: appearance + the retired knob keys

    fn no_appearance() -> RequestedAppearance {
        RequestedAppearance { value: None, from_query: false }
    }

    fn set_cookies(resp: &Response) -> Vec<&str> {
        resp.headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .collect()
    }

    fn location(resp: &Response) -> &str {
        resp.headers()
            .get(header::LOCATION)
            .expect("303 has Location")
            .to_str()
            .expect("ascii")
    }

    /// Each retired key, with a value or bare, is stripped with no cookie;
    /// unrelated params (`lang`, `view=grid`) and a near-miss key survive.
    #[test]
    fn override_redirect_strips_each_retired_key_without_a_cookie() {
        for retired in [
            "accent=green",
            "background=neutral",
            "typeface=figtree",
            "reset",
            "accent",
        ] {
            let raw = format!("/alice-in-daschland/characters?lang=de&{retired}&accents=1&view=grid");
            let uri: Uri = raw.parse().expect("test uri parses");
            let resp = override_redirect(&uri, &no_appearance(), &cpe_core::url::ROOT)
                .unwrap_or_else(|| panic!("{retired} must produce a redirect"));
            assert_eq!(resp.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                location(&resp),
                "/alice-in-daschland/characters?lang=de&accents=1&view=grid",
                "{retired}"
            );
            assert!(set_cookies(&resp).is_empty(), "{retired} sets no cookie");
        }
    }

    /// `?appearance=dark` alongside a retired key: both are stripped, and only
    /// the appearance cookie is set.
    #[test]
    fn override_redirect_with_appearance_sets_only_the_appearance_cookie() {
        let uri: Uri = "/alice-in-daschland/?appearance=dark&accent=red&lang=de"
            .parse()
            .expect("test uri parses");
        let appearance = RequestedAppearance { value: Some(Appearance::Dark), from_query: true };
        let resp = override_redirect(&uri, &appearance, &cpe_core::url::ROOT).expect("redirect");
        assert_eq!(location(&resp), "/alice-in-daschland/?lang=de");
        assert_eq!(set_cookies(&resp), vec!["appearance=dark; Path=/; SameSite=Lax"]);
    }

    #[test]
    fn override_redirect_none_without_override_keys() {
        let uri: Uri = "/alice-in-daschland/?lang=de".parse().expect("test uri parses");
        assert!(
            override_redirect(&uri, &no_appearance(), &cpe_core::url::ROOT).is_none(),
            "no stripped keys → no redirect"
        );
    }

    // axum_route: CPE config syntax → axum 0.8 matchit syntax

    #[test]
    fn axum_route_translates_named_param() {
        assert_eq!(axum_route("/characters/:id"), "/characters/{id}");
    }

    #[test]
    fn axum_route_translates_wildcard_suffix() {
        assert_eq!(axum_route("/_static/*path"), "/_static/{*path}");
    }

    #[test]
    fn axum_route_no_params_unchanged() {
        assert_eq!(axum_route("/alice-in-daschland/characters"), "/alice-in-daschland/characters");
    }

    #[test]
    fn axum_route_root_unchanged() {
        assert_eq!(axum_route("/"), "/");
    }

    #[test]
    fn axum_route_multi_segment_no_params_unchanged() {
        assert_eq!(axum_route("/alice-in-daschland/"), "/alice-in-daschland/");
    }

    #[test]
    fn axum_route_param_in_middle() {
        // A route with multiple named segments (hypothetical but validates correctness).
        assert_eq!(axum_route("/projects/:proj/pages/:page"), "/projects/{proj}/pages/{page}");
    }

    // Unit tests for parse_page_param

    #[test]
    fn parse_page_param_absent_query_returns_1() {
        assert_eq!(parse_page_param(None), 1);
    }

    #[test]
    fn parse_page_param_no_page_key_returns_1() {
        assert_eq!(parse_page_param(Some("lang=de")), 1);
        assert_eq!(parse_page_param(Some("img=2&lang=en")), 1);
    }

    #[test]
    fn parse_page_param_empty_value_returns_1() {
        assert_eq!(parse_page_param(Some("page=")), 1);
    }

    #[test]
    fn parse_page_param_non_numeric_returns_1() {
        assert_eq!(parse_page_param(Some("page=bad")), 1);
        assert_eq!(parse_page_param(Some("page=abc")), 1);
    }

    #[test]
    fn parse_page_param_zero_returns_1() {
        // Zero is not a valid 1-based page number.
        assert_eq!(parse_page_param(Some("page=0")), 1);
    }

    #[test]
    fn parse_page_param_negative_returns_1() {
        // u32 parse fails for negative strings.
        assert_eq!(parse_page_param(Some("page=-1")), 1);
    }

    #[test]
    fn parse_page_param_overflow_saturates_to_u32_max() {
        // Value exceeds u32::MAX — saturates rather than falling back to 1:
        // pagination clamps to the last page before multiplying, so an
        // absurd page still lands on the last page.
        assert_eq!(parse_page_param(Some("page=99999999999")), u32::MAX);
        // A 40-digit numeric string lands the same way.
        assert_eq!(
            parse_page_param(Some("page=9999999999999999999999999999999999999999")),
            u32::MAX
        );
    }

    #[test]
    fn parse_page_param_valid_returns_value() {
        assert_eq!(parse_page_param(Some("page=1")), 1);
        assert_eq!(parse_page_param(Some("page=2")), 2);
        assert_eq!(parse_page_param(Some("page=42")), 42);
    }

    #[test]
    fn parse_page_param_first_match_wins() {
        // In a well-formed query, page appears once; if somehow repeated we
        // pick the first occurrence (consistent with query_img / query_lang).
        assert_eq!(parse_page_param(Some("lang=de&page=3&img=0")), 3);
    }

    // Unit tests for raw_page_param

    #[test]
    fn raw_page_param_absent_query_returns_none() {
        assert_eq!(raw_page_param(None), None);
    }

    #[test]
    fn raw_page_param_no_page_key_returns_none() {
        assert_eq!(raw_page_param(Some("lang=de")), None);
    }

    #[test]
    fn raw_page_param_bare_key_returns_some_empty() {
        assert_eq!(raw_page_param(Some("page")), Some(""));
    }

    #[test]
    fn raw_page_param_empty_value_returns_some_empty() {
        assert_eq!(raw_page_param(Some("page=")), Some(""));
    }

    #[test]
    fn raw_page_param_first_occurrence_wins_even_when_empty() {
        // Unlike query_param_one, an empty first occurrence is NOT skipped —
        // this is the property canonical_page_query's idempotence relies on.
        assert_eq!(raw_page_param(Some("page=&page=2")), Some(""));
    }

    // Unit tests for canonical_page_query

    #[test]
    fn canonical_page_query_none_query() {
        assert_eq!(canonical_page_query(None, 2), "page=2");
    }

    #[test]
    fn canonical_page_query_drops_all_page_pairs_appends_one() {
        assert_eq!(canonical_page_query(Some("page=&page=2"), 1), "page=1");
        assert_eq!(canonical_page_query(Some("page=999&page=1"), 1), "page=1");
        assert_eq!(canonical_page_query(Some("page&page=2"), 3), "page=3");
    }

    #[test]
    fn canonical_page_query_preserves_other_pairs_in_order() {
        assert_eq!(canonical_page_query(Some("role=R_01&page=999"), 3), "role=R_01&page=3");
    }

    #[test]
    fn canonical_page_query_normalises_bare_key() {
        assert_eq!(canonical_page_query(Some("flag&page=999"), 3), "flag=&page=3");
    }

    #[test]
    fn canonical_page_query_skips_empty_key_pairs() {
        assert_eq!(canonical_page_query(Some("&page=2"), 1), "page=1");
    }

    // Unit tests for canonical_page_redirect

    /// Build a one-element `ComponentData` slice carrying a `List` with
    /// `Pagination::PerPage(_)` at the given effective `page`.
    fn per_page_components(page: u32) -> Vec<cpe_core::view_data::ComponentData> {
        vec![cpe_core::view_data::ComponentData::List(
            cpe_core::ir::ResourceListPage {
                items: Vec::new(),
                total: 0,
                page,
                pagination: cpe_core::ir::Pagination::PerPage(20),
                unfiltered_total: None,
            },
        )]
    }

    /// Same, but `Pagination::Limit` — never carries a pager, so the guard
    /// must never redirect regardless of the raw page param.
    fn limit_components() -> Vec<cpe_core::view_data::ComponentData> {
        vec![cpe_core::view_data::ComponentData::List(
            cpe_core::ir::ResourceListPage {
                items: Vec::new(),
                total: 0,
                page: 1,
                pagination: cpe_core::ir::Pagination::Limit(20),
                unfiltered_total: None,
            },
        )]
    }

    #[test]
    fn canonical_page_redirect_absent_page_param_returns_none() {
        let components = per_page_components(3);
        assert!(canonical_page_redirect("/proj/characters", Some("role=R_01"), StatusCode::OK, &components).is_none());
    }

    #[test]
    fn canonical_page_redirect_raw_matches_effective_returns_none() {
        let components = per_page_components(3);
        assert!(canonical_page_redirect("/proj/characters", Some("page=3"), StatusCode::OK, &components).is_none());
    }

    fn redirect_location(resp: Response) -> String {
        resp.headers()
            .get(header::LOCATION)
            .expect("redirect must carry a Location header")
            .to_str()
            .expect("Location header must be valid UTF-8")
            .to_owned()
    }

    #[test]
    fn canonical_page_redirect_out_of_range_303s_to_effective_page() {
        let components = per_page_components(3);
        let resp = canonical_page_redirect("/proj/characters", Some("role=R_01&page=999"), StatusCode::OK, &components)
            .expect("out-of-range page must redirect");
        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        let location = redirect_location(resp);
        assert_eq!(location, "/proj/characters?role=R_01&page=3");
        assert!(location.starts_with('/'));
        assert!(!location.starts_with("//"));
    }

    #[test]
    fn canonical_page_redirect_zero_abc_empty_bare_all_redirect_to_page_1() {
        let components = per_page_components(1);
        for query in ["page=0", "page=abc", "page=", "page"] {
            let resp = canonical_page_redirect("/proj/characters", Some(query), StatusCode::OK, &components)
                .unwrap_or_else(|| panic!("query {query:?} must redirect"));
            let location = redirect_location(resp);
            assert_eq!(
                location, "/proj/characters?page=1",
                "query {query:?} produced unexpected Location"
            );
        }
    }

    #[test]
    fn canonical_page_redirect_repeated_page_key_yields_exactly_one_page_pair() {
        let components = per_page_components(1);
        for query in ["page=&page=2", "page=999&page=1", "page&page=2"] {
            let resp = canonical_page_redirect("/proj/characters", Some(query), StatusCode::OK, &components)
                .unwrap_or_else(|| panic!("query {query:?} must redirect"));
            let location = redirect_location(resp);
            let page_count = location.matches("page=").count();
            assert_eq!(
                page_count, 1,
                "query {query:?} produced Location with {page_count} page= occurrences: {location}"
            );
        }
    }

    #[test]
    fn canonical_page_redirect_bare_key_normalised_and_idempotent() {
        let components = per_page_components(3);
        let resp = canonical_page_redirect("/proj/characters", Some("flag&page=999"), StatusCode::OK, &components)
            .expect("out-of-range page must redirect");
        let location = redirect_location(resp);
        assert_eq!(location, "/proj/characters?flag=&page=3");

        // Idempotence: feeding the produced query back in must yield no redirect.
        let produced_query = location.split('?').nth(1).expect("query present");
        assert!(
            canonical_page_redirect("/proj/characters", Some(produced_query), StatusCode::OK, &components).is_none()
        );
    }

    #[test]
    fn canonical_page_redirect_general_idempotence() {
        let components = per_page_components(3);
        let resp = canonical_page_redirect("/proj/characters", Some("role=R_01&page=999"), StatusCode::OK, &components)
            .expect("out-of-range page must redirect");
        let location = redirect_location(resp);
        let produced_query = location.split('?').nth(1).expect("query present");
        assert!(
            canonical_page_redirect("/proj/characters", Some(produced_query), StatusCode::OK, &components).is_none()
        );
    }

    #[test]
    fn canonical_page_redirect_limit_pagination_returns_none() {
        let components = limit_components();
        assert!(canonical_page_redirect("/proj/characters", Some("page=999"), StatusCode::OK, &components).is_none());
    }

    #[test]
    fn canonical_page_redirect_non_200_status_returns_none() {
        let components = per_page_components(3);
        assert!(
            canonical_page_redirect("/proj/characters", Some("page=999"), StatusCode::NOT_FOUND, &components).is_none()
        );
    }

    // Unit tests for query_param_all

    #[test]
    fn query_param_all_absent_query_returns_empty() {
        assert_eq!(query_param_all(None, "role"), Vec::<String>::new());
    }

    #[test]
    fn query_param_all_key_absent_returns_empty() {
        assert_eq!(query_param_all(Some("page=1&lang=en"), "role"), Vec::<String>::new());
    }

    #[test]
    fn query_param_all_single_value() {
        assert_eq!(query_param_all(Some("role=R_01"), "role"), vec!["R_01".to_owned()]);
    }

    #[test]
    fn query_param_all_repeated_key_returns_all_values() {
        assert_eq!(
            query_param_all(Some("role=R_01&role=R_02"), "role"),
            vec!["R_01".to_owned(), "R_02".to_owned()]
        );
    }

    #[test]
    fn query_param_all_preserves_order() {
        // Values must come out in query-string order
        assert_eq!(
            query_param_all(Some("role=R_02&role=R_01&role=R_03"), "role"),
            vec!["R_02".to_owned(), "R_01".to_owned(), "R_03".to_owned()]
        );
    }

    #[test]
    fn query_param_all_ignores_other_keys() {
        assert_eq!(
            query_param_all(Some("keyword=K_01&role=R_01&keyword=K_02"), "role"),
            vec!["R_01".to_owned()]
        );
    }

    #[test]
    fn query_param_all_empty_value_skipped() {
        // A `key=` with no value should not add an empty string
        assert_eq!(query_param_all(Some("role=&role=R_01"), "role"), vec!["R_01".to_owned()]);
    }

    #[test]
    fn query_param_all_mixed_with_other_params() {
        assert_eq!(
            query_param_all(Some("page=1&role=R_01&lang=de&role=R_02"), "role"),
            vec!["R_01".to_owned(), "R_02".to_owned()]
        );
    }

    // Unit tests for facet_param_values

    #[test]
    fn facet_param_values_dedupes_preserving_first_occurrence_order() {
        assert_eq!(
            facet_param_values(Some("role=R_02&role=R_01&role=R_02&role=R_03"), "role"),
            vec!["R_02".to_owned(), "R_01".to_owned(), "R_03".to_owned()]
        );
    }

    #[test]
    fn facet_param_values_caps_at_max() {
        let query: String = (0..300).map(|i| format!("role=V_{i:03}")).collect::<Vec<_>>().join("&");
        let result = facet_param_values(Some(&query), "role");
        assert_eq!(result.len(), MAX_FACET_VALUES);
        let expected: Vec<String> = (0..MAX_FACET_VALUES).map(|i| format!("V_{i:03}")).collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn facet_param_values_duplicates_do_not_consume_cap() {
        // 256 distinct values, each repeated twice, interleaved so a naive
        // "cap before dedupe" implementation would starve on duplicates.
        let mut pairs: Vec<String> = Vec::with_capacity(MAX_FACET_VALUES * 2);
        for i in 0..MAX_FACET_VALUES {
            pairs.push(format!("role=V_{i:03}"));
            pairs.push(format!("role=V_{i:03}"));
        }
        let query = pairs.join("&");
        let result = facet_param_values(Some(&query), "role");
        let expected: Vec<String> = (0..MAX_FACET_VALUES).map(|i| format!("V_{i:03}")).collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn facet_param_values_ignores_empty_and_other_keys() {
        assert_eq!(facet_param_values(Some("role=&keyword=K&role=R"), "role"), vec!["R".to_owned()]);
    }

    // Unit tests for query_param_one

    #[test]
    fn query_param_one_absent_query_returns_none() {
        assert_eq!(query_param_one(None, "sort"), None);
    }

    #[test]
    fn query_param_one_key_absent_returns_none() {
        assert_eq!(query_param_one(Some("page=1&lang=en"), "sort"), None);
    }

    #[test]
    fn query_param_one_empty_value_returns_none() {
        // A `key=` with no value is treated as absent.
        assert_eq!(query_param_one(Some("sort="), "sort"), None);
    }

    #[test]
    fn query_param_one_returns_value() {
        assert_eq!(query_param_one(Some("sort=id"), "sort"), Some("id"));
    }

    #[test]
    fn query_param_one_first_match_wins_on_repeated_key() {
        assert_eq!(query_param_one(Some("sort=id&sort=class"), "sort"), Some("id"));
    }

    #[test]
    fn query_param_one_mixed_with_other_params() {
        assert_eq!(
            query_param_one(Some("page=2&sort=name-desc&lang=de"), "sort"),
            Some("name-desc")
        );
    }

    // Unit tests for resolve_sorts (?sort= override + config fallback)

    fn name_asc_config_sorts() -> Vec<cpe_core::ir::Sort> {
        vec![cpe_core::ir::Sort {
            key: cpe_core::ir::SortKey::Property("name".to_owned()),
            direction: cpe_core::ir::SortDir::Asc,
        }]
    }

    #[test]
    fn resolve_sorts_absent_falls_back_to_config() {
        let config = name_asc_config_sorts();
        assert_eq!(resolve_sorts(None, &config, &[], None), config);
    }

    #[test]
    fn resolve_sorts_literal_default_falls_back_to_config() {
        let config = name_asc_config_sorts();
        assert_eq!(resolve_sorts(Some("sort=default"), &config, &[], None), config);
    }

    #[test]
    fn resolve_sorts_bogus_falls_back_to_config() {
        let config = name_asc_config_sorts();
        assert_eq!(resolve_sorts(Some("sort=bogus"), &config, &[], None), config);
    }

    #[test]
    fn resolve_sorts_id_overrides_config() {
        let config = name_asc_config_sorts();
        assert_eq!(
            resolve_sorts(Some("sort=id"), &config, &[], None),
            vec![cpe_core::ir::Sort {
                key: cpe_core::ir::SortKey::Id,
                direction: cpe_core::ir::SortDir::Asc,
            }]
        );
    }

    #[test]
    fn resolve_sorts_name_desc_overrides_config() {
        let config = name_asc_config_sorts();
        assert_eq!(
            resolve_sorts(Some("sort=name-desc"), &config, &[], None),
            vec![cpe_core::ir::Sort {
                key: cpe_core::ir::SortKey::Property("name".to_owned()),
                direction: cpe_core::ir::SortDir::Desc,
            }]
        );
    }

    /// `order-name-property` swaps the property the built-in name rows order
    /// by, in both directions, without touching the wire token.
    #[test]
    fn resolve_sorts_name_rows_use_the_name_property() {
        let config = name_asc_config_sorts();
        let by = |direction| {
            vec![cpe_core::ir::Sort {
                key: cpe_core::ir::SortKey::Property("sort-key".to_owned()),
                direction,
            }]
        };
        assert_eq!(
            resolve_sorts(Some("sort=name"), &config, &[], Some("sort-key")),
            by(cpe_core::ir::SortDir::Asc)
        );
        assert_eq!(
            resolve_sorts(Some("sort=name-desc"), &config, &[], Some("sort-key")),
            by(cpe_core::ir::SortDir::Desc)
        );
    }

    /// The name property reaches only the name rows: `id`, a declared key and
    /// the config fallback are untouched, and with no name property `name`
    /// still orders by `name`.
    #[test]
    fn resolve_sorts_name_property_reaches_only_the_name_rows() {
        let config = name_asc_config_sorts();
        assert_eq!(
            resolve_sorts(Some("sort=id"), &config, &[], Some("sort-key")),
            vec![cpe_core::ir::Sort {
                key: cpe_core::ir::SortKey::Id,
                direction: cpe_core::ir::SortDir::Asc,
            }]
        );
        assert_eq!(
            resolve_sorts(None, &config, &[], Some("sort-key")),
            config,
            "the config sort is never rewritten, even when it orders by name"
        );
        assert_eq!(
            resolve_sorts(Some("sort=name"), &config, &[], None),
            config,
            "absent name property ⇒ ?sort=name orders by name"
        );
    }

    #[test]
    fn resolve_sorts_class_overrides_config() {
        let config = name_asc_config_sorts();
        assert_eq!(
            resolve_sorts(Some("sort=class"), &config, &[], None),
            vec![cpe_core::ir::Sort {
                key: cpe_core::ir::SortKey::Class,
                direction: cpe_core::ir::SortDir::Asc,
            }]
        );
    }

    #[test]
    fn resolve_sorts_mixed_with_other_params() {
        let config = name_asc_config_sorts();
        assert_eq!(
            resolve_sorts(Some("page=2&sort=id&lang=de"), &config, &[], None),
            vec![cpe_core::ir::Sort {
                key: cpe_core::ir::SortKey::Id,
                direction: cpe_core::ir::SortDir::Asc,
            }]
        );
    }

    // resolve_sorts: declared `order-by` options

    fn declared_order_options() -> Vec<cpe_core::ir::OrderOption> {
        vec![
            cpe_core::ir::OrderOption {
                key: "date".to_owned(),
                property: "pub-date".to_owned(),
                direction: cpe_core::ir::SortDir::Asc,
                label: "order-book-date".to_owned(),
            },
            cpe_core::ir::OrderOption {
                key: "extent".to_owned(),
                property: "extent".to_owned(),
                direction: cpe_core::ir::SortDir::Desc,
                label: "order-book-extent".to_owned(),
            },
        ]
    }

    /// Tier 2: a declared key resolves to its option's `Sort` (property +
    /// direction), overriding the config sort.
    #[test]
    fn resolve_sorts_declared_key_overrides_config() {
        let config = name_asc_config_sorts();
        let options = declared_order_options();
        assert_eq!(
            resolve_sorts(Some("page=2&sort=extent"), &config, &options, None),
            vec![cpe_core::ir::Sort {
                key: cpe_core::ir::SortKey::Property("extent".to_owned()),
                direction: cpe_core::ir::SortDir::Desc,
            }]
        );
        assert_eq!(
            resolve_sorts(Some("sort=date"), &config, &options, None),
            vec![options[0].sort()]
        );
    }

    /// Tier 1 still wins: a built-in token is parsed by `Sort::parse_wire`
    /// even when the list declares options.
    #[test]
    fn resolve_sorts_builtin_still_resolves_with_declared_options() {
        let config = name_asc_config_sorts();
        assert_eq!(
            resolve_sorts(Some("sort=id"), &config, &declared_order_options(), None),
            vec![cpe_core::ir::Sort {
                key: cpe_core::ir::SortKey::Id,
                direction: cpe_core::ir::SortDir::Asc,
            }]
        );
    }

    /// Tier 3: an undeclared token (and `default`, and an absent param) still
    /// falls back silently to the config sort — and a key declared by a
    /// DIFFERENT list (here: none passed) means nothing.
    #[test]
    fn resolve_sorts_unknown_token_falls_back_with_declared_options() {
        let config = name_asc_config_sorts();
        let options = declared_order_options();
        assert_eq!(resolve_sorts(Some("sort=bogus"), &config, &options, None), config);
        assert_eq!(resolve_sorts(Some("sort=default"), &config, &options, None), config);
        assert_eq!(resolve_sorts(None, &config, &options, None), config);
        assert_eq!(resolve_sorts(Some("sort=date"), &config, &[], None), config);
    }

    // search-bar query injection

    #[test]
    fn zeta_search_query_narrows_list_and_sets_unfiltered_total() {
        let zeta = load_zeta_project();
        let page = zeta
            .config()
            .pages
            .iter()
            .find(|p| p.route == "/things")
            .expect("things page exists in zeta config");
        let lang = cpe_core::lang::resolve(&[], &zeta.config().site);
        let (components, status) = build_component_data(&zeta, page, &HashMap::new(), Some("q=rab"), &lang);
        assert_eq!(status, StatusCode::OK);
        let list_page = components
            .iter()
            .find_map(|c| match c {
                cpe_core::view_data::ComponentData::List(lp) => Some(lp),
                _ => None,
            })
            .expect("things page has a ResourceList component");
        assert_eq!(list_page.total, 1, "q=rab narrows to the single RabbIT resource");
        assert_eq!(
            list_page.unfiltered_total,
            Some(4),
            "unfiltered_total must be the config-wheres-only count (all 4 Things), even \
             though this page has no facet-panel — search-only narrowing still shows N of M"
        );
    }

    #[test]
    fn zeta_no_query_param_full_list_no_unfiltered_total() {
        let zeta = load_zeta_project();
        let page = zeta
            .config()
            .pages
            .iter()
            .find(|p| p.route == "/things")
            .expect("things page exists in zeta config");
        let lang = cpe_core::lang::resolve(&[], &zeta.config().site);
        let (components, status) = build_component_data(&zeta, page, &HashMap::new(), None, &lang);
        assert_eq!(status, StatusCode::OK);
        let list_page = components
            .iter()
            .find_map(|c| match c {
                cpe_core::view_data::ComponentData::List(lp) => Some(lp),
                _ => None,
            })
            .expect("things page has a ResourceList component");
        assert_eq!(list_page.total, 4, "no ?q= → full list of 4 Things");
        assert_eq!(
            list_page.unfiltered_total, None,
            "no query param at all → unfiltered_total stays None"
        );
    }

    #[test]
    fn zeta_empty_query_param_full_list_no_unfiltered_total() {
        let zeta = load_zeta_project();
        let page = zeta
            .config()
            .pages
            .iter()
            .find(|p| p.route == "/things")
            .expect("things page exists in zeta config");
        let lang = cpe_core::lang::resolve(&[], &zeta.config().site);
        let (components, status) = build_component_data(&zeta, page, &HashMap::new(), Some("q="), &lang);
        assert_eq!(status, StatusCode::OK);
        let list_page = components
            .iter()
            .find_map(|c| match c {
                cpe_core::view_data::ComponentData::List(lp) => Some(lp),
                _ => None,
            })
            .expect("things page has a ResourceList component");
        assert_eq!(list_page.total, 4, "?q= (empty value) → full list of 4 Things");
        assert_eq!(
            list_page.unfiltered_total, None,
            "empty ?q= → inactive search → unfiltered_total stays None"
        );
    }

    #[test]
    fn zeta_whitespace_only_query_full_list_no_unfiltered_total() {
        // A REAL percent-encoded single space (`%20`), exactly as a browser
        // would send it — this is the case that proves the "active" test is
        // NOT raw-string non-emptiness. `read_search_query_param` decodes
        // `%20` to an actual space before `is_active_search_query` sees it,
        // so this must still be judged *inactive* (no alphanumeric token
        // survives tokenization) even though the raw query-string bytes are
        // non-empty.
        let zeta = load_zeta_project();
        let page = zeta
            .config()
            .pages
            .iter()
            .find(|p| p.route == "/things")
            .expect("things page exists in zeta config");
        let lang = cpe_core::lang::resolve(&[], &zeta.config().site);
        let (components, status) = build_component_data(&zeta, page, &HashMap::new(), Some("q=%20"), &lang);
        assert_eq!(status, StatusCode::OK, "whitespace-only q must not error");
        let list_page = components
            .iter()
            .find_map(|c| match c {
                cpe_core::view_data::ComponentData::List(lp) => Some(lp),
                _ => None,
            })
            .expect("things page has a ResourceList component");
        assert_eq!(list_page.total, 4, "whitespace-only ?q= → full list of 4 Things");
        assert_eq!(
            list_page.unfiltered_total, None,
            "whitespace-only ?q= is inactive → unfiltered_total stays None, no clause pushed"
        );
    }

    #[test]
    fn zeta_multi_word_query_percent_encoded_space_matches() {
        // `read_search_query_param` must percent-decode `%20` (the
        // `encodeURIComponent` convention the DataStar debounced-live path
        // uses) back to a real space BEFORE tokenization, so a two-word
        // query narrows correctly instead of silently matching nothing (the
        // literal string "quantum%20cat" contains no space and would never
        // match "Quantum Cat").
        let zeta = load_zeta_project();
        let page = zeta
            .config()
            .pages
            .iter()
            .find(|p| p.route == "/things")
            .expect("things page exists in zeta config");
        let lang = cpe_core::lang::resolve(&[], &zeta.config().site);
        let (components, status) = build_component_data(&zeta, page, &HashMap::new(), Some("q=quantum%20cat"), &lang);
        assert_eq!(status, StatusCode::OK);
        let list_page = components
            .iter()
            .find_map(|c| match c {
                cpe_core::view_data::ComponentData::List(lp) => Some(lp),
                _ => None,
            })
            .expect("things page has a ResourceList component");
        assert_eq!(
            list_page.items.len(),
            1,
            "\"quantum%20cat\" must decode to \"quantum cat\" and match only Quantum Cat"
        );
        assert_eq!(list_page.items[0].id, "thing-02");
    }

    #[test]
    fn zeta_multi_word_query_plus_encoded_space_matches() {
        // The zero-JS `<form method=get>` fallback submits
        // `application/x-www-form-urlencoded`, which escapes a space as a
        // literal `+`, not `%20`. `read_search_query_param` must handle BOTH
        // conventions.
        let zeta = load_zeta_project();
        let page = zeta
            .config()
            .pages
            .iter()
            .find(|p| p.route == "/things")
            .expect("things page exists in zeta config");
        let lang = cpe_core::lang::resolve(&[], &zeta.config().site);
        let (components, status) = build_component_data(&zeta, page, &HashMap::new(), Some("q=quantum+cat"), &lang);
        assert_eq!(status, StatusCode::OK);
        let list_page = components
            .iter()
            .find_map(|c| match c {
                cpe_core::view_data::ComponentData::List(lp) => Some(lp),
                _ => None,
            })
            .expect("things page has a ResourceList component");
        assert_eq!(
            list_page.items.len(),
            1,
            "\"quantum+cat\" must decode to \"quantum cat\" and match only Quantum Cat"
        );
        assert_eq!(list_page.items[0].id, "thing-02");
    }

    /// GET `/zeta/things?q=rab` → 200 + narrowed list content (zero-JS `?q=`
    /// narrowing end-to-end).
    #[tokio::test]
    async fn zeta_http_get_things_with_query_narrows_response_body() {
        let zeta = load_zeta_project();
        let router = build(
            &[zeta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/zeta/things?q=rab")
                    .body(Body::empty())
                    .expect("well-formed request"),
            )
            .await
            .expect("oneshot succeeds");
        assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");
        assert!(
            body.contains("RabbIT"),
            "narrowed response must contain the matching resource's name, got: (truncated) {}",
            &body[..body.len().min(600)]
        );
        assert!(
            !body.contains("Quantum Cat"),
            "narrowed response must not contain a non-matching resource's name"
        );
        assert!(
            !body.contains("Looking Glass"),
            "narrowed response must not contain a non-matching resource's name"
        );
        assert!(
            !body.contains("Teapot"),
            "narrowed response must not contain a non-matching resource's name"
        );
    }

    // `_search` fragment handler

    /// A faceted page's `_search` fragment (`eta`, facet-panel + search-bar
    /// both bound to `widgets`) returns THREE `datastar-patch-elements`
    /// events — filterbar (`#filters-widgets`) + list (`#widgets`) + the
    /// recomposed crumb line (`#crumb-widgets`) — never the panel
    /// (`#facets-widgets`; facet counts stay static).
    #[tokio::test]
    async fn search_fragment_faceted_returns_two_events() {
        let eta = load_eta_project();
        let router = build(
            &[eta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/eta/widgets/_search?q=rab")
                    .body(Body::empty())
                    .expect("well-formed request"),
            )
            .await
            .expect("oneshot succeeds");
        assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");

        let event_count = body.matches("event: datastar-patch-elements").count();
        assert_eq!(
            event_count, 3,
            "a faceted page's _search fragment must emit exactly 3 events, got {event_count} in: {body}"
        );
        assert!(
            body.contains("selector #filters-widgets"),
            "must patch the filterbar #filters-widgets, got: {body}"
        );
        assert!(body.contains("selector #widgets"), "must patch the list #widgets, got: {body}");
        assert!(
            !body.contains("selector #facets-widgets"),
            "must NOT patch the facet panel — facet counts stay static, got: {body}"
        );
        // Third patch target here: the recomposed crumb line, carrying the
        // active `?q=` in its composed href and its state-bearing accessible
        // name.
        assert!(
            body.contains("selector #crumb-widgets"),
            "must patch the crumb line #crumb-widgets, got: {body}"
        );
        assert!(
            body.contains("q=rab") && body.contains("\u{201C}rab\u{201D}"),
            "crumb payload must carry the recomposed href (q=rab) and the \
             quoted active-query accessible-name suffix, got: {body}"
        );
    }

    /// A non-faceted page's `_search` fragment (`zeta`, search-bar only, no
    /// facet-panel) returns exactly TWO events — the list plus the
    /// recomposed crumb line (`#crumb-things`); there is no filterbar
    /// element in the DOM to patch.
    #[tokio::test]
    async fn search_fragment_non_faceted_returns_one_event() {
        let zeta = load_zeta_project();
        let router = build(
            &[zeta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/zeta/things/_search?q=rab")
                    .body(Body::empty())
                    .expect("well-formed request"),
            )
            .await
            .expect("oneshot succeeds");
        assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");

        let event_count = body.matches("event: datastar-patch-elements").count();
        assert_eq!(
            event_count, 2,
            "a non-faceted page's _search fragment must emit exactly 2 events, got {event_count} in: {body}"
        );
        assert!(body.contains("selector #things"), "must patch the list #things, got: {body}");
        assert!(
            !body.contains("filters-things"),
            "a non-faceted page has no filterbar to patch, got: {body}"
        );
        assert!(
            body.contains("selector #crumb-things"),
            "must patch the crumb line #crumb-things, got: {body}"
        );
        assert!(
            body.contains("q=rab") && body.contains("\u{201C}rab\u{201D}"),
            "crumb payload must carry the recomposed href (q=rab) and the \
             quoted active-query accessible-name suffix, got: {body}"
        );
    }

    /// A page with BOTH a facet-panel that resolves to ZERO groups (`theta` —
    /// `facet-panel target="widgets"` with no `facet` children) AND a search-bar
    /// on the same list must emit exactly TWO `datastar-patch-elements` events
    /// (the list plus the recomposed crumb line), never THREE. A gate in
    /// `make_search_fragment_handler` on mere `facet_groups.is_some()` instead
    /// of "non-empty groups" would emit an extra event trying to patch
    /// `#filters-widgets`, a `<div>` the zero-groups (non-faceted) layout never
    /// renders. `search_fragment_faceted_returns_two_events` (real, non-empty
    /// facet group) and `search_fragment_non_faceted_returns_one_event` (no
    /// facet-panel component at all) pass under that gate too — this is the
    /// one case that catches it.
    #[tokio::test]
    async fn search_fragment_zero_groups_facet_panel_returns_one_event() {
        let theta = load_theta_project();
        let router = build(
            &[theta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/theta/widgets/_search?q=rab")
                    .body(Body::empty())
                    .expect("well-formed request"),
            )
            .await
            .expect("oneshot succeeds");
        assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");

        let event_count = body.matches("event: datastar-patch-elements").count();
        assert_eq!(
            event_count, 2,
            "a zero-groups facet-panel + search-bar page's _search fragment must emit exactly \
             2 events, got {event_count} in: {body}"
        );
        assert!(body.contains("selector #widgets"), "must patch the list #widgets, got: {body}");
        assert!(
            body.contains("selector #crumb-widgets"),
            "must patch the crumb line #crumb-widgets, got: {body}"
        );
        assert!(
            !body.contains("filters-widgets"),
            "a zero-groups facet-panel renders no filterbar to patch, got: {body}"
        );
    }

    /// The `_search` fragment's swapped-in list shows the localized "N of M"
    /// narrowed count: the `unfiltered_total` gate reaches the RENDERED
    /// fragment via `SearchFragmentSurface`.
    #[tokio::test]
    async fn search_fragment_query_narrows_with_unfiltered_total() {
        let eta = load_eta_project();
        let router = build(
            &[eta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/eta/widgets/_search?q=rab")
                    .body(Body::empty())
                    .expect("well-formed request"),
            )
            .await
            .expect("oneshot succeeds");
        assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");

        assert!(
            body.contains("RabbIT Widget"),
            "narrowed response must contain the matching resource, got: (truncated) {}",
            &body[..body.len().min(800)]
        );
        assert!(
            !body.contains("Quantum Cat Widget") && !body.contains("Teapot Widget"),
            "narrowed response must not contain non-matching resources, got: (truncated) {}",
            &body[..body.len().min(800)]
        );
        assert!(
            body.contains("1 of 3"),
            "the surf-head must show the narrowed \"1 of 3\" count, got: (truncated) {}",
            &body[..body.len().min(800)]
        );
        assert!(
            body.contains(r#"class="filterchip filterchip--query""#),
            "the filterbar must render the query chip, got: (truncated) {}",
            &body[..body.len().min(2000)]
        );
    }

    /// Caret-safety contract: the search row's `#q` input
    /// and `.search-row` element are in NEITHER `_search` target selector —
    /// they never appear in the fragment response at all, so the live
    /// keystroke that triggered the request can never disturb its own
    /// caret/selection.
    #[tokio::test]
    async fn search_fragment_never_includes_the_search_row_itself() {
        let eta = load_eta_project();
        let router = build(
            &[eta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/eta/widgets/_search?q=rab")
                    .body(Body::empty())
                    .expect("well-formed request"),
            )
            .await
            .expect("oneshot succeeds");
        assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");

        assert!(
            !body.contains(r#"id="q""#),
            "the _search fragment must never re-render the #q input itself, got: {body}"
        );
        assert!(
            !body.contains("search-row"),
            "the _search fragment must never re-render .search-row, got: {body}"
        );
    }

    /// An inactive (whitespace-only) `?q=` on the `_search` endpoint clears
    /// the narrowing — the list returns to the full unfiltered set and the
    /// query chip disappears, mirroring the `×` clear button's own request.
    #[tokio::test]
    async fn search_fragment_inactive_query_shows_full_list_no_chip() {
        let eta = load_eta_project();
        let router = build(
            &[eta],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/eta/widgets/_search?q=")
                    .body(Body::empty())
                    .expect("well-formed request"),
            )
            .await
            .expect("oneshot succeeds");
        assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");

        assert!(
            body.contains("RabbIT Widget") && body.contains("Quantum Cat Widget") && body.contains("Teapot Widget"),
            "an inactive query must show the full unnarrowed list, got: (truncated) {}",
            &body[..body.len().min(1200)]
        );
        assert!(
            !body.contains("filterchip--query"),
            "an inactive query must not render the query chip, got: (truncated) {}",
            &body[..body.len().min(1200)]
        );
    }

    // Parameterised compound-viewer pages: real-navigation paging

    /// A minimal equivalent of `cpe-web`'s `tests/projects/good-compound-viewer-param/`
    /// (that fixture has no `compound_parts` rows, so it cannot answer for a
    /// *known* id): two parameterised pages with a `$id`-sourced
    /// `compound-viewer` —
    /// - `/story/:id` — compound-only (no `resource-detail`);
    /// - `/books/:id` — `resource-detail id="$id"` beside the compound-viewer (the Incunabula
    ///   `/books/:id` reader shape);
    ///
    /// plus one static page with NO `CompoundViewer` at all (`/plain`) —
    /// the `_compound` route-existence negative case.
    fn load_param_compound_project() -> Arc<cpe_web::project::Project> {
        let config_src = r#"
site {
    title "Param Compound"
    languages "de" "en"
}

page id="story" route="/story/:id" {
    compound-viewer source="$id" layout="contents" index="rail"
}

page id="book" route="/books/:id" {
    resource-detail id="$id" title-property="name"
    compound-viewer source="$id" layout="contents" index="rail"
}

page id="plain" route="/plain" {
    narrative file="plain.md"
}
"#;
        let data_sql = "\
INSERT INTO resources (id, class) VALUES ('ST_001', 'Story');
INSERT INTO resources (id, class) VALUES ('SCH_001', 'StoryChapter');
INSERT INTO resources (id, class) VALUES ('SCH_002', 'StoryChapter');
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('ST_001', 'SCH_001', 0);
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('ST_001', 'SCH_002', 1);
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('ST_001', 'name', 0, 'text', 'Alice''s Story');
";
        let ctx = cpe_core::validate::context::ValidationContext {
            project_id: "param-compound",
            reserved_ids: &["_static"],
            narrative_files: &[],
            narrative_files_complete: false,
            asset_files: &[],
        };
        Arc::new(
            cpe_web::project::Project::load(&ctx, config_src, data_sql, &[]).expect("param-compound project loads"),
        )
    }

    /// GET `uri` against a router over the param-compound project; returns
    /// (status, content-type, body).
    async fn get_param_compound(uri: &str) -> (StatusCode, String, String) {
        let router = build(
            &[load_param_compound_project()],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(Request::builder().uri(uri).body(Body::empty()).expect("well-formed request"))
            .await
            .expect("oneshot succeeds");
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");
        (status, content_type, body)
    }

    // Incoming links

    /// A synthetic strip/page/book bag: `b1`/`b2` BorderStrips (only `b1` is
    /// linked to), Pages `p1`/`p2` that are members of Book `bk1`, and a
    /// `/books/:id` reader page so rung 2 (member inside its parent's reader)
    /// resolves. Page has no detail binding of its own.
    fn load_incoming_project() -> Arc<cpe_web::project::Project> {
        let config_src = r#"
site {
    title "Incoming"
    languages "de" "en"
    chrome {
        string "pages-using" {
            de "Verwendet auf"
            en "Used on"
        }
    }
}

page id="book" route="/books/:id" {
    resource-detail id="$id" class="Book"
    compound-viewer source="$id" layout="contents" index="rail"
}

page id="strip" route="/strips/:id" {
    resource-detail id="$id" class="BorderStrip" {
        incoming-links label="pages-using" class="Page" "strip-left" "strip-right" order="compound-seq"
    }
}

page id="strip-limited" route="/strips-limited/:id" {
    resource-detail id="$id" {
        incoming-links label="pages-using" class="Page" "strip-left" "strip-right" order="compound-seq" limit=1
    }
}
"#;
        let data_sql = "\
INSERT INTO resources (id, class) VALUES ('b1', 'BorderStrip');
INSERT INTO resources (id, class) VALUES ('b2', 'BorderStrip');
INSERT INTO resources (id, class) VALUES ('bk1', 'Book');
INSERT INTO resources (id, class) VALUES ('p1', 'Page');
INSERT INTO resources (id, class) VALUES ('p2', 'Page');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('b1', 'name', 0, 'text', 'Strip one');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('b2', 'name', 0, 'text', 'Strip two');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('bk1', 'name', 0, 'text', 'Book one');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('p1', 'name', 0, 'text', 'a1r');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('p2', 'name', 0, 'text', 'a1v');
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES ('p1', 'strip-left', 0, 'link', 'b1');
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES ('p2', 'strip-right', 0, 'link', 'b1');
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('bk1', 'p1', 0);
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('bk1', 'p2', 1);
";
        let ctx = cpe_core::validate::context::ValidationContext {
            project_id: "incoming",
            reserved_ids: &["_static"],
            narrative_files: &[],
            narrative_files_complete: false,
            asset_files: &[],
        };
        Arc::new(cpe_web::project::Project::load(&ctx, config_src, data_sql, &[]).expect("incoming project loads"))
    }

    async fn get_incoming(uri: &str) -> (StatusCode, String) {
        let router = build(
            &[load_incoming_project()],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(Request::builder().uri(uri).body(Body::empty()).expect("well-formed request"))
            .await
            .expect("oneshot succeeds");
        let status = response.status();
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");
        (status, body)
    }

    #[test]
    fn resolve_incoming_links_fetches_rows_for_the_pages_spec() {
        let project = load_incoming_project();
        let page = project.config().pages.iter().find(|p| p.id == "strip").expect("strip page");
        let lang = cpe_core::lang::resolve(&["en".to_owned()], &project.config().site);
        let resource = project.get_resource("b1", &lang).expect("query ok").expect("b1 exists");
        let components = [cpe_core::view_data::ComponentData::Detail(Some(resource))];
        let rows = resolve_incoming_links(&project, page, &components, &lang).expect("Some");
        let ids: Vec<&str> = rows.rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["p1", "p2"], "compound-seq order");

        // A page whose resource-detail declares no incoming-links ⇒ None.
        let book = project.config().pages.iter().find(|p| p.id == "book").expect("book page");
        assert!(resolve_incoming_links(&project, book, &components, &lang).is_none());
        // No resolved resource ⇒ None.
        assert!(
            resolve_incoming_links(&project, page, &[cpe_core::view_data::ComponentData::Detail(None)], &lang)
                .is_none()
        );
    }

    #[test]
    fn resolve_incoming_links_honours_small_limit() {
        // A spec whose `limit` is smaller than its full match count.
        let project = load_incoming_project();
        let page = project
            .config()
            .pages
            .iter()
            .find(|p| p.id == "strip-limited")
            .expect("strip-limited page");
        let lang = cpe_core::lang::resolve(&["en".to_owned()], &project.config().site);
        let resource = project.get_resource("b1", &lang).expect("query ok").expect("b1 exists");
        let components = [cpe_core::view_data::ComponentData::Detail(Some(resource))];
        let result = resolve_incoming_links(&project, page, &components, &lang).expect("Some");
        assert_eq!(result.rows.len(), 1, "capped to limit=1");
        assert_eq!(result.total, 2, "uncapped total is still 2");
        assert_eq!(result.rows[0].id, "p1", "compound-seq order, first row");
    }

    #[tokio::test]
    async fn strip_page_renders_incoming_rows_into_the_parents_reader() {
        let (status, body) = get_incoming("/incoming/strips/b1?img=2").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(r#"<span class="pgroup__title">Verwendet auf</span>"#), "{body}");
        assert!(
            body.contains(r#"href="/incoming/books/bk1?page=p1"><span class="ref">a1r</span></a>"#),
            "rung 2, the strip page's `img=` not carried over: {body}"
        );
        assert!(
            body.contains(r#"href="/incoming/books/bk1?page=p2"><span class="ref">a1v</span></a>"#),
            "{body}"
        );
    }

    #[tokio::test]
    async fn strip_nothing_links_to_renders_the_empty_sentence() {
        let (status, body) = get_incoming("/incoming/strips/b2").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(r#"<span class="pgroup__title">Verwendet auf</span>"#), "{body}");
        assert!(body.contains("Nichts verweist hierher."), "{body}");
    }

    /// The markup of the first element opening with `open` up to its first
    /// `close` — scopes a "no `data-on`" assertion to one element.
    fn element<'o>(body: &'o str, open: &str, close: &str) -> &'o str {
        let start = body.find(open).unwrap_or_else(|| panic!("`{open}` missing, got: {body}"));
        let end = body[start..]
            .find(close)
            .unwrap_or_else(|| panic!("`{close}` after `{open}` missing, got: {body}"));
        &body[start..start + end + close.len()]
    }

    /// Member paging on a parameterised reader is a real page load AND
    /// progressively enhanced: `?page=<id>` and the legacy `?member=N` both
    /// land on the same member of a full page (the no-JS path), and every
    /// active pager step / rail link ALSO carries `data-on:click__prevent`.
    #[tokio::test]
    async fn param_compound_paging_is_real_navigation() {
        for base in ["/param-compound/story/ST_001", "/param-compound/books/ST_001"] {
            for query in ["?page=SCH_002", "?member=2"] {
                let uri = format!("{base}{query}");
                let (status, content_type, body) = get_param_compound(&uri).await;
                assert_eq!(status, StatusCode::OK, "{uri} must answer 200");
                assert!(
                    content_type.contains("text/html"),
                    "{uri} is a full page, got content-type {content_type}"
                );
                let pager = element(&body, "<nav class=\"pg\"", "</nav>");
                assert!(
                    pager.contains(">2 / 2</span>"),
                    "{uri} must land on member 2 of 2, got: {pager}"
                );
                let rail = element(&body, "<nav class=\"cv-toc__contents\"", "</nav>");
                assert!(
                    rail.contains(&format!("href=\"{base}?page=SCH_002\"")) && rail.contains("aria-current=\"page\""),
                    "{uri}: SCH_002 is the current rail entry, got: {rail}"
                );
                // Every active pager step / rail link ALSO carries
                // `data-on:click__prevent` (the `_compound` fragment swap,
                // progressively enhancing the same plain `href` this test
                // already asserts above), unlike an inert boundary span.
                for (name, el) in [("pager", &pager), ("rail", &rail)] {
                    assert!(
                        el.contains("data-on:click__prevent=\"@get('"),
                        "{uri}: the {name}'s active links must carry the fragment-swap @get, got: {el}"
                    );
                }
            }
        }
    }

    /// The `_compound` fragment endpoint: on both parameterised page shapes
    /// it answers a real SSE stream (not the router fallback), with an event
    /// patching `#compound-viewer`.
    #[tokio::test]
    async fn param_compound_fragment_route_exists() {
        for uri in [
            "/param-compound/story/ST_001/_compound?page=SCH_002",
            "/param-compound/books/ST_001/_compound?page=SCH_002",
        ] {
            let (status, content_type, body) = get_param_compound(uri).await;
            assert_eq!(status, StatusCode::OK, "{uri} must answer 200, got: {body}");
            assert!(
                content_type.contains("text/event-stream"),
                "{uri} must answer SSE, got content-type {content_type}"
            );
            assert!(
                body.contains("selector #compound-viewer"),
                "{uri} must patch #compound-viewer, got: {body}"
            );
        }
    }

    /// The route-existence NEGATIVE case: a page with no `CompoundViewer`
    /// component never gets a `_compound` route registered at all, so it 404s
    /// gracefully through the router fallback — not an SSE stream, not a panic.
    #[tokio::test]
    async fn compound_fragment_route_absent_for_page_without_compound_viewer() {
        let (status, content_type, _body) = get_param_compound("/param-compound/plain/_compound").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "must 404 gracefully");
        assert!(
            !content_type.contains("text/event-stream"),
            "must not answer SSE, got content-type {content_type}"
        );
    }

    /// A route-param compound page's `_compound` request resolves the REAL
    /// `$id` value — not `ComponentData::Other` — via
    /// `make_compound_fragment_handler_param`'s `Path` extraction.
    /// The book page (`/books/:id`, mirroring Incunabula's reader) is the
    /// load-bearing case: its `compound-viewer source="$id"` needs the route
    /// param, unlike `_facets`/`_sort`/`_search`, which never need one. The
    /// swapped-in `#compound-viewer` markup for `?page=SCH_002` must show
    /// member 2 of 2 — proving `$id` (`ST_001`, from the URL's `:id`
    /// segment) resolved to the right compound, not an empty/default one.
    #[tokio::test]
    async fn param_compound_fragment_resolves_real_route_param_id() {
        let (status, _content_type, body) =
            get_param_compound("/param-compound/books/ST_001/_compound?page=SCH_002").await;
        assert_eq!(status, StatusCode::OK, "got body: {body}");
        assert!(
            body.contains("SCH_002"),
            "the fragment must resolve $id=ST_001's compound member SCH_002 \
             (route-param Path extraction, Step 1), got: {body}"
        );
        assert!(
            body.contains(">2 / 2<") || body.contains("2 / 2"),
            "must land on member 2 of 2, got: {body}"
        );
    }

    // The `_compound` fragment's events

    /// The morph contract (mirroring DPE's
    /// `morph_contract_project_tabs_open_tag_identical`-style test): the
    /// `_compound` fragment's `#compound-viewer` markup must open with the
    /// byte-identical tag `render_page`'s own CompoundViewer render produces
    /// for the same page/state — both paths go through the SAME
    /// `render_compound_fragment`/`render_component` call internally, so this
    /// guards against drift between the two.
    #[tokio::test]
    async fn compound_fragment_viewer_open_tag_matches_full_page_render() {
        let (full_status, _ct, full_body) = get_param_compound("/param-compound/books/ST_001?page=SCH_002").await;
        assert_eq!(full_status, StatusCode::OK);
        let (frag_status, _ct2, frag_body) =
            get_param_compound("/param-compound/books/ST_001/_compound?page=SCH_002").await;
        assert_eq!(frag_status, StatusCode::OK);

        fn open_tag(body: &str) -> &str {
            let id_pos = body.find(" id=\"compound-viewer\"").expect("compound-viewer id present");
            let start = body[..id_pos].rfind('<').expect("tag start");
            let end = body[start..].find('>').expect("tag end") + start + 1;
            &body[start..end]
        }

        assert_eq!(
            open_tag(&full_body),
            open_tag(&frag_body),
            "full-page and fragment #compound-viewer open tags must be \
             byte-identical (PRD I3), full:\n{full_body}\nfragment:\n{frag_body}"
        );
    }

    /// A `layout="pages"` compound-viewer fixture — the param-compound
    /// project above is `layout="contents"` throughout, which cannot cover
    /// the viewer-signal `PatchSignals` reset (pages-layout only).
    fn load_pages_compound_project() -> Arc<cpe_web::project::Project> {
        let config_src = r#"
site {
    title "Pages Compound"
    languages "de" "en"
}

page id="story" route="/story/:id" {
    compound-viewer source="$id" layout="pages" index="none" media="scan"
}
"#;
        let data_sql = "\
INSERT INTO resources (id, class) VALUES ('ST_001', 'Story');
INSERT INTO resources (id, class) VALUES ('SCH_001', 'StoryChapter');
INSERT INTO resources (id, class) VALUES ('SCH_002', 'StoryChapter');
INSERT INTO resources (id, class) VALUES ('IMG_001', 'Image');
INSERT INTO resources (id, class) VALUES ('IMG_002', 'Image');
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('ST_001', 'SCH_001', 0);
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('ST_001', 'SCH_002', 1);
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES ('SCH_001', 'scan', 0, 'link', 'IMG_001');
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES ('SCH_002', 'scan', 0, 'link', 'IMG_002');
INSERT INTO representations (resource_id, kind, uri, width, height, seq) VALUES ('IMG_001', 'image', 'https://iiif.example.org/IMG_001/full/max/0/default.jpg', 1000, 1200, 0);
INSERT INTO representations (resource_id, kind, uri, width, height, seq) VALUES ('IMG_002', 'image', 'https://iiif.example.org/IMG_002/full/max/0/default.jpg', 1000, 1200, 0);
";
        let ctx = cpe_core::validate::context::ValidationContext {
            project_id: "pages-compound",
            reserved_ids: &["_static"],
            narrative_files: &[],
            narrative_files_complete: false,
            asset_files: &[],
        };
        Arc::new(
            cpe_web::project::Project::load(&ctx, config_src, data_sql, &[]).expect("pages-compound project loads"),
        )
    }

    /// GET `uri` against a router over the pages-compound project; returns
    /// (status, body). The handler is reached only by a forward click, so
    /// there is nothing to vary per request beyond the URI.
    async fn get_pages_compound(uri: &str) -> (StatusCode, String) {
        let router = build(
            &[load_pages_compound_project()],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(Request::builder().uri(uri).body(Body::empty()).expect("well-formed request"))
            .await
            .expect("oneshot succeeds");
        let status = response.status();
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");
        (status, body)
    }

    /// SSE event count and shape, `layout="contents"`:
    /// viewer `PatchElements`, crumb `PatchElements`, `#header-controls`
    /// `PatchElements` (sent even with no crumb) — the compound-only
    /// `/story/:id` fixture page has no sibling `ResourceDetail`/
    /// `ResourceList`/`Narrative`, so `render_compound_fragment` returns no
    /// crumb here; the param-compound project's `/books/:id` page (used in
    /// the other tests in this section) DOES pair a `resource-detail`, so
    /// its crumb patch is exercised there. No viewer-signal `PatchSignals`
    /// (contents layout has no viewer unit) — only the `$copied` reset — then
    /// the `ExecuteScript`.
    #[tokio::test]
    async fn compound_fragment_sse_shape_contents_layout() {
        let (status, _ct, body) = get_param_compound("/param-compound/story/ST_001/_compound?page=SCH_002").await;
        assert_eq!(status, StatusCode::OK, "got body: {body}");
        assert_eq!(
            body.matches("selector #compound-viewer").count(),
            1,
            "exactly one viewer PatchElements, got: {body}"
        );
        assert_eq!(
            body.matches("selector #header-controls").count(),
            1,
            "the header controls are patched even without a crumb, got: {body}"
        );
        assert_eq!(
            body.matches("event: datastar-patch-signals").count(),
            1,
            "contents layout: 1 PatchSignals ($copied reset only — no \
             viewer-signal reset, no currentPageQuery signal any more), got: {body}"
        );
        assert!(
            !body.contains("currentPageQuery"),
            "Option B retired the currentPageQuery signal entirely, got: {body}"
        );
        assert!(
            body.contains("selector body"),
            "the ExecuteScript event must target body (its own selector), got: {body}"
        );
    }

    /// SSE event count and shape, `layout="pages"`: the SAME
    /// shape as the contents-layout case (`#header-controls` included),
    /// PLUS one extra `PatchSignals` event resetting the `<image-viewer>` unit's page-global
    /// signals — 2 `PatchSignals` total ($copied + the viewer reset).
    #[tokio::test]
    async fn compound_fragment_sse_shape_pages_layout() {
        let (status, body) = get_pages_compound("/pages-compound/story/ST_001/_compound?page=SCH_002").await;
        assert_eq!(status, StatusCode::OK, "got body: {body}");
        assert_eq!(
            body.matches("selector #compound-viewer").count(),
            1,
            "exactly one viewer PatchElements, got: {body}"
        );
        assert_eq!(
            body.matches("selector #header-controls").count(),
            1,
            "exactly one header-controls PatchElements, got: {body}"
        );
        assert_eq!(
            body.matches("event: datastar-patch-signals").count(),
            2,
            "pages layout: 2 PatchSignals ($copied reset + the Step 3a \
             viewer-signal reset — no currentPageQuery signal any more), got: {body}"
        );
        assert!(
            body.contains("rail: 'closed'") && body.contains("atHome: true"),
            "the viewer-signal reset must cover BOTH the .rv-level and \
             .rv__canvas-level signal groups, got: {body}"
        );
        assert!(
            !body.contains("currentPageQuery"),
            "Option B retired the currentPageQuery signal entirely, got: {body}"
        );
        assert!(
            body.contains("selector body"),
            "the ExecuteScript event must target body (its own selector), got: {body}"
        );
        assert!(
            !body.contains("cv-annotations"),
            "a reader without annotations= patches no register target, got: {body}"
        );
    }

    /// A book composition whose `pages` reader has `annotations=#true`; only
    /// `SCH_002` carries an annotation.
    fn load_annotated_book_project() -> Arc<cpe_web::project::Project> {
        let config_src = r#"
site {
    title "Annotated Book"
    languages "de" "en"
}

page id="book" route="/books/:id" {
    resource-detail id="$id" title-property="name"
    compound-viewer source="$id" layout="pages" index="none" media="scan" annotations=#true
}
"#;
        let data_sql = "\
INSERT INTO resources (id, class) VALUES ('ST_001', 'Story');
INSERT INTO resources (id, class) VALUES ('SCH_001', 'StoryChapter');
INSERT INTO resources (id, class) VALUES ('SCH_002', 'StoryChapter');
INSERT INTO resources (id, class) VALUES ('IMG_001', 'Image');
INSERT INTO resources (id, class) VALUES ('IMG_002', 'Image');
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('ST_001', 'SCH_001', 0);
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('ST_001', 'SCH_002', 1);
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('ST_001', 'name', 0, 'text', 'Book');
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES ('SCH_001', 'scan', 0, 'link', 'IMG_001');
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES ('SCH_002', 'scan', 0, 'link', 'IMG_002');
INSERT INTO representations (resource_id, kind, uri, width, height, seq) VALUES ('IMG_001', 'image', 'https://iiif.example.org/IMG_001/full/max/0/default.jpg', 1000, 1200, 0);
INSERT INTO representations (resource_id, kind, uri, width, height, seq) VALUES ('IMG_002', 'image', 'https://iiif.example.org/IMG_002/full/max/0/default.jpg', 1000, 1200, 0);
INSERT INTO annotations (id, resource_id, shape, x, y, w, h, color) VALUES ('A1', 'SCH_002', 'rectangle', 0.1, 0.1, 0.2, 0.2, '#ff3333');
";
        let ctx = cpe_core::validate::context::ValidationContext {
            project_id: "annotated-book",
            reserved_ids: &["_static"],
            narrative_files: &[],
            narrative_files_complete: false,
            asset_files: &[],
        };
        Arc::new(
            cpe_web::project::Project::load(&ctx, config_src, data_sql, &[]).expect("annotated-book project loads"),
        )
    }

    async fn get_annotated_book(uri: &str) -> (StatusCode, String) {
        let router = build(
            &[load_annotated_book_project()],
            &[],
            &AssetInventory::default(),
            crate::request::OriginConfig::fixed("http://localhost:8080"),
            &cpe_core::url::ROOT,
        );
        let response = router
            .oneshot(Request::builder().uri(uri).body(Body::empty()).expect("well-formed request"))
            .await
            .expect("oneshot succeeds");
        let status = response.status();
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body readable")
                .into(),
        )
        .expect("UTF-8 body");
        (status, body)
    }

    /// An annotated reader's swap patches the register target once, after
    /// the viewer and the crumb and before every signal reset and the script.
    #[tokio::test]
    async fn compound_fragment_patches_the_annotation_register() {
        let (status, body) = get_annotated_book("/annotated-book/books/ST_001/_compound?page=SCH_002").await;
        assert_eq!(status, StatusCode::OK, "got body: {body}");
        assert_eq!(body.matches("selector #cv-annotations").count(), 1, "{body}");
        assert!(body.contains(r#"id="annotation-A1""#), "{body}");
        let at = |needle: &str| body.find(needle).unwrap_or_else(|| panic!("{needle}: {body}"));
        let register = at("selector #cv-annotations");
        assert!(at("selector #compound-viewer") < register, "{body}");
        assert!(at("selector #crumb-") < register, "{body}");
        assert!(register < at("event: datastar-patch-signals"), "{body}");
        assert!(register < at("selector body"), "{body}");
    }

    /// The header controls are patched after the viewer and the crumb and
    /// before the register, so every DOM patch lands before the signal resets.
    #[tokio::test]
    async fn compound_fragment_patches_the_header_controls_between_crumb_and_register() {
        let (status, body) = get_annotated_book("/annotated-book/books/ST_001/_compound?page=SCH_002").await;
        assert_eq!(status, StatusCode::OK, "got body: {body}");
        let at = |needle: &str| body.find(needle).unwrap_or_else(|| panic!("{needle}: {body}"));
        let header = at("selector #header-controls");
        assert!(at("selector #compound-viewer") < header, "{body}");
        assert!(at("selector #crumb-") < header, "{body}");
        assert!(header < at("selector #cv-annotations"), "{body}");
    }

    /// An annotated reader's swap resets hover, pin, arrival and the arrival
    /// note in their own `PatchSignals`, after the register patch, and never
    /// the toggle.
    #[tokio::test]
    async fn compound_fragment_resets_annotation_signals_but_not_the_toggle() {
        for uri in [
            "/annotated-book/books/ST_001/_compound?page=SCH_002",
            "/annotated-book/books/ST_001/_compound?page=SCH_001",
        ] {
            let (status, body) = get_annotated_book(uri).await;
            assert_eq!(status, StatusCode::OK, "got body: {body}");
            let reset = cpe_components::component::annotation::annotation_reset_signals_json();
            assert_eq!(reset, "{_annoHover: '', _annoPin: '', _annoArrive: '', _annoNote: ''}");
            assert_eq!(body.matches(reset.as_str()).count(), 1, "{body}");
            let register = body.find("selector #cv-annotations").expect("register patch");
            let at = body.find(reset.as_str()).expect("annotation reset");
            assert!(register < at, "{body}");
            // The patched markup reads `$_annoShow`; no signal patch writes it.
            assert!(
                body.split("event: ")
                    .filter(|e| e.starts_with("datastar-patch-signals"))
                    .all(|e| !e.contains("_annoShow")),
                "the toggle is never reset: {body}"
            );
        }
    }

    /// A page of an annotated reader without a register still patches the
    /// target, empty, so the previous page's register never lingers.
    #[tokio::test]
    async fn compound_fragment_patches_the_empty_register_target() {
        let (status, body) = get_annotated_book("/annotated-book/books/ST_001/_compound?page=SCH_001").await;
        assert_eq!(status, StatusCode::OK, "got body: {body}");
        assert_eq!(body.matches("selector #cv-annotations").count(), 1, "{body}");
        assert!(body.contains(r#"<div id="cv-annotations"></div>"#), "{body}");
    }

    /// A forward click always emits `history.pushState` and the paired
    /// `window.__cpeReaderUrl` update — never `replaceState` (Back/Forward is
    /// a real page load, see `cpe_components::component::compound_viewer`'s
    /// `data-on:popstate__window` doc comment).
    #[tokio::test]
    async fn compound_fragment_execute_script_pushes_state_on_forward_click() {
        let (status, body) = get_pages_compound("/pages-compound/story/ST_001/_compound?page=SCH_002").await;
        assert_eq!(status, StatusCode::OK, "got body: {body}");
        assert!(
            body.contains("history.pushState(null, '', '"),
            "a forward click must pushState, got: {body}"
        );
        assert!(
            !body.contains("replaceState"),
            "replaceState no longer exists anywhere (Option B), got: {body}"
        );
        assert!(
            body.contains("window.__cpeReaderUrl = '"),
            "the pushState script must also update window.__cpeReaderUrl, \
             the popstate listener's one-way comparison value, got: {body}"
        );
    }

    /// The URL in the `ExecuteScript` payload's `pushState` call is RELATIVE
    /// (origin-free — starts with `/`, no scheme/host), never the absolute
    /// `canonical_url` (`SecurityError` risk per the WHATWG spec if origin
    /// derivation ever mismatches), and carries neither the `_compound`
    /// fragment-endpoint segment nor the `#compound-viewer` anchor (the URL is
    /// anchor-free at the source). The `window.__cpeReaderUrl` update
    /// carries the SAME relative URL — the two must never drift.
    #[tokio::test]
    async fn compound_fragment_execute_script_canonical_url_is_anchor_and_segment_free() {
        let (status, _ct, body) = get_param_compound("/param-compound/books/ST_001/_compound?page=SCH_002").await;
        assert_eq!(status, StatusCode::OK, "got body: {body}");
        let script_at = body.find("history.").expect("history call present");
        let script_tail = &body[script_at..];
        assert!(
            script_tail.starts_with(
                "history.pushState(null, '', '/param-compound/books/ST_001?page=SCH_002'); \
                 window.__cpeReaderUrl = '/param-compound/books/ST_001?page=SCH_002';"
            ),
            "the pushState URL must be RELATIVE (origin-free) — the plain \
             page path+query, no _compound segment, no anchor — and \
             window.__cpeReaderUrl must carry the SAME URL, got: {body}"
        );
        assert!(
            !script_tail.contains("http://localhost:8080"),
            "the pushState URL must never carry the absolute origin (review \
             Fix 5 — SecurityError risk), got: {body}"
        );
        assert!(
            !script_tail.contains("_compound"),
            "the pushState URL must never address the _compound fragment endpoint, got: {body}"
        );
        assert!(
            !script_tail.contains("#compound-viewer"),
            "the pushState URL must never carry the #compound-viewer anchor, got: {body}"
        );
    }

    /// A `_compound` fragment response carries no `X-CPE-Popstate` handling, no
    /// guarded `replaceState` and no `currentPageQuery` `PatchSignals`.
    #[tokio::test]
    async fn compound_fragment_response_never_carries_retired_popstate_machinery() {
        let (status, body) = get_pages_compound("/pages-compound/story/ST_001/_compound?page=SCH_002").await;
        assert_eq!(status, StatusCode::OK, "got body: {body}");
        for needle in ["X-CPE-Popstate", "x-cpe-popstate", "currentPageQuery", "replaceState"] {
            assert!(
                !body.contains(needle),
                "retired popstate machinery `{needle}` must not appear, got: {body}"
            );
        }
    }

    /// Both the title and the URL are escaped via `js_string_escape`
    /// — asserted on a title containing an apostrophe (`ST_001`'s
    /// `name` = "Alice's Story", `title-property="name"` on `/books/:id`).
    #[tokio::test]
    async fn compound_fragment_execute_script_escapes_title_and_url() {
        let (status, _ct, body) = get_param_compound("/param-compound/books/ST_001/_compound?page=SCH_002").await;
        assert_eq!(status, StatusCode::OK, "got body: {body}");
        assert!(
            body.contains("document.title = 'Param Compound \u{2014} Alice\\'s Story';"),
            "the apostrophe in the title must be escaped via js_string_escape, got: {body}"
        );
        assert!(
            body.contains("document.getElementById('compound-viewer')?.focus({ preventScroll: true });"),
            "the focus-restore call must be present and pass preventScroll: true \
             (matching every other programmatic focus call in the codebase), got: {body}"
        );
    }
}
