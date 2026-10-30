//! Page assembly for the Configurable Presentation Environment.
//!
//! `render_page` / `render_compound_fragment` / `document_head` — full document
//! rendering, plus the overview of the projects and the two full-page
//! not-found renderers. The component
//! renderers live in `cpe_components::component`; the private `use` below
//! brings each child this file calls into scope by name.

use cpe_components::component::context::{NarrativeStore, RenderContext};
use cpe_components::component::{
    annotation, breadcrumb, compound_viewer, data_links, dom, editorial, facet_panel, links, media, narrative, project,
    resource_detail, resource_list, search_bar, values,
};
use cpe_core::ir::{
    any_facet_selected, Component, DataArks, FacetGroup, NarrativeCompanion, NarrativeSide, Resource, SiteConfig,
    TextAlign,
};
use cpe_core::labels::{
    chrome_lookup, class_plural_declared, nav_label, resolve_class_label, resolve_class_plural,
    resolve_class_plural_capitalized, resolve_gloss, resolve_order_name_labels, resolve_order_options, resolve_parts,
    site_title,
};
use cpe_core::lang::Lang;
use cpe_core::markup_contract::{HEADER_CONTROLS_ID, HEADER_DESK_BREAKPOINT_PX, HEADER_MENU_ID};
use cpe_core::theme::{Appearance, Typeface};
use cpe_core::util::page_title_heading;
use cpe_core::view_data::ComponentData;
use maud::{html, Markup, DOCTYPE};

/// The single source of truth for the CPE document `<head>`.
///
/// Emits: charset/viewport meta, `<title>`, two Google Fonts preconnects
/// (with `display=swap` to avoid FOIT), the Newsreader + IBM Plex Mono font
/// `<link>`, the 7 CSS `<link>`s in load order:
/// `tokens.css` → `shell.css` → `editorial.css` → `record.css` → `grid.css`
/// → `media.css` → `incunabula.css`, and `cpe_image_viewer::head_tag`
/// — the `<image-viewer>` custom-element registration, emitted globally on
/// every page.
///
/// Every full page calls this (`render_page`, the overview, the two not-found renderers)
/// so the design-system contract lives in one place — no `<head>` drift. The
/// host's 503 page is the exception: it loads no file at all.
///
/// `view_transitions`: when `true`, the head also
/// opts the document into cross-document View Transitions, under
/// `prefers-reduced-motion: no-preference` only. Both documents of a
/// navigation must opt in, so `render_page` passes `true` only on pages whose
/// body carries a `compound-viewer` — member paging inside a reader
/// cross-fades, and every other CPE navigation is unchanged. Every other
/// caller passes `false`, which emits nothing extra.
///
/// `typeface` picks the Google Fonts families to load: `Some(t)` (a project
/// page) loads `t`'s families plus Lora (the fixed `.dasch-link` face) and IBM
/// Plex Mono; `None` (a page outside any project) loads Newsreader and Plex Mono.
///
/// `statics` is where this page loads the shared static files from: its
/// project's directory, or CPE's own for a page without a project.
pub fn document_head(
    site_title: &str,
    view_transitions: bool,
    typeface: Option<Typeface>,
    statics: &cpe_core::url::StaticDir,
) -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            title { (site_title) }
            // Google Fonts preconnects — deviation #7 from the bundle (display=swap).
            link rel="preconnect" href="https://fonts.googleapis.com";
            link rel="preconnect" href="https://fonts.gstatic.com" crossorigin;
            // Only the faces this page renders (see `google_fonts_href`).
            // (Self-hosting them as woff2 is backlogged.)
            link rel="stylesheet" href=(google_fonts_href(typeface));
            // Design-system CSS — order is load order (tokens first, grid last).
            link rel="stylesheet" href=(statics.file("cpe/tokens.css"));
            link rel="stylesheet" href=(statics.file("cpe/shell.css"));
            link rel="stylesheet" href=(statics.file("cpe/editorial.css"));
            link rel="stylesheet" href=(statics.file("cpe/record.css"));
            link rel="stylesheet" href=(statics.file("cpe/grid.css"));
            // The plate column + .pg pager
            // primitive. Sixth file; see
            // dsp-incubator cpe/docs/adr/0005-css-baseline.md.
            link rel="stylesheet" href=(statics.file("cpe/media.css"));
            // Incunabula's per-project components (the timeline, the
            // showcase figures). Seventh file, after media.css.
            link rel="stylesheet" href=(statics.file("cpe/incunabula.css"));
            // DataStar v1.0.2 — ES module (defer-by-default; no async).
            script type="module" src=(statics.file("datastar.js")) {}
            // Registers the `<image-viewer>`
            // custom element globally. Emitted unconditionally (not gated on
            // whether the page actually mounts a viewer) so the design-system
            // contract stays in this one function — the module itself is
            // tiny and the heavy OpenSeadragon bundle stays lazy-loaded by
            // the element only on pages that actually contain one.
            (cpe_image_viewer::head_tag(&statics.file("image-viewer")))
            @if view_transitions {
                style { (VIEW_TRANSITION_STYLE) }
            }
        }
    }
}

/// IBM Plex Mono, the fixed meta face (`--font-meta` in `tokens.css`).
const PLEX_MONO_FAMILY: &str = "IBM+Plex+Mono:wght@400;500;600";

/// The Google Fonts stylesheet URL for [`document_head`]'s `typeface`: the
/// face's own families, then Lora (project pages only, for `.dasch-link`),
/// then Plex Mono, each family once.
fn google_fonts_href(typeface: Option<Typeface>) -> String {
    let mut families: Vec<&str> = typeface.unwrap_or_default().font_families().to_vec();
    if typeface.is_some() && !families.contains(&cpe_core::theme::LORA_FAMILY) {
        families.push(cpe_core::theme::LORA_FAMILY);
    }
    families.push(PLEX_MONO_FAMILY);
    let query: Vec<String> = families.iter().map(|f| format!("family={f}")).collect();
    format!("https://fonts.googleapis.com/css2?{}&display=swap", query.join("&"))
}

/// The reader pages' cross-document View Transition opt-in,
/// emitted by [`document_head`] when `view_transitions` is `true`.
const VIEW_TRANSITION_STYLE: &str =
    "@media (prefers-reduced-motion: no-preference) { @view-transition { navigation: auto; } }";

/// Compose the full `<title>` string for a page.
///
/// Derivation order:
/// 1. First `resource-detail` body component with a positionally-resolved resource →
///    `resolve_title_str` (the same string the h1 / breadcrumb uses). Else (rung 1b), when the
///    first `resource-detail` body component's slot is `Detail(None)` (an unknown id), the
///    not-found body's own `h1` (`resource_detail::not_found_heading`), so the 404 tab names what
///    is missing.
/// 2. Else first `narrative` body component → `page_title_heading` of the language-resolved
///    Markdown content (same variant-resolution chain as `render_narrative`). That helper prefers
///    the `<h1>` over an earlier lower-level heading, so an editorial narrative opening with a
///    leading `###` kicker (rendered as `p.eyebrow.editorial__kicker`, a label — see
///    `component::editorial`) contributes its real title, not the label.
/// 3. Else no page title → return the site title alone.
///
/// The site title is `site.title` resolved chrome-id-or-literal for `lang`
/// ([`site_title`]).
///
/// `ResourceList` and `CompoundViewer` body components do **not** contribute a
/// page title; they are skipped by the `find_map`. The page title falls through
/// to the narrative-heading path or `site_title` alone.
///
/// Dedupe: when the derived page title equals `site_title` (exact match,
/// trimmed, case-sensitive) the result is `site_title` alone so the home page
/// (whose first heading *is* the site title) doesn't show "Site — Site".
/// Any other value (including near-misses and translated headings) produces
/// `"Site — Page title"` (em-dash with spaces).
/// Return the resolved title of the first `ResourceDetail` body component
/// with a hydrated resource, if any.
///
/// The shared first rung of both `compose_page_title` (the `<title>` text)
/// and `page_heading` (the visually-hidden `<h1>` text) — factored out so the
/// two derivations can never diverge on this rung (a divergent parallel
/// fallback chain would be a smell here).
fn resolved_resource_title(page: &cpe_core::ir::Page, components: &[ComponentData], lang: &Lang) -> Option<String> {
    page.body
        .iter()
        .zip(components.iter().chain(std::iter::repeat(&ComponentData::Other)))
        .find_map(|(c, slot)| {
            if let Component::ResourceDetail { title_property, .. } = c {
                let resource = match slot {
                    ComponentData::Detail(Some(r)) => r,
                    _ => return None,
                };
                Some(values::resolve_title_str(resource, title_property.as_deref(), lang).to_owned())
            } else {
                None
            }
        })
}

/// The not-found `h1` text when the page's FIRST `ResourceDetail` body component has
/// an unresolved slot (`Detail(None)` — the unknown-id 404 body), else
/// `None`. `compose_page_title`'s rung 1b; the heading comes from the same
/// `resource_detail::not_found_heading` the not-found body renders.
fn not_found_resource_heading(
    site: &SiteConfig,
    page: &cpe_core::ir::Page,
    components: &[ComponentData],
    lang: &Lang,
) -> Option<String> {
    page.body
        .iter()
        .zip(components.iter().chain(std::iter::repeat(&ComponentData::Other)))
        .find_map(|(c, slot)| match c {
            Component::ResourceDetail { class, .. } => Some((class, slot)),
            _ => None,
        })
        .and_then(|(class, slot)| {
            matches!(slot, ComponentData::Detail(None))
                .then(|| resource_detail::not_found_heading(class.as_deref(), &site.class_labels, lang))
        })
}

/// The first `Narrative` component's language-resolved page title, or
/// `None` when the page has no `Narrative` component or that narrative has
/// no heading. Shares the exact 3-step fallback chain `render_narrative`
/// uses (requested variant → default-language variant → base file) via
/// `narrative::resolve_narrative_content`, so title derivation and body
/// rendering can never diverge. One function so
/// `compose_page_title`'s rung 2 and the narrative crumb-line's page title
/// (`render_page`'s breadcrumb wiring) resolve identically — never
/// re-derive this chain separately.
///
/// Uses `util::page_title_heading` (the `<h1>`-preferring helper), NOT the
/// level-blind `first_heading`: an editorial narrative may open with a
/// leading `###` **kicker**, which `component::editorial` renders as
/// `p.eyebrow.editorial__kicker` — a label, not the page's title. Because
/// this one fn feeds both the `<title>` and the narrative breadcrumb's
/// page-title segment, the level-blind helper would put the kicker in both.
fn resolved_narrative_component_title(
    page: &cpe_core::ir::Page,
    narrative: &NarrativeStore,
    lang: &Lang,
    default_lang: &Lang,
) -> Option<String> {
    page.body.iter().find_map(|c| {
        if let Component::Narrative { file, .. } = c {
            narrative::resolve_narrative_content(file, lang, default_lang, narrative).and_then(page_title_heading)
        } else {
            None
        }
    })
}

fn compose_page_title(
    site: &SiteConfig,
    page: &cpe_core::ir::Page,
    narrative: &NarrativeStore,
    lang: &Lang,
    default_lang: &Lang,
    components: &[ComponentData],
) -> String {
    let site_title = self::site_title(site, lang);

    // 1. resource-detail component with a resolved resource
    let resource_title: Option<String> = resolved_resource_title(page, components, lang)
        // 1b. the first resource-detail is unresolved → its h1
        .or_else(|| not_found_resource_heading(site, page, components, lang));

    if let Some(rt) = resource_title {
        let page_title = rt.trim();
        if page_title == site_title.trim() {
            return site_title.to_owned();
        }
        return format!("{site_title} \u{2014} {page_title}");
    }

    // 2. narrative component — first heading of language-resolved content
    let narrative_title: Option<String> = resolved_narrative_component_title(page, narrative, lang, default_lang);

    if let Some(nt) = narrative_title {
        let page_title = nt.trim();
        if page_title == site_title.trim() {
            return site_title.to_owned();
        }
        return format!("{site_title} \u{2014} {page_title}");
    }

    // 3. No page title
    site_title.to_owned()
}

/// Whether `page` already renders its own visible `<h1>` without any
/// injection from `render_page`.
///
/// - A `ResourceDetail` component always emits `h1.record__title` — both the resolved-resource
///   branch (`resource_detail::render_resource_detail`) and the not-found branch
///   (`resource_detail::render_not_found`) carry one. Its presence in `page.body` is therefore
///   sufficient regardless of whether the resource actually resolved.
/// - A `Narrative` component's Markdown passes through `editorial::render_editorial`, which emits
///   any source heading as a literal `<hN>` tag — so a visible `<h1>` exists iff the
///   language-resolved content has a level-1 heading **anywhere**
///   (`cpe_core::util::narrative_contains_h1`), not merely as its first heading. A narrative that
///   renders a visible h1 that isn't first (e.g. `"## intro\n\n# title"`) would otherwise be
///   misclassified as headless, injecting a hidden h1 IN ADDITION to the real one — rare, but CPE
///   hosts arbitrary project configs. `page_title_heading` (used below, by `page_heading`, for a
///   *different* decision — which heading's TEXT to reuse) prefers the h1's text but still answers
///   `Some` for an h1-less document, so it cannot answer "is there an h1 anywhere" either; hence
///   this dedicated boolean helper. All three heading helpers share one traversal in `util`
///   (`headings`).
/// - `ResourceList`, `CompoundViewer`, and `FacetPanel` never contribute a page-level `<h1>` (their
///   own internal headings are `h2` or lower).
fn page_has_visible_h1(
    page: &cpe_core::ir::Page,
    narrative: &NarrativeStore,
    lang: &Lang,
    default_lang: &Lang,
) -> bool {
    page.body.iter().any(|c| match c {
        Component::ResourceDetail { .. } => true,
        Component::Narrative { file, .. } => narrative::resolve_narrative_content(file, lang, default_lang, narrative)
            .is_some_and(cpe_core::util::narrative_contains_h1),
        _ => false,
    })
}

/// Derive the text for a **visually-hidden** `<h1>` injected as the first
/// child of `main.shell__main` on a page with no visible heading of its own
/// (see [`page_has_visible_h1`]).
///
/// Derivation order — kept in the same rung order as `compose_page_title`
/// (sharing [`resolved_resource_title`] for rung 1) so the two chains cannot
/// diverge, with one addition: `nav_label` is tried *before* the narrative
/// heading, so a headless page never surfaces a lower-level heading as its
/// hidden-h1 text when a nav label is available (avoiding a hidden-h1 /
/// visible-h2 duplication — e.g. a browse page's facet-group heading):
///
/// 1. First resolved `ResourceDetail` title (`resolved_resource_title`) — in practice unreachable
///    from `render_page`'s call site (a page with a `ResourceDetail` always has a visible h1
///    already, per `page_has_visible_h1`); kept only so this chain shares its first rung with
///    `compose_page_title` rather than forking a divergent one.
/// 2. The current route's nav label (`nav_label`), when `page.route` matches a declared `site.nav`
///    item — the same match `shell_header` uses for `aria-current`. Yields e.g. "Characters" for
///    `/characters`.
/// 3. The first narrative component's page-title heading (`page_title_heading` — the `<h1>` when
///    there is one, else the first heading of any level), language-resolved the same way
///    `compose_page_title` resolves it.
/// 4. `site.title` (resolved via [`site_title`]) — the final fallback.
fn page_heading(
    site: &SiteConfig,
    page: &cpe_core::ir::Page,
    narrative: &NarrativeStore,
    lang: &Lang,
    default_lang: &Lang,
    components: &[ComponentData],
) -> String {
    if let Some(title) = resolved_resource_title(page, components, lang) {
        return title;
    }

    if let Some(item) = site.nav.iter().find(|item| item.route == page.route) {
        return nav_label(site, item, lang).to_owned();
    }

    // Rung 3: the narrative's page-title heading, via `page_title_heading`
    // (the same helper `resolved_narrative_component_title` uses, so the two
    // chains cannot drift on which heading counts as the title — a leading
    // `###` kicker never does). Behaviour-neutral for a kicker-bearing page,
    // which always has an h1 and is therefore never headless.
    // Known limit: this rung is only
    // reached by a headless narrative page whose route has NO matching
    // `nav` entry (rung 2 above already returns for every page that does).
    // For such an off-nav page, if the first heading happens to be an `h2`
    // (not `h1` — `page_has_visible_h1` already excluded true h1-having
    // pages from injection), the injected hidden h1's TEXT duplicates the
    // visible `h2` text further down the page.
    let narrative_title: Option<String> = page.body.iter().find_map(|c| {
        if let Component::Narrative { file, .. } = c {
            narrative::resolve_narrative_content(file, lang, default_lang, narrative).and_then(page_title_heading)
        } else {
            None
        }
    });
    if let Some(nt) = narrative_title {
        return nt;
    }

    site_title(site, lang).to_owned()
}

/// Render a complete HTML document for `page` using `site` chrome, `narrative`
/// content, and the request-scoped `ctx`.
///
/// ## `components` — single carrier slice
///
/// `components` is aligned 1:1 with `page.body`. Each element carries the
/// pre-fetched data for the component at that position:
/// - `ComponentData::Detail(opt)` — for `ResourceDetail` components.
/// - `ComponentData::List(page)` — for `ResourceList` components.
/// - `ComponentData::Compound(page)` — for `CompoundViewer` components.
/// - `ComponentData::Other` — for non-data components (narrative, …).
///
/// The `zip` naturally extends with `Other` for any slots beyond the slice.
/// Callers with no data-backed components pass `&[]`; the `zip` treats every
/// slot as `Other`.
///
/// `render_component` receives the pre-selected `&ComponentData` for its index —
/// it does NOT receive whole slices and does not re-zip.
///
/// `data_arks` is the project's boot-built ARK map ([`crate::project::Project::data_arks`]);
/// only the record and narrative crumb lines read it.
///
/// Returns a `maud::Markup` value (the full document, including `<!DOCTYPE html>`).
/// The caller is responsible for supplying `IntoResponse`.
/// HTTP status is NOT encoded here — the host's handler maps its own `Result` to
/// 200/404/500.
pub fn render_page(
    site: &SiteConfig,
    page: &cpe_core::ir::Page,
    narrative: &NarrativeStore,
    ctx: &RenderContext<'_>,
    components: &[ComponentData],
    data_arks: &DataArks,
) -> Markup {
    // The site's default-language `Lang`, resolved once per request.
    // `lang::resolve` with an empty preference slice always returns
    // `Lang(site.default_language.clone())` (see
    // `cpe_core::lang`). Shared by
    // `compose_page_title` (title) and `render_body` → `render_narrative`
    // (body) so both stages resolve the narrative fallback chain identically.
    let default_lang = cpe_core::lang::resolve(&[], site);

    // Compose the per-page <title> once before the html! block.
    // `compose_page_title` derives a page title from the first resolved
    // resource-detail or first narrative heading, then dedupes it against
    // site.title (equal → site.title alone; different → "Site — Page").
    let page_title = compose_page_title(site, page, narrative, &ctx.lang, &default_lang, components);

    // A11y: every page must have exactly one <h1>. When the page has no
    // visible heading of its own (browse/list pages, a narrative whose first
    // heading isn't level 1), inject a visually-hidden one as the first child
    // of main.shell__main — see `page_has_visible_h1` / `page_heading`.
    let hidden_heading: Option<String> = (!page_has_visible_h1(page, narrative, &ctx.lang, &default_lang))
        .then(|| page_heading(site, page, narrative, &ctx.lang, &default_lang, components));

    // Derive the breadcrumb (crumb-line + share control) for this page. A
    // `ResourceDetail` component gets the ordinary 3-crumb trail (resolved
    // resource) or the not-found variant (unresolved id; the class crumb appears
    // ONLY on the 404 page — a resolved record page never gains a class
    // crumb here). Otherwise, a page with a `ResourceList` component (a
    // declared `id`) gets the browse crumb-line variant
    // (`render_browse_breadcrumb_for_page`) — `CPE › project ›
    // class-label`, with a state-bearing accessible-name suffix. Otherwise,
    // a page with a `Narrative` component gets the narrative crumb-line
    // variant — `CPE › project › page-title`, or
    // `CPE › project` alone on the project home page.
    //
    // **Browse is checked BEFORE narrative.** The host's `_facets`/`_sort`/`_search`
    // fragment handlers unconditionally patch `#crumb-{list_id}` (the browse
    // variant's id) — they have no narrative branch at all. A
    // page combining a `ResourceList` (with declared `id`) and a `Narrative`
    // component must therefore render the SAME crumb id those handlers
    // patch, or a refinement response would target an id the full-page
    // render never emitted. This function owns the
    // decision — the host sets no flag.
    let breadcrumb_markup: Option<Markup> =
        render_crumb_line_for_page(page, ctx, narrative, site, components, data_arks);

    // Only a reader page opts into cross-document View
    // Transitions (see `document_head`).
    let view_transitions = page.body.iter().any(|c| matches!(c, Component::CompoundViewer { .. }));

    // Rendered before the shell: whether the page has a rail is only known
    // once its first narrative is drawn, and `<html>` carries the answer.
    let (body, layout) = render_body(page, ctx, narrative, components, &default_lang, site);

    let inner = html! {
        ({
            document_head(
                &page_title,
                view_transitions,
                Some(site.theme.typeface),
                &ctx.urls().static_dir(),
            )
        })
        body {
            // A11y: skip-to-content link, first child of <body> on every
            // full-page renderer (`site_page` and the project's not-found page mirror it).
            (skip_link(&ctx.lang))
            div.shell {
                (shell_header(site, ctx, &page.route))
                main.shell__main id="main" tabindex="-1" {
                    // A11y: visually-hidden <h1> for pages with no visible
                    // heading of their own — always the first child.
                    @if let Some(ref h) = hidden_heading {
                        h1.visually-hidden { (h) }
                    }
                    // Breadcrumb — rendered above page content on resource-detail,
                    // narrative, and browse pages:
                    // a sibling above `render_body`'s output, so on a browse
                    // page it sits above the `.browse` grid rather than becoming a
                    // 5th flat grid child.
                    @if let Some(ref bc) = breadcrumb_markup { (bc) }
                    (body)
                }
                (shell_footer(site, &ctx.lang, &ctx.urls().static_dir()))
            }
        }
    };
    project_html(&ctx.lang, ctx.appearance, site, layout, inner)
}

/// The document shell of a project's pages: the doctype and `<html>`, with
/// the language, the appearance, the project's `theme {}` knob, its text
/// alignment when that is not the default (`data-align`) and, on a page with
/// a narrative, its layout (`data-narr`, `data-narr-side`), around `inner`
/// (the head and the body). `render_page` and the project's not-found page
/// (`render_not_found_project`) both call it, so the two project shells
/// carry the same attributes. A page without a project writes its own
/// `<html>` ([`site_page`]).
///
/// No attribute written here may be named like a DataStar plugin
/// (`data-text`, `data-show`, …): DataStar would evaluate its value.
pub(crate) fn project_html(
    lang: &Lang,
    appearance: Appearance,
    site: &SiteConfig,
    layout: Option<NarrativeLayout>,
    inner: Markup,
) -> Markup {
    let align = (site.text_align != TextAlign::default()).then(|| site.text_align.as_data_attr());
    html! {
        (DOCTYPE)
        html
            lang=(lang.as_str())
            data-appearance=[(appearance == Appearance::Dark).then_some("dark")]
            data-direction="edition"
            data-accent=(site.theme.accent.as_data_attr())
            data-bg=(site.theme.background.as_data_attr())
            data-pick=(site.theme.typeface.as_data_attr())
            data-narr=[layout.map(|l| l.companion.as_data_attr())]
            data-narr-side=[layout.map(|l| l.side.as_data_attr())]
            data-align=[align]
        { (inner) }
    }
}

/// Render the project's not-found page: its own bespoke 404 for an
/// unmatched path under a resolved project prefix.
///
/// Bypasses `render_page` entirely — this is an independent full-page
/// renderer, so it re-implements the skip link / `id="main"` /
/// single-visible-`<h1>` discipline the others share.
/// The full project chrome and the project's
/// configured `theme {}` apply, through the same `shell_header` every project
/// page uses. No `aria-current` on any nav item falls out of passing a
/// `current_route` that matches no configured route (`""`). The body is
/// `resource_detail::render_not_found_project_body`.
///
/// `ctx.current_path` MUST be pinned to `/<project_id>/` (project home, no
/// query) by the caller — never the unmatched request path — so the
/// unmatched path is reflected nowhere, not even in the language-switcher or
/// appearance-toggle hrefs `shell_header` builds from `ctx.current_path`/
/// `ctx.current_query`. The consequence (deliberate, per the no-reflection
/// rule) is that the switcher/toggle navigate to the project home rather
/// than re-rendering this 404.
pub fn render_not_found_project(site: &SiteConfig, ctx: &RenderContext<'_>) -> Markup {
    use cpe_components::chrome::ChromeMsg;
    let lang = &ctx.lang;
    let title = format!("{} — {}", site_title(site, lang), ChromeMsg::NfPathTitle.text(lang));
    let inner = html! {
        ({
            document_head(
                &title,
                false,
                Some(site.theme.typeface),
                &ctx.urls().static_dir(),
            )
        })
        body {
            (skip_link(lang))
            div.shell {
                (shell_header(site, ctx, ""))
                main.shell__main id="main" tabindex="-1" {
                    ({
                        resource_detail::render_not_found_project_body(
                            site,
                            ctx.urls(),
                            lang,
                        )
                    })
                }
                (shell_footer(site, lang, &ctx.urls().static_dir()))
            }
        }
    };
    project_html(lang, ctx.appearance, site, None, inner)
}

/// Render CPE's own not-found page: the 404 for a request whose first
/// path segment does not match any loaded project id.
///
/// Same `.notfound` anatomy as the project's (shared
/// `resource_detail::render_notfound_shell`), dressed by bare chrome rather
/// than a project's ([`site_page`]). No project nav, no "or browse" line
/// (there is no project whose pages could be offered); its one exit is the
/// overview ([`render_site_index`]). `lang` is one of the built-in languages
/// (`cpe_core::lang::resolve_builtin`): there is no project to declare any.
pub fn render_not_found_site(base: &cpe_core::url::Base, lang: &Lang, appearance: Appearance) -> Markup {
    use cpe_components::chrome::ChromeMsg;
    let home = base.home();
    let body = resource_detail::render_notfound_shell(
        false,
        ChromeMsg::NfCode.text(lang),
        ChromeMsg::NfSiteTitle.text(lang),
        Some(ChromeMsg::NfSiteMsg.text(lang)),
        Some((ChromeMsg::NfAllProjects.text(lang), &home)),
        None,
    );
    site_page(base, lang, appearance, body)
}

/// One project on the overview ([`render_site_index`]), its texts already in
/// the language they are shown in ([`crate::project::Project::teaser`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTeaser<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub eyebrow: Option<&'a str>,
    pub image_credit: Option<&'a str>,
    /// The project's own `theme.accent`, as its `data-accent` value.
    pub accent: &'static str,
    /// `site.cover-image`: a path in the project folder.
    pub cover_image: Option<&'a str>,
}

/// Render CPE's overview: the page at its base, one teaser per served
/// project, each linking to the project's start page.
///
/// Bare chrome ([`site_page`]) and the degenerate crumb line (`CPE`, current,
/// unlinked) with the share control, whose link is `public_origin` and the
/// base. The page has no visible heading of its own (the brand mark is a
/// wordmark), so its one `<h1>` is visually hidden. A teaser's eyebrow and
/// credit lines are rendered empty when the project has none: titles align
/// across a row. `lang` is one of the built-in languages, as for
/// [`render_not_found_site`].
pub fn render_site_index(
    base: &cpe_core::url::Base,
    projects: &[ProjectTeaser<'_>],
    lang: &Lang,
    appearance: Appearance,
    public_origin: &str,
) -> Markup {
    use cpe_components::chrome::{index_count_text, ChromeMsg};
    let share_href = crate::share_link::compose_absolute_url(public_origin, &base.home(), "");
    let share = breadcrumb::ShareControl {
        id: "share-site-index",
        href: &share_href,
        accessible_name_suffix: None,
        reactive_href_attr: None,
    };
    let main = html! {
        h1 class="visually-hidden" { (SITE_TITLE) }
        (breadcrumb::render_site_index_breadcrumb(lang, &share))
        div class="index-list-head" {
            h2 { (ChromeMsg::SiteIndexProjectsHeading.text(lang)) }
            span class="count" { (index_count_text(projects.len(), lang)) }
        }
        @if projects.is_empty() {
            p class="resource-list__empty" { (ChromeMsg::SiteIndexNoProjects.text(lang)) }
        } @else {
            ul class="project-list" {
                @for project in projects {
                    @let urls = base.project(project.id);
                    @let home = urls.home();
                    li {
                        a class="teaser" data-accent=(project.accent) href=(home) {
                            div class="teaser__plate" {
                                @if let Some(cover) = project.cover_image {
                                    img class="teaser__img" src=(urls.asset(cover)) alt="";
                                } @else {
                                    div class="teaser__ghost" aria-hidden="true" {}
                                }
                                p class="teaser__credit" {
                                    @if let Some(credit) = project.image_credit { (credit) }
                                }
                            }
                            div class="teaser__body" {
                                p class="eyebrow teaser__eyebrow" {
                                    @if let Some(eyebrow) = project.eyebrow { (eyebrow) }
                                }
                                h3 class="teaser__title" { (project.title) }
                                @if let Some(description) = project.description {
                                    p class="teaser__desc" { (description) }
                                }
                            }
                            div class="teaser__foot" {
                                span class="teaser__enter" {
                                    (ChromeMsg::SiteIndexEnterCta.text(lang))
                                    " "
                                    span aria-hidden="true" { "→" }
                                }
                                span class="teaser__path" { (home) }
                            }
                        }
                    }
                }
            }
        }
    };
    site_page(base, lang, appearance, main)
}

/// The `<title>` of a page without a project, and the overview's heading.
const SITE_TITLE: &str = "CPE — Configurable Presentation Environment";

/// A full page without a project: bare chrome around `main`, with the
/// product-default theme knob (ochre · tint · newsreader).
///
/// The header and footer are written out here: `shell_header` and
/// `shell_footer` need a project's `SiteConfig`, so a change to their shared
/// markup (the brand mark, the appearance toggle, the DaSCH logo) is repeated
/// here by hand. The header is the brand mark and the appearance toggle only.
/// The toggle links to the overview, which is the page that takes
/// `?appearance=`.
fn site_page(base: &cpe_core::url::Base, lang: &Lang, appearance: Appearance, main: Markup) -> Markup {
    use cpe_components::chrome::ChromeMsg;
    let home = base.home();
    let statics = &base.static_dir();

    let data_accent = cpe_core::theme::Accent::Ochre.as_data_attr();
    let data_bg = cpe_core::theme::Background::Tint.as_data_attr();
    let data_pick = cpe_core::theme::Typeface::Newsreader.as_data_attr();

    let (toggle_target, toggle_label) = match appearance {
        Appearance::Dark => ("light", ChromeMsg::ThemeToggleToLight.text(lang)),
        Appearance::Light => ("dark", ChromeMsg::ThemeToggleToDark.text(lang)),
    };

    html! {
        (DOCTYPE)
        html
            lang=(lang.as_str())
            data-appearance=[(appearance == Appearance::Dark).then_some("dark")]
            data-direction="edition"
            data-accent=(data_accent)
            data-bg=(data_bg)
            data-pick=(data_pick)
        {
            (document_head(SITE_TITLE, false, None, statics))
            body {
                a class="skip-link" href="#main" { (ChromeMsg::SkipToContent.text(lang)) }
                div class="shell" {
                    header class="shell__header" {
                        div class="header__inner" {
                            a   class="brand"
                                href=(home)
                                aria-label=(ChromeMsg::SiteIndexAriaLabel.text(lang))
                            {
                                span class="brand__mark" { "CPE" }
                            }
                            div class="header__controls header__controls--push" {
                                a   class="theme-toggle"
                                    href=(format!("{home}?appearance={toggle_target}"))
                                    aria-label=(toggle_label)
                                    title=(toggle_label)
                                {
                                    span class="theme-toggle__icon" aria-hidden="true" {}
                                }
                            }
                        }
                    }
                    main class="shell__main" id="main" tabindex="-1" { (main) }
                    footer class="shell__footer shell__footer--index" {
                        div class="footer-index__inner" {
                            div class="footer-index__product" {
                                span class="brand__mark footer__mark" { "CPE" }
                                p class="footer-index__tagline" {
                                    (ChromeMsg::SiteIndexTagline.text(lang))
                                }
                                p class="footer-index__desc" {
                                    (ChromeMsg::SiteIndexPocDesc.text(lang))
                                }
                            }
                            div class="footer-index__org" {
                                a   class="dasch-logo"
                                    href="https://www.dasch.swiss/"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    aria-label=(ChromeMsg::DaschAriaLabel.text(lang))
                                {
                                    img class="dasch-logo__img dasch-logo__img--light"
                                        src=(statics.file("cpe/dasch-logo-colour.svg"))
                                        alt=(ChromeMsg::DaschAriaLabel.text(lang));
                                    img class="dasch-logo__img dasch-logo__img--dark"
                                        src=(statics.file("cpe/dasch-logo-white.svg"))
                                        alt=(ChromeMsg::DaschAriaLabel.text(lang));
                                }
                                p class="footer-index__orgname" {
                                    "Swiss National Data and Service Center for the Humanities"
                                }
                                ul class="footer-index__links" {
                                    li {
                                        a   href="https://www.dasch.swiss/"
                                            target="_blank"
                                            rel="noopener noreferrer"
                                        { (ChromeMsg::SiteIndexDaschLinkLabel.text(lang)) }
                                    }
                                    li {
                                        a   href="https://app.dasch.swiss/"
                                            target="_blank"
                                            rel="noopener noreferrer"
                                        { "DSP APP" }
                                    }
                                }
                            }
                        }
                        div class="footer-index__base" {
                            p class="footer-index__rights" {
                                "© DaSCH — Swiss National Data and Service Center for the Humanities"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The output of [`render_compound_fragment`]: everything the `_compound`
/// DataStar fragment handler (the host's)
/// needs to build its SSE response — the re-rendered `#compound-viewer`
/// markup, the recomposed crumb-line markup + its DOM patch-target id (when
/// the page has a crumb line at all — see `crumb`'s own doc comment), the
/// re-rendered header controls, and
/// this page's derived `<title>` text + canonical URL — anchor-free, since
/// `share_href_for_reader` is unconditionally
/// anchor-free.
pub struct CompoundFragmentOutput {
    /// The re-rendered `<article id="compound-viewer">` markup — built via
    /// the SAME `render_component` call `render_page`'s own `CompoundViewer`
    /// arm makes, so its root open tag is unconditionally identical between
    /// a full-page render and a fragment response.
    pub viewer: Markup,
    /// The recomposed crumb-line markup for this page (paired with
    /// `crumb_id`, `None` together), via the SAME
    /// `render_record_breadcrumb_for_page` / `render_browse_breadcrumb_for_page`
    /// / `render_narrative_breadcrumb_for_page` precedence chain `render_page`
    /// uses. `None` when none of the three match — e.g. a compound-only page
    /// with no sibling `ResourceDetail`/`ResourceList`/`Narrative` component
    /// (uncommon; every real project pairs a `CompoundViewer` with a
    /// `ResourceDetail` or `Narrative`, but a minimal test fixture can lack
    /// both), mirroring `render_page`'s own `Option<Markup>` breadcrumb slot
    /// — never a hard requirement.
    pub crumb: Option<Markup>,
    /// The crumb line's own DOM id (`crumb-<share-id>`, via
    /// `breadcrumb::crumb_element_id`) — the `_compound` handler's
    /// `PatchElements` target for the crumb, when `crumb` is `Some`.
    pub crumb_id: Option<String>,
    /// This page's derived `<title>` text — via the SAME `compose_page_title`
    /// call `render_page` makes, so title and canonical URL can never drift
    /// from what a full-page load would show.
    pub title: String,
    /// The canonical URL for the current compound-reader state — via the
    /// SAME `share_href_for_reader`/`compose_canonical_query` composition the
    /// crumb-line builders use internally for their own Copy-link control.
    pub canonical_url: String,
    /// The reader's annotation patch target (`#` +
    /// [`annotation::ANNOTATIONS_TARGET_ID`]), via the SAME renderer the full page uses.
    /// `Some` exactly when the full page renders it — a book composition
    /// whose reader has `annotations=#true` — including on a page without a
    /// register, where it is the empty target.
    pub annotations: Option<Markup>,
    /// The header controls (`#` + [`HEADER_CONTROLS_ID`]), rendered by
    /// `shell_header_controls` with the same flags `render_page`'s
    /// `shell_header` gets (`PROJECT_PAGE_*`), so their
    /// language and appearance hrefs carry the swap's own `?page=`. Always
    /// present: every page that has a reader has the shell header.
    pub header_controls: Markup,
}

/// Render the `_compound` fragment-swap payload for `page` at its current
/// member (from `components`): the re-rendered viewer, the recomposed crumb
/// line, the header controls (whose language and appearance hrefs carry the
/// new `?page=` from `ctx.current_query`; `ctx.appearance` must be the
/// request's resolved appearance, as on a full load), the derived title, and
/// the canonical URL — all via the SAME
/// internal calls `render_page` makes for a compound-viewer page, so a
/// fragment response can never drift from what a full-page load of the same
/// state would show. This is the ONE `pub` entry point for the host's
/// `_compound` handler, rather than making `CompoundViewerCfg`,
/// `render_compound_viewer`, `member_href_for`, both breadcrumb variants,
/// `compose_page_title` and `js_string_escape` individually `pub`.
///
/// Returns `None` when `page` has no `CompoundViewer` component, or when
/// `components` has no `ComponentData::Compound` payload at that component's
/// slot — the caller (the host's `_compound` handler) treats either
/// as "nothing to render" and returns a bare non-200 status, per the other
/// fragment handlers' convention.
///
/// `data_arks` must be the same map `render_page` receives, so the swapped
/// crumb line matches a full-page load.
///
/// `components` must be built the same way `render_page`'s caller builds it
/// — the FULL, aligned `&[ComponentData]` slice for every component on the
/// page (`PageData.components` from `Project::page_data`), not just the `CompoundViewer`
/// slot: the crumb-line selection chain and `compose_page_title` both need
/// to see a sibling `ResourceDetail`/`Narrative` slot's data too (e.g.
/// Incunabula's book reader also carries a `Detail` slot for the book's own
/// resource, which the record-crumb arm reads to decide "record crumb, not
/// narrative crumb").
pub fn render_compound_fragment(
    site: &SiteConfig,
    page: &cpe_core::ir::Page,
    narrative: &NarrativeStore,
    ctx: &RenderContext<'_>,
    components: &[ComponentData],
    data_arks: &DataArks,
) -> Option<CompoundFragmentOutput> {
    let idx = page.body.iter().position(|c| matches!(c, Component::CompoundViewer { .. }))?;
    let data = components.get(idx)?;
    if !matches!(data, ComponentData::Compound(_)) {
        return None;
    }

    let default_lang = cpe_core::lang::resolve(&[], site);
    let component = &page.body[idx];
    let viewer = render_compound_component(component, ctx, data, site, false);

    // Same precedence `render_page` uses (record → browse → narrative); the
    // browse arm is included for symmetry but is structurally unreachable
    // here — a compound-viewer page never also carries a paginated
    // `resource-list` (boot hard-fail, see `DiagnosticKind` for the
    // compound/list-pagination conflict). `None` when a page has none of the
    // three sibling components (a compound-viewer with no `ResourceDetail`/
    // `ResourceList`/`Narrative`) — mirrors `render_page`'s own
    // `Option<Markup>` breadcrumb slot; not a hard requirement here either.
    let crumb = render_crumb_line_for_page(page, ctx, narrative, site, components, data_arks);

    // The crumb-line builders above always derive `share_id` as
    // `format!("share-{}", ctx.project_id)` — mirrored here (not returned by
    // either builder) so the handler knows which DOM id to patch.
    let crumb_id = crumb
        .is_some()
        .then(|| breadcrumb::crumb_element_id(&format!("share-{}", ctx.project_id)));

    let title = compose_page_title(site, page, narrative, &ctx.lang, &default_lang, components);

    // Mirrors the crumb-line builders' own `share_href` composition (they
    // don't expose it) — record surface when the page has a
    // `ResourceDetail` (Incunabula's navigator-keyed book page), else the
    // narrative/reader surface (a narrative page with a reader).
    let has_resource_detail = page.body.iter().any(|c| matches!(c, Component::ResourceDetail { .. }));
    let surface = if has_resource_detail {
        crate::share_link::ViewSurface::Record
    } else {
        crate::share_link::ViewSurface::Reader
    };
    let reader_member = resolved_reader_member(components);
    let share_query = crate::share_link::compose_canonical_query(ctx.current_query, &[], surface, reader_member, &[]);
    let canonical_url = share_href_for_reader(ctx.public_origin, ctx.current_path, &share_query, reader_member);

    Some(CompoundFragmentOutput {
        viewer,
        crumb,
        crumb_id,
        title,
        canonical_url,
        annotations: render_reader_annotations(page, ctx, components, site),
        header_controls: shell_header_controls(site, ctx),
    })
}

/// The page's crumb line: the record, browse or narrative variant, the first
/// that matches in that order. `render_page` and `render_compound_fragment`
/// both call this one chain, so a full page and a `_compound` swap cannot pick
/// a different variant or a different dataset panel.
fn render_crumb_line_for_page(
    page: &cpe_core::ir::Page,
    ctx: &RenderContext<'_>,
    narrative: &NarrativeStore,
    site: &SiteConfig,
    components: &[ComponentData],
    data_arks: &DataArks,
) -> Option<Markup> {
    render_record_breadcrumb_for_page(page, ctx, components, site, data_arks)
        .or_else(|| render_browse_breadcrumb_for_page(page, ctx, components, site))
        .or_else(|| render_narrative_breadcrumb_for_page(page, ctx, narrative, site, components, data_arks))
}

/// Resource-detail surfaces: the record crumb line (`CPE › project ›
/// [class ›] resource-title`) plus its Copy-link control, or the
/// not-found crumb when the page's `resource-detail` id did not resolve.
/// Returns `None` when the page has no `ResourceDetail` component.
///
/// The FIRST arm of [`render_crumb_line_for_page`]'s chain (before browse
/// and narrative); never duplicate this composition.
///
/// On such a record-with-compound page the resolved reader member
/// ([`resolved_reader_member`]) rides on the `Record` composition — both
/// the static `href` and every entry of the reactive `$plate` URL list —
/// so a copied link reopens the same page of the book.
pub(crate) fn render_record_breadcrumb_for_page(
    page: &cpe_core::ir::Page,
    ctx: &RenderContext<'_>,
    components: &[ComponentData],
    site: &SiteConfig,
    data_arks: &DataArks,
) -> Option<Markup> {
    let reader_member = resolved_reader_member(components);
    page.body
        .iter()
        .zip(components.iter().chain(std::iter::repeat(&ComponentData::Other)))
        .find_map(|(c, slot)| {
            match c {
                Component::ResourceDetail { title_property, class, .. } => {
                    match first_resolved_resource(std::slice::from_ref(slot)) {
                        Some(resource) => {
                            let resource_title =
                                values::resolve_title_str(resource, title_property.as_deref(), &ctx.lang);
                            // The detail-page control's
                            // href is the load-time canonical URL for the
                            // current page (no facets on a detail page, hence
                            // the empty facet-property slice). The record `?img=`
                            // client-side tracking is the reactive href below, not this
                            // load-time href.
                            let share_query = crate::share_link::compose_canonical_query(
                                ctx.current_query,
                                &[],
                                crate::share_link::ViewSurface::Record,
                                reader_member,
                                &[],
                            );
                            let share_href =
                                share_href_for_reader(ctx.public_origin, ctx.current_path, &share_query, reader_member);
                            let share_id = format!("share-{}", ctx.project_id);
                            // When the record-viewer offers
                            // a `.pg` sequence pager (`media::has_sequence_pager`
                            // — the SAME gate `render_media_viewer` uses to
                            // decide whether `$plate` has anything to page
                            // through), the control's `href` additionally
                            // tracks `$plate` client-side via a reactive
                            // `data-attr` expression. The static `share_href`
                            // above (the load-time value) is untouched — it
                            // stays the no-JS/initial-load href; this is an
                            // ADDITIONAL directive on the same `<a>`. Zero/
                            // one-image records have no pager and so nothing to
                            // track — `reactive_href_js` stays `None` and the
                            // control keeps its plain static href.
                            let reactive_href_js: Option<String> =
                                media::has_sequence_pager(ctx.media_items).then(|| {
                                    // Canonical-by-construction (was: hand-spliced
                                    // `img_prefix` appended AFTER the static
                                    // remainder, e.g. `?lang=de&img=2` — wrong
                                    // canonical order, since `img` sorts before
                                    // `lang`). Precompute the FULL per-index URL
                                    // list server-side, once per plate index
                                    // (1-based, mirroring `media.rs`'s per-index
                                    // precomputation pattern, e.g.
                                    // `plate_hidden_attr`), each one composed via
                                    // the SAME `compose_canonical_query` +
                                    // `compose_absolute_url` the server itself
                                    // uses to serve that `?img=N` — so every entry
                                    // is byte-identical to what a fresh load of
                                    // that state would compose, order included.
                                    // `$plate` is 0-based; index `i` (0-based)
                                    // holds the URL for `img=i+1`. `img` is
                                    // stripped from the raw query before
                                    // recomposing so it can never leak through
                                    // twice (once from the raw query, once
                                    // injected per-index).
                                    let query_no_img: String = ctx
                                        .current_query
                                        .unwrap_or("")
                                        .split('&')
                                        .filter(|pair| !pair.is_empty() && pair.split('=').next() != Some("img"))
                                        .collect::<Vec<_>>()
                                        .join("&");
                                    let urls_js: Vec<String> = (1..=ctx.media_items.len())
                                        .map(|i| {
                                            let raw = if query_no_img.is_empty() {
                                                format!("img={i}")
                                            } else {
                                                format!("{query_no_img}&img={i}")
                                            };
                                            let query = crate::share_link::compose_canonical_query(
                                                Some(&raw),
                                                &[],
                                                crate::share_link::ViewSurface::Record,
                                                reader_member,
                                                &[],
                                            );
                                            let url = crate::share_link::compose_absolute_url(
                                                ctx.public_origin,
                                                ctx.current_path,
                                                &query,
                                            );
                                            // Every string interpolated into the JS
                                            // expression must be escaped — a URL can
                                            // in principle carry a `'` (e.g. via `q=`)
                                            // that would otherwise break out of the
                                            // single-quoted JS string literal.
                                            format!("'{}'", cpe_core::util::js_string_escape(&url))
                                        })
                                        .collect();
                                    // Object-literal `data-attr` idiom (mirrors
                                    // `media::plate_hidden_attr`) — `href` is a
                                    // plain HTML attribute, not `data-*`, so
                                    // writing it reactively does not hit the
                                    // `data-*` MutationObserver collision (see
                                    // `ShareControl::reactive_href_attr`'s doc
                                    // comment). Indexing a plain JS array literal
                                    // by `$plate` is simpler AND more obviously
                                    // correct than a hand-built ternary/string
                                    // concatenation — every value the expression
                                    // can ever produce is visible, pre-composed,
                                    // right here.
                                    format!("{{'href': [{}][$plate]}}", urls_js.join(","))
                                });
                            let share = breadcrumb::ShareControl {
                                id: &share_id,
                                href: &share_href,
                                accessible_name_suffix: None,
                                reactive_href_attr: reactive_href_js.as_deref(),
                            };
                            // Class crumb — a live link between the project and
                            // resource-title crumbs, present only when the
                            // resource's class has a registered browse route.
                            // `resolve_class_plural_capitalized` is used
                            // deliberately: it's the SAME resolved+capitalized
                            // noun `render_browse_breadcrumb_for_page` itself
                            // shows as the browse page's own current
                            // crumb/eyebrow, so the two surfaces read the same
                            // capitalized-plural class noun for the same class
                            // (e.g. "Characters", not the singular "Character":
                            // the browse crumb itself
                            // resolves the plural too, so the two agree).
                            let class_crumb: Option<(String, String)> = class.as_deref().and_then(|c| {
                                ctx.class_list_routes.get(c).map(|route| {
                                    let label = resolve_class_plural_capitalized(&site.class_labels, c, &ctx.lang);
                                    let href = ctx.urls().page(route);
                                    (label, href)
                                })
                            });
                            // No listing source: a record page has no figure
                            // blocks.
                            let panel = data_links::page_data_panel(
                                page,
                                components,
                                &site.class_labels,
                                &ctx.lang,
                                data_arks,
                                None,
                            );
                            Some(breadcrumb::render_breadcrumb(
                                ctx.urls(),
                                site_title(site, &ctx.lang),
                                class_crumb.as_ref().map(|(label, href)| (label.as_str(), href.as_str())),
                                resource_title,
                                &ctx.lang,
                                &share,
                                panel.as_ref(),
                            ))
                        }
                        None => {
                            // Unresolved id. A class crumb is only added
                            // when the page declares a `class` AND that class
                            // has a registered browse route (the advisory
                            // class→list-route map) — otherwise fall back to
                            // the project crumb alone (mirrors
                            // `render_not_found`'s own browse-all fallback rule).
                            let class_crumb: Option<(String, String)> = class.as_deref().and_then(|c| {
                                ctx.class_list_routes.get(c).map(|route| {
                                    let label = resolve_class_label(&site.class_labels, c, &ctx.lang);
                                    let href = ctx.urls().page(route);
                                    (label, href)
                                })
                            });
                            Some(breadcrumb::render_not_found_breadcrumb(
                                ctx.urls(),
                                site_title(site, &ctx.lang),
                                class_crumb.as_ref().map(|(label, href)| (label.as_str(), href.as_str())),
                                &ctx.lang,
                            ))
                        }
                    }
                }
                _ => None,
            }
        })
}

/// The resolved compound-reader member id for the Copy link:
/// the id of the member on screen, read from the page's
/// `ComponentData::Compound` (`members[index - 1]`), or `None` on member 1,
/// on an empty compound, or on a page with no compound-viewer. This is the
/// canonical form of whatever the request typed (`?page=<id|name|prefix>`
/// or a legacy `?member=N`) — the raw query cannot supply it.
fn resolved_reader_member(components: &[ComponentData]) -> Option<&str> {
    components.iter().find_map(|c| match c {
        ComponentData::Compound(cp) if cp.index > 1 => cp.members.get(cp.index as usize - 1).map(|m| m.id.as_str()),
        _ => None,
    })
}

/// Compose the Copy link href for a surface that may carry a resolved
/// compound-reader member.
///
/// **Unconditionally anchor-free**: no `#compound-viewer` may appear
/// anywhere in a copied link. Member paging is a URL-writing
/// `pushState`/`popstate` swap (not a same-document anchor scroll), so no
/// Copy-link caller needs the anchor to land correctly — the
/// resolved `page=<member-id>` query param alone is enough. `reader_member`
/// is kept as a parameter for call-site symmetry with
/// `compose_canonical_query` (which needs it to decide `page=`/`img=`
/// composition); this fn does not branch on it.
fn share_href_for_reader(origin: &str, path: &str, query: &str, _reader_member: Option<&str>) -> String {
    crate::share_link::compose_absolute_url(origin, path, query)
}

/// Static surfaces: a page with no
/// `ResourceDetail` component but WITH a `Narrative` component gets the
/// narrative crumb-line variant (`CPE › project › page-title`, or
/// `CPE › project` alone for the project home page).
///
/// **Invariant: a full-page
/// variant choice must match which fragment handlers patch which crumb id.**
/// `render_page`'s `.or_else` chain calls [`render_browse_breadcrumb_for_page`]
/// BEFORE this fn, so a page that has BOTH a `ResourceList` (declared `id`)
/// and a `Narrative` component renders the browse variant, never this one —
/// the host's `_facets`/`_sort`/`_search` fragment handlers unconditionally
/// patch `#crumb-{list_id}` (the browse variant's id) and have no narrative
/// branch at all, so this fn must never win that race:
/// it must stay the LOWER-precedence arm.
///
/// A narrative page paired with a `compound-viewer`
/// is the "reader" scope
/// surface — its href is composed via `ViewSurface::Reader`, which emits
/// the resolved member as `page=<member-id>` (omitted entirely on member 1)
/// and never `member=`. A plain narrative
/// page (no compound-viewer) carries no img/member/facet state at all;
/// `ViewSurface::Browse` composes the same (empty) result for such a page
/// since none of its gated params are present.
///
/// Called from `render_page`'s breadcrumb `.or_else` chain. A member change
/// (`?page=` or a legacy `?member=`) is a full page load, so both the
/// composed `href` and the last crumb segment (which names the on-screen
/// member) follow it. Never duplicate this
/// composition.
pub(crate) fn render_narrative_breadcrumb_for_page(
    page: &cpe_core::ir::Page,
    ctx: &RenderContext<'_>,
    narrative: &NarrativeStore,
    site: &SiteConfig,
    components: &[ComponentData],
    data_arks: &DataArks,
) -> Option<Markup> {
    let has_narrative = page.body.iter().any(|c| matches!(c, Component::Narrative { .. }));
    if !has_narrative {
        return None;
    }

    let has_compound = page.body.iter().any(|c| matches!(c, Component::CompoundViewer { .. }));
    let surface = if has_compound {
        crate::share_link::ViewSurface::Reader
    } else {
        crate::share_link::ViewSurface::Browse
    };
    // The resolved reader member — `None` on member 1 and on a
    // plain narrative page (no `ComponentData::Compound`).
    let reader_member = resolved_reader_member(components);
    let share_query = crate::share_link::compose_canonical_query(ctx.current_query, &[], surface, reader_member, &[]);
    let share_href = share_href_for_reader(ctx.public_origin, ctx.current_path, &share_query, reader_member);
    let share_id = format!("share-{}", ctx.project_id);
    let share = breadcrumb::ShareControl {
        id: &share_id,
        href: &share_href,
        // Design: on record/reader the control never announces which
        // state it caught — same rule extended to plain narrative pages
        // here.
        accessible_name_suffix: None,
        // No plate pager on a narrative/reader page — always a plain
        // static href (the reactive href is record-surface-only).
        reactive_href_attr: None,
    };

    // Project home (route "/") is always the degenerate `CPE › Project`
    // crumb, regardless of whether home.md has its own
    // heading. Every other narrative page's title is its language-resolved
    // first heading, sharing the exact same derivation `compose_page_title`
    // uses.
    let default_lang = cpe_core::lang::resolve(&[], site);
    let page_title = if page.route == "/" {
        None
    } else {
        resolved_narrative_component_title(page, narrative, &ctx.lang, &default_lang)
    };

    // On a reader (compound-
    // viewer) page, the crumb's last segment must name the ON-SCREEN
    // member — for EVERY member index, including 1 (the crumb should
    // always describe what's on screen, not just "not the default"). The
    // page-title segment becomes a live link back to the bare page path
    // (member 1, no `?member=`); the member segment becomes the new
    // current item. Uses the SAME `"{class-label} · NN"` convention as
    // `compound_viewer.rs`'s `cv-member__eyebrow` (same `resolve_class_label`
    // seam, same zero-padded ordinal), so the two can never drift.
    // `member_label` is only computed (and only consumed) when the compound
    // actually resolved a member (`total >= 1`) — an empty compound
    // (`ComponentData::Compound` with `member: None`) falls back to the
    // plain page-title-only crumb, matching the reader's own empty-state
    // degrade elsewhere.
    let compound_data = has_compound.then(|| {
        components.iter().find_map(|c| match c {
            ComponentData::Compound(cp) => Some(cp),
            _ => None,
        })
    });
    let member_label = compound_data.flatten().and_then(|cp| {
        cp.member.as_ref().map(|member| {
            let class_label = resolve_class_label(&site.class_labels, &member.class, &ctx.lang);
            format!("{class_label} · {:02}", cp.index)
        })
    });
    let page_href = ctx.urls().page(&page.route);
    let member_segment = member_label
        .as_deref()
        .map(|label| breadcrumb::NarrativeMemberSegment { page_href: &page_href, label });
    let listing = data_links::ListingSource {
        project_id: ctx.project_id,
        site,
        narrative,
        default_lang: &default_lang,
    };
    let panel = data_links::page_data_panel(page, components, &site.class_labels, &ctx.lang, data_arks, Some(&listing));

    Some(breadcrumb::render_narrative_breadcrumb(
        ctx.urls(),
        site_title(site, &ctx.lang),
        page_title.as_deref(),
        member_segment.as_ref(),
        &ctx.lang,
        &share,
        panel.as_ref(),
    ))
}

/// Browse surfaces: a page whose body has a
/// `ResourceList` component with a declared `id` (required for it to be a
/// `_facets`/`_sort` DataStar patch target in the first place)
/// gets the browse crumb-line variant
/// (`CPE › project › class-label`, [`breadcrumb::render_browse_breadcrumb`]).
/// Called as the SECOND arm of `render_page`'s breadcrumb `.or_else` chain —
/// only reached when the resource-detail arm didn't match, and checked
/// BEFORE `render_narrative_breadcrumb_for_page`.
///
/// **Invariant: a full-page variant choice must match which fragment
/// handlers patch which crumb id.** The host's `_facets`/`_sort`/`_search`
/// fragment handlers unconditionally patch `#crumb-{list_id}` — this fn's
/// own crumb id — with no narrative branch at all. A page combining a
/// `ResourceList` (declared `id`) with a `Narrative` component
/// must therefore render THIS variant, not the narrative
/// one, or a refinement response would target an id the full-page render
/// never emitted. This fn must stay the HIGHER-precedence arm relative to
/// `render_narrative_breadcrumb_for_page`.
///
/// Unlike every other crumb-line variant, the browse control's accessible
/// name carries a **state-bearing** suffix ("On
/// record/reader the control never announces which state it caught" is
/// explicitly NOT true here): `<count-or-count-of-total> · <selected facet
/// option labels, rail order> · “<active query>”`, segments joined by
/// `" · "` and omitted when inactive. The count segment reuses the EXACT
/// same [`cpe_components::chrome::count_resources_text`] /
/// [`cpe_components::chrome::count_of_text`] split `render_surf_head` uses (never/M vs. of/M) **and
/// the exact same class nouns** ([`resolve_class_label`] singular / [`resolve_class_plural`]
/// plural — not the capitalized crumb segment) so the two can't drift;
/// the facet segment reads `ComponentData::Facets`' resolved
/// `FacetOption::selected` flags for the SAME `FacetPanel` (if any) whose
/// `Facet::property` order the href's rail-order facet-property slice also
/// uses — one source of order for both.
///
/// Called from `render_page`'s breadcrumb `.or_else` chain AND
/// from the host's `_facets`/`_sort`/`_search` fragment
/// handlers, which recompose this SAME crumb line after a refinement —
/// never duplicate this composition.
pub fn render_browse_breadcrumb_for_page(
    page: &cpe_core::ir::Page,
    ctx: &RenderContext<'_>,
    components: &[ComponentData],
    site: &SiteConfig,
) -> Option<Markup> {
    let list_idx = page
        .body
        .iter()
        .position(|c| matches!(c, Component::ResourceList { id: Some(_), .. }))?;
    let Component::ResourceList { id: Some(list_id), class, .. } = &page.body[list_idx] else {
        unreachable!("filtered by the `position` predicate above");
    };
    let ComponentData::List(list_page) = components.get(list_idx)? else {
        return None;
    };

    let class_label = resolve_class_plural_capitalized(&site.class_labels, class, &ctx.lang);

    // The sibling `facet-panel` targeting this list, if any — its
    // `Facet::property` order IS the rail order both the href's facet-param
    // ordering and the accessible-name's facet-value ordering share.
    let facet_panel_idx = page
        .body
        .iter()
        .position(|c| matches!(c, Component::FacetPanel { target, .. } if target == list_id));
    let facet_properties: Vec<&str> = match facet_panel_idx.map(|i| &page.body[i]) {
        Some(Component::FacetPanel { facets, .. }) => facets.iter().map(|f| f.property.as_str()).collect(),
        _ => Vec::new(),
    };

    // The list's declared `order-by` keys — so a
    // `?sort=<declared-key>` survives the Copy link. Derived here, IR-aware,
    // exactly like `facet_properties`; `share_link` stays IR-agnostic.
    let declared_sort_keys: Vec<&str> = match &page.body[list_idx] {
        Component::ResourceList { order_options, .. } => order_options.iter().map(|o| o.key.as_str()).collect(),
        _ => Vec::new(),
    };

    let share_query = crate::share_link::compose_canonical_query(
        ctx.current_query,
        &facet_properties,
        crate::share_link::ViewSurface::Browse,
        None,
        &declared_sort_keys,
    );
    let share_href = crate::share_link::compose_absolute_url(ctx.public_origin, ctx.current_path, &share_query);
    let share_id = format!("share-{list_id}");

    // Count segment — same split as `render_surf_head` (resource_list.rs).
    // Nouns: the SAME pair `render_surf_head` uses — singular
    // `resolve_class_label`, plural `resolve_class_plural` (NOT the
    // capitalized `class_label` above, which is the crumb's own display
    // segment). The head and this accessible name must not drift.
    let count_one = resolve_class_label(&site.class_labels, class, &ctx.lang);
    let count_other = resolve_class_plural(&site.class_labels, class, &ctx.lang);
    let count_segment = match list_page.unfiltered_total {
        Some(m) if m != list_page.total => cpe_components::chrome::count_of_text(
            list_page.total,
            m,
            &count_other,
            class_plural_declared(&site.class_labels, class),
            &ctx.lang,
        ),
        _ => cpe_components::chrome::count_resources_text(list_page.total, &count_one, &count_other, &ctx.lang),
    };
    let mut segments = vec![count_segment];

    // Facet segment — selected option labels, rail (group) order.
    if let Some(i) = facet_panel_idx {
        if let Some(ComponentData::Facets(groups)) = components.get(i) {
            for group in groups {
                for option in &group.options {
                    if option.selected {
                        segments.push(option.label.clone());
                    }
                }
            }
        }
    }

    // Query segment — active search query, localized-quote-wrapped.
    if let Some(q) = raw_query_value(ctx.current_query, "q") {
        if cpe_core::util::is_active_search_query(&q) {
            segments.push(cpe_components::chrome::localized_quote(&q, &ctx.lang));
        }
    }

    let accessible_name_suffix = segments.join(" \u{00B7} ");
    let share = breadcrumb::ShareControl {
        id: &share_id,
        href: &share_href,
        accessible_name_suffix: Some(&accessible_name_suffix),
        // Browse surfaces have no plate pager (that's a record-detail-only
        // concern) — always a plain static href.
        reactive_href_attr: None,
    };

    Some(breadcrumb::render_browse_breadcrumb(
        ctx.urls(),
        site_title(site, &ctx.lang),
        &class_label,
        &ctx.lang,
        &share,
    ))
}

/// Decode a single query-string value for `key` out of `current_query`, or
/// `None` when absent. Mirrors `resource_list::sort_param`'s tiny reader
/// shape but decodes the value via [`cpe_core::util::decode_query_value`] — `q`
/// is free text,
/// unlike `sort`'s closed-vocabulary identifier.
fn raw_query_value(current_query: Option<&str>, key: &str) -> Option<String> {
    let q = current_query?;
    for pair in q.split('&') {
        let (k, v) = match pair.find('=') {
            Some(idx) => (&pair[..idx], &pair[idx + 1..]),
            None => (pair, ""),
        };
        if k == key {
            return Some(cpe_core::util::decode_query_value(v));
        }
    }
    None
}

/// The skip-to-content link: `a.skip-link[href="#main"]`. Must be the first
/// child of `<body>` on every full-page renderer — `render_page` (here),
/// the project's `render_not_found_project`, and `render_not_found_site`.
/// The host's 503 page has none: it has no header to skip.
/// Its target is `id="main" tabindex="-1"` on `main.shell__main` — `main` is
/// never a fragment-swap region of
/// `dsp-incubator cpe/docs/adr/0007-fragment-swaps-are-transient.md`, so a `tabindex="-1"` focus
/// target here does not conflict with that decision. Visually hidden until
/// `:focus` (`.skip-link` in shell.css).
pub(crate) fn skip_link(lang: &Lang) -> Markup {
    html! {
        a.skip-link href="#main" { (cpe_components::chrome::ChromeMsg::SkipToContent.text(lang)) }
    }
}

/// The shared shell header: brand lockup, primary nav, and header controls
/// (language switcher, appearance toggle).
///
/// Used by `render_page` and `render_not_found_project` so the chrome is
/// identical. `current_route` is the route of the page being rendered, used
/// for the nav `aria-current` highlight; the latter passes a route that matches no nav
/// item (a 404 is nowhere in the project — no nav item may be current there).
///
/// A nav item is `aria-current` for its own route AND for any route
/// nested under it (e.g. a `/books` nav item stays current on the
/// `/books/:id` detail page, per DESIGN.md §4.1). `item_route` is never
/// treated as a prefix when it is the project-home route `"/"` — every
/// route is nested under `/`, so a naive prefix test would mark the home
/// item current everywhere. A route is "nested" only when `current_route`
/// starts with `item_route` followed by a literal `/` (a same-prefix
/// sibling like `/booksX` must not match `/books`).
fn nav_item_is_active(item_route: &str, current_route: &str) -> bool {
    if item_route == current_route {
        return true;
    }
    item_route != "/" && current_route.starts_with(&format!("{item_route}/"))
}

/// The desk query as CSS and `matchMedia` spell it.
fn header_desk_query() -> String {
    format!("(min-width: {}rem)", HEADER_DESK_BREAKPOINT_PX / 16)
}

/// `#header-menu`'s resize handler: a panel still open when the window
/// widens into the desk row is hidden, since an open popover stays in the
/// top layer, where the desk form's `position: static` cannot reach it.
/// `.trailing` (on the attribute) lets the last event of a drag run too.
fn header_menu_resize_expr() -> String {
    format!(
        "el.hidePopover && window.matchMedia('{}').matches && el.matches(':popover-open') && el.hidePopover()",
        header_desk_query()
    )
}

/// The one shell header (one DOM at every width): brand, the burger, and
/// `#header-menu`, a `popover="auto"` that `shell.css` shows in flow as the
/// desk row from the desk breakpoint and as a panel below the row otherwise.
pub(crate) fn shell_header(site: &SiteConfig, ctx: &RenderContext<'_>, current_route: &str) -> Markup {
    let title = site_title(site, &ctx.lang);
    html! {
        header.shell__header {
            div.header__inner {
                // Brand: CPE wordmark → "/" + project name → project home
                div.brand {
                    a.brand__mark href=(ctx.base.home()) { "CPE" }
                    span.brand__sep { "/" }
                    // `title` carries the full name where the row ellipsises it.
                    a.brand__project href=(ctx.urls().home()) title=(title) { (title) }
                }
                button.header__menu-btn type="button" popovertarget=(HEADER_MENU_ID) {
                    span.header__menu-bars aria-hidden="true" {
                        span {}
                        span {}
                        span {}
                    }
                    span.header__menu-label {
                        (cpe_components::chrome::ChromeMsg::MenuLabel.text(&ctx.lang))
                    }
                }
                div id=(HEADER_MENU_ID)
                    popover="auto"
                    "data-on:resize__window__throttle.200ms.trailing"=(header_menu_resize_expr())
                {
                    nav .header__nav
                        aria-label=({
                            cpe_components::chrome::ChromeMsg::NavPrimaryLabel
                                .text(&ctx.lang)
                        })
                    {
                        @for item in &site.nav {
                            @let is_active = nav_item_is_active(&item.route, current_route);
                            a   href=(ctx.urls().page(&item.route))
                                aria-current=[is_active.then_some("page")]
                            { (nav_label(site, item, &ctx.lang)) }
                        }
                    }
                    (shell_header_controls(site, ctx))
                }
            }
        }
    }
}

/// The header controls: the language row (two or more languages only) and
/// the appearance row. `shell_header` and `render_compound_fragment` both
/// render them through this one function, so a `_compound` patch matches a
/// full load. The children carry no ids: DataStar's morph pairs them by tag
/// and position, which holds because a project's language count is fixed.
///
/// The language `details` never carries `open` from the server, so a
/// morph closes it (a transient disclosure, like `#crumb-data`). The row
/// labels are `aria-hidden`: each control carries its own name.
pub(crate) fn shell_header_controls(site: &SiteConfig, ctx: &RenderContext<'_>) -> Markup {
    use cpe_components::chrome::ChromeMsg;
    use dom::{PICKER_ESCAPE_EXPR, PICKER_FOCUSOUT_EXPR};

    // Light is the default; the toggle offers the opposite of the current
    // scheme. The desk name is also the `title`; the panel's visible label
    // names the target mode.
    let (toggle_target, desk_name, panel_label) = match ctx.appearance {
        Appearance::Dark => (
            "light",
            ChromeMsg::ThemeToggleToLight.text(&ctx.lang),
            ChromeMsg::ThemeLightModeLabel.text(&ctx.lang),
        ),
        Appearance::Light => (
            "dark",
            ChromeMsg::ThemeToggleToDark.text(&ctx.lang),
            ChromeMsg::ThemeDarkModeLabel.text(&ctx.lang),
        ),
    };
    let current_name = lang_label(ctx.lang.as_str());

    html! {
        div.header__controls id=(HEADER_CONTROLS_ID) {
            @if site.languages.len() > 1 {
                div.header__row {
                    span.header__row-label aria-hidden="true" {
                        (ChromeMsg::LanguageSwitcherLabel.text(&ctx.lang))
                    }
                    details
                        .lang-menu
                        data-on:keydown__window=(PICKER_ESCAPE_EXPR)
                        data-on:click__outside="el.open = false"
                        data-on:focusout=(PICKER_FOCUSOUT_EXPR)
                    {
                        summary {
                            span.code aria-hidden="true" { (ctx.lang.as_str().to_uppercase()) }
                            span.name aria-hidden="true" { (current_name) }
                            span.caret aria-hidden="true" { "▾" }
                            span.visually-hidden {
                                ({
                                    cpe_components::chrome::language_current_text(
                                        &current_name,
                                        &ctx.lang,
                                    )
                                })
                            }
                        }
                        div.lang-menu__list {
                            @for code in &site.languages {
                                @let is_active = code == ctx.lang.as_str();
                                a   lang=(code)
                                    hreflang=(code)
                                    href=({
                                        format!(
                                            "{}?{}",
                                            ctx.current_path,
                                            resource_list::override_query_param(
                                                ctx.current_query,
                                                "lang",
                                                code,
                                            ),
                                        )
                                    })
                                    aria-current=[is_active.then_some("true")]
                                {
                                    (lang_label(code))
                                    span.code { (code.to_uppercase()) }
                                }
                            }
                        }
                    }
                }
            }
            div.header__row {
                span.header__row-label aria-hidden="true" {
                    (ChromeMsg::AppearanceLabel.text(&ctx.lang))
                }
                // No `aria-label`: `shell.css` hides one of the two name
                // spans per form, so each form gets exactly its own name.
                a   .theme-toggle
                    href=({
                        format!(
                            "{}?{}",
                            ctx.current_path,
                            resource_list::override_query_param(
                                ctx.current_query,
                                "appearance",
                                toggle_target,
                            ),
                        )
                    })
                    title=(desk_name)
                {
                    span.theme-toggle__icon aria-hidden="true" {}
                    span.theme-toggle__desk-name { (desk_name) }
                    span.theme-toggle__panel-name {
                        (panel_label)
                        span.visually-hidden { " – " (ChromeMsg::ThemeSwitchSuffix.text(&ctx.lang)) }
                    }
                }
            }
        }
    }
}

/// The shared shell footer (CPE mark, optional project description, DaSCH
/// lockup, attribution).
///
/// The lockup follows the description in the DOM; `grid.css` places it on the
/// mark's row, so the row is explicit there, not left to auto-placement.
///
/// `site.description` follows the same "chrome string id OR literal" pattern
/// as nav `label` (see `nav_label`/`chrome_lookup`): resolved via
/// `chrome_lookup(site, desc, lang)`, falling back to the literal value when
/// `desc` doesn't match a declared chrome-string id.
pub(crate) fn shell_footer(site: &SiteConfig, lang: &Lang, statics: &cpe_core::url::StaticDir) -> Markup {
    html! {
        footer.shell__footer {
            div.footer__inner {
                span.brand__mark.footer__mark { "CPE" }
                @if let Some(desc) = &site.description {
                    p.footer__desc { (chrome_lookup(site, desc, lang).unwrap_or(desc)) }
                }
                a   .dasch-logo
                    .footer__lockup
                    href="https://www.dasch.swiss"
                    aria-label=(cpe_components::chrome::ChromeMsg::FooterDaschLabel.text(lang))
                {
                    img .dasch-logo__img
                        .dasch-logo__img--light
                        src=(statics.file("cpe/dasch-logo-colour.svg"))
                        alt=""
                        width="2147"
                        height="621";
                    img .dasch-logo__img
                        .dasch-logo__img--dark
                        src=(statics.file("cpe/dasch-logo-white.svg"))
                        alt=""
                        width="2148"
                        height="622";
                }
                // The attribution's final "DaSCH" is emphasized. We bold the
                // last occurrence (not a fixed offset) so the catalog can keep
                // clean flat sentences — "DaSCH" lands in different positions
                // across languages. Degrades gracefully (whole string, no bold)
                // if the sentence contains no "DaSCH".
                //
                // A project's `site { footer-attribution }` override
                // replaces the built-in
                // string — chrome-id-or-literal, the `site.description` rule —
                // and renders as PLAIN text: the DaSCH emphasis is the built-in
                // string's own markup, never applied to project copy.
                p.footer__attr {
                    @if let Some(custom) = &site.footer_attribution {
                        (chrome_lookup(site, custom, lang).unwrap_or(custom))
                    } @else {
                        @let attr = cpe_components::chrome::ChromeMsg::FooterAttribution
                            .text(lang);
                        @if let Some((lead, tail)) = attr.rsplit_once("DaSCH") {
                            (lead)
                            strong { "DaSCH" }
                            (tail)
                        } @else { (attr) }
                    }
                }
            }
        }
    }
}

/// Map an ISO 639-1 code to a display name in the language itself.
///
/// Falls back to the code uppercased for any code not in the small map.
fn lang_label(code: &str) -> String {
    match code {
        "en" => "English".to_owned(),
        "de" => "Deutsch".to_owned(),
        "fr" => "Français".to_owned(),
        "it" => "Italiano".to_owned(),
        other => other.to_uppercase(),
    }
}

/// Return the first `ComponentData::Detail(Some(r))` found in `components`.
///
/// Used by `compose_page_title` and the breadcrumb derivation so both share
/// one extraction path.
fn first_resolved_resource(components: &[ComponentData]) -> Option<&Resource> {
    components.iter().find_map(|cd| match cd {
        ComponentData::Detail(Some(r)) => Some(r),
        _ => None,
    })
}

/// Groups a `ResourceList` (index `list`) with its optional target
/// `FacetPanel` (`facet`) and/or `SearchBar` (`search`) — the generalization
/// of the facet+list pairing to a 3-way grouping.
/// See `render_body`'s doc comment for the render-time
/// consequences of each combination.
struct BrowseGroup {
    search: Option<usize>,
    facet: Option<usize>,
    list: usize,
}

/// Owned, resolved search-fragment rendering data for one `resource-list`'s
/// sibling `search-bar`.
///
/// **Two callers, one resolver.** `render_body` (this module) builds one via
/// [`Self::resolve`] from IR it already has in hand (`BrowseGroup`/`Page`/
/// `RenderContext`), for the full-page render path. The host's
/// `_facets`/`_sort`/`_search` fragment handlers — which
/// call `render_resource_list`/`render_active_filters` DIRECTLY, bypassing
/// `render_body`'s composition entirely — build one via the SAME
/// [`Self::resolve`] from pieces they extract from `page.body` +
/// `ComponentData` themselves. This is the ONE seam that resolves
/// `chrome_lookup`, `resolve_class_plural`, `narrowed_to_zero_text`, and
/// `facet_panel::clear_filters_query`, so both callers can never drift on
/// scope-note/no-match/clear-href semantics. The host must not call them
/// itself, or its fragments drift from the full page.
pub struct SearchFragmentSurface {
    /// The raw, already-decoded `?q=` value (may be empty/inactive — see
    /// [`Self::resolve`]'s doc comment; only [`Self::search_surface`] gates
    /// on activity).
    q: String,
    scope_note: String,
    no_match_msg: maud::Markup,
    no_match_hint: String,
    clear_search_href: String,
    clear_all_href: Option<String>,
}

impl SearchFragmentSurface {
    /// Resolve every search-fragment field for a `resource-list` of `class`
    /// whose sibling `search-bar` has `scope` (its `Component::SearchBar.scope`
    /// chrome-string id) and the given already-decoded `q`.
    ///
    /// Always returns a value regardless of whether `q` is currently active —
    /// that gate belongs to [`Self::search_surface`] alone, so this fn never
    /// duplicates it. Call this ONLY when a search-bar targets the list at
    /// all (callers know this independently — e.g. `render_body`'s grouping,
    /// or a host scanning `page.body` for a matching `Component::SearchBar`);
    /// there is no "no search-bar" `None` case here.
    ///
    /// The no-match noun comes from `resolve_class_plural` (the localized
    /// **plural** class noun) — never a `label`
    /// chrome id. The scope note is the project-declared `search-bar
    /// scope="…"` chrome-string id (mirrors `placeholder` — resolved via
    /// `chrome_lookup`, never composed from `searches`/property names).
    ///
    /// `facet_groups` is the list's sibling facet-panel's RESOLVED
    /// `ComponentData::Facets` payload (the same slice `render_active_filters`
    /// already reads for its own "Clear all" href, via
    /// `facet_panel::clear_filters_query`) — deliberately NOT
    /// `Component::FacetPanel.facets` (the config-declared facet list).
    /// `filters_active` (drives whether `clear_all_href` exists at all —
    /// that action only appears "when facets are ACTUALLY
    /// narrowing": presence of a facet-panel alone is NOT
    /// enough) is derived from [`any_facet_selected`], not from
    /// `facet_groups.is_some()` — a page with a facet-panel but a
    /// hand-crafted bogus facet value (`?role=NOPE`) must NOT show "Clear
    /// all filters" (there is nothing to clear).
    #[allow(clippy::too_many_arguments)]
    pub fn resolve(
        site: &SiteConfig,
        scope: Option<&str>,
        class: &str,
        q: String,
        current_path: &str,
        current_query: Option<&str>,
        facet_groups: Option<&[FacetGroup]>,
        lang: &Lang,
    ) -> Self {
        let filters_active = facet_groups.map(any_facet_selected).unwrap_or(false);
        let query_active = cpe_core::util::is_active_search_query(&q);

        let noun = resolve_class_plural(&site.class_labels, class, lang);
        // The query slot is `Some(&q)` ONLY when the query is actually
        // active — passing it unconditionally would render an empty-quoted
        // "…match ""." on a filters-only narrowing (this fn is reachable
        // with an inactive/absent `q`).
        let query_slot = query_active.then_some(q.as_str());
        // `narrowed_to_zero_text` panics (unreachable!) when called with
        // neither active — a real, frequent case here (`resolve` runs on
        // every render of a searchable/facetable list, not just a narrowed
        // one; `NarrowedSurface::build` below discards `no_match_msg` when
        // inactive). Gate the call itself rather than relying on the
        // discard downstream.
        let no_match_msg = if query_active || filters_active {
            cpe_components::chrome::narrowed_to_zero_text(&noun, query_slot, filters_active, lang)
        } else {
            maud::html! {}
        };

        // The scope note is a project-declared `chrome { string }` id
        // (`search-bar` gains ONLY `scope`, mirroring
        // `placeholder`'s "id, not literal" convention) — a literal, per-page,
        // per-language string the project author writes directly (e.g.
        // "matching name and description"), resolved exactly like `placeholder`.
        // NOT computed from `resolve_property_label`/`searches` — this crate does
        // no property-name-to-prose composition here, only chrome-id lookup.
        // Undeclared/unresolved → empty string, which `render_surf_head` treats
        // as "omit the scope-note span entirely" (no dangling "· " artifact).
        let scope_note = scope.and_then(|id| chrome_lookup(site, id, lang)).unwrap_or("").to_owned();

        let no_match_hint = cpe_components::chrome::ChromeMsg::SearchHint.text(lang).to_owned();

        // "Clear search" preserves facets/sort — only `q` is dropped (contrast
        // with "Clear all" below, which also drops the facet keys + page).
        let clear_search_href = match clear_search_only_query(current_query) {
            Some(qs) => format!("{current_path}?{qs}"),
            None => current_path.to_owned(),
        };

        // "Clear all" only exists when at least one facet is ACTUALLY
        // selected: the no-match state's second action appears when a
        // facet narrows the list, not whenever the page has a facet panel.
        let facet_keys: Vec<&str> = facet_groups
            .map(|groups| groups.iter().map(|g| g.property.as_str()).collect())
            .unwrap_or_default();
        let clear_all_href = filters_active.then(|| {
            facet_panel::clear_filters_query(current_query, &facet_keys)
                .map(|qs| format!("{current_path}?{qs}"))
                .unwrap_or_else(|| current_path.to_owned())
        });

        Self {
            q,
            scope_note,
            no_match_msg,
            no_match_hint,
            clear_search_href,
            clear_all_href,
        }
    }

    /// The search-aware rendering surface for `ResourceListCfg.search` —
    /// `Some` iff `q` is active (≥1 usable term after tokenize —
    /// [`resource_list::SearchSurface::for_query`]'s gate, never raw
    /// non-emptiness). `None` when `q` is empty/whitespace-/punctuation-only,
    /// regardless of whether a search-bar is configured.
    pub fn search_surface(&self) -> Option<resource_list::SearchSurface<'_>> {
        resource_list::SearchSurface::for_query(&self.q, &self.scope_note)
    }

    /// The filterbar's query-chip display text — `Some(q)` iff active. Uses
    /// the SAME gate as [`Self::search_surface`] (derived from it, not a
    /// second tokenizer check), so the chip and the surf-head/no-match state
    /// can never disagree about what counts as "active".
    pub fn active_query(&self) -> Option<&str> {
        self.search_surface().is_some().then_some(self.q.as_str())
    }

    /// The narrowed-to-zero rendering surface for `ResourceListCfg.narrowed`
    /// — `Some` iff an active search
    /// query and/or an active facet narrowing is in effect. Unlike
    /// [`Self::search_surface`] (gated on the query ALONE), this is what
    /// makes a facet-only page with zero matches and NO search-bar at all
    /// still show the narrowed-to-zero body instead of degrading to the
    /// genuinely-empty-class message.
    pub fn narrowed_surface(&self) -> Option<resource_list::NarrowedSurface<'_>> {
        let query_active = cpe_core::util::is_active_search_query(&self.q);
        resource_list::NarrowedSurface::build(
            query_active,
            self.no_match_msg.clone(),
            &self.no_match_hint,
            &self.clear_search_href,
            self.clear_all_href.as_deref(),
        )
    }
}

/// Compute a [`SearchFragmentSurface`] for `group`'s `resource-list`, always: there
/// is no "no search-bar" `None` case here: `render_body` only ever calls this
/// for a group that has a facet-panel and/or a search-bar (a facet-only
/// group needs a narrowed-to-zero
/// surface too, even with no search-bar to gate on). Thin IR-extraction
/// wrapper around [`SearchFragmentSurface::resolve`] for `render_body`'s
/// `BrowseGroup`-based call site; see that fn's doc comment for the
/// field-resolution rules.
///
/// **`?q=` is never read when the group has no search-bar** (`group.search:
/// None`) — `cpe_core::util::search_query_param` is only called inside the
/// `Some(s)` arm below, so a page with a facet-panel but no search-bar never
/// touches the query string's `q` key at all, keeping the "no search-bar ⇒
/// `?q=` is never read" invariant explicit (mirrors the host's
/// `build_search_fragment_surface`).
fn build_narrowed_owned(
    group: &BrowseGroup,
    page: &cpe_core::ir::Page,
    ctx: &RenderContext<'_>,
    site: &SiteConfig,
    facet_data: Option<&[FacetGroup]>,
) -> SearchFragmentSurface {
    let Component::ResourceList { class, .. } = &page.body[group.list] else {
        unreachable!(
            "group.list always indexes a Component::ResourceList — BrowseGroup \
             is only ever constructed keyed off a ResourceList match in \
             render_body's grouping loop"
        );
    };

    let (scope, q) = match group.search {
        Some(s) => {
            let Component::SearchBar { scope, .. } = &page.body[s] else {
                unreachable!(
                    "group.search only ever indexes a Component::SearchBar — built \
                     exclusively from `if let Component::SearchBar` in render_body's \
                     grouping loop"
                );
            };
            (scope.as_deref(), cpe_core::util::search_query_param(ctx.current_query))
        }
        None => (None, String::new()),
    };

    SearchFragmentSurface::resolve(
        site,
        scope,
        class,
        q,
        ctx.current_path,
        ctx.current_query,
        facet_data,
        &ctx.lang,
    )
}

/// Build the query string for the search-empty state's "Clear search" href:
/// every param from `current_query` EXCEPT `q` (facets/sort/lang/appearance/…
/// preserved — unlike "Clear all", clearing the search alone must not also
/// reset facet selections or the sort order). Mirrors
/// `facet_panel::clear_filters_query`'s "drop set, preserve the rest" shape,
/// with a single fixed key instead of a caller-supplied set. `page` is
/// deliberately NOT dropped (unlike `clear_filters_query`, which always
/// drops it) — clearing the query alone has no reason to force a page reset.
///
/// Returns `None` when the preserved query string is empty (caller should use
/// the bare path); returns `Some(qs)` (no leading `?`) otherwise — same
/// convention as `clear_filters_query`.
fn clear_search_only_query(current_query: Option<&str>) -> Option<String> {
    let kept: Vec<&str> = current_query
        .unwrap_or("")
        .split('&')
        .filter(|pair| {
            if pair.is_empty() {
                return false;
            }
            let k = match pair.find('=') {
                Some(idx) => &pair[..idx],
                None => pair,
            };
            k != "q"
        })
        .collect();
    if kept.is_empty() {
        None
    } else {
        Some(kept.join("&"))
    }
}

/// Build the `SearchBarCfg` for the `SearchBar` component at `c` and render
/// it, with `standalone` decided by the caller.
///
/// `render_body` builds this DIRECTLY (bypassing `render_component`'s generic
/// dispatch) because whether a search row is `standalone` depends on grouping
/// information (whether its target list also has a `FacetPanel`) that
/// `render_component` — which renders one component at a time, structurally
/// unaware of its siblings — cannot see. Mirrors `render_component`'s
/// `Component::SearchBar` arm's placeholder-resolution logic exactly.
fn render_search_row(c: &Component, ctx: &RenderContext<'_>, site: &SiteConfig, standalone: bool) -> Markup {
    let Component::SearchBar { placeholder, .. } = c else {
        unreachable!(
            "render_search_row is only ever called with a Component::SearchBar — \
             see the `group_at` construction in render_body, which only ever \
             records a position found via a `Component::SearchBar` match"
        );
    };
    let placeholder_text = placeholder.as_deref().and_then(|id| chrome_lookup(site, id, &ctx.lang));
    let cfg = search_bar::SearchBarCfg {
        action_path: ctx.current_path,
        current_query: ctx.current_query,
        placeholder: placeholder_text,
        standalone,
    };
    search_bar::render_search_bar(&cfg, &ctx.lang)
}

/// Render the page body, grouping each `resource-list` with its optional
/// target `facet-panel` and/or `search-bar`.
/// Four shapes, by what a `ResourceList` is targeted by:
///
/// 1. **Facet + search**: ONE `section.browse.page` wrapper, direct children in this order: the
///    search row, the facet rail, the active-filter chip row, the list. `.search-row` carries
///    **NO** `.page` (a flat grid child, like `.filterbar` — CSS places it via `.browse >
///    .search-row { grid-column: 2; grid-row: 1 }`).
/// 2. **Search only, no facet**: **NO** `.browse` wrapper at all — the search row (**WITH**
///    `.page`, since nothing else supplies centering here) immediately followed by the list, both
///    standalone — regardless of the two components' declared order in `page.body`.
/// 3. **Facet only, no search**: the flat three-child `.browse.page` wrapper (rail, chip row,
///    list): a wrapper div would make the list a *grandchild* of `.browse` and break the `.browse >
///    .page` neutralizer (record.css), reintroducing a doubled dead band. Grid placement is done
///    entirely in CSS via child-combinator selectors — do NOT refactor this into threading facet
///    labels through `ResourceListCfg`; that alternative was explicitly rejected (the chip row is a
///    second *view* of the facet-panel's own `groups` data, not list data).
/// 4. **Neither** — unchanged: the list renders standalone via `render_component`, exactly as any
///    other unpaired component.
///
/// **An empty facet panel:** a `facet-panel` whose
/// RESOLVED `FacetGroup` list is empty (zero configured facet children — a
/// static config condition) is treated as if it were absent for layout
/// purposes: it degrades to case 2 (if paired with a search-bar) or case 4
/// (if not), never shape 1/3's `.browse` wrapper with an empty rail. The
/// facet-panel's index stays a member of the `BrowseGroup` (so it stays
/// `consumed` and can never be re-dispatched through `render_component`'s
/// `FacetPanel` arm) — only which branch below renders it changes.
///
/// A `facet-panel target="X"` / `search-bar target="X"` is paired with the
/// `resource-list id="X"` on the same page (pairings the boot validator
/// already guarantees resolve — `UnresolvedTarget`). All rail/list/search-row
/// children keep their own `.page` class per the LAYOUT CONTRACT where they
/// stand alone (case 2's search row, case 4's list); inside a `.browse.page`
/// wrapper (cases 1/3) the filterbar and search row deliberately do NOT carry
/// `.page` (flat grid children — the wrapper supplies the centered box; CSS
/// neutralizes the inner centering for rail/list). `render_component` is
/// reused unchanged for the rail and list; the filterbar is rendered directly
/// via `render_active_filters` (it isn't a `Component` variant of its own);
/// the search row is rendered directly via `render_search_row` (its
/// `standalone`-ness can't be resolved inside `render_component`'s generic
/// dispatch — see that fn's doc comment).
///
/// **Figure blocks.** A component with a figure-block slot
/// (`Page::figure_slot`) is rendered first and handed, with its resolved
/// count line and more link, to the nearest `narrative` before it, which
/// renders in figure mode; it is `consumed` at its own position.
///
/// **Narratives.** Every narrative of the page is drawn here, never through
/// `render_component`: only this fn knows a narrative's figure blocks, its
/// page-wide heading-id offset and whether it carries the page rail. The
/// rail lists the top-level `##` headings of all the page's narratives and
/// goes to the first one. The second value returned is the page's
/// [`NarrativeLayout`], `None` on a page without a narrative.
fn render_body(
    page: &cpe_core::ir::Page,
    ctx: &RenderContext<'_>,
    narrative: &NarrativeStore,
    components: &[ComponentData],
    default_lang: &Lang,
    site: &SiteConfig,
) -> (Markup, Option<NarrativeLayout>) {
    use std::collections::{HashMap, HashSet};

    // Group each resource-list (index l) with its optional target facet-panel
    // and/or search-bar. `group_at` is keyed by the EARLIEST index among a
    // group's members (mirrors the old `wrapper_at`'s earlier-index keying)
    // so the wrapper renders at the group's first position in body order;
    // `consumed` marks every later member so it's skipped when the main loop
    // reaches its own position.
    let mut group_at: HashMap<usize, BrowseGroup> = HashMap::new();
    let mut consumed: HashSet<usize> = HashSet::new();
    for (l, c) in page.body.iter().enumerate() {
        let Component::ResourceList { id: Some(list_id), .. } = c else {
            continue;
        };
        let facet = page
            .body
            .iter()
            .position(|x| matches!(x, Component::FacetPanel { target, .. } if target == list_id));
        let search = page
            .body
            .iter()
            .position(|x| matches!(x, Component::SearchBar { target, .. } if target == list_id));
        if facet.is_none() && search.is_none() {
            continue; // Case 4 — no pairing at all; rendered standalone below.
        }
        let mut members = vec![l];
        members.extend(facet);
        members.extend(search);
        let earliest = *members.iter().min().expect("non-empty — `l` is always a member");
        for &m in &members {
            if m != earliest {
                consumed.insert(m);
            }
        }
        group_at.insert(earliest, BrowseGroup { search, facet, list: l });
    }

    // Slot accessor: ComponentData aligned 1:1 with page.body; missing → Other.
    let other = ComponentData::Other;
    let slot = |i: usize| components.get(i).unwrap_or(&other);

    let nested = nested_reader(page, components);
    if let Some((_, r)) = nested {
        consumed.insert(r);
    }

    // Figure blocks: a component with a figure-block slot renders into the
    // nearest narrative before it (boot guarantees one), never at its own
    // position; the narrative gets its blocks in body order.
    let mut figures_at: HashMap<usize, Vec<editorial::FigureBlock>> = HashMap::new();
    for (i, c) in page.body.iter().enumerate() {
        let (Some(figure_slot), Some(n)) = (page.figure_slot(i), page.figure_host(i)) else {
            continue;
        };
        consumed.insert(i);
        // A label's `{n}` is the number the component's registry row reads
        // from its data (`slot_n`); without one the label renders as written.
        let n_text = project::slot_n(ctx.project_id, c, slot(i)).map(|n| n.to_string());
        let label = |id: &str| {
            chrome_lookup(site, id, &ctx.lang).map(|text| match &n_text {
                Some(digits) => cpe_core::util::substitute_count(text, digits),
                None => text.to_owned(),
            })
        };
        figures_at.entry(n).or_default().push(editorial::FigureBlock {
            section: figure_slot.section.clone(),
            block: render_component(c, ctx, narrative, slot(i), default_lang, site),
            count: figure_slot.count.as_deref().and_then(label),
            more: figure_slot.more.as_ref().and_then(|more| {
                label(&more.label).map(|label| editorial::FigureLink { label, href: ctx.urls().page(&more.route) })
            }),
        });
    }

    // Parse-only pass over the page's narratives, in body order. Each one's
    // id offset is the number of top-level `##` headings before it on the
    // page, so generated heading ids (`sec-<n>`) never repeat across them.
    // The same pass collects the page's rail entries, which is how the first
    // narrative's rail can list the headings of a later one.
    let mut id_offsets: HashMap<usize, usize> = HashMap::new();
    let mut rail_entries: Vec<editorial::RailEntry> = Vec::new();
    for (i, c) in page.body.iter().enumerate() {
        let Component::Narrative { file, .. } = c else {
            continue;
        };
        let offset = rail_entries.len();
        id_offsets.insert(i, offset);
        let content =
            cpe_components::component::narrative::resolve_narrative_content(file, &ctx.lang, default_lang, narrative);
        if let Some(src) = content {
            rail_entries.extend(editorial::rail_entries(src, offset));
        }
    }
    // A heading without a name keeps its number and its id, but the rail does
    // not list it. Only after the loop: the offsets above count every
    // heading, as the renderer does.
    rail_entries.retain(editorial::RailEntry::has_name);

    // The layout the page asks for: the first narrative's `companion=` and
    // `side=`, else the site's defaults.
    let requested = page.body.iter().enumerate().find_map(|(i, c)| match c {
        Component::Narrative { companion, side, .. } => Some((
            i,
            companion.unwrap_or(site.narrative_companion),
            side.unwrap_or(site.narrative_side),
        )),
        _ => None,
    });
    let rail = requested
        .filter(|&(_, companion, _)| companion == NarrativeCompanion::Rail && rail_entries.len() >= RAIL_MIN_ENTRIES)
        .map(|_| editorial::Rail {
            entries: &rail_entries,
            label: cpe_components::chrome::ChromeMsg::NarrativeRailLabel.text(&ctx.lang),
        });
    let mut rail_placed = false;

    let mut blocks: Vec<Markup> = Vec::with_capacity(page.body.len());
    for i in 0..page.body.len() {
        if let Component::Narrative { file, .. } = &page.body[i] {
            let is_first = requested.is_some_and(|(first, ..)| first == i);
            let rendered = cpe_components::component::narrative::render_narrative(
                ctx.base,
                file,
                &ctx.lang,
                default_lang,
                narrative,
                figures_at.get(&i).map_or(&[], Vec::as_slice),
                id_offsets.get(&i).copied().unwrap_or(0),
                rail.as_ref().filter(|_| is_first),
            );
            rail_placed |= rendered.rail_placed;
            blocks.push(rendered.markup);
        } else if let Some((_, j)) = nested.filter(|&(d, _)| d == i) {
            let reader = render_component(&page.body[j], ctx, narrative, slot(j), default_lang, site);
            blocks.push(render_detail_component(
                &page.body[i],
                ctx,
                narrative,
                slot(i),
                default_lang,
                site,
                Some(NestedReader {
                    reader,
                    annotations: render_reader_annotations(page, ctx, components, site),
                }),
            ));
        } else if let Some(group) = group_at.get(&i) {
            // The group's resolved facet DATA (not `Component::FacetPanel`'s
            // config-declared facet list) — see `build_narrowed_owned`'s doc
            // comment for why this is the source `clear_all_href` must use.
            let facet_data: Option<&[FacetGroup]> = group.facet.and_then(|f| match slot(f) {
                ComponentData::Facets(groups) => Some(groups.as_slice()),
                _ => None,
            });
            // The group's narrowed-to-zero rendering surface
            // — computed once per group, regardless of
            // whether it has a search-bar and/or a facet-panel, so all three
            // consumers below (`search_surface`/`active_query`/
            // `narrowed_surface`) share one source of truth.
            let narrowed_owned = build_narrowed_owned(group, page, ctx, site, facet_data);
            // Search-only surface (populated-list scope note) — `None` unless
            // a search query is active.
            let search_surface = narrowed_owned.search_surface();
            // The filterbar's query chip shows the
            // SAME "active" gate as `search_surface` itself — see
            // `SearchFragmentSurface::active_query`'s doc comment.
            let active_query: Option<&str> = narrowed_owned.active_query();
            // The empty-branch narrowed-to-zero surface — `Some` iff a
            // search query and/or a facet is actually active, regardless of
            // whether this group has a search-bar at all.
            let narrowed_surface = narrowed_owned.narrowed_surface();

            // A `facet-panel`
            // targeting this list is only treated as "faceted" (browse-grid
            // + rail) when its RESOLVED groups are non-empty. A `facet-panel`
            // with zero configured facet children (`groups.is_empty()` — a
            // static config condition, genuinely reachable per
            // `lower_facet_panel`) degrades this list to the SAME layout as
            // "no facet-panel at all" (case 2 / case 4 below) rather than
            // rendering an empty rail — an empty rail is furniture with
            // nothing to do. `f` stays a member of this
            // `BrowseGroup` regardless (see `group_at` construction above),
            // so it is already marked `consumed` and will never fall through
            // to `render_component`'s `FacetPanel` arm and re-emit the rail.
            let effective_facet: Option<usize> = group
                .facet
                .filter(|&f| matches!(slot(f), ComponentData::Facets(groups) if !groups.is_empty()));

            match effective_facet {
                Some(f) => {
                    // Case 1 or 3: faceted — wrap in `.browse.page`. The
                    // search row (if any) is a flat grid child (NO `.page`),
                    // first, before the rail.
                    let Component::FacetPanel { target, .. } = &page.body[f] else {
                        unreachable!(
                            "group_at only ever pairs a FacetPanel index (f) — \
                             built exclusively from `if let Component::FacetPanel` above"
                        );
                    };
                    let filterbar = match slot(f) {
                        ComponentData::Facets(groups) => {
                            let panel_cfg = facet_panel::FacetPanelCfg {
                                list_id: target,
                                current_path: ctx.current_path,
                                current_query: ctx.current_query,
                            };
                            facet_panel::render_active_filters(groups, &panel_cfg, &ctx.lang, active_query)
                        }
                        // No facet data resolved for this slot — same defensive
                        // fallback as render_component's other ComponentData arms.
                        _ => html! {},
                    };
                    let search_row = group.search.map(|s| render_search_row(&page.body[s], ctx, site, false));
                    let browse = html! {
                        section.browse.page {
                            @if let Some(row) = &search_row { (row) }
                            ({
                                render_component(
                                    &page.body[f],
                                    ctx,
                                    narrative,
                                    slot(f),
                                    default_lang,
                                    site,
                                )
                            })
                            (filterbar)
                            ({
                                render_list_component(
                                    &page.body[group.list],
                                    ctx,
                                    slot(group.list),
                                    site,
                                    search_surface,
                                    narrowed_surface,
                                )
                            })
                        }
                    };
                    blocks.push(browse);
                }
                None => {
                    // Case 2 (search-only, non-faceted) OR a facet-panel that
                    // targets this list but resolved to zero groups — NO
                    // `.browse` wrapper, no rail. The search row, if any,
                    // carries `.page` (its own centering — `render_search_row`'s
                    // `standalone: true` decides this) followed by the list,
                    // both standalone. `group.search` may legitimately be
                    // `None` here (an empty facet panel, no search-bar), in which
                    // case only the list renders — mirroring case 4.
                    let search_row = group.search.map(|s| render_search_row(&page.body[s], ctx, site, true));
                    let standalone = html! {
                        @if let Some(row) = &search_row { (row) }
                        ({
                            render_list_component(
                                &page.body[group.list],
                                ctx,
                                slot(group.list),
                                site,
                                search_surface,
                                narrowed_surface,
                            )
                        })
                    };
                    blocks.push(standalone);
                }
            }
        } else if !consumed.contains(&i) {
            blocks.push(render_component(&page.body[i], ctx, narrative, slot(i), default_lang, site));
        }
        // else: this index is a later member of a group — already rendered at
        // the group's earliest index; skip.
    }

    // The effective companion is `rail` only when the rail is in the markup:
    // a page with too few entries, or whose first narrative had no head or
    // intro to put it beside, is laid out as `text`.
    let layout = requested.map(|(_, _, side)| NarrativeLayout {
        companion: if rail_placed {
            NarrativeCompanion::Rail
        } else {
            NarrativeCompanion::Text
        },
        side,
    });

    (
        html! {
            @for block in &blocks { (block) }
        },
        layout,
    )
}

/// The fewest entries a page rail is rendered for: a list of one section
/// names nothing the page does not already show.
const RAIL_MIN_ENTRIES: usize = 2;

/// How a page with a narrative is laid out. `project_html` writes it on
/// `<html>` as `data-narr` and `data-narr-side`, where the stylesheets read
/// it: a page can hold several narratives and other components between
/// them, and all of them line up on the one composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NarrativeLayout {
    /// The effective companion: `Rail` exactly when the page's first
    /// narrative rendered the rail (`render_body`), so the stylesheets never
    /// lay out a rail the markup does not hold.
    pub companion: NarrativeCompanion,
    pub side: NarrativeSide,
}

/// The page's book composition, as `(record index, reader index)`: a
/// navigator-bound record and its populated `layout="pages"` reader render as
/// one composition, the reader inside the record's article (the 2026-09-29
/// amendment of `dsp-incubator cpe/docs/adr/0012-one-member-navigator-declared-layout.md`). The
/// pairing itself is the boot facts `navigator`/`paired`, both set by `pairs_with_navigator`; a
/// page holds at most one of each component (`DuplicateSingletonComponent`). A missing
/// record or a zero-page compound is not nested: both render as siblings.
fn nested_reader(page: &cpe_core::ir::Page, components: &[ComponentData]) -> Option<(usize, usize)> {
    let detail_at = page
        .body
        .iter()
        .position(|c| matches!(c, Component::ResourceDetail { navigator: true, .. }));
    let reader_at = page.body.iter().position(|c| {
        matches!(
            c,
            Component::CompoundViewer {
                paired: true,
                layout: cpe_core::ir::CompoundLayout::Pages,
                ..
            }
        )
    });
    detail_at.zip(reader_at).filter(|&(d, r)| {
        matches!(components.get(d), Some(ComponentData::Detail(Some(_))))
            && matches!(components.get(r), Some(ComponentData::Compound(p)) if p.total > 0)
    })
}

/// The reader's `#cv-annotations` patch target for the shown page, or `None`
/// unless the page is a book composition ([`nested_reader`]) whose reader
/// has `annotations=#true`. The full page ([`render_body`]) and the
/// `_compound` fragment ([`render_compound_fragment`]) both call this, so the
/// two can never render a different register for the same state.
fn render_reader_annotations(
    page: &cpe_core::ir::Page,
    ctx: &RenderContext<'_>,
    components: &[ComponentData],
    site: &SiteConfig,
) -> Option<Markup> {
    let (d, r) = nested_reader(page, components)?;
    let Component::CompoundViewer { annotations: true, .. } = &page.body[r] else {
        return None;
    };
    let Component::ResourceDetail { title_property, .. } = &page.body[d] else {
        return None;
    };
    let (Some(ComponentData::Detail(Some(record))), Some(ComponentData::Compound(compound))) =
        (components.get(d), components.get(r))
    else {
        return None;
    };
    let member = compound.member.as_ref()?;
    let member_class_label = resolve_class_label(&site.class_labels, &member.class, &ctx.lang);
    let book_class_label = resolve_class_label(&site.class_labels, &record.class, &ctx.lang);
    let bookmark = annotation::Bookmark {
        class_label: &book_class_label,
        id: &record.id,
        title: values::resolve_title_str(record, title_property.as_deref(), &ctx.lang),
    };
    let links = links::LinkResolver::new(ctx.urls(), ctx.link_routes);
    Some(annotation::render_annotations_target(
        compound,
        &member_class_label,
        &bookmark,
        &links,
        &ctx.lang,
    ))
}

/// Render a `resource-list` component, given an externally-computed, optional
/// search-aware rendering surface.
///
/// Extracted out of `render_component`'s `Component::ResourceList` arm
/// so both callers share one path: `render_component`
/// itself (a lone, unpaired list — always `search: None`) and `render_body`
/// (a list paired with a `search-bar`, which computes a real `SearchSurface`
/// from cross-sibling data `render_component`'s generic one-component-at-a-
/// time dispatch cannot see — mirrors why `render_body` calls
/// `render_search_row` directly instead of going through `render_component`
/// for the search row itself).
fn render_list_component(
    c: &Component,
    ctx: &RenderContext<'_>,
    data: &ComponentData,
    site: &SiteConfig,
    search: Option<resource_list::SearchSurface<'_>>,
    narrowed: Option<resource_list::NarrowedSurface<'_>>,
) -> Markup {
    let Component::ResourceList {
        id,
        class,
        gloss,
        chips,
        grid,
        tile_ratio,
        order_name_label,
        order_name_desc_label,
        order_default_label,
        order_options,
        ..
    } = c
    else {
        unreachable!(
            "render_list_component is only ever called with a Component::ResourceList — \
             see both call sites (render_component's arm and render_body's group handling)"
        );
    };
    // Chrome-id ordering-label overrides (id-only, never literal
    // text), resolved through the SAME `pub` helpers the host's
    // `_facets`/`_sort`/`_search` fragment handlers call,
    // so the full-page and fragment paths are provably
    // identical: an absent attribute or an id that fails to resolve both
    // fall through to `None`, which `order_label_for_key` reads as "use the
    // `ChromeMsg` default" — never an empty label, and never the literal
    // attribute text. The declared `order-by` rows resolve the same way.
    let (order_name_label_text, order_name_desc_label_text, order_default_label_text) = resolve_order_name_labels(
        site,
        order_name_label.as_deref(),
        order_name_desc_label.as_deref(),
        order_default_label.as_deref(),
        &ctx.lang,
    );
    let order_rows = resolve_order_options(site, order_options, &ctx.lang);
    let resolved_gloss = gloss.as_ref().map(|g| resolve_gloss(site, g, &ctx.lang));
    match data {
        ComponentData::List(list_page) => {
            let resolver = links::LinkResolver::new(ctx.urls(), ctx.link_routes);
            let cfg = resource_list::ResourceListCfg {
                links: Some(resolver),
                current_path: ctx.current_path,
                current_query: ctx.current_query,
                // Pass the list's config id so the section can be a DataStar
                // swap target (id="<list-id>"). `None` when no id is declared.
                list_id: id.as_deref(),
                // Annotated-row config: gloss + chip properties.
                gloss: resolved_gloss.as_ref(),
                chips,
                // The raw class name — resolved to a display label by
                // `render_surf_head` itself, independent of `list.items`.
                class,
                class_labels: &site.class_labels,
                search,
                narrowed,
                media_thumbs: ctx.media_thumbs,
                grid: *grid,
                tile_ratio: *tile_ratio,
                order_name_label: order_name_label_text,
                order_name_desc_label: order_name_desc_label_text,
                order_default_label: order_default_label_text,
                order_options: &order_rows,
            };
            resource_list::render_resource_list(list_page, &cfg, &ctx.lang)
        }
        // No list data resolved for this slot — render nothing. In practice
        // the host always supplies data for a ResourceList component, but
        // fallback to empty is safer than a panic for a missing slot.
        _ => html! {},
    }
}

/// Render a `compound-viewer` component — the full-page arm of
/// [`render_component`] and [`render_compound_fragment`]'s viewer share it.
/// `emit_url_correction` is `true` only on the full page: a stale `?page=`
/// gets its `history.replaceState` there, never in the `_compound` fragment
/// (`compound_viewer::stale_url_correction`).
fn render_compound_component(
    c: &Component,
    ctx: &RenderContext<'_>,
    data: &ComponentData,
    site: &SiteConfig,
    emit_url_correction: bool,
) -> Markup {
    let Component::CompoundViewer {
        source,
        media,
        picker,
        layout,
        index,
        paired,
        member_links,
        annotations: _,
    } = c
    else {
        unreachable!("render_compound_component is only called for a CompoundViewer");
    };
    match data {
        ComponentData::Compound(compound_page) => {
            let resolver = links::LinkResolver::new(ctx.urls(), ctx.link_routes);
            // The member's media property: the `compound-viewer media="…"`
            // attribute when set, else
            // the class→media-property map
            // entry for the member's class — only resolvable once the
            // member is known, so always `None` on the empty branch.
            let member_media_property = compound_page.member.as_ref().and_then(|m| {
                media
                    .as_deref()
                    .or_else(|| ctx.media_properties.get(&m.class).map(String::as_str))
            });

            // Resolve the compound parent from `ctx.compound_parents`,
            // keyed by the SAME
            // source-id resolution the store's page builder (`query::page`)
            // uses for its CompoundViewer arm (literal id, or `$param` looked
            // up in `ctx.route_params`) — `CompoundPage` carries no source id
            // of its own. Non-empty lookup only matters on the empty branch;
            // the store only ever populates an entry when a compound
            // resolved to zero members.
            let source_id: Option<&str> = match source.strip_prefix('$') {
                Some(param_name) => ctx.route_params.iter().find(|(k, _)| k == param_name).map(|(_, v)| v.as_str()),
                None => Some(source.as_str()),
            };
            let parent = source_id.and_then(|id| ctx.compound_parents.get(id));

            // On the empty branch, resolve `media_property` from the
            // PARENT's class (the member-derived one above is always
            // `None` there) — a media-declaring parent renders
            // consistently with its own detail page.
            let media_property = member_media_property
                .or_else(|| parent.and_then(|p| ctx.media_properties.get(&p.class)).map(String::as_str));

            let cfg = compound_viewer::CompoundViewerCfg {
                links: Some(resolver),
                current_path: ctx.current_path,
                current_query: ctx.current_query,
                media_property,
                class_labels: &site.class_labels,
                property_labels: &site.property_labels,
                parent,
                picker: *picker,
                layout: *layout,
                index: *index,
                paired: *paired,
                date_precision_word: site.date_precision_word,
                // The `group` caption rule: an undeclared id is already
                // boot-flagged `UnknownGroupLabel`.
                member_links: member_links.as_ref().map(|l| {
                    let caption = chrome_lookup(site, &l.label, &ctx.lang)
                        .unwrap_or_else(|| cpe_components::chrome::ChromeMsg::PropertyGroupTitle.text(&ctx.lang));
                    (caption, l.properties.as_slice())
                }),
                emit_url_correction,
            };
            compound_viewer::render_compound_viewer(compound_page, &cfg, &ctx.lang)
        }
        // No compound data resolved for this slot — render nothing (same
        // defensive fallback as the ResourceList arm).
        _ => html! {},
    }
}

/// Dispatch a single page body component to its renderer.
///
/// `data` carries the pre-fetched payload for this component slot, pre-selected
/// by `render_page`. Non-data components (e.g. `SearchBar`) match
/// `ComponentData::Other` and ignore any data payload. A `Narrative` is never
/// dispatched here: `render_body` draws it.
///
/// This function receives the **pre-selected** `&ComponentData` for its own
/// index — it does NOT receive whole slices and does not re-zip.
fn render_component(
    c: &Component,
    ctx: &RenderContext<'_>,
    narrative: &NarrativeStore,
    data: &ComponentData,
    default_lang: &Lang,
    site: &SiteConfig,
) -> Markup {
    match c {
        // A narrative drawn here would lose its figure blocks and the page
        // rail, and its heading ids would restart at `sec-1` and repeat
        // those of the page's other narratives.
        Component::Narrative { .. } => unreachable!(
            "render_body draws every narrative itself (figure blocks, id offset, the page \
             rail) and no other caller dispatches one"
        ),
        Component::ResourceDetail { .. } => render_detail_component(c, ctx, narrative, data, default_lang, site, None),
        // A lone `ResourceList` (no sibling `search-bar`) is the only shape
        // `render_component` is ever reached for generically — a
        // search-paired list is rendered directly by `render_body` via
        // `render_list_component` with a real `SearchSurface`
        // (like the `render_search_row` bypass, for the
        // same reason: cross-sibling context `render_component` can't see).
        // `search: None` is therefore always correct here, but the call goes
        // through the shared helper regardless so the two call sites can
        // never drift.
        Component::ResourceList { .. } => render_list_component(c, ctx, data, site, None, None),
        Component::CompoundViewer { .. } => render_compound_component(c, ctx, data, site, true),
        Component::FacetPanel { target, .. } => match data {
            ComponentData::Facets(groups) => {
                let cfg = facet_panel::FacetPanelCfg {
                    list_id: target,
                    current_path: ctx.current_path,
                    current_query: ctx.current_query,
                };
                facet_panel::render_facet_panel(groups, &cfg, &ctx.lang)
            }
            // No facet data resolved for this slot — render nothing (same
            // defensive fallback as the ResourceList/CompoundViewer arms).
            _ => html! {},
        },
        Component::SearchBar { placeholder, .. } => {
            // Search-bar renders purely from config + `ctx` — no
            // `ComponentData` variant of its own (`data` is ignored). In
            // practice `render_body`'s grouping (`group_at`) always
            // intercepts a `SearchBar` slot directly via `render_search_row`
            // (so it can decide `standalone` per its facet-pairing), so this
            // arm is reached only defensively (e.g. a `SearchBar` rendered
            // outside `render_body`'s grouping logic) — `standalone: true` is
            // the safer default in that case (a lone search row with no
            // enclosing `.browse.page` still gets its own centering). `scope`
            // is intentionally NOT read here — `resource_list.rs` resolves it.
            let placeholder_text = placeholder.as_deref().and_then(|id| chrome_lookup(site, id, &ctx.lang));
            let cfg = search_bar::SearchBarCfg {
                action_path: ctx.current_path,
                current_query: ctx.current_query,
                placeholder: placeholder_text,
                standalone: true,
            };
            search_bar::render_search_bar(&cfg, &ctx.lang)
        }
        // Per-project components render through the `(project, name)`
        // registry (`component::project`,
        // dsp-incubator cpe/docs/adr/0011-per-project-components.md). Every
        // per-project variant is named in this one arm — never a `_ =>`
        // catch-all — so a future generic variant still has to get its own
        // render arm to compile.
        Component::Timeline { .. }
        | Component::WoodcutCases { .. }
        | Component::EditionCards { .. }
        | Component::StripRow { .. } => project::render(c, ctx, data, site),
    }
}

/// The pre-rendered pages reader `render_body` injects into its record, with
/// the reader's annotation patch target when it has one.
struct NestedReader {
    reader: Markup,
    annotations: Option<Markup>,
}

/// Render a `resource-detail` component, with the paired pages reader
/// `render_body` injected (`nested: Some`) or none.
///
/// Extracted out of `render_component`'s `Component::ResourceDetail` arm so
/// both callers share one path: `render_component` itself (always `nested:
/// None`) and `render_body`, which alone sees the sibling `compound-viewer`
/// and pre-renders it (mirrors `render_list_component`).
fn render_detail_component(
    c: &Component,
    ctx: &RenderContext<'_>,
    narrative: &NarrativeStore,
    data: &ComponentData,
    default_lang: &Lang,
    site: &SiteConfig,
    nested: Option<NestedReader>,
) -> Markup {
    let (reader, annotations) = match nested {
        Some(n) => (Some(n.reader), n.annotations),
        None => (None, None),
    };
    let Component::ResourceDetail {
        id,
        show_properties,
        title_property,
        class,
        groups,
        narrative_for,
        navigator,
        incoming_links,
        head,
        ..
    } = c
    else {
        unreachable!(
            "render_detail_component is only ever called with a Component::ResourceDetail — \
             see both call sites (render_component's arm and render_body's reader pairing)"
        );
    };
    match data {
        ComponentData::Detail(Some(resource)) => {
            let resolver = links::LinkResolver::new(ctx.urls(), ctx.link_routes);
            // The media link property —
            // resolved here (render_component has the resource's class)
            // and threaded via cfg so render_resource_detail can exclude
            // it from the plain `.props` list (it renders separately via
            // `ctx.media_items`). `render_property_block` itself never
            // learns the class.
            let media_property = ctx.media_properties.get(&resource.class).map(String::as_str);
            // Resolve each group's `label=` chrome id for this
            // language, falling back to the built-in cap (an undeclared id
            // is already boot-flagged as `UnknownGroupLabel`). A local the
            // cfg borrows — it outlives `render_resource_detail` below.
            let resolved_groups: Vec<resource_detail::GroupCfg<'_>> = groups
                .iter()
                .map(|g| {
                    let caption = chrome_lookup(site, &g.label, &ctx.lang)
                        .unwrap_or_else(|| cpe_components::chrome::ChromeMsg::PropertyGroupTitle.text(&ctx.lang));
                    // `empty=`: an undeclared id is already
                    // boot-flagged `UnknownGroupEmptyLabel`; it renders
                    // as if unset.
                    let empty = g.empty.as_deref().and_then(|id| chrome_lookup(site, id, &ctx.lang));
                    // The closing links row, only when the
                    // record has resource-level partners; an undeclared
                    // caption is already boot-flagged `UnknownGroupLinksLabel`.
                    let links = g.links_label.as_deref().filter(|_| !ctx.resource_links.is_empty()).map(|id| {
                        resource_detail::GroupLinks {
                            caption: chrome_lookup(site, id, &ctx.lang)
                                .unwrap_or_else(|| cpe_components::chrome::ChromeMsg::AnnoRelated.text(&ctx.lang)),
                            links: ctx.resource_links,
                        }
                    });
                    resource_detail::GroupCfg { caption, properties: g.properties.as_slice(), empty, links }
                })
                .collect();
            // A `narrative-for` entry naming the RENDERED resource's
            // id attaches its narrative file as the record's intro. The
            // content resolves through the one shared 3-rung chain
            // (requested → default variant → base) and renders through
            // the wrapper-free Markdown pipeline inside `div.prose` —
            // never `render_narrative`/`render_editorial`, which wrap in
            // `article.editorial.page` and would nest a second `.page`
            // root inside `article.record.page`. A miss (no entry, or —
            // past boot validation, unreachable — no content) ⇒ no intro.
            let intro = narrative_for
                .iter()
                .find(|entry| entry.resource_id == resource.id)
                .and_then(|entry| {
                    cpe_components::component::narrative::resolve_narrative_content(
                        &entry.file,
                        &ctx.lang,
                        default_lang,
                        narrative,
                    )
                })
                .map(|src| {
                    html! {
                        div.prose {
                            ({
                                cpe_components::component::markdown::render_markdown_fragment(
                                    ctx.base,
                                    src,
                                )
                            })
                        }
                    }
                });
            // The incoming-links caption — the spec's `label=` chrome
            // id for this language, else the SOURCE class's capitalized
            // plural (a list of Pages reads "Seiten"; an undeclared id is
            // already boot-flagged `UnknownIncomingLinksLabel`). Only
            // resolved when the spec is declared; the list itself renders
            // only when the store also resolved rows (`ctx.incoming_links`).
            let incoming_caption: Option<String> = incoming_links.as_ref().map(|spec| {
                chrome_lookup(site, &spec.label, &ctx.lang)
                    .map(str::to_owned)
                    .unwrap_or_else(|| resolve_class_plural_capitalized(&site.class_labels, &spec.class, &ctx.lang))
            });
            // The grouped list's `note-label` chrome id,
            // resolved for this language like every other chrome id (an
            // undeclared id is already boot-flagged
            // `UnknownIncomingLinksNoteLabel` ⇒ no note). `{n}` is
            // substituted by the renderer, which knows the row count.
            let incoming_note: Option<&str> = incoming_links
                .as_ref()
                .and_then(|spec| spec.grouping.as_ref())
                .and_then(|g| g.note_label.as_deref())
                .and_then(|id| chrome_lookup(site, id, &ctx.lang));
            let incoming = match (incoming_links.as_ref(), incoming_caption.as_deref()) {
                (Some(spec), Some(caption)) => ctx.incoming_links.map(|links| resource_detail::IncomingCfg {
                    caption,
                    links,
                    properties: &spec.properties,
                    grouping: spec.grouping.as_ref(),
                    note: incoming_note,
                }),
                _ => None,
            };
            let cfg = resource_detail::ResourceDetailCfg {
                id: id.as_deref().unwrap_or(&resource.id),
                title_property: title_property.as_deref(),
                show_properties: show_properties.as_deref().unwrap_or(&[]),
                media_property,
                selected_image: ctx.selected_image,
                strip_open: ctx.strip_open,
                current_path: ctx.current_path,
                current_query: ctx.current_query,
                media_items: ctx.media_items,
                compound_members: ctx.compound_members,
                links: Some(resolver),
                property_labels: &site.property_labels,
                class_labels: &site.class_labels,
                groups: &resolved_groups,
                intro,
                navigator: *navigator,
                reader,
                annotations,
                incoming,
                date_precision_word: site.date_precision_word,
                head: head.as_ref().map(|h| resource_detail::ResolvedHead {
                    alt_title: h.alt_title.as_deref(),
                    author: h.author.as_deref(),
                    imprint: resolve_parts(site, &h.imprint, &ctx.lang),
                }),
            };
            resource_detail::render_resource_detail(resource, cfg, &ctx.lang)
        }
        _ => resource_detail::render_not_found(class.as_deref(), ctx.urls(), site, ctx.class_list_routes, &ctx.lang),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cpe_core::ir::{
        ChromeString, ClassLabel, NavItem, PropertyLabel, Representation, RepresentationKind, ResourceListPage,
        SiteConfig,
    };
    use cpe_core::labels::{resolve_property_label, OrderRow, ResolvedGloss, ResolvedGlossPart};
    use cpe_core::lang::resolve;

    use super::*;

    /// Empty class-routes map for tests that don't need link resolution.
    /// All `RenderContext` literals in tests pass `&EMPTY_ROUTES` so they
    /// compile after the `link_routes` field was added. Link rendering is
    /// tested separately in `resource_detail` tests.
    static EMPTY_ROUTES: std::sync::LazyLock<HashMap<String, String>> = std::sync::LazyLock::new(HashMap::new);

    /// Empty media-items slice, mirroring `EMPTY_ROUTES` above
    /// (`RenderContext.media_items`).
    static EMPTY_MEDIA: std::sync::LazyLock<Vec<Resource>> = std::sync::LazyLock::new(Vec::new);

    /// Empty compound-parents map, mirroring `EMPTY_ROUTES` above
    /// (`RenderContext.compound_parents`).
    /// `EMPTY_ROUTES` (a `HashMap<String, String>`) doubles as the
    /// `class_list_routes` placeholder in every `RenderContext` literal below.
    static EMPTY_PARENTS: std::sync::LazyLock<HashMap<String, Resource>> = std::sync::LazyLock::new(HashMap::new);

    /// Empty media-thumbs map, mirroring `EMPTY_ROUTES` above — its own const
    /// because `RenderContext.media_thumbs` is `HashMap<String, ir::MediaThumb>`,
    /// not `HashMap<String, String>` (carries the width-clamp's stored width).
    static EMPTY_MEDIA_THUMBS: std::sync::LazyLock<HashMap<String, cpe_core::ir::MediaThumb>> =
        std::sync::LazyLock::new(HashMap::new);

    // share_href_for_reader

    /// A resolved reader member appends no `#compound-viewer` — the fn is
    /// unconditionally anchor-free, regardless of `reader_member`.
    #[test]
    fn share_href_for_reader_no_anchor_when_member_resolved() {
        let href = share_href_for_reader("https://example.com", "/incunabula/books/b1", "page=b1-a1r", Some("b1-a1r"));
        assert_eq!(href, "https://example.com/incunabula/books/b1?page=b1-a1r");
    }

    #[test]
    fn share_href_for_reader_no_anchor_on_member_one() {
        let href = share_href_for_reader("https://example.com", "/incunabula/books/b1", "", None);
        assert_eq!(href, "https://example.com/incunabula/books/b1");
    }

    #[test]
    fn share_href_for_reader_no_anchor_when_resolved_member_is_empty() {
        let href = share_href_for_reader("https://example.com", "/incunabula/books/b1", "", Some(""));
        assert_eq!(href, "https://example.com/incunabula/books/b1");
    }

    // resolved_reader_member

    fn compound_at(index: u32) -> ComponentData {
        use cpe_core::ir::{CompoundPage, MemberHeader};
        ComponentData::Compound(CompoundPage {
            member: None,
            index,
            total: 2,
            members: vec![
                MemberHeader {
                    id: "M_1".to_owned(),
                    class: "Page".to_owned(),
                    name: "One".to_owned(),
                },
                MemberHeader {
                    id: "M_2".to_owned(),
                    class: "Page".to_owned(),
                    name: "Two".to_owned(),
                },
            ],
            member_media: vec![],
            stale_page: None,
            annotations: vec![],
            annotated_members: std::collections::BTreeSet::new(),
        })
    }

    /// Member 1 always resolves to `None` — the bare route URL is member 1's
    /// own canonical URL uniformly (narrative-reader pages and
    /// navigator-keyed pages alike; the bare URL already renders the record
    /// AND the reader at member 1, so there is no collision to avoid). See
    /// `resolved_reader_member`'s doc comment.
    #[test]
    fn resolved_reader_member_omits_member_one() {
        let components = vec![compound_at(1)];
        assert_eq!(
            resolved_reader_member(&components),
            None,
            "member 1 must always omit its id — the bare URL is its own canonical URL"
        );
    }

    /// Member 2 (and beyond) always resolves.
    #[test]
    fn resolved_reader_member_resolves_non_first_member() {
        let components = vec![compound_at(2)];
        assert_eq!(resolved_reader_member(&components), Some("M_2"));
    }

    // nav_item_is_active

    #[test]
    fn nav_item_is_active_exact_match() {
        assert!(nav_item_is_active("/books", "/books"));
    }

    #[test]
    fn nav_item_is_active_detail_route_marks_parent_nav_item() {
        assert!(nav_item_is_active("/books", "/books/:id"));
    }

    #[test]
    fn nav_item_is_active_home_route_never_prefix_matches() {
        assert!(!nav_item_is_active("/", "/books"));
        assert!(!nav_item_is_active("/", "/books/:id"));
    }

    #[test]
    fn nav_item_is_active_rejects_same_prefix_sibling_route() {
        assert!(!nav_item_is_active("/books", "/booksX"));
    }

    #[test]
    fn nav_item_is_active_unrelated_routes_are_not_active() {
        assert!(!nav_item_is_active("/books", "/strips"));
        assert!(!nav_item_is_active("/books", "/strips/:id"));
    }

    // test helpers

    /// The three states of an order-name label override at the resolution seam itself:
    /// absent, a declared chrome id, and — the branch the lower-layer
    /// `order_label_for_key` tests cannot reach, because they are handed an
    /// already-collapsed `None` — an attribute naming an id that is NOT
    /// declared in the project's `chrome {}` block (a config typo). That last
    /// case must collapse to `None`, so `order_label_for_key` falls back to its
    /// `ChromeMsg` default; a `Some("")` would render an empty ordering row.
    #[test]
    fn resolve_order_name_labels_unresolvable_id_collapses_to_none() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);

        assert_eq!(
            resolve_order_name_labels(&site, None, None, None, &lang),
            (None, None, None),
            "an absent attribute resolves to None on every slot"
        );

        assert_eq!(
            resolve_order_name_labels(&site, Some("nav-home"), Some("nav-home"), Some("nav-home"), &lang),
            (Some("Home"), Some("Home"), Some("Home")),
            "a declared chrome id resolves to its translation for the language"
        );

        assert_eq!(
            resolve_order_name_labels(
                &site,
                Some("order-typo-not-declared"),
                Some("order-typo-not-declared"),
                Some("order-typo-not-declared"),
                &lang,
            ),
            (None, None, None),
            "an id absent from the chrome block must collapse to None (the \
             ChromeMsg default), never to the raw id or an empty string"
        );
    }

    // Composed gloss resolution

    /// A unit resolves to its template for a language that has one, and to
    /// `None` (bare digits at render time, never the raw id) for one that
    /// does not; the legacy form passes through untouched.
    #[test]
    fn resolve_gloss_unit_per_language_and_legacy_passthrough() {
        let mut site = make_alice_site();
        site.chrome.push(cpe_core::ir::ChromeString {
            id: "gloss-extent".to_owned(),
            translations: vec![("de".to_owned(), "{n} Seiten".to_owned())],
        });
        let gloss = cpe_core::ir::Gloss::Composed {
            parts: vec![cpe_core::ir::GlossPart {
                property: "extent".to_owned(),
                unit: Some("gloss-extent".to_owned()),
            }],
            teaser: Some("teaser".to_owned()),
        };
        let resolved = |lang: &str| resolve_gloss(&site, &gloss, &resolve(&[lang.to_owned()], &site));
        assert_eq!(
            resolved("de"),
            ResolvedGloss::Composed {
                parts: vec![ResolvedGlossPart { property: "extent", unit: Some("{n} Seiten") }],
                teaser: Some("teaser"),
            }
        );
        assert_eq!(
            resolved("en"),
            ResolvedGloss::Composed {
                parts: vec![ResolvedGlossPart { property: "extent", unit: None }],
                teaser: Some("teaser"),
            }
        );
        let legacy = cpe_core::ir::Gloss::Property("description".to_owned());
        assert_eq!(
            resolve_gloss(&site, &legacy, &resolve(&["en".to_owned()], &site)),
            ResolvedGloss::Property("description")
        );
    }

    // Declared `order-by` options

    fn make_order_options() -> Vec<cpe_core::ir::OrderOption> {
        vec![
            cpe_core::ir::OrderOption {
                key: "date".to_owned(),
                property: "pub-date".to_owned(),
                direction: cpe_core::ir::SortDir::Asc,
                label: "nav-home".to_owned(),
            },
            cpe_core::ir::OrderOption {
                key: "extent".to_owned(),
                property: "extent".to_owned(),
                direction: cpe_core::ir::SortDir::Desc,
                label: "order-typo-not-declared".to_owned(),
            },
        ]
    }

    /// The one resolver: a declared chrome id resolves for the language; an
    /// id that fails to resolve falls back to the option's KEY (never an
    /// empty label, never the raw id); config order is kept.
    #[test]
    fn resolve_order_options_resolves_labels_and_falls_back_to_key() {
        let site = make_alice_site();
        let options = make_order_options();
        let de = resolve(&["de".to_owned()], &site);
        assert_eq!(
            resolve_order_options(&site, &options, &de),
            vec![
                OrderRow { key: "date", label: "Startseite" },
                OrderRow { key: "extent", label: "extent" },
            ]
        );
        assert!(resolve_order_options(&site, &[], &de).is_empty());
    }

    /// Through `render_component` (the full-page path, which calls the `pub`
    /// resolvers): a declared key marks its row
    /// `aria-current`, the panel has six rows, and the resolved label shows.
    #[test]
    fn resource_list_render_marks_declared_order_row_current() {
        use cpe_core::ir::{Pagination, ResourceListPage};
        let site = make_alice_site();
        let mut page = make_nonfaceted_browse_page();
        let Component::ResourceList { order_options, .. } = &mut page.body[0] else {
            unreachable!("make_nonfaceted_browse_page's body is one ResourceList");
        };
        *order_options = make_order_options();
        let lang = resolve(&["de".to_owned()], &site);
        let ctx = make_browse_ctx(lang, "/alice-in-daschland/chapters", Some("sort=date"));
        let default_lang = resolve(&["en".to_owned()], &site);
        let data = ComponentData::List(ResourceListPage {
            items: vec![Resource {
                id: "SC_001".to_owned(),
                class: "StoryChapter".to_owned(),
                properties: vec![],
                representations: vec![],
            }],
            total: 1,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: None,
        });
        let out =
            render_component(&page.body[0], &ctx, &make_alice_narrative(), &data, &default_lang, &site).into_string();
        assert_eq!(out.matches("orderctl__opt").count(), 6, "{out}");
        assert!(
            out.contains(r#"value="date" aria-current="true">Startseite</button>"#),
            "the declared row must be current and carry its resolved label: {out}"
        );
        assert!(
            out.contains(r#"value="extent">extent</button>"#),
            "an unresolvable label id falls back to the key: {out}"
        );
    }

    /// The browse Copy link keeps a declared `?sort=` key (the call site
    /// derives the keys from the page's ResourceList) and still drops an
    /// undeclared token — through the real `render_browse_breadcrumb_for_page`
    /// → `compose_canonical_query` path.
    #[test]
    fn browse_breadcrumb_share_link_keeps_declared_sort_key_only() {
        use cpe_core::ir::{Pagination, ResourceListPage};
        let site = make_alice_site();
        let mut page = make_nonfaceted_browse_page();
        let Component::ResourceList { order_options, .. } = &mut page.body[0] else {
            unreachable!("make_nonfaceted_browse_page's body is one ResourceList");
        };
        *order_options = make_order_options();
        let components = || {
            vec![ComponentData::List(ResourceListPage {
                items: vec![],
                total: 13,
                page: 1,
                pagination: Pagination::PerPage(24),
                unfiltered_total: None,
            })]
        };
        let href_for = |query: &'static str| {
            let lang = resolve(&["en".to_owned()], &site);
            let ctx = make_browse_ctx(lang, "/alice-in-daschland/chapters", Some(query));
            render_browse_breadcrumb_for_page(&page, &ctx, &components(), &site)
                .expect("id-bearing ResourceList must yield a browse breadcrumb")
                .into_string()
        };
        let kept = href_for("sort=extent");
        assert!(
            kept.contains(r#"href="http://localhost:8080/alice-in-daschland/chapters?sort=extent""#),
            "a declared key must survive the Copy link: {kept}"
        );
        let dropped = href_for("sort=bogus");
        assert!(
            dropped.contains(r#"href="http://localhost:8080/alice-in-daschland/chapters""#),
            "an undeclared token must still be dropped: {dropped}"
        );
    }

    fn make_alice_site() -> SiteConfig {
        SiteConfig {
            title: "Alice in DaSCHland".to_owned(),
            description: Some("A DaSCH showcase project.".to_owned()),
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["en".to_owned(), "de".to_owned()],
            default_language: "en".to_owned(),
            nav: vec![
                NavItem {
                    label: "nav-home".to_owned(), // chrome-id reference
                    route: "/".to_owned(),
                },
                NavItem {
                    label: "About".to_owned(), // literal label (no chrome entry)
                    route: "/about".to_owned(),
                },
            ],
            chrome: vec![
                ChromeString {
                    id: "nav-home".to_owned(),
                    translations: vec![
                        ("en".to_owned(), "Home".to_owned()),
                        ("de".to_owned(), "Startseite".to_owned()),
                    ],
                },
                // The search-bar `scope` attribute is a
                // project-declared chrome-string id (mirrors `placeholder`) —
                // this project-authored literal is what `render_surf_head`'s
                // scope note renders verbatim, NOT computed from `searches`.
                ChromeString {
                    id: "search-scope-note".to_owned(),
                    translations: vec![
                        ("en".to_owned(), "matching Name and Description".to_owned()),
                        ("de".to_owned(), "in Name und Beschreibung".to_owned()),
                    ],
                },
                // A DISTINCT scope-note id/text for the
                // "non-faceted full-text scope" state (searches over
                // name/description/full-text) — kept
                // separate from `search-scope-note` above so the two states'
                // snapshots are never accidentally identical.
                ChromeString {
                    id: "search-scope-note-fulltext".to_owned(),
                    translations: vec![
                        ("en".to_owned(), "matching title, description and full text".to_owned()),
                        ("de".to_owned(), "in Titel, Beschreibung und Volltext".to_owned()),
                    ],
                },
            ],
            property_labels: vec![],
            class_labels: vec![],
            theme: cpe_core::theme::ThemeKnob::default(),
            date_precision_word: false,
            footer_attribution: None,
            not_found_browse: vec![],
            narrative_companion: Default::default(),
            narrative_side: Default::default(),
            text_align: Default::default(),
        }
    }

    fn make_alice_page() -> cpe_core::ir::Page {
        cpe_core::ir::Page {
            id: "home".to_owned(),
            route: "/".to_owned(),
            params: vec![],
            body: vec![Component::Narrative { file: "home.md".to_owned(), companion: None, side: None }],
            figure_slots: Vec::new(),
        }
    }

    fn make_alice_narrative() -> NarrativeStore {
        let pairs: Vec<(String, String)> = vec![
            ("home.md".to_owned(), "# Welcome\n\nThis is the English narrative.".to_owned()),
            (
                "home.de.md".to_owned(),
                "# Willkommen\n\nDies ist die deutsche Fassung.".to_owned(),
            ),
        ];
        NarrativeStore::from_pairs(&pairs)
    }

    // Layer-1: compose_page_title (dedupe rule)

    /// When the derived page title equals site.title (exact, trimmed,
    /// case-sensitive), the result is site.title alone — no "Site — Site".
    #[test]
    fn compose_page_title_dedupes_equal_to_site_title() {
        let site = make_alice_site();
        let page = make_alice_page(); // narrative with "# Welcome" heading
        let narrative = {
            // Use a heading that matches site.title exactly.
            let pairs: Vec<(String, String)> = vec![(
                "home.md".to_owned(),
                // site.title is "Alice in DaSCHland"
                "# Alice in DaSCHland\n\nSome text.".to_owned(),
            )];
            NarrativeStore::from_pairs(&pairs)
        };
        let lang = resolve(&["en".to_owned()], &site);
        let title = compose_page_title(&site, &page, &narrative, &lang, &lang, &[]);
        assert_eq!(
            title, "Alice in DaSCHland",
            "when heading == site.title, result must be site.title alone (no duplicate)"
        );
    }

    /// A heading that differs from site.title (even by case) produces the
    /// "Site — Heading" form (exact case-sensitive match only dedupes).
    #[test]
    fn compose_page_title_keeps_near_miss_case_different() {
        let site = make_alice_site(); // title = "Alice in DaSCHland"
        let page = make_alice_page();
        let narrative = {
            let pairs: Vec<(String, String)> = vec![(
                "home.md".to_owned(),
                // Differs by case: "alice in daschland" ≠ "Alice in DaSCHland"
                "# alice in daschland\n\nSome text.".to_owned(),
            )];
            NarrativeStore::from_pairs(&pairs)
        };
        let lang = resolve(&["en".to_owned()], &site);
        let title = compose_page_title(&site, &page, &narrative, &lang, &lang, &[]);
        assert_eq!(
            title, "Alice in DaSCHland \u{2014} alice in daschland",
            "a case-differing near-miss must NOT be deduped"
        );
    }

    /// A genuinely different heading produces "Site — Heading".
    #[test]
    fn compose_page_title_different_heading_produces_combined() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative(); // "# Welcome" heading in EN
        let lang = resolve(&["en".to_owned()], &site);
        let title = compose_page_title(&site, &page, &narrative, &lang, &lang, &[]);
        assert_eq!(
            title, "Alice in DaSCHland \u{2014} Welcome",
            "a different heading must produce 'Site \u{2014} Heading'"
        );
    }

    /// A leading `###` before the `# H1` is the design's **eyebrow/kicker** —
    /// a label, which `component::editorial` renders as
    /// `p.eyebrow.editorial__kicker`, not the page's title. The composed
    /// `<title>` must therefore read the `<h1>` and never the kicker.
    ///
    /// This pins the **wiring** that `util`'s own unit tests cannot reach:
    /// `resolved_narrative_component_title` (and `compose_page_title`'s
    /// rung 3) must call `util::page_title_heading`, the `<h1>`-preferring
    /// helper — reverting either to the level-blind `first_heading` brings
    /// the observed defect back (Incunabula's home `<title>` reading
    /// "… — Basler Frühdrucke · 1471–1504", `/incunabula/narrenschiff`'s
    /// title reading "Showcase I") while every `util`-level test still passes.
    #[test]
    fn compose_page_title_prefers_h1_over_leading_kicker() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = {
            let pairs: Vec<(String, String)> = vec![(
                "home.md".to_owned(),
                // Leading `###` kicker above the real `# H1`.
                "### Showcase I\n\n# The Narrenschiff\n\nBody.".to_owned(),
            )];
            NarrativeStore::from_pairs(&pairs)
        };
        let lang = resolve(&["en".to_owned()], &site);
        let title = compose_page_title(&site, &page, &narrative, &lang, &lang, &[]);
        assert_eq!(
            title, "Alice in DaSCHland \u{2014} The Narrenschiff",
            "the page title must come from the <h1>, never from the leading \
             `###` kicker (got: {title:?})"
        );
        assert!(
            !title.contains("Showcase I"),
            "the kicker text must never appear in the page title (got: {title:?})"
        );
    }

    /// When the FIRST body component is a resource-detail whose resolved
    /// resource title exactly equals `site.title`, the result must be
    /// `site.title` alone — the dedupe rule fires on the resource-detail
    /// branch, not only the narrative branch.
    #[test]
    fn compose_page_title_resource_detail_dedupes_when_title_equals_site_title() {
        use cpe_core::ir::{Property, PropertyValue, Resource};
        let site = make_alice_site(); // title = "Alice in DaSCHland"
        let page = make_character_page(); // ResourceDetail with title_property = Some("name")

        // Build a resource whose "name" property in EN is exactly "Alice in DaSCHland"
        // — the same string as site.title — so dedupe must fire.
        let resource = Resource {
            id: "CH_ALICE".to_owned(),
            class: "Character".to_owned(),
            properties: vec![Property {
                name: "name".to_owned(),
                values: vec![PropertyValue::Text {
                    text: "Alice in DaSCHland".to_owned(),
                    lang: Some("en".to_owned()),
                }],
            }],
            representations: vec![],
        };

        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let components: Vec<ComponentData> = vec![ComponentData::Detail(Some(resource))];

        let title = compose_page_title(&site, &page, &narrative, &lang, &lang, &components);
        assert_eq!(
            title, "Alice in DaSCHland",
            "resource-detail title equal to site.title must be deduped to site.title alone \
             (got: {title:?})"
        );
    }

    /// Rung 1b: an unknown id (the first resource-detail's slot is
    /// `Detail(None)`, the not-found body) titles the tab with that body's own h1 —
    /// class-specific when the detail declares a `class`, class-neutral
    /// otherwise — localized, after the site title.
    #[test]
    fn compose_page_title_unknown_resource_names_the_not_found_heading() {
        let site = make_alice_site();
        let narrative = make_alice_narrative();
        let unresolved = vec![ComponentData::Detail(None)];
        let mut page = make_character_page();
        let Component::ResourceDetail { class, .. } = &mut page.body[0] else {
            unreachable!("make_character_page's body is one ResourceDetail");
        };
        *class = Some("Character".to_owned());

        let en = resolve(&["en".to_owned()], &site);
        assert_eq!(
            compose_page_title(&site, &page, &narrative, &en, &en, &unresolved),
            "Alice in DaSCHland \u{2014} There is no Character at this address."
        );
        let de = resolve(&["de".to_owned()], &site);
        assert_eq!(
            compose_page_title(&site, &page, &narrative, &de, &en, &unresolved),
            format!(
                "Alice in DaSCHland \u{2014} {}",
                resource_detail::not_found_heading(Some("Character"), &[], &de)
            )
        );

        let class_less = make_character_page(); // class: None
        assert_eq!(
            compose_page_title(&site, &class_less, &narrative, &en, &en, &unresolved),
            "Alice in DaSCHland \u{2014} There is nothing at this address."
        );
    }

    /// When no heading exists, the result is site.title alone.
    #[test]
    fn compose_page_title_no_heading_returns_site_title() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = {
            let pairs: Vec<(String, String)> = vec![("home.md".to_owned(), "Just prose, no heading.".to_owned())];
            NarrativeStore::from_pairs(&pairs)
        };
        let lang = resolve(&["en".to_owned()], &site);
        let title = compose_page_title(&site, &page, &narrative, &lang, &lang, &[]);
        assert_eq!(title, "Alice in DaSCHland", "no heading → result must be site.title alone");
    }

    /// When the requested language differs from the default and
    /// only the default-language variant is present (no requested-language
    /// variant, no bare base file — the "variant-only" case),
    /// `compose_page_title` must derive its title from the SAME chain
    /// `render_narrative` uses (`resolve_narrative_content`), landing on the
    /// default-language variant's heading — not falling through to
    /// `site.title` alone. This is the regression the shared-resolver
    /// refactor (mod.rs:288-291's old duplicated chain) exists to prevent:
    /// a requested≠default divergence in the wiring (e.g. swapped
    /// `lang`/`default_lang` arguments) would make this test fail.
    #[test]
    fn compose_page_title_falls_back_to_default_variant_when_requested_absent() {
        let site = make_alice_site(); // languages en/de, default "en"
        let page = make_alice_page(); // narrative file="home.md"
        let narrative = {
            // Only the default-language ("en") variant exists — no "home.de.md"
            // (the requested variant) and no bare "home.md" (the base file).
            let pairs: Vec<(String, String)> = vec![(
                "home.en.md".to_owned(),
                "# Fallback Heading\n\nEnglish default-variant content.".to_owned(),
            )];
            NarrativeStore::from_pairs(&pairs)
        };
        let requested_lang = resolve(&["de".to_owned()], &site); // "de" — absent variant
        let default_lang = resolve(&[], &site); // "en" — falls back to default
        let title = compose_page_title(&site, &page, &narrative, &requested_lang, &default_lang, &[]);
        assert_eq!(
            title, "Alice in DaSCHland \u{2014} Fallback Heading",
            "requested-lang variant absent must fall back to the default-lang \
             variant's heading (same chain render_narrative uses), got: {title:?}"
        );
    }

    // page_heading (hidden-<h1> text derivation)

    /// Rung 1: a resolved resource-detail title wins over everything else.
    /// (Unreachable from `render_page`'s actual call site — a page with a
    /// `ResourceDetail` always has a visible h1 already, per
    /// `page_has_visible_h1` — but `page_heading` itself must still honor the
    /// shared first rung, per its doc comment.)
    #[test]
    fn page_heading_prefers_resolved_resource_title() {
        use cpe_core::ir::{Property, PropertyValue, Resource};
        let site = make_alice_site();
        let page = make_character_page(); // route "/characters/:id", no nav match
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let resource = Resource {
            id: "CH_001".to_owned(),
            class: "Character".to_owned(),
            properties: vec![Property {
                name: "name".to_owned(),
                values: vec![PropertyValue::Text { text: "Alice".to_owned(), lang: Some("en".to_owned()) }],
            }],
            representations: vec![],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(Some(resource))];
        let heading = page_heading(&site, &page, &narrative, &lang, &lang, &components);
        assert_eq!(heading, "Alice");
    }

    /// Rung 2: no resolved resource, but `page.route` matches a declared nav
    /// item — its label wins over any narrative heading.
    #[test]
    fn page_heading_uses_nav_label_when_route_matches() {
        let site = make_alice_site(); // nav: "/" → "nav-home" (chrome), "/about" → "About"
                                      // (literal)
        let page = cpe_core::ir::Page {
            id: "about".to_owned(),
            route: "/about".to_owned(),
            params: vec![],
            body: vec![],
            figure_slots: Vec::new(),
        };
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let heading = page_heading(&site, &page, &narrative, &lang, &lang, &[]);
        assert_eq!(heading, "About", "must use the matching nav item's label, got: {heading:?}");
    }

    /// Rung 2 (chrome-resolved label): the home route's nav label resolves
    /// through `chrome_lookup` ("nav-home" → "Home"), same as `shell_header`.
    #[test]
    fn page_heading_nav_label_resolves_chrome_string() {
        let site = make_alice_site();
        let page = cpe_core::ir::Page {
            id: "home".to_owned(),
            route: "/".to_owned(),
            params: vec![],
            body: vec![], // no ResourceDetail/Narrative — isolates the nav-label rung
            figure_slots: Vec::new(),
        };
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let heading = page_heading(&site, &page, &narrative, &lang, &lang, &[]);
        assert_eq!(heading, "Home");
    }

    /// Rung 3: no resource, no matching nav item — falls back to the first
    /// narrative component's first heading (any level — `page_heading` only
    /// derives text here, `page_has_visible_h1` decides visibility).
    #[test]
    fn page_heading_falls_back_to_narrative_heading_when_no_nav_match() {
        let site = make_alice_site();
        let page = cpe_core::ir::Page {
            id: "nomatch".to_owned(),
            route: "/nomatch".to_owned(), // not in site.nav
            params: vec![],
            body: vec![Component::Narrative { file: "home.md".to_owned(), companion: None, side: None }],
            figure_slots: Vec::new(),
        };
        let narrative = {
            let pairs: Vec<(String, String)> = vec![("home.md".to_owned(), "## Section Heading\n\nBody.".to_owned())];
            NarrativeStore::from_pairs(&pairs)
        };
        let lang = resolve(&["en".to_owned()], &site);
        let heading = page_heading(&site, &page, &narrative, &lang, &lang, &[]);
        assert_eq!(heading, "Section Heading");
    }

    /// Rung 4: nothing else resolves — falls back to `site.title`.
    #[test]
    fn page_heading_falls_back_to_site_title() {
        let site = make_alice_site();
        let page = cpe_core::ir::Page {
            id: "nomatch".to_owned(),
            route: "/nomatch".to_owned(),
            params: vec![],
            body: vec![],
            figure_slots: Vec::new(),
        };
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let heading = page_heading(&site, &page, &narrative, &lang, &lang, &[]);
        assert_eq!(heading, site.title);
    }

    // page_has_visible_h1

    #[test]
    fn page_has_visible_h1_true_for_resource_detail_component() {
        let page = make_character_page();
        let narrative = make_alice_narrative();
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        assert!(page_has_visible_h1(&page, &narrative, &lang, &lang));
    }

    #[test]
    fn page_has_visible_h1_true_for_narrative_starting_with_h1() {
        let page = make_alice_page(); // narrative file="home.md"
        let narrative = make_alice_narrative(); // "# Welcome" — H1
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        assert!(page_has_visible_h1(&page, &narrative, &lang, &lang));
    }

    #[test]
    fn page_has_visible_h1_false_for_narrative_starting_with_h2() {
        let page = cpe_core::ir::Page {
            id: "home".to_owned(),
            route: "/".to_owned(),
            params: vec![],
            body: vec![Component::Narrative { file: "home.md".to_owned(), companion: None, side: None }],
            figure_slots: Vec::new(),
        };
        let narrative = {
            let pairs: Vec<(String, String)> = vec![("home.md".to_owned(), "## Not an h1\n\nBody.".to_owned())];
            NarrativeStore::from_pairs(&pairs)
        };
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        assert!(!page_has_visible_h1(&page, &narrative, &lang, &lang));
    }

    /// A narrative whose first heading is level 2 but which
    /// contains a level-1 heading LATER in the document must still count as
    /// "has a visible h1" for the injection decision — checking only the
    /// FIRST heading's level would misclassify this page as headless and
    /// inject a hidden h1 IN ADDITION to the real (non-first) visible one,
    /// producing two `<h1>`s.
    #[test]
    fn page_has_visible_h1_true_when_h1_is_not_first_heading() {
        let page = cpe_core::ir::Page {
            id: "home".to_owned(),
            route: "/".to_owned(),
            params: vec![],
            body: vec![Component::Narrative { file: "home.md".to_owned(), companion: None, side: None }],
            figure_slots: Vec::new(),
        };
        let narrative = {
            let pairs: Vec<(String, String)> = vec![("home.md".to_owned(), "## intro\n\n# title".to_owned())];
            NarrativeStore::from_pairs(&pairs)
        };
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        assert!(page_has_visible_h1(&page, &narrative, &lang, &lang));
    }

    #[test]
    fn page_has_visible_h1_false_for_headless_resource_list_page() {
        let page = cpe_core::ir::Page {
            id: "characters".to_owned(),
            route: "/characters".to_owned(),
            params: vec![],
            body: vec![Component::ResourceList {
                id: Some("character-list".to_owned()),
                class: "Character".to_owned(),
                wheres: vec![],
                sorts: vec![],
                pagination: cpe_core::ir::Pagination::PerPage(24),
                gloss: None,
                chips: vec![],
                media: None,
                grid: false,
                tile_ratio: None,
                order_name_label: None,
                order_name_desc_label: None,
                order_default_label: None,
                order_name_property: None,
                order_options: vec![],
            }],
            figure_slots: Vec::new(),
        };
        let narrative = make_alice_narrative();
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        assert!(!page_has_visible_h1(&page, &narrative, &lang, &lang));
    }

    // Layer-1: chrome_lookup

    #[test]
    fn chrome_lookup_returns_translation_for_declared_id() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let result = chrome_lookup(&site, "nav-home", &lang);
        assert_eq!(result, Some("Home"), "expected Some(\"Home\") for en nav-home");
    }

    #[test]
    fn chrome_lookup_returns_de_translation() {
        let site = make_alice_site();
        let lang = resolve(&["de".to_owned()], &site);
        let result = chrome_lookup(&site, "nav-home", &lang);
        assert_eq!(result, Some("Startseite"), "expected Some(\"Startseite\") for de nav-home");
    }

    #[test]
    fn chrome_lookup_returns_none_for_unknown_id() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let result = chrome_lookup(&site, "does-not-exist", &lang);
        assert_eq!(result, None, "unknown chrome id must return None");
    }

    // Layer-1: site_title (chrome-id-or-literal)

    /// Alice's site with `site.title` declared as a chrome-string id
    /// (`site-title`) instead of a literal — the Incunabula shape.
    fn make_translated_title_site() -> SiteConfig {
        let mut site = make_alice_site();
        site.title = "site-title".to_owned();
        site.chrome.push(ChromeString {
            id: "site-title".to_owned(),
            translations: vec![
                ("en".to_owned(), "The Looking-Glass Archive".to_owned()),
                ("de".to_owned(), "Das Spiegelarchiv".to_owned()),
            ],
        });
        site
    }

    #[test]
    fn site_title_literal_is_language_independent() {
        let site = make_alice_site();
        for code in ["en", "de"] {
            let lang = resolve(&[code.to_owned()], &site);
            assert_eq!(
                site_title(&site, &lang),
                "Alice in DaSCHland",
                "a literal site.title renders verbatim in {code}"
            );
        }
    }

    #[test]
    fn site_title_chrome_id_resolves_per_language() {
        let site = make_translated_title_site();
        let en = resolve(&["en".to_owned()], &site);
        let de = resolve(&["de".to_owned()], &site);
        assert_eq!(site_title(&site, &en), "The Looking-Glass Archive");
        assert_eq!(site_title(&site, &de), "Das Spiegelarchiv");
    }

    /// Every chrome surface that shows the site name — `<title>`, the brand
    /// lockup and the breadcrumb's project crumb — resolves the chrome id; the
    /// raw id never leaks.
    #[test]
    fn render_page_localizes_site_title_in_title_brand_and_crumb() {
        let site = make_translated_title_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["de".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert!(
            out.contains("<title>Das Spiegelarchiv — Willkommen</title>"),
            "<title> must use the DE site title: {out}"
        );
        assert!(
            out.contains(r#"href="/alice-in-daschland/" title="Das Spiegelarchiv">Das Spiegelarchiv</a>"#),
            "the brand lockup must use the DE site title: {out}"
        );
        assert_eq!(
            out.matches("Das Spiegelarchiv").count(),
            4,
            "title, brand (text and `title`) and crumb each carry the DE site title: {out}"
        );
        assert!(!out.contains("site-title"), "the raw chrome id must never render: {out}");
    }

    /// Under a base, a page's links, the files it loads and its share link
    /// all start with the base, and none holds it twice.
    #[test]
    fn render_page_writes_the_base_in_front_of_every_url() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let base = cpe_core::url::Base::parse("/cpe").expect("/cpe is a base");
        let ctx = RenderContext {
            base: &base,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/cpe/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "https://repository.example",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert!(
            out.contains(r#"href="/cpe/alice-in-daschland/""#),
            "the project's start page: {out}"
        );
        assert!(out.contains(r#"href="/cpe/""#), "CPE's own start: {out}");
        assert!(
            out.contains(r#"href="/cpe/alice-in-daschland/_static/cpe/tokens.css""#),
            "a stylesheet: {out}"
        );
        assert!(
            out.contains(r#"src="/cpe/alice-in-daschland/_static/datastar.js""#),
            "a script: {out}"
        );
        assert!(
            out.contains(r#"href="https://repository.example/cpe/alice-in-daschland/""#),
            "the share link: {out}"
        );
        assert!(!out.contains("/cpe/cpe"), "the base is written once: {out}");
        for path in out.split('"').filter(|text| text.starts_with('/') && !text.starts_with("//")) {
            assert!(path.starts_with("/cpe/"), "{path} leaves the base: {out}");
        }
    }

    // Layer-1: resolve_property_label

    #[test]
    fn resolve_property_label_returns_translation_for_declared_hit() {
        // The declared translation is deliberately DIFFERENT from
        // `humanize_property_name("role")` ("Role") — a translation that
        // happened to equal the humanized fallback couldn't distinguish "the
        // declared label was used" from "the code always falls back to
        // humanize", i.e. it wouldn't catch a backwards implementation.
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let labels = vec![PropertyLabel {
            property: "role".to_owned(),
            translations: vec![
                ("en".to_owned(), "Character Role".to_owned()),
                ("de".to_owned(), "Rolle".to_owned()),
            ],
        }];
        let result = resolve_property_label(&labels, "role", &lang);
        assert_eq!(
            result, "Character Role",
            "expected declared en translation, not the humanized fallback \
             ('Role') — proves the labels slice actually won"
        );
        assert_ne!(
            result,
            cpe_core::util::humanize_property_name("role"),
            "sanity check: declared translation must differ from the humanized \
             fallback, or this test can't distinguish the two code paths"
        );
    }

    #[test]
    fn resolve_property_label_falls_back_to_humanize_for_missing_lang() {
        let site = make_alice_site();
        let lang = resolve(&["fr".to_owned()], &site); // no fr declared -> falls back to default
                                                       // "en"
        let labels = vec![PropertyLabel {
            property: "location-type".to_owned(),
            translations: vec![("de".to_owned(), "Ortstyp".to_owned())],
        }];
        let result = resolve_property_label(&labels, "location-type", &lang);
        assert_eq!(
            result,
            cpe_core::util::humanize_property_name("location-type"),
            "label declared but with no translation for the requested lang must fall back \
             to the humanized property name"
        );
    }

    #[test]
    fn resolve_property_label_falls_back_to_humanize_for_undeclared_key() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let labels: Vec<PropertyLabel> = vec![];
        let result = resolve_property_label(&labels, "location-type", &lang);
        assert_eq!(
            result,
            cpe_core::util::humanize_property_name("location-type"),
            "no label at all for the property must fall back to the humanized name"
        );
    }

    // Layer-1: resolve_class_label

    #[test]
    fn resolve_class_label_returns_translation_for_declared_hit() {
        // The declared translation is deliberately DIFFERENT from
        // `humanize_property_name("Character")` ("Character", a no-op for
        // this already-PascalCase single word) — see the sibling
        // `resolve_property_label_returns_translation_for_declared_hit` test
        // for why an identical translation/fallback pair can't catch a
        // backwards implementation.
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let labels = vec![ClassLabel {
            class: "Character".to_owned(),
            translations: vec![
                ("en".to_owned(), "Story Character".to_owned()),
                ("de".to_owned(), "Figur".to_owned()),
            ],
            plural: vec![],
        }];
        let result = resolve_class_label(&labels, "Character", &lang);
        assert_eq!(
            result, "Story Character",
            "expected declared en translation, not the humanized fallback \
             ('Character') — proves the labels slice actually won"
        );
        assert_ne!(
            result,
            cpe_core::util::humanize_property_name("Character"),
            "sanity check: declared translation must differ from the humanized \
             fallback, or this test can't distinguish the two code paths"
        );
    }

    #[test]
    fn resolve_class_label_falls_back_to_humanize_for_missing_lang() {
        let site = make_alice_site();
        let lang = resolve(&["fr".to_owned()], &site); // no fr declared -> falls back to default
                                                       // "en"
        let labels = vec![ClassLabel {
            class: "Location".to_owned(),
            translations: vec![("de".to_owned(), "Ort".to_owned())],
            plural: vec![],
        }];
        let result = resolve_class_label(&labels, "Location", &lang);
        assert_eq!(
            result,
            cpe_core::util::humanize_property_name("Location"),
            "label declared but with no translation for the requested lang must fall back \
             to the humanized class name"
        );
    }

    #[test]
    fn resolve_class_label_falls_back_to_humanize_for_undeclared_key() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let labels: Vec<ClassLabel> = vec![];
        let result = resolve_class_label(&labels, "Location", &lang);
        assert_eq!(
            result,
            cpe_core::util::humanize_property_name("Location"),
            "no label at all for the class must fall back to the humanized name"
        );
    }

    // Layer-1: resolve_class_plural

    #[test]
    fn resolve_class_plural_returns_declared_plural_for_exact_lang() {
        let site = make_alice_site();
        let lang = resolve(&["de".to_owned()], &site);
        let labels = vec![ClassLabel {
            class: "Character".to_owned(),
            translations: vec![],
            plural: vec![
                ("en".to_owned(), "characters".to_owned()),
                ("de".to_owned(), "Figuren".to_owned()),
            ],
        }];
        let result = resolve_class_plural(&labels, "Character", &lang);
        assert_eq!(result, "Figuren", "exact-lang plural must be returned verbatim");
    }

    #[test]
    fn resolve_class_plural_falls_back_to_first_declared_plural_when_lang_missing() {
        // `plural` has entries but none for the requested lang -> rung 2:
        // the FIRST declared plural entry's text, NOT the singular label
        // (there is no default-lang parameter to prefer specifically; see
        // this fn's doc comment).
        let site = make_alice_site();
        let lang = resolve(&["fr".to_owned()], &site); // no fr declared -> falls back to default
                                                       // "en"
        let labels = vec![ClassLabel {
            class: "Character".to_owned(),
            translations: vec![("en".to_owned(), "Character".to_owned())],
            plural: vec![
                ("de".to_owned(), "Figuren".to_owned()),
                ("it".to_owned(), "personaggi".to_owned()),
            ],
        }];
        let result = resolve_class_plural(&labels, "Character", &lang);
        assert_eq!(
            result, "Figuren",
            "no plural for the requested lang must fall back to the FIRST \
             declared plural entry, not the singular label"
        );
    }

    #[test]
    fn resolve_class_plural_falls_back_to_singular_label_when_plural_absent() {
        // `ClassLabel.plural` is empty but `translations` (singular) has an
        // entry for the requested lang -> rung 3: the SINGULAR translation,
        // not humanized.
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let labels = vec![ClassLabel {
            class: "Character".to_owned(),
            translations: vec![("en".to_owned(), "Story Character".to_owned())],
            plural: vec![],
        }];
        let result = resolve_class_plural(&labels, "Character", &lang);
        assert_eq!(
            result, "Story Character",
            "absent plural must fall back to the singular label, not humanize"
        );
    }

    #[test]
    fn resolve_class_plural_falls_back_to_humanize_when_neither_declared() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let labels: Vec<ClassLabel> = vec![];
        let result = resolve_class_plural(&labels, "Location", &lang);
        assert_eq!(
            result,
            cpe_core::util::humanize_property_name("Location"),
            "no plural and no singular label at all must fall back to the humanized name"
        );
    }

    // Layer-1: resolve_class_plural_capitalized (breadcrumb copy fix)

    #[test]
    fn resolve_class_plural_capitalized_capitalizes_declared_plural() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let labels = vec![ClassLabel {
            class: "Character".to_owned(),
            translations: vec![],
            plural: vec![("en".to_owned(), "characters".to_owned())],
        }];
        let result = resolve_class_plural_capitalized(&labels, "Character", &lang);
        assert_eq!(
            result, "Characters",
            "declared lowercase sentence-copy plural must be capitalized for \
             breadcrumb/title copy"
        );
    }

    #[test]
    fn resolve_class_plural_capitalized_falls_back_to_capitalized_singular() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let labels = vec![ClassLabel {
            class: "Character".to_owned(),
            translations: vec![("en".to_owned(), "story character".to_owned())],
            plural: vec![],
        }];
        let result = resolve_class_plural_capitalized(&labels, "Character", &lang);
        assert_eq!(
            result, "Story character",
            "no declared plural must fall back to the capitalized singular label"
        );
    }

    #[test]
    fn resolve_class_plural_capitalized_falls_back_to_humanized() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let labels: Vec<ClassLabel> = vec![];
        let result = resolve_class_plural_capitalized(&labels, "Location", &lang);
        assert_eq!(
            result,
            cpe_core::util::humanize_property_name("Location"),
            "no plural and no singular label at all must fall back to the (already \
             capitalized) humanized name"
        );
    }

    // Layer-1: nav_label

    #[test]
    fn nav_label_resolves_chrome_id() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let item = &site.nav[0]; // label = "nav-home", a chrome id
        let label = nav_label(&site, item, &lang);
        assert_eq!(label, "Home", "chrome-id label should resolve to translation");
    }

    #[test]
    fn nav_label_falls_back_to_literal() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let item = &site.nav[1]; // label = "About", no chrome entry
        let label = nav_label(&site, item, &lang);
        assert_eq!(label, "About", "literal label (no chrome entry) should be returned as-is");
    }

    // Layer-3: insta snapshots

    #[test]
    fn snapshot_render_page_en() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        // Narrative-only page: pass an empty resolved slice — no data-backed components.
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_render_page_de() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["de".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        // Narrative-only page: pass an empty resolved slice — no data-backed components.
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        // The de snapshot must:
        // - show the German nav label ("Startseite")
        // - show the German narrative content ("Willkommen")
        // - show aria-current="true" on the de language-switcher link
        insta::assert_snapshot!(out);
    }

    // Shell: data-direction and CSS links

    /// The rendered `<html>` element must carry `data-direction="edition"` (fixed).
    #[test]
    fn render_page_emits_data_direction_edition() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert!(
            out.contains("data-direction=\"edition\""),
            "html element must carry data-direction=\"edition\", got: {out}"
        );
    }

    /// The design-system CSS links must be present in load order; pico must be absent.
    ///
    /// Required order: tokens < shell < editorial < record < grid.
    /// This matches the `document_head` contract: tokens first, grid last.
    #[test]
    fn render_page_has_design_css_links_and_no_pico() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        // Every stylesheet is linked, in load order.
        let order = [
            "tokens.css",
            "shell.css",
            "editorial.css",
            "record.css",
            "grid.css",
            "media.css",
            "incunabula.css",
        ];
        let positions: Vec<usize> = order
            .iter()
            .map(|css| {
                out.find(&format!("\"/alice-in-daschland/_static/cpe/{css}\""))
                    .unwrap_or_else(|| panic!("output must link {css}, got: {out}"))
            })
            .collect();
        for (pair, at) in order.windows(2).zip(positions.windows(2)) {
            assert!(
                at[0] < at[1],
                "{} must appear before {} (byte offsets: {} vs {})",
                pair[0],
                pair[1],
                at[0],
                at[1]
            );
        }
        assert!(
            !out.contains("timeline.css"),
            "the per-project stylesheet is incunabula.css, got: {out}"
        );
        // pico must be absent.
        assert!(!out.contains("pico"), "pico must not appear in output, got: {out}");
    }

    /// The shell structure must be present: .shell, .shell__header, .shell__main, .shell__footer.
    #[test]
    fn render_page_has_shell_structure() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        for cls in &[
            "class=\"shell\"",
            "class=\"shell__header\"",
            "class=\"shell__main\"",
            "class=\"shell__footer\"",
        ] {
            assert!(out.contains(cls), "output must contain {cls}, got: {out}");
        }
    }

    /// A11y: the skip-link is the first child of `<body>`, and its
    /// `href="#main"` target — `id="main" tabindex="-1"` — is on
    /// `main.shell__main`.
    #[test]
    fn render_page_has_skip_link_targeting_main() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        let body_idx = out.find("<body>").expect("body present") + "<body>".len();
        assert!(
            out[body_idx..].starts_with(r##"<a class="skip-link" href="#main">"##),
            "skip-link must be the first child of <body>, got: {out}"
        );
        assert!(
            out.contains(r#"<main class="shell__main" id="main" tabindex="-1">"#),
            "main.shell__main must carry id=\"main\" tabindex=\"-1\" as the skip-link target, got: {out}"
        );
    }

    // Appearance: data-appearance attribute emission

    #[test]
    fn render_page_emits_data_appearance_when_dark() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Dark,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert!(
            out.contains("data-appearance=\"dark\""),
            "data-appearance=\"dark\" must appear when appearance is Dark, got: {out}"
        );
    }

    #[test]
    fn render_page_omits_data_appearance_when_light() {
        // Light is the CSS baseline, so it emits no attribute at all.
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert!(
            !out.contains("data-appearance"),
            "data-appearance must NOT appear when appearance is Light, got: {out}"
        );
    }

    // No header-height measurement script

    /// `shell_header` emits no `--shell-header-h` measurement
    /// script: no
    /// JS-enhanced control triggers an anchor scroll, no emitted href/action
    /// carries the `#compound-viewer` fragment, and `record.css` has no
    /// `scroll-margin-top` rule for it.
    #[test]
    fn shell_header_does_not_emit_header_height_script() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = shell_header(&site, &ctx, "/").into_string();

        assert!(
            !out.contains("<script"),
            "shell_header must not emit any <script> now that the header-height \
             measurement script is retired, got: {out}"
        );
        assert!(
            !out.contains("--shell-header-h"),
            "shell_header must not reference the retired --shell-header-h custom property, got: {out}"
        );
    }

    // Edge-case: single-language site has no language menu

    #[test]
    fn render_page_omits_language_menu_for_single_language() {
        let mut site = make_alice_site();
        site.languages = vec!["en".to_owned()];
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        for marker in ["lang-menu", "hreflang=", "<details"] {
            assert!(
                !out.contains(marker),
                "the language menu must be omitted for a single-language site ({marker}): {out}"
            );
        }
    }

    // Theme knob: data-accent and data-bg emission

    /// `render_page` must always emit `data-accent` and `data-bg` on `<html>`.
    /// With `ThemeKnob::default()` (Ochre/Tint) the values are "ochre"/"tint".
    #[test]
    fn render_page_emits_data_accent_and_data_bg_for_default_knob() {
        let site = make_alice_site(); // theme: ThemeKnob::default() = Ochre/Tint
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert!(
            out.contains("data-accent=\"ochre\""),
            "html must carry data-accent=\"ochre\" for default knob, got: {out}"
        );
        assert!(
            out.contains("data-bg=\"tint\""),
            "html must carry data-bg=\"tint\" for default knob, got: {out}"
        );
    }

    /// `render_page` emits `data-accent`/`data-bg`/`data-pick` from the
    /// project's `site.theme` (a non-default knob here).
    #[test]
    fn render_page_emits_data_attrs_from_site_theme() {
        use cpe_core::theme::{Accent, Background, ThemeKnob, Typeface};
        let mut site = make_alice_site();
        site.theme = ThemeKnob {
            accent: Accent::Green,
            background: Background::Neutral,
            typeface: Typeface::Figtree,
        };
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert!(
            out.contains("data-accent=\"green\""),
            "html must carry data-accent=\"green\" from site.theme, got: {out}"
        );
        assert!(
            out.contains("data-bg=\"neutral\""),
            "html must carry data-bg=\"neutral\" from site.theme, got: {out}"
        );
        assert!(
            out.contains("data-pick=\"figtree\""),
            "html must carry data-pick=\"figtree\" from site.theme, got: {out}"
        );
    }

    // The language menu's links: lang, hreflang, aria-current="true"

    /// Each language link carries `lang` and `hreflang` set to its code and
    /// names its language by endonym; only the current one carries
    /// `aria-current="true"`.
    #[test]
    fn render_page_language_links_carry_lang_hreflang_and_current() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["de".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert_eq!(site.languages, ["en", "de"], "the fixture's languages");
        for (code, endonym) in [("en", "English"), ("de", "Deutsch")] {
            let link = tag_containing(&out, &format!(r#"hreflang="{code}""#));
            // The leading space: `hreflang="…"` must not satisfy the check.
            assert!(link.contains(&format!(r#" lang="{code}""#)), "{link}");
            assert!(link.contains(&format!("?lang={code}")), "{link}");
            let current = link.contains(r#"aria-current="true""#);
            assert_eq!(current, code == "de", "{code}: {link}");
            let text_at = out.find(link).expect("the link is in the page") + link.len();
            assert!(
                out[text_at..].starts_with(&format!(">{endonym}<span class=\"code\">")),
                "{code}: {}",
                out[text_at..].chars().take(60).collect::<String>()
            );
        }
        // Confirm the German narrative is used
        assert!(out.contains("Willkommen"), "de render must contain German heading, got: {out}");
        // Confirm the German nav label is used
        assert!(
            out.contains("Startseite"),
            "de render must contain German nav label, got: {out}"
        );
    }

    // Footer attribution override

    /// Extract the `p.footer__attr` paragraph's inner HTML.
    fn footer_attr_of(out: &str) -> &str {
        let open = "<p class=\"footer__attr\">";
        let start = out.find(open).expect("footer renders an attribution") + open.len();
        let len = out[start..].find("</p>").expect("attribution closes");
        &out[start..start + len]
    }

    fn site_with_footer_attribution(value: &str) -> SiteConfig {
        let mut site = make_alice_site();
        site.chrome.push(ChromeString {
            id: "footer-credit".to_owned(),
            translations: vec![
                ("en".to_owned(), "Data CC BY 4.0, DaSCH.".to_owned()),
                ("de".to_owned(), "Daten CC BY 4.0, DaSCH.".to_owned()),
            ],
        });
        site.footer_attribution = Some(value.to_owned());
        site
    }

    #[test]
    fn footer_attribution_absent_renders_engine_string_with_dasch_emphasis() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let out = shell_footer(&site, &lang, &cpe_core::url::ROOT.project("p").static_dir()).into_string();
        let attr = footer_attr_of(&out);
        assert!(
            attr.contains("<strong>DaSCH</strong>"),
            "absent override keeps the engine string's emphasis, got: {attr}"
        );
        assert_eq!(
            attr.replace("<strong>", "").replace("</strong>", ""),
            cpe_components::chrome::ChromeMsg::FooterAttribution.text(&lang)
        );
    }

    #[test]
    fn footer_attribution_chrome_id_translates_per_language_as_plain_text() {
        let site = site_with_footer_attribution("footer-credit");
        let en = shell_footer(
            &site,
            &resolve(&["en".to_owned()], &site),
            &cpe_core::url::ROOT.project("p").static_dir(),
        )
        .into_string();
        assert_eq!(footer_attr_of(&en), "Data CC BY 4.0, DaSCH.");
        let de = shell_footer(
            &site,
            &resolve(&["de".to_owned()], &site),
            &cpe_core::url::ROOT.project("p").static_dir(),
        )
        .into_string();
        assert_eq!(
            footer_attr_of(&de),
            "Daten CC BY 4.0, DaSCH.",
            "the override renders as plain text — no <strong>DaSCH</strong> wrap"
        );
    }

    #[test]
    fn footer_attribution_literal_renders_verbatim() {
        let site = site_with_footer_attribution("Scans courtesy of DaSCH and UB Basel.");
        for code in ["en", "de"] {
            let out = shell_footer(
                &site,
                &resolve(&[code.to_owned()], &site),
                &cpe_core::url::ROOT.project("p").static_dir(),
            )
            .into_string();
            assert_eq!(footer_attr_of(&out), "Scans courtesy of DaSCH and UB Basel.");
        }
    }

    // DaSCH lockup in the project footer

    /// Extract the `a.footer__lockup` element, open tag through `</a>`.
    fn footer_lockup_of(out: &str) -> &str {
        let start = out
            .find("<a class=\"dasch-logo footer__lockup\"")
            .expect("footer renders the DaSCH lockup");
        let len = out[start..].find("</a>").expect("lockup closes") + "</a>".len();
        &out[start..start + len]
    }

    #[test]
    fn footer_lockup_renders_in_every_language_with_its_long_name() {
        let mut site = make_alice_site();
        site.languages = ["en", "de", "fr", "it"].map(str::to_owned).to_vec();
        for code in ["en", "de", "fr", "it"] {
            let lang = resolve(&[code.to_owned()], &site);
            assert_eq!(lang.as_str(), code);
            let out = shell_footer(&site, &lang, &cpe_core::url::ROOT.project("p").static_dir()).into_string();
            let lockup = footer_lockup_of(&out);
            assert!(lockup.contains("href=\"https://www.dasch.swiss\""), "{code}: {lockup}");
            assert!(!lockup.contains("target="), "{code}: same tab, {lockup}");
            let name = cpe_components::chrome::ChromeMsg::FooterDaschLabel.text(&lang);
            assert!(name.starts_with("DaSCH — "), "{code}: {name}");
            assert!(lockup.contains(&format!("aria-label=\"{name}\"")), "{code}: {lockup}");
            for img in [
                "<img class=\"dasch-logo__img dasch-logo__img--light\" \
                 src=\"/p/_static/cpe/dasch-logo-colour.svg\" alt=\"\"",
                "<img class=\"dasch-logo__img dasch-logo__img--dark\" \
                 src=\"/p/_static/cpe/dasch-logo-white.svg\" alt=\"\"",
            ] {
                assert!(lockup.contains(img), "{code}: {lockup}");
            }
            // Between the description and the attribution, in DOM order.
            let desc = out.find("footer__desc").expect("description");
            let lock = out.find("footer__lockup").expect("lockup");
            let attr = out.find("footer__attr").expect("attribution");
            assert!(desc < lock && lock < attr, "{code}: {out}");
        }
    }

    #[test]
    fn footer_lockup_survives_a_footer_attribution_override() {
        let site = site_with_footer_attribution("footer-credit");
        let out = shell_footer(
            &site,
            &resolve(&["de".to_owned()], &site),
            &cpe_core::url::ROOT.project("p").static_dir(),
        )
        .into_string();
        assert!(footer_lockup_of(&out).contains("Schweizerisches Daten-"));
        assert_eq!(footer_attr_of(&out), "Daten CC BY 4.0, DaSCH.");
    }

    // Footer attribution localizes (chrome-localization gap-fill)

    /// The footer's `p.footer__attr` sentence must localize via
    /// `ChromeMsg::FooterAttribution` — a de render shows the German
    /// attribution, not the English one, and the final "DaSCH" is emphasized
    /// with a `<strong>` tag (bolded via a render-time split on the last
    /// "DaSCH" occurrence, so the catalog keeps clean flat sentences).
    #[test]
    fn render_page_footer_attribution_localizes() {
        let site = make_alice_site();
        let page = make_alice_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["de".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert!(
            out.contains("<strong>DaSCH</strong>"),
            "footer attribution must emphasize the final DaSCH with <strong>, got: {out}"
        );
        // German-distinctive lead text (present before the emphasized DaSCH).
        assert!(
            out.contains("Forschungsinfrastruktur-Angebot"),
            "de render must contain the German footer attribution, got: {out}"
        );
        // English-distinctive text must be absent.
        assert!(
            !out.contains("research infrastructure service"),
            "de render must NOT contain the English footer attribution, got: {out}"
        );
    }

    // Breadcrumb: present on resource-detail AND narrative pages

    /// The project home page (route "/") is narrative-only and gets the
    /// degenerate narrative crumb-line variant: `CPE › Project` alone (no
    /// page-title crumb).
    #[test]
    fn render_page_project_home_narrative_has_degenerate_breadcrumb() {
        let site = make_alice_site();
        let page = make_alice_page(); // narrative-only, route "/"
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert!(
            out.contains("class=\"breadcrumb\""),
            "project home page must have a breadcrumb (narrative variant), got: {out}"
        );
        // `render_narrative_breadcrumb(page_title: None)` yields the CPE
        // link + the project-title current item — 2 items, no third
        // page-title crumb.
        assert_eq!(
            out.matches("breadcrumb__item").count(),
            2,
            "project home has no page-title crumb, got: {out}"
        );
        let current_idx = out.find("breadcrumb__current").expect("breadcrumb__current not found");
        assert!(
            out[current_idx..].contains("Alice in DaSCHland"),
            "current crumb must show the project title, got: {out}"
        );
        assert!(
            out.contains("crumb-share"),
            "narrative breadcrumb must carry the share control, got: {out}"
        );
    }

    /// A non-home narrative page gets the 3-crumb narrative variant with the
    /// page's own resolved title as the current item.
    #[test]
    fn render_page_non_home_narrative_has_page_title_breadcrumb() {
        let site = make_alice_site();
        let mut page = make_alice_page();
        page.id = "wonderland".to_owned();
        page.route = "/wonderland".to_owned();
        let narrative = make_alice_narrative(); // "home.md" -> "# Welcome\n\n..."
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/wonderland",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert!(
            out.contains("class=\"breadcrumb\""),
            "non-home narrative page must have a breadcrumb, got: {out}"
        );
        assert_eq!(
            out.matches("breadcrumb__item").count(),
            3,
            "non-home narrative page must have the 3-item trail, got: {out}"
        );
        let current_idx = out.find("breadcrumb__current").expect("breadcrumb__current not found");
        assert!(
            out[current_idx..].contains("Welcome"),
            "current crumb must show the resolved narrative title, got: {out}"
        );
        assert!(
            out.contains("crumb-share"),
            "narrative breadcrumb must carry the share control, got: {out}"
        );
    }

    /// A resource-detail page with a resolved resource must include a breadcrumb.
    #[test]
    fn render_page_resource_detail_has_breadcrumb() {
        let site = make_alice_site();
        let page = make_character_page();
        let narrative = make_alice_narrative();
        let resource = make_alice_resource();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/CH_001",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(Some(resource))];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        assert!(
            out.contains("class=\"breadcrumb\""),
            "resource-detail page must have a breadcrumb, got: {out}"
        );
        // The leaf crumb must show the resolved title "Alice".
        assert!(
            out.contains("breadcrumb__current"),
            "breadcrumb must have a current crumb, got: {out}"
        );
        assert!(
            out.contains("aria-current=\"page\""),
            "breadcrumb current item must have aria-current=page, got: {out}"
        );
        // Project crumb links to /alice-in-daschland/.
        assert!(
            out.contains("href=\"/alice-in-daschland/\""),
            "project breadcrumb crumb must link to project home, got: {out}"
        );
        // The resolved title "Alice" must appear INSIDE the breadcrumb__current crumb
        // — not just somewhere in the page. Extract the breadcrumb__current span and
        // verify it contains "Alice".
        let current_start = out.find("breadcrumb__current").expect("breadcrumb__current must be present");
        let after_current = &out[current_start..];
        // Find the closing span for this crumb (simple heuristic: first </span> after the open)
        let crumb_end = after_current.find("</span>").expect("breadcrumb__current span must be closed");
        let crumb_slice = &after_current[..crumb_end + "</span>".len()];
        assert!(
            crumb_slice.contains("Alice"),
            "the breadcrumb__current crumb must contain the resolved title 'Alice', \
             got crumb slice: {crumb_slice}"
        );
        assert_eq!(
            out.matches("breadcrumb__item").count(),
            3,
            "class-less resource-detail (make_character_page's class: None) must yield \
             the 3-item trail (no class crumb), got: {out}"
        );
    }

    /// Render one `resource-detail` whose single `group` is captioned by
    /// chrome id `label`, through `render_component`'s real caption
    /// resolution (declared id → `chrome_lookup`, else the built-in fallback).
    fn render_grouped_detail(label: &str) -> String {
        let site = make_alice_site();
        let component = Component::ResourceDetail {
            id: Some("$id".to_owned()),
            show_properties: None,
            title_property: Some("name".to_owned()),
            class: None,
            media: None,
            groups: vec![cpe_core::ir::PropertyGroup {
                label: label.to_owned(),
                properties: vec!["description".to_owned()],
                empty: None,
                links_label: None,
            }],
            narrative_for: vec![],
            incoming_links: None,
            head: None,
            navigator: false,
        };
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang: resolve(&["de".to_owned()], &site),
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/CH_001",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let default_lang = resolve(&["en".to_owned()], &site);
        let data = ComponentData::Detail(Some(make_alice_resource()));
        render_component(&component, &ctx, &make_alice_narrative(), &data, &default_lang, &site).into_string()
    }

    #[test]
    fn resource_detail_group_caption_uses_declared_chrome_id() {
        let out = render_grouped_detail("nav-home");
        assert!(
            out.contains(r#"<span class="pgroup__title">Startseite</span>"#),
            "a declared chrome id must caption the group in the current language: {out}"
        );
    }

    #[test]
    fn resource_detail_group_caption_falls_back_for_undeclared_id() {
        let out = render_grouped_detail("no-such-id");
        assert!(
            out.contains(r#"<span class="pgroup__title">Angaben</span>"#),
            "an undeclared id must fall back to ChromeMsg::PropertyGroupTitle: {out}"
        );
        assert!(!out.contains("no-such-id"), "never the raw id: {out}");
    }

    // The pages reader inside its record

    /// A navigator page — `resource-detail id="$id"` beside its
    /// `layout="pages"` reader over `total` members — through `render_body`,
    /// the layer that pairs them.
    fn render_book_body(total: usize) -> String {
        render_book_body_with(total, cpe_core::ir::CompoundLayout::Pages, true)
    }

    fn render_book_body_with(total: usize, layout: cpe_core::ir::CompoundLayout, record_found: bool) -> String {
        render_annotated_book(total, layout, record_found, None).0
    }

    /// [`render_book_body_with`] with the reader's `annotations=` set when
    /// `register` is `Some` (its entries the shown page's register): the
    /// body, and the `_compound` fragment's annotation target.
    fn render_annotated_book(
        total: usize,
        layout: cpe_core::ir::CompoundLayout,
        record_found: bool,
        register: Option<Vec<cpe_core::ir::AnnotationEntry>>,
    ) -> (String, Option<String>) {
        let site = make_alice_site();
        let (page, components) = book_fixture(total, layout, record_found, register);
        let route_params = [("id".to_owned(), "CH_001".to_owned())];
        let ctx = book_ctx(&site, &route_params, None, Appearance::Light);
        let default_lang = resolve(&["en".to_owned()], &site);
        let narrative = make_alice_narrative();
        let body = render_body(&page, &ctx, &narrative, &components, &default_lang, &site)
            .0
            .into_string();
        let fragment = render_compound_fragment(&site, &page, &narrative, &ctx, &components, &DataArks::default())
            .and_then(|f| f.annotations)
            .map(Markup::into_string);
        (body, fragment)
    }

    /// The book composition behind [`render_annotated_book`]: a
    /// navigator-paired record plus its reader of `total` pages, and the
    /// aligned component data (the reader on page 1).
    fn book_fixture(
        total: usize,
        layout: cpe_core::ir::CompoundLayout,
        record_found: bool,
        register: Option<Vec<cpe_core::ir::AnnotationEntry>>,
    ) -> (cpe_core::ir::Page, Vec<ComponentData>) {
        use cpe_core::ir::{CompoundPage, MemberHeader, MemberIndex};
        let page = cpe_core::ir::Page {
            id: "book".to_owned(),
            route: "/books/:id".to_owned(),
            params: vec!["id".to_owned()],
            body: vec![
                Component::ResourceDetail {
                    head: None,
                    id: Some("$id".to_owned()),
                    show_properties: None,
                    title_property: Some("name".to_owned()),
                    class: None,
                    media: None,
                    groups: vec![],
                    narrative_for: vec![],
                    incoming_links: None,
                    navigator: true,
                },
                Component::CompoundViewer {
                    source: "$id".to_owned(),
                    media: None,
                    picker: None,
                    layout,
                    index: MemberIndex::Off,
                    paired: true,
                    member_links: None,
                    annotations: register.is_some(),
                },
            ],
            figure_slots: Vec::new(),
        };
        let members: Vec<MemberHeader> = (1..=total)
            .map(|n| MemberHeader {
                id: format!("p{n}"),
                class: "Page".to_owned(),
                name: format!("a{n}r"),
            })
            .collect();
        let compound = CompoundPage {
            member: (total > 0).then(make_alice_resource),
            index: 1,
            total,
            members,
            member_media: vec![],
            stale_page: None,
            annotations: register.unwrap_or_default(),
            annotated_members: std::collections::BTreeSet::new(),
        };
        let components = vec![
            ComponentData::Detail(record_found.then(make_alice_resource)),
            ComponentData::Compound(compound),
        ];
        (page, components)
    }

    /// A German request for [`book_fixture`]'s page.
    fn book_ctx<'a>(
        site: &SiteConfig,
        route_params: &'a [(String, String)],
        current_query: Option<&'a str>,
        appearance: Appearance,
    ) -> RenderContext<'a> {
        RenderContext {
            base: &cpe_core::url::ROOT,
            lang: resolve(&["de".to_owned()], site),
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/books/CH_001",
            route_params,
            current_query,
            appearance,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        }
    }

    /// The book page's full render and its `_compound` fragment's header
    /// controls, for one request.
    fn render_book_page_and_header_controls(current_query: Option<&str>, appearance: Appearance) -> (String, String) {
        render_book_page_and_header_controls_for(&make_alice_site(), current_query, appearance)
    }

    fn render_book_page_and_header_controls_for(
        site: &SiteConfig,
        current_query: Option<&str>,
        appearance: Appearance,
    ) -> (String, String) {
        let (page, components) = book_fixture(3, cpe_core::ir::CompoundLayout::Pages, true, None);
        let route_params = [("id".to_owned(), "CH_001".to_owned())];
        let ctx = book_ctx(site, &route_params, current_query, appearance);
        let narrative = make_alice_narrative();
        let arks = DataArks::default();
        let full = render_page(site, &page, &narrative, &ctx, &components, &arks).into_string();
        let controls = render_compound_fragment(site, &page, &narrative, &ctx, &components, &arks)
            .expect("a book page renders a fragment")
            .header_controls
            .into_string();
        (full, controls)
    }

    /// The Alice fixture site with German as its only language.
    fn single_language_site() -> SiteConfig {
        let mut site = make_alice_site();
        site.languages = vec!["de".to_owned()];
        site.default_language = "de".to_owned();
        site
    }

    /// The opening tag (without its `>`) of the first element in `html`
    /// whose tag contains `marker`.
    fn tag_containing<'h>(html: &'h str, marker: &str) -> &'h str {
        let at = html.find(marker).unwrap_or_else(|| panic!("{marker} in {html}"));
        let start = html[..at].rfind('<').expect("a tag opens before the marker");
        let end = at + html[at..].find('>').expect("the tag closes");
        &html[start..end]
    }

    /// After a page turn the swap's header controls link to the page shown:
    /// the language and appearance hrefs carry the new `page=` and keep the
    /// request's other query keys.
    #[test]
    fn compound_fragment_header_controls_carry_the_new_page() {
        let (_, controls) = render_book_page_and_header_controls(Some("page=p2&lang=de"), Appearance::Light);
        let en = tag_containing(&controls, r#"hreflang="en""#);
        assert!(en.contains("lang=en") && en.contains("page=p2"), "{en}");
        let toggle = tag_containing(&controls, r#"class="theme-toggle""#);
        assert!(
            toggle.contains("appearance=dark") && toggle.contains("page=p2") && toggle.contains("lang=de"),
            "{toggle}"
        );
    }

    /// Drift guard: the fragment's header controls are byte-identical to the
    /// full page's for the same request, so the morph changes only hrefs.
    /// The controls carry both row wrappers with their `aria-hidden` labels
    /// and both appearance name spans, so none of them can drift either.
    #[test]
    fn compound_fragment_header_controls_match_the_full_page() {
        for (appearance, panel_label) in [(Appearance::Light, "Dunkler Modus"), (Appearance::Dark, "Heller Modus")] {
            let (full, controls) = render_book_page_and_header_controls(Some("page=p2"), appearance);
            assert!(full.contains(&controls), "fragment controls differ from the page: {controls}");
            assert_eq!(controls.matches(r#"<div class="header__row">"#).count(), 2, "{controls}");
            for label in ["Sprache", "Darstellung"] {
                assert!(
                    controls.contains(&format!(r#"<span class="header__row-label" aria-hidden="true">{label}</span>"#)),
                    "{label}: {controls}"
                );
            }
            assert!(controls.contains(r#"<span class="theme-toggle__desk-name">"#), "{controls}");
            assert!(
                controls.contains(&format!(
                    r#"<span class="theme-toggle__panel-name">{panel_label}<span class="visually-hidden"> – umschalten</span></span>"#
                )),
                "{controls}"
            );
        }
    }

    /// A one-language project renders no language menu: no `details`, no
    /// "Sprache" row and no hidden "Sprache: …" name; the fragment still
    /// matches the full page.
    #[test]
    fn single_language_header_has_no_language_menu() {
        let (full, controls) =
            render_book_page_and_header_controls_for(&single_language_site(), None, Appearance::Light);
        assert!(full.contains(&controls), "{controls}");
        assert!(!controls.contains("<details"), "{controls}");
        assert!(!controls.contains("lang-menu"), "{controls}");
        assert!(!controls.contains("Sprache"), "{controls}");
        assert!(!full.contains("Sprache:"), "no hidden language name");
        // Canary: the multi-language header has all three.
        let (multi, _) = render_book_page_and_header_controls(None, Appearance::Light);
        assert!(multi.contains("Sprache: Deutsch"));
    }

    /// The morph pairs `#header-controls`' children by tag and position,
    /// so the full page and the fragment render the same children, two rows
    /// with two or more languages and one with a single language.
    #[test]
    fn header_controls_children_match_in_page_and_fragment() {
        fn row_tags(html: &str) -> Vec<&str> {
            let inner = html
                .split_once(&format!(r#"id="{HEADER_CONTROLS_ID}">"#))
                .expect("the controls open")
                .1;
            inner
                .match_indices(r#"<div class="header__row">"#)
                .map(|(i, _)| {
                    let rest = &inner[i + r#"<div class="header__row">"#.len()..];
                    let span_end = rest.find("</span>").expect("the row label closes");
                    let after = &rest[span_end + "</span>".len()..];
                    &after[..after.find([' ', '>']).expect("a tag name")]
                })
                .collect()
        }
        for (site, expected) in [
            (make_alice_site(), vec!["<details", "<a"]),
            (single_language_site(), vec!["<a"]),
        ] {
            let (full, controls) = render_book_page_and_header_controls_for(&site, None, Appearance::Light);
            assert_eq!(row_tags(&controls), expected, "{controls}");
            let page_controls_at = full
                .find(&format!(r#"<div class="header__controls" id="{HEADER_CONTROLS_ID}">"#))
                .expect("the page has the controls");
            assert_eq!(row_tags(&full[page_controls_at..]), expected);
        }
    }

    /// `shell.css` spells the desk breakpoint as a literal, twice (the desk
    /// query and the menu form's complement); a stylesheet that moves alone
    /// would leave a widened panel stuck in the top layer.
    #[test]
    fn header_desk_breakpoint_matches_shell_css() {
        const SHELL_CSS: &str = include_str!("../../components/style/shell.css");
        let query = header_desk_query();
        let both = "HEADER_DESK_BREAKPOINT_PX and the vendored style/shell.css's two \
                    desk queries change together";
        assert!(SHELL_CSS.contains(&format!("@media {query} {{")), "{both}");
        assert!(SHELL_CSS.contains(&format!("@media not all and {query} {{")), "{both}");
    }

    /// The server never writes `open` on the language menu, so a morph
    /// always leaves it closed.
    #[test]
    fn language_menu_never_renders_open() {
        for appearance in [Appearance::Light, Appearance::Dark] {
            let (full, controls) = render_book_page_and_header_controls(None, appearance);
            for html in [&full, &controls] {
                let details = tag_containing(html, r#"class="lang-menu""#);
                assert!(details.starts_with("<details "), "{details}");
                assert!(!details.contains(" open"), "{details}");
            }
        }
    }

    /// `#header-menu` wraps the controls on the page but is no part of the
    /// `_compound` fragment's header controls, so a swap cannot close it.
    #[test]
    fn header_menu_is_outside_the_fragment_header_controls() {
        let (full, controls) = render_book_page_and_header_controls(None, Appearance::Light);
        assert!(!controls.contains(HEADER_MENU_ID), "{controls}");
        assert!(!controls.contains("popover"), "{controls}");
        let menu_at = full
            .find(&format!(r#"<div id="{HEADER_MENU_ID}" popover="auto""#))
            .expect("the page has the menu");
        let controls_at = full.find(&controls).expect("the page has the controls");
        assert!(menu_at < controls_at, "the menu wraps the controls");
        let button = tag_containing(&full, "popovertarget=");
        assert!(button.contains(&format!(r#"popovertarget="{HEADER_MENU_ID}""#)), "{button}");
    }

    /// Every page rendered through `shell_header` carries the controls' id
    /// exactly once (a duplicate id would drop out of DataStar's morph
    /// pairing).
    #[test]
    fn shell_pages_carry_the_header_controls_id_once() {
        let marker = format!(r#"id="{HEADER_CONTROLS_ID}""#);
        let (full, _) = render_book_page_and_header_controls(None, Appearance::Light);
        let site = make_alice_site();
        let route_params = [("id".to_owned(), "CH_001".to_owned())];
        let ctx = book_ctx(&site, &route_params, None, Appearance::Light);
        let not_found = render_not_found_project(&site, &ctx).into_string();
        for (name, out) in [("page", full), ("N1", not_found)] {
            assert_eq!(out.matches(&marker).count(), 1, "{name}: {out}");
        }
    }

    fn font_families_of(typeface: Option<Typeface>) -> Vec<String> {
        let href = google_fonts_href(typeface);
        let query = href
            .strip_prefix("https://fonts.googleapis.com/css2?")
            .expect("Google Fonts css2 endpoint");
        assert!(query.ends_with("&display=swap"), "{href}");
        query
            .split('&')
            .filter_map(|p| p.strip_prefix("family="))
            .map(|f| f.split(':').next().unwrap_or(f).to_owned())
            .collect()
    }

    /// A project page loads its own face, Lora for `.dasch-link` and Plex
    /// Mono; a pairing loads both of its families; a page with no project
    /// loads Newsreader and Plex Mono only.
    #[test]
    fn google_fonts_href_loads_only_the_faces_a_page_renders() {
        assert_eq!(
            font_families_of(Some(Typeface::Newsreader)),
            ["Newsreader", "Lora", "IBM+Plex+Mono"]
        );
        assert_eq!(
            font_families_of(Some(Typeface::SpectralFigtree)),
            ["Spectral", "Figtree", "Lora", "IBM+Plex+Mono"]
        );
        assert_eq!(font_families_of(Some(Typeface::Lora)), ["Lora", "IBM+Plex+Mono"]);
        assert_eq!(font_families_of(None), ["Newsreader", "IBM+Plex+Mono"]);
        for &t in Typeface::all() {
            assert!(!t.font_families().is_empty(), "{t:?} loads no family");
        }
    }

    /// A slotted per-project component renders once, inside the narrative
    /// section its slot names, with the slot's resolved count line and more
    /// link — never at its own body position.
    #[test]
    fn render_body_places_a_slotted_component_in_its_narrative_section() {
        let mut site = make_alice_site();
        site.chrome.push(ChromeString {
            id: "tl-count".to_owned(),
            translations: vec![
                ("en".to_owned(), "19 books".to_owned()),
                ("de".to_owned(), "19 Bücher".to_owned()),
            ],
        });
        let timeline = Component::Timeline {
            class: "Book".to_owned(),
            date_property: "pub-date".to_owned(),
            lane_property: "office".to_owned(),
            extent_property: None,
            axis_start: 1470,
            axis_end: 1506,
        };
        let page = cpe_core::ir::Page {
            id: "home".to_owned(),
            route: "/".to_owned(),
            params: vec![],
            body: vec![
                Component::Narrative { file: "home.md".to_owned(), companion: None, side: None },
                timeline,
            ],
            figure_slots: vec![
                None,
                Some(cpe_core::ir::FigureSlot {
                    section: "offices".to_owned(),
                    count: Some("tl-count".to_owned()),
                    more: Some(cpe_core::ir::FigureMore { label: "nav-home".to_owned(), route: "/".to_owned() }),
                }),
            ],
        };
        let narrative = NarrativeStore::from_pairs(&[(
            "home.md".to_owned(),
            "# Home\n\nIntro.\n\n## Offices {#offices}\n\nOffice prose.\n".to_owned(),
        )]);
        let components = [
            ComponentData::Other,
            ComponentData::Timeline(cpe_core::ir::TimelinePayload {
                resources: ResourceListPage {
                    items: vec![],
                    total: 0,
                    page: 1,
                    pagination: cpe_core::ir::Pagination::Limit(0),
                    unfiltered_total: None,
                },
                lanes: vec![],
            }),
        ];
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang: resolve(&["en".to_owned()], &site),
            project_id: "incunabula",
            current_path: "/incunabula/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let default_lang = resolve(&["en".to_owned()], &site);
        let out = render_body(&page, &ctx, &narrative, &components, &default_lang, &site)
            .0
            .into_string();
        assert_eq!(out.matches(r#"class="tl page""#).count(), 1, "{out}");
        let part = out
            .split(r#"<section class="editorial__part" aria-labelledby="offices">"#)
            .nth(1)
            .unwrap_or_else(|| panic!("no part for 'offices': {out}"));
        let figure = part
            .split(r#"<div class="editorial__figure">"#)
            .nth(1)
            .unwrap_or_else(|| panic!("no figure in the part: {out}"));
        assert!(figure.starts_with(r#"<section class="tl page""#), "{out}");
        assert!(
            figure.contains(concat!(
                r#"<p class="editorial__figure-count">19 books</p>"#,
                r#"<p class="editorial__figure-more"><a href="/incunabula/">Home "#,
            )),
            "{out}"
        );
        assert!(out.ends_with("</section></div></div></article>"), "{out}");
    }

    /// Generated heading ids are numbered over the whole page: the second
    /// narrative counts on from the first one's top-level `##` headings, in
    /// the language variant being rendered, so no id repeats.
    #[test]
    fn render_body_numbers_generated_heading_ids_across_narratives() {
        let site = make_alice_site();
        let narrative_at = |file: &str| Component::Narrative { file: file.to_owned(), companion: None, side: None };
        let page = cpe_core::ir::Page {
            id: "home".to_owned(),
            route: "/".to_owned(),
            params: vec![],
            body: vec![narrative_at("home.md"), narrative_at("more.md")],
            figure_slots: vec![None, None],
        };
        let narrative = NarrativeStore::from_pairs(&[
            (
                "home.md".to_owned(),
                "# Home\n\n## A {#a}\n\nText.\n\n## B\n\nText.\n".to_owned(),
            ),
            // The German variant has one section more: its ids shift.
            (
                "home.de.md".to_owned(),
                "# Start\n\n## A {#a}\n\nText.\n\n## B\n\nText.\n\n## B2\n\nText.\n".to_owned(),
            ),
            ("more.md".to_owned(), "## C\n\nText.\n".to_owned()),
        ]);
        let ids_in = |lang: &str| {
            let ctx = RenderContext {
                base: &cpe_core::url::ROOT,
                lang: resolve(&[lang.to_owned()], &site),
                project_id: "alice-in-daschland",
                current_path: "/alice-in-daschland/",
                route_params: &[],
                current_query: None,
                appearance: Appearance::Light,
                selected_image: 0,
                link_routes: &EMPTY_ROUTES,
                media_properties: &EMPTY_ROUTES,
                media_items: &EMPTY_MEDIA,
                strip_open: true,
                media_thumbs: &EMPTY_MEDIA_THUMBS,
                compound_members: &[],
                compound_parents: &EMPTY_PARENTS,
                class_list_routes: &EMPTY_ROUTES,
                public_origin: "http://localhost:8080",
                incoming_links: None,
                resource_links: &[],
            };
            let default_lang = resolve(&["en".to_owned()], &site);
            let out = render_body(&page, &ctx, &narrative, &[], &default_lang, &site).0.into_string();
            out.split("<h2 id=\"")
                .skip(1)
                .map(|rest| rest.split('"').next().unwrap_or_default().to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids_in("en"), ["a", "sec-2", "sec-3"]);
        assert_eq!(ids_in("de"), ["a", "sec-2", "sec-3", "sec-4"]);
    }

    // The page rail and the narrative layout

    /// An h1, an intro and one top-level `##`.
    const RAIL_HOME: &str = "# Home\n\nIntro.\n\n## A {#a}\n\nText.\n";
    /// One more top-level `##`, with neither head nor intro.
    const RAIL_MORE: &str = "## B\n\n- [X](/x) \u{2014} gloss\n";

    fn rail_narrative(file: &str) -> Component {
        Component::Narrative { file: file.to_owned(), companion: None, side: None }
    }

    fn rail_page(body: Vec<Component>) -> cpe_core::ir::Page {
        cpe_core::ir::Page {
            id: "home".to_owned(),
            route: "/".to_owned(),
            params: vec![],
            figure_slots: vec![None; body.len()],
            body,
        }
    }

    fn rail_store(files: &[(&str, &str)]) -> NarrativeStore {
        let pairs: Vec<(String, String)> = files
            .iter()
            .map(|(name, src)| ((*name).to_owned(), (*src).to_owned()))
            .collect();
        NarrativeStore::from_pairs(&pairs)
    }

    /// `render_body`'s two values for `page`, in English.
    fn rail_body(
        site: &SiteConfig,
        page: &cpe_core::ir::Page,
        store: &NarrativeStore,
    ) -> (String, Option<NarrativeLayout>) {
        let lang = resolve(&["en".to_owned()], site);
        let ctx = make_browse_ctx(lang.clone(), "/alice-in-daschland/", None);
        let (body, layout) = render_body(page, &ctx, store, &[], &lang, site);
        (body.into_string(), layout)
    }

    fn rail_layout(companion: NarrativeCompanion, side: NarrativeSide) -> Option<NarrativeLayout> {
        Some(NarrativeLayout { companion, side })
    }

    /// One rail per page: it stands in the first narrative and lists the
    /// top-level `##` headings of every narrative, with the ids they render.
    #[test]
    fn one_rail_per_page_lists_the_headings_of_every_narrative() {
        let site = make_alice_site();
        let page = rail_page(vec![rail_narrative("home.md"), rail_narrative("more.md")]);
        let store = rail_store(&[("home.md", RAIL_HOME), ("more.md", RAIL_MORE)]);
        let (out, layout) = rail_body(&site, &page, &store);

        assert_eq!(out.matches("narr__side").count(), 1, "{out}");
        assert!(
            out.contains(concat!(
                r##"<ul class="toc"><li><a href="#a">A</a></li>"##,
                r##"<li><a href="#sec-2">B</a></li></ul>"##,
            )),
            "{out}"
        );
        assert!(out.contains(r#"<h2 id="sec-2">B</h2>"#), "{out}");
        let (first, second) = out.split_once("</article>").unwrap_or_else(|| panic!("two articles: {out}"));
        assert!(first.contains("narr__side"), "{out}");
        assert!(!second.contains("narr__side"), "{out}");
        assert_eq!(layout, rail_layout(NarrativeCompanion::Rail, NarrativeSide::End));
    }

    /// A heading without text gets no rail entry but keeps its number: the
    /// headings after it, in this narrative and the next, are listed with
    /// the ids they render.
    #[test]
    fn a_heading_without_text_is_not_listed_and_keeps_its_number() {
        let site = make_alice_site();
        let page = rail_page(vec![rail_narrative("home.md"), rail_narrative("more.md")]);
        let home = "# Home\n\nIntro.\n\n##\n\nText.\n\n## A\n\nText.\n";
        let store = rail_store(&[("home.md", home), ("more.md", RAIL_MORE)]);
        let (out, _) = rail_body(&site, &page, &store);
        assert!(
            out.contains(concat!(
                r##"<ul class="toc"><li><a href="#sec-2">A</a></li>"##,
                r##"<li><a href="#sec-3">B</a></li></ul>"##,
            )),
            "{out}"
        );
        assert!(out.contains(r#"<h2 id="sec-2">A</h2>"#), "{out}");
        assert!(out.contains(r#"<h2 id="sec-3">B</h2>"#), "{out}");
    }

    /// A list of one section names nothing: the page is laid out as `text`.
    #[test]
    fn no_rail_under_two_entries() {
        let site = make_alice_site();
        let page = rail_page(vec![rail_narrative("home.md")]);
        let (out, layout) = rail_body(&site, &page, &rail_store(&[("home.md", RAIL_HOME)]));
        assert!(!out.contains("narr__side"), "{out}");
        assert_eq!(layout, rail_layout(NarrativeCompanion::Text, NarrativeSide::End));
    }

    /// The rail needs a head or an intro to stand beside, in the FIRST
    /// narrative: with the two files swapped the page has the same two
    /// entries and no rail, so it is `text`.
    #[test]
    fn no_rail_when_the_first_narrative_has_neither_head_nor_intro() {
        let site = make_alice_site();
        let page = rail_page(vec![rail_narrative("more.md"), rail_narrative("home.md")]);
        let store = rail_store(&[("home.md", RAIL_HOME), ("more.md", RAIL_MORE)]);
        let (out, layout) = rail_body(&site, &page, &store);
        assert!(!out.contains("narr__side"), "{out}");
        assert_eq!(layout, rail_layout(NarrativeCompanion::Text, NarrativeSide::End));
    }

    /// "First narrative" means the first `narrative` in the body, whatever
    /// component stands before it.
    #[test]
    fn the_rail_is_in_the_first_narrative_after_another_component() {
        let site = make_alice_site();
        let mut body = make_nonfaceted_browse_page().body;
        body.extend([rail_narrative("home.md"), rail_narrative("more.md")]);
        let page = rail_page(body);
        let store = rail_store(&[("home.md", RAIL_HOME), ("more.md", RAIL_MORE)]);
        let (out, layout) = rail_body(&site, &page, &store);
        assert_eq!(out.matches("narr__side").count(), 1, "{out}");
        let rail_at = out.find("narr__side").unwrap_or_default();
        let first_article_end = out.find("</article>").unwrap_or_default();
        assert!(rail_at < first_article_end, "{out}");
        assert_eq!(layout, rail_layout(NarrativeCompanion::Rail, NarrativeSide::End));
    }

    /// `companion="text"` on the first narrative, or as the site's default,
    /// switches the rail off; the narrative's own attribute wins over the
    /// site's.
    #[test]
    fn the_requested_companion_and_side_come_from_the_narrative_then_the_site() {
        let store = rail_store(&[("home.md", RAIL_HOME), ("more.md", RAIL_MORE)]);
        let page_with = |companion, side| {
            rail_page(vec![
                Component::Narrative { file: "home.md".to_owned(), companion, side },
                rail_narrative("more.md"),
            ])
        };

        let mut site = make_alice_site();
        let (out, layout) = rail_body(
            &site,
            &page_with(Some(NarrativeCompanion::Text), Some(NarrativeSide::Start)),
            &store,
        );
        assert!(!out.contains("narr__side"), "{out}");
        assert_eq!(layout, rail_layout(NarrativeCompanion::Text, NarrativeSide::Start));

        site.narrative_companion = NarrativeCompanion::Text;
        site.narrative_side = NarrativeSide::Start;
        let (out, layout) = rail_body(&site, &page_with(None, None), &store);
        assert!(!out.contains("narr__side"), "{out}");
        assert_eq!(layout, rail_layout(NarrativeCompanion::Text, NarrativeSide::Start));

        let (out, layout) = rail_body(
            &site,
            &page_with(Some(NarrativeCompanion::Rail), Some(NarrativeSide::End)),
            &store,
        );
        assert!(out.contains("narr__side"), "{out}");
        assert_eq!(layout, rail_layout(NarrativeCompanion::Rail, NarrativeSide::End));
    }

    /// `render_page` for `page`, in English.
    fn rail_document(site: &SiteConfig, page: &cpe_core::ir::Page, store: &NarrativeStore) -> String {
        let lang = resolve(&["en".to_owned()], site);
        let ctx = make_browse_ctx(lang, "/alice-in-daschland/", None);
        render_page(site, page, store, &ctx, &[], &DataArks::default()).into_string()
    }

    /// The opening `<html …>` tag of a document.
    fn html_tag(document: &str) -> &str {
        let start = document.find("<html").unwrap_or_default();
        let end = document[start..].find('>').unwrap_or_default();
        &document[start..start + end]
    }

    #[test]
    fn side_start_is_emitted_on_html() {
        let site = make_alice_site();
        let page = rail_page(vec![
            Component::Narrative {
                file: "home.md".to_owned(),
                companion: None,
                side: Some(NarrativeSide::Start),
            },
            rail_narrative("more.md"),
        ]);
        let store = rail_store(&[("home.md", RAIL_HOME), ("more.md", RAIL_MORE)]);
        let out = rail_document(&site, &page, &store);
        let tag = html_tag(&out);
        assert!(tag.contains(r#"data-narr="rail""#), "{tag}");
        assert!(tag.contains(r#"data-narr-side="start""#), "{tag}");
    }

    /// The two attributes belong to a page with a narrative: a browse page
    /// and both 404 shells carry neither.
    #[test]
    fn pages_without_a_narrative_carry_no_narrative_layout() {
        let site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let browse = rail_document(&site, &make_nonfaceted_browse_page(), &rail_store(&[]));
        let ctx = make_browse_ctx(lang.clone(), "/alice-in-daschland/", None);
        let n1 = render_not_found_project(&site, &ctx).into_string();
        let n2 = render_not_found_site(&cpe_core::url::ROOT, &lang, Appearance::Light).into_string();
        for (name, document) in [("browse", browse), ("N1", n1), ("N2", n2)] {
            assert!(html_tag(&document).contains("data-accent"), "{name}");
            assert!(!document.contains("data-narr"), "{name}: {document}");
        }
    }

    /// `<html>` says `rail` exactly when the markup holds the rail: the
    /// stylesheets lay out the side track from the attribute alone.
    #[test]
    fn html_says_rail_exactly_when_the_rail_is_rendered() {
        let site = make_alice_site();
        let store = rail_store(&[
            ("home.md", RAIL_HOME),
            ("more.md", RAIL_MORE),
            ("prose.md", "Just prose.\n"),
        ]);
        let text_first = Component::Narrative {
            file: "home.md".to_owned(),
            companion: Some(NarrativeCompanion::Text),
            side: None,
        };
        let fixtures = [
            (vec!["home.md", "more.md"], true),
            (vec!["home.md"], false),
            (vec!["more.md", "home.md"], false),
            (vec!["prose.md", "home.md", "more.md"], false),
            (vec!["home.md", "more.md", "more.md"], true),
        ];
        let mut pages: Vec<(cpe_core::ir::Page, bool)> = fixtures
            .into_iter()
            .map(|(files, rail)| (rail_page(files.into_iter().map(rail_narrative).collect()), rail))
            .collect();
        pages.push((rail_page(vec![text_first, rail_narrative("more.md")]), false));
        for (page, rail) in pages {
            let out = rail_document(&site, &page, &store);
            let tag = html_tag(&out);
            let expected = if rail { "rail" } else { "text" };
            assert!(tag.contains(&format!(r#"data-narr="{expected}""#)), "{tag}");
            assert_eq!(out.contains("narr__side"), rail, "{out}");
        }
    }

    /// A justified project says so on both of its shells, a page and its
    /// 404; `"start"`, the default, writes no attribute; the site's own
    /// 404 has no project and never carries one.
    #[test]
    fn text_align_is_emitted_on_both_project_shells() {
        let mut site = make_alice_site();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = make_browse_ctx(lang.clone(), "/alice-in-daschland/", None);
        let page = make_nonfaceted_browse_page();
        let store = rail_store(&[]);

        let start_page = rail_document(&site, &page, &store);
        let start_n1 = render_not_found_project(&site, &ctx).into_string();
        for (name, document) in [("page", start_page), ("N1", start_n1)] {
            assert!(html_tag(&document).contains("data-accent"), "{name}");
            assert!(!document.contains("data-align"), "{name}: {document}");
        }

        site.text_align = TextAlign::Justify;
        let page = rail_document(&site, &page, &store);
        let n1 = render_not_found_project(&site, &ctx).into_string();
        for (name, document) in [("page", page), ("N1", n1)] {
            let tag = html_tag(&document);
            assert!(tag.contains(r#" data-align="justify""#), "{name}: {tag}");
        }
        let n2 = render_not_found_site(&cpe_core::url::ROOT, &lang, Appearance::Light).into_string();
        assert!(html_tag(&n2).contains("data-accent"), "{n2}");
        assert!(!n2.contains("data-align"), "{n2}");
    }

    /// The stylesheets select on the attribute `project_html` writes; a
    /// renamed value would leave every justified rule dead with no other
    /// test failing.
    #[test]
    fn text_align_attribute_matches_the_stylesheets() {
        const EDITORIAL_CSS: &str = include_str!("../../components/style/editorial.css");
        const RECORD_CSS: &str = include_str!("../../components/style/record.css");
        let selector = format!(r#"html[data-align="{}"]"#, TextAlign::Justify.as_data_attr());
        for (name, css) in [("editorial.css", EDITORIAL_CSS), ("record.css", RECORD_CSS)] {
            assert!(css.contains(&selector), "{name} has no `{selector}` rule");
            assert_eq!(
                css.matches("html[data-align=").count(),
                css.matches(&selector).count(),
                "{name} selects a data-align value nothing emits"
            );
        }
    }

    /// The justified rules select `.prose > p`: a narrative's intro must emit
    /// its paragraphs as direct children of the prose block, or the text
    /// silently stays ragged. A figure-mode part's prose block is pinned by
    /// `figure_mode_splits_at_each_idd_h2_and_places_blocks_in_order`.
    #[test]
    fn intro_paragraphs_are_direct_children_of_their_prose_block() {
        let site = make_alice_site();
        let page = rail_page(vec![rail_narrative("home.md")]);
        let store = rail_store(&[("home.md", "# Home\n\nIntro.\n\nMore.\n\n## A {#a}\n\nText.\n")]);
        let out = rail_document(&site, &page, &store);
        let (_, after) = out
            .split_once(r#"<div class="prose editorial__intro">"#)
            .expect("the narrative renders an intro");
        // No wrapper element inside, so the first `</div>` closes the prose
        // block itself.
        let (intro, _) = after.split_once("</div>").expect("the intro closes");
        let opened: Vec<&str> = intro
            .split('<')
            .skip(1)
            .filter(|tag| !tag.starts_with('/'))
            .map(|tag| tag.split(['>', ' ']).next().unwrap_or(""))
            .collect();
        assert_eq!(opened, ["p", "p", "h2", "p"], "{intro}");
    }

    /// DataStar evaluates the value of any `data-<plugin>` attribute, so no
    /// attribute a layout rule reads may be named like one: `data-text` on
    /// `<html>` would replace the document's text. The names are read from
    /// the `<html>` tag of each shell with every optional attribute present,
    /// plus the two written on other elements (`ul.gm`, the record article).
    #[test]
    fn no_layout_attribute_is_named_like_a_datastar_plugin() {
        const DATASTAR_JS: &str = include_str!("../../public/vendor/datastar.js");
        // The attribute plugins of DataStar 1.0; the last three are handled
        // by DataStar's core and have no `name:"…"` entry in the bundle.
        const PLUGINS: [&str; 20] = [
            "attr",
            "bind",
            "class",
            "computed",
            "effect",
            "indicator",
            "init",
            "json-signals",
            "on",
            "on-intersect",
            "on-interval",
            "on-signal-patch",
            "ref",
            "show",
            "signals",
            "style",
            "text",
            "ignore",
            "ignore-morph",
            "preserve-attr",
        ];
        let mut plugins: Vec<&str> = PLUGINS.to_vec();
        plugins.extend(DATASTAR_JS.split("name:\"").skip(1).filter_map(|rest| rest.split('"').next()));
        assert!(
            plugins.len() > PLUGINS.len(),
            "the vendored bundle no longer names its plugins as `name:\"…\"`"
        );

        let mut site = make_alice_site();
        site.text_align = TextAlign::Justify;
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = make_browse_ctx(lang.clone(), "/alice-in-daschland/", None);
        let page = rail_page(vec![
            Component::Narrative {
                file: "home.md".to_owned(),
                companion: None,
                side: Some(NarrativeSide::Start),
            },
            rail_narrative("more.md"),
        ]);
        let store = rail_store(&[("home.md", RAIL_HOME), ("more.md", RAIL_MORE)]);
        let documents = [
            rail_document(&site, &page, &store),
            render_not_found_project(&site, &ctx).into_string(),
            render_not_found_site(&cpe_core::url::ROOT, &lang, Appearance::Dark).into_string(),
        ];

        let mut names: Vec<String> = vec!["tile".to_owned(), "frame".to_owned()];
        for document in &documents {
            names.extend(
                html_tag(document)
                    .split(" data-")
                    .skip(1)
                    .filter_map(|rest| rest.split(['=', ' ']).next())
                    .map(str::to_owned),
            );
        }
        for expected in ["narr", "narr-side", "align", "appearance"] {
            assert!(names.iter().any(|n| n == expected), "{expected}: {names:?}");
        }
        for name in &names {
            assert!(!plugins.contains(&name.as_str()), "data-{name} is a DataStar plugin name");
        }
    }

    /// A slot label's `{n}` is the number the component's registry row reads
    /// from its data (the strip row's class total), in the count line and the
    /// more link alike; a component without that hook leaves `{n}` as written.
    #[test]
    fn render_body_fills_a_slot_labels_n_from_the_registry_row() {
        let mut site = make_alice_site();
        site.chrome.push(ChromeString {
            id: "strips-all".to_owned(),
            translations: vec![
                ("en".to_owned(), "All {n} border strips".to_owned()),
                ("de".to_owned(), "Alle {n} Randleisten".to_owned()),
            ],
        });
        let slot = || {
            Some(cpe_core::ir::FigureSlot {
                section: "strips".to_owned(),
                count: Some("strips-all".to_owned()),
                more: Some(cpe_core::ir::FigureMore { label: "strips-all".to_owned(), route: "/strips".to_owned() }),
            })
        };
        let strip_row = Component::StripRow {
            class: "BorderStrip".to_owned(),
            media: "image".to_owned(),
            uses_property: "page-uses".to_owned(),
            min_uses: 9,
            limit: 12,
        };
        let timeline = Component::Timeline {
            class: "Book".to_owned(),
            date_property: "pub-date".to_owned(),
            lane_property: "office".to_owned(),
            extent_property: None,
            axis_start: 1470,
            axis_end: 1506,
        };
        let page = cpe_core::ir::Page {
            id: "narrenschiff".to_owned(),
            route: "/narrenschiff".to_owned(),
            params: vec![],
            body: vec![
                Component::Narrative { file: "n.md".to_owned(), companion: None, side: None },
                strip_row,
                timeline,
            ],
            figure_slots: vec![None, slot(), slot()],
        };
        let narrative = NarrativeStore::from_pairs(&[(
            "n.md".to_owned(),
            "# N\n\nIntro.\n\n## Strips {#strips}\n\nProse.\n".to_owned(),
        )]);
        let components = [
            ComponentData::Other,
            ComponentData::StripRow(cpe_core::ir::StripRowPayload {
                strips: vec![],
                thumbs: HashMap::new(),
                class_total: 38,
            }),
            ComponentData::Timeline(cpe_core::ir::TimelinePayload {
                resources: ResourceListPage {
                    items: vec![],
                    total: 0,
                    page: 1,
                    pagination: cpe_core::ir::Pagination::Limit(0),
                    unfiltered_total: None,
                },
                lanes: vec![],
            }),
        ];
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang: resolve(&["en".to_owned()], &site),
            project_id: "incunabula",
            current_path: "/incunabula/narrenschiff",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let default_lang = resolve(&["en".to_owned()], &site);
        let out = render_body(&page, &ctx, &narrative, &components, &default_lang, &site)
            .0
            .into_string();
        let (strips, timeline) = out
            .split_once(r#"<section class="tl page""#)
            .unwrap_or_else(|| panic!("no timeline block: {out}"));
        assert!(
            strips.contains(concat!(
                r#"<p class="editorial__figure-count">All 38 border strips</p>"#,
                r#"<p class="editorial__figure-more"><a href="/incunabula/strips">All 38 border strips "#,
            )),
            "{out}"
        );
        assert!(
            timeline.contains(r#"<p class="editorial__figure-count">All {n} border strips</p>"#),
            "{out}"
        );
    }

    fn one_annotation() -> Vec<cpe_core::ir::AnnotationEntry> {
        vec![cpe_core::ir::AnnotationEntry::Annotation {
            number: 1,
            annotation: cpe_core::ir::Annotation {
                id: "a1".to_owned(),
                resource_id: "CH_001".to_owned(),
                shape: cpe_core::ir::AnnotationShape::Rectangle,
                x: 0.1,
                y: 0.1,
                w: 0.2,
                h: 0.2,
                points: vec![],
                color: Some("#ff3333".to_owned()),
                remark: None,
                remark_lang: None,
            },
            partners: vec![],
        }]
    }

    /// The register target sits in the tracks after the reader and before the
    /// body (the stacked order), and the `_compound` fragment carries the very
    /// same markup.
    #[test]
    fn annotated_reader_renders_its_register_after_the_reader() {
        let (out, fragment) =
            render_annotated_book(2, cpe_core::ir::CompoundLayout::Pages, true, Some(one_annotation()));
        let reader_at = out.find(r#"id="compound-viewer""#).expect("reader");
        let target_at = out.find(r#"<div id="cv-annotations">"#).expect("target");
        let body_at = out.find(r#"<div class="record__body"#).expect("body");
        assert!(reader_at < target_at && target_at < body_at, "{out}");
        assert!(out.contains(r#"id="annotation-a1""#), "{out}");
        let fragment = fragment.expect("the fragment carries the target");
        assert!(out.contains(&fragment), "fragment differs from the page: {fragment}");
    }

    /// An annotated reader's page without a register still renders the empty
    /// target, in the page and in the fragment, so a swap always has one.
    #[test]
    fn annotated_reader_renders_the_empty_target_on_a_bare_page() {
        let (out, fragment) = render_annotated_book(2, cpe_core::ir::CompoundLayout::Pages, true, Some(vec![]));
        assert!(out.contains(r#"<div id="cv-annotations"></div>"#), "{out}");
        assert_eq!(fragment.as_deref(), Some(r#"<div id="cv-annotations"></div>"#));
    }

    /// Without `annotations=` there is no target at all, in either path.
    #[test]
    fn reader_without_annotations_has_no_target() {
        let (out, fragment) = render_annotated_book(2, cpe_core::ir::CompoundLayout::Pages, true, None);
        assert!(!out.contains("cv-annotations"), "{out}");
        assert_eq!(fragment, None);
    }

    /// A populated reader renders once, inside the record's tracks, between
    /// the head and the body — never again as a sibling after the record.
    #[test]
    fn navigator_page_renders_the_reader_once_inside_the_record() {
        let out = render_book_body(2);
        assert_eq!(out.matches(r#"id="compound-viewer""#).count(), 1, "{out}");
        assert!(
            out.starts_with(
                r#"<article class="record page record--reader"><div class="record__tracks"><header class="record__head">"#
            ),
            "{out}"
        );
        let reader_at = out.find(r#"id="compound-viewer""#).expect("reader");
        let head_end = out.find("</header>").expect("head");
        let body_at = out.find(r#"<div class="record__body"#).expect("body");
        assert!(head_end < reader_at && reader_at < body_at, "{out}");
        assert!(out.ends_with("</article>"), "{out}");
        assert_eq!(out.matches("<article").count(), 2, "{out}");
    }

    /// A zero-page compound is not nested: the record renders untracked and
    /// the reader's own empty sentence follows it as a sibling.
    #[test]
    fn zero_page_compound_is_not_nested_in_the_record() {
        let out = render_book_body(0);
        assert!(!out.contains("record__tracks"), "{out}");
        assert!(!out.contains("record--reader"), "{out}");
        let record_end = out.find("</article>").expect("record");
        let reader_at = out.find(r#"id="compound-viewer""#).expect("reader");
        assert!(record_end < reader_at, "{out}");
    }

    /// Only a `pages` reader nests: a paired `contents` navigator stays a
    /// sibling after the record.
    #[test]
    fn contents_layout_navigator_is_not_nested_in_the_record() {
        let out = render_book_body_with(2, cpe_core::ir::CompoundLayout::Contents, true);
        assert!(!out.contains("record__tracks"), "{out}");
        assert_eq!(out.matches(r#"id="compound-viewer""#).count(), 1, "{out}");
    }

    /// A record that did not resolve is not nested either: the not-found
    /// body and the reader both still render once.
    #[test]
    fn unresolved_record_does_not_nest_the_reader() {
        let out = render_book_body_with(2, cpe_core::ir::CompoundLayout::Pages, false);
        assert!(!out.contains("record__tracks"), "{out}");
        assert_eq!(out.matches(r#"id="compound-viewer""#).count(), 1, "{out}");
    }

    /// A stale `?page=` on a two-page `pages` reader,
    /// rendered as the full page (`render_page`) and as the `_compound`
    /// fragment (`render_compound_fragment`) for the same query.
    fn render_stale_book(query: &str) -> (String, String) {
        render_stale_book_at(query, 1)
    }

    /// [`render_stale_book`], showing the `index`-th page (the `?member=`
    /// fallback beside the stale value).
    fn render_stale_book_at(query: &str, index: u32) -> (String, String) {
        use cpe_core::ir::{CompoundLayout, CompoundPage, MemberHeader, MemberIndex};
        let site = make_alice_site();
        let page = cpe_core::ir::Page {
            id: "reader".to_owned(),
            route: "/books/:id".to_owned(),
            params: vec!["id".to_owned()],
            body: vec![Component::CompoundViewer {
                source: "$id".to_owned(),
                media: None,
                picker: None,
                layout: CompoundLayout::Pages,
                index: MemberIndex::Off,
                paired: false,
                member_links: None,
                annotations: false,
            }],
            figure_slots: Vec::new(),
        };
        let members: Vec<MemberHeader> = (1..=2)
            .map(|n| MemberHeader {
                id: format!("p{n}"),
                class: "Page".to_owned(),
                name: format!("a{n}r"),
            })
            .collect();
        let compound = CompoundPage {
            member: Some(make_alice_resource()),
            index,
            total: 2,
            members,
            member_media: vec![],
            stale_page: Some("zzz".to_owned()),
            annotations: vec![],
            annotated_members: std::collections::BTreeSet::new(),
        };
        let route_params = [("id".to_owned(), "B1".to_owned())];
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang: resolve(&["de".to_owned()], &site),
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/books/B1",
            route_params: &route_params,
            current_query: Some(query),
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components = [ComponentData::Compound(compound)];
        let narrative = make_alice_narrative();
        let full = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        let fragment = render_compound_fragment(&site, &page, &narrative, &ctx, &components, &DataArks::default())
            .expect("a compound-viewer page renders a fragment")
            .viewer
            .into_string();
        (full, fragment)
    }

    /// The full page corrects a stale address before the reader's URL seed
    /// runs, in the same script, to the path and query minus `page` and
    /// `member`, JS-string-escaped; the `_compound` fragment never does.
    #[test]
    fn stale_page_replace_state_is_full_page_only_and_precedes_the_seed() {
        let (full, fragment) = render_stale_book("page=zzz&lang=de&member=2");
        assert!(
            full.contains(
                "<script>history.replaceState(null, '', '/alice-in-daschland/books/B1?lang=de');window.__cpeReaderUrl = location.pathname + location.search;</script>"
            ),
            "got: {full}"
        );
        assert_eq!(full.matches("replaceState").count(), 1, "got: {full}");
        assert!(!fragment.contains("replaceState"), "got: {fragment}");
        assert!(
            fragment.contains("<script>window.__cpeReaderUrl = location.pathname + location.search;</script>"),
            "got: {fragment}"
        );
        let (bare, _) = render_stale_book("page=zzz");
        assert!(bare.contains("history.replaceState(null, '', '/alice-in-daschland/books/B1');"));
        let (hostile, _) = render_stale_book("page=zzz&x=%27</script><b>");
        assert!(
            hostile
                .contains(r"history.replaceState(null, '', '/alice-in-daschland/books/B1?x=%27\x3C/script>\x3Cb>');"),
            "got: {hostile}"
        );
    }

    /// A `?member=N` beside the stale value decides the page shown; the
    /// corrected address names that page, never the bare (page-1) URL.
    #[test]
    fn stale_page_replace_state_keeps_the_page_shown_by_member() {
        let (full, _) = render_stale_book_at("page=zzz&lang=de&member=2", 2);
        assert!(
            full.contains("history.replaceState(null, '', '/alice-in-daschland/books/B1?lang=de&page=CH_001');"),
            "got: {full}"
        );
        let (only, _) = render_stale_book_at("page=zzz&member=2", 2);
        assert!(
            only.contains("history.replaceState(null, '', '/alice-in-daschland/books/B1?page=CH_001');"),
            "got: {only}"
        );
    }

    // The incoming-links caption, through `render_component`
    //
    // The ResourceDetail arm is the layer that PERFORMS the caption fallback
    // (chrome id → source-class plural) and pairs it with the host's rows.

    fn render_incoming_detail(label: &str, rows: Option<&[cpe_core::ir::IncomingLink]>, lang: &str) -> String {
        render_incoming_detail_grouped(label, rows, None, lang)
    }

    fn render_incoming_detail_grouped(
        label: &str,
        rows: Option<&[cpe_core::ir::IncomingLink]>,
        grouping: Option<cpe_core::ir::IncomingGrouping>,
        lang: &str,
    ) -> String {
        let incoming_links = rows.map(|r| cpe_core::ir::IncomingLinks { rows: r.to_vec(), total: r.len() });
        let mut site = make_alice_site();
        site.class_labels = vec![ClassLabel {
            class: "Folio".to_owned(),
            translations: vec![
                ("en".to_owned(), "Folio".to_owned()),
                ("de".to_owned(), "Blatt".to_owned()),
            ],
            plural: vec![
                ("en".to_owned(), "folios".to_owned()),
                ("de".to_owned(), "Blätter".to_owned()),
            ],
        }];
        let component = Component::ResourceDetail {
            head: None,
            id: Some("$id".to_owned()),
            show_properties: None,
            title_property: Some("name".to_owned()),
            class: None,
            media: None,
            groups: vec![],
            narrative_for: vec![],
            incoming_links: Some(cpe_core::ir::IncomingLinksSpec {
                label: label.to_owned(),
                class: "Folio".to_owned(),
                properties: vec!["band-left".to_owned()],
                order: cpe_core::ir::IncomingOrder::Name,
                limit: cpe_core::ir::DEFAULT_INCOMING_LINKS_LIMIT,
                grouping,
                row_label: None,
                row_aside: None,
                sort_property: None,
            }),
            navigator: false,
        };
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang: resolve(&[lang.to_owned()], &site),
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/CH_001",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: incoming_links.as_ref(),
            resource_links: &[],
        };
        let default_lang = resolve(&["en".to_owned()], &site);
        let data = ComponentData::Detail(Some(make_alice_resource()));
        render_component(&component, &ctx, &make_alice_narrative(), &data, &default_lang, &site).into_string()
    }

    #[test]
    fn incoming_caption_uses_declared_chrome_id() {
        let out = render_incoming_detail("nav-home", Some(&[]), "de");
        assert!(out.contains(r#"<span class="pgroup__title">Startseite</span>"#), "{out}");
    }

    #[test]
    fn incoming_caption_falls_back_to_source_class_plural() {
        let out = render_incoming_detail("no-such-id", Some(&[]), "de");
        assert!(
            out.contains(r#"<span class="pgroup__title">Blätter</span>"#),
            "undeclared id ⇒ the source class's capitalized plural: {out}"
        );
        let en = render_incoming_detail("no-such-id", Some(&[]), "en");
        assert!(en.contains(r#"<span class="pgroup__title">Folios</span>"#), "capitalized: {en}");
        assert!(!out.contains("no-such-id"), "never the raw id: {out}");
    }

    fn incoming_note_grouping(note_label: &str) -> Option<cpe_core::ir::IncomingGrouping> {
        Some(cpe_core::ir::IncomingGrouping {
            by: cpe_core::ir::IncomingGroupBy::Via,
            layout: cpe_core::ir::IncomingLayout::Columns,
            note_label: Some(note_label.to_owned()),
        })
    }

    fn incoming_detail_row() -> cpe_core::ir::IncomingLink {
        cpe_core::ir::IncomingLink {
            id: "F_1".to_owned(),
            class: "Folio".to_owned(),
            name: "Folio one".to_owned(),
            via: "band-left".to_owned(),
            parent: None,
            group: None,
            label: None,
            aside: None,
        }
    }

    #[test]
    fn incoming_note_label_resolves_the_chrome_id_for_the_language() {
        let rows = [incoming_detail_row()];
        let out = render_incoming_detail_grouped("nav-home", Some(&rows), incoming_note_grouping("nav-home"), "de");
        assert!(out.contains(r#"<span class="incoming__note">Startseite</span>"#), "{out}");
        assert!(out.contains("incoming--grouped incoming--columns"), "{out}");
    }

    #[test]
    fn incoming_undeclared_note_label_renders_no_note() {
        let rows = [incoming_detail_row()];
        let out = render_incoming_detail_grouped("nav-home", Some(&rows), incoming_note_grouping("no-such-id"), "de");
        assert!(!out.contains("incoming__note"), "{out}");
        assert!(!out.contains("no-such-id"), "never the raw id: {out}");
    }

    #[test]
    fn incoming_spec_without_resolved_rows_renders_nothing() {
        // A failed lookup reaches the renderer as `ctx.incoming_links: None`.
        let out = render_incoming_detail("nav-home", None, "de");
        assert!(!out.contains("incoming"), "{out}");
        assert!(!out.contains("Nichts verweist hierher."), "{out}");
    }

    // `narrative-for` intros, through `render_component`
    //
    // `render_component`'s ResourceDetail arm is the layer that PERFORMS the
    // id match and the language fallback (`resolve_narrative_content`), so the
    // tests sit here, not on `render_resource_detail` (which only receives the
    // already-resolved `Markup`).

    /// Synthetic intro files: `intro-a` has EN (default) + DE variants,
    /// `intro-b` only the default-language EN variant (the fallback case).
    fn make_intro_narrative() -> NarrativeStore {
        let pairs: Vec<(String, String)> = vec![
            (
                "intro-a.en.md".to_owned(),
                "An *English* introduction.\n\nSecond paragraph.".to_owned(),
            ),
            ("intro-a.de.md".to_owned(), "Eine *deutsche* Einführung.".to_owned()),
            ("intro-b.en.md".to_owned(), "Only the default-language intro.".to_owned()),
        ];
        NarrativeStore::from_pairs(&pairs)
    }

    /// Render the synthetic `CH_001` record through `render_component` with
    /// the given `narrative_for` entries, requested in `lang` (site default
    /// `en`).
    fn render_intro_detail(narrative_for: Vec<cpe_core::ir::NarrativeFor>, lang: &str) -> String {
        let site = make_alice_site();
        let component = Component::ResourceDetail {
            id: Some("$id".to_owned()),
            show_properties: Some(vec!["description".to_owned(), "role".to_owned()]),
            title_property: Some("name".to_owned()),
            class: None,
            media: None,
            groups: vec![],
            narrative_for,
            incoming_links: None,
            head: None,
            navigator: false,
        };
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang: resolve(&[lang.to_owned()], &site),
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/CH_001",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let default_lang = resolve(&["en".to_owned()], &site);
        let data = ComponentData::Detail(Some(make_alice_resource()));
        render_component(&component, &ctx, &make_intro_narrative(), &data, &default_lang, &site).into_string()
    }

    fn intro_for(resource_id: &str, file: &str) -> cpe_core::ir::NarrativeFor {
        cpe_core::ir::NarrativeFor { resource_id: resource_id.to_owned(), file: file.to_owned() }
    }

    #[test]
    fn resource_detail_intro_absent_when_no_entry_matches() {
        let with_other = render_intro_detail(vec![intro_for("CH_999", "intro-a.md")], "en");
        let with_none = render_intro_detail(vec![], "en");
        assert_eq!(
            with_other, with_none,
            "an entry for another resource must leave the record byte-identical"
        );
        assert!(!with_other.contains("record__intro"), "{with_other}");
        assert!(!with_other.contains("record__apparat"), "{with_other}");
        assert!(
            with_other.contains(r#"<div class="record__body record__body--noimg">"#),
            "body class unchanged: {with_other}"
        );
    }

    #[test]
    fn resource_detail_intro_renders_for_matching_id() {
        let out = render_intro_detail(vec![intro_for("CH_001", "intro-a.md")], "en");
        assert!(
            out.contains(
                r#"<div class="record__body record__body--noimg record__body--intro"><div class="record__intro"><div class="prose"><p>An <em>English</em> introduction.</p>"#
            ),
            "the intro must be the body's first child, in div.prose: {out}"
        );
        let intro_at = out.find("record__intro").expect("intro present");
        let apparat_at = out
            .find(r#"<div class="record__apparat"><div class="props">"#)
            .expect("the property rows must be wrapped in div.record__apparat");
        assert!(intro_at < apparat_at, "intro precedes the apparat: {out}");
        assert!(
            !out.contains("editorial"),
            "never the editorial wrapper (a nested .page root): {out}"
        );
        assert!(
            !out[intro_at..apparat_at].contains("page"),
            "the intro must carry no `.page` (not a component root): {out}"
        );
    }

    #[test]
    fn resource_detail_intro_prefers_requested_language_variant() {
        let out = render_intro_detail(vec![intro_for("CH_001", "intro-a.md")], "de");
        assert!(
            out.contains("Eine <em>deutsche</em> Einführung."),
            "the requested-language (de) variant must win: {out}"
        );
        assert!(!out.contains("English"), "{out}");
    }

    #[test]
    fn resource_detail_intro_falls_back_to_default_language_variant() {
        // `intro-b` has no `de` variant and no base file: the chain must fall
        // back to the site default (`en`), not render an empty intro.
        let out = render_intro_detail(vec![intro_for("CH_001", "intro-b.md")], "de");
        assert!(
            out.contains(r#"<div class="record__intro"><div class="prose"><p>Only the default-language intro.</p>"#),
            "missing requested variant must fall back to the default-language one: {out}"
        );
    }

    #[test]
    fn resource_detail_intro_uses_first_matching_entry_only() {
        let out = render_intro_detail(vec![intro_for("CH_999", "intro-a.md"), intro_for("CH_001", "intro-b.md")], "en");
        assert!(out.contains("Only the default-language intro."), "{out}");
        assert!(!out.contains("English"), "{out}");
    }

    #[test]
    fn snapshot_resource_detail_with_intro() {
        let out = render_intro_detail(vec![intro_for("CH_001", "intro-a.md")], "en");
        insta::assert_snapshot!(out);
    }

    /// The resource-detail breadcrumb gains a live-linked class segment
    /// (`CPE › project › class › resource-title`) when the page's
    /// `resource-detail` declares a `class` AND that class has a registered
    /// browse route in `ctx.class_list_routes` — mirrors the not-found variant's
    /// `render_page_not_found_breadcrumb_has_class_crumb_when_class_routed`,
    /// but on the resolved-record path.
    #[test]
    fn render_page_resource_detail_breadcrumb_has_class_crumb_when_class_routed() {
        let mut site = make_alice_site();
        site.class_labels = vec![ClassLabel {
            class: "Character".to_owned(),
            translations: vec![("en".to_owned(), "Character".to_owned())],
            plural: vec![("en".to_owned(), "characters".to_owned())],
        }];
        let page = cpe_core::ir::Page {
            id: "character".to_owned(),
            route: "/characters/:id".to_owned(),
            params: vec!["id".to_owned()],
            body: vec![Component::ResourceDetail {
                id: Some("$id".to_owned()),
                show_properties: Some(vec!["name".to_owned()]),
                title_property: Some("name".to_owned()),
                class: Some("Character".to_owned()),
                media: None,
                groups: vec![],
                narrative_for: vec![],
                incoming_links: None,
                head: None,
                navigator: false,
            }],
            figure_slots: Vec::new(),
        };
        let narrative = make_alice_narrative();
        let resource = make_alice_resource();
        let lang = resolve(&["en".to_owned()], &site);
        let class_list_routes: HashMap<String, String> = [("Character".to_owned(), "/characters".to_owned())].into();
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/CH_001",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &class_list_routes,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(Some(resource))];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        assert_eq!(
            out.matches("breadcrumb__item").count(),
            4,
            "a class with a registered browse route must yield the 4-item trail \
             (CPE, project, class, resource), got: {out}"
        );
        assert!(
            out.contains("href=\"/alice-in-daschland/characters\""),
            "class crumb must be a live link to the browse route, got: {out}"
        );
        // `resolve_class_plural_capitalized` is used — the SAME
        // capitalized-plural noun `render_browse_breadcrumb_for_page` shows
        // on the browse page's own current crumb:
        // "Characters", not the singular "Character".
        // This fixture declares `class
        // "Character" { en "Character" plural { en "characters" } }`, so the
        // capitalized-plural and the raw singular/declared-plural strings
        // are all distinguishable here. Scope the assertions to the
        // `nav.breadcrumb` element itself — `record__class`'s own eyebrow
        // (a SEPARATE, deliberately-singular resolution, `resolve_class_label`
        // via `resource_detail.rs`) also legitimately renders ">Character<"
        // elsewhere on this same page.
        let breadcrumb_start = out.find("<nav class=\"breadcrumb\"").expect("breadcrumb present");
        let breadcrumb_end = out[breadcrumb_start..].find("</nav>").expect("breadcrumb nav must be closed");
        let breadcrumb_slice = &out[breadcrumb_start..breadcrumb_start + breadcrumb_end];
        assert!(
            breadcrumb_slice.contains(">Characters<"),
            "class crumb label must be the capitalized plural class noun, \
             matching what the browse page's own crumb shows, got: {breadcrumb_slice}"
        );
        assert!(
            !breadcrumb_slice.contains(">Character<"),
            "class crumb must not use the singular noun, got: {breadcrumb_slice}"
        );
        assert!(
            !breadcrumb_slice.contains(">characters<"),
            "class crumb must not use the lowercase (sentence-copy) plural noun, \
             got: {breadcrumb_slice}"
        );
    }

    /// Fallback: a `resource-detail` declaring a `class` with NO registered
    /// browse route in `ctx.class_list_routes` renders the original 3-crumb
    /// trail unchanged — no dead segment, no unlinked text.
    #[test]
    fn render_page_resource_detail_breadcrumb_omits_class_crumb_when_no_route() {
        let site = make_alice_site();
        let page = cpe_core::ir::Page {
            id: "character".to_owned(),
            route: "/characters/:id".to_owned(),
            params: vec!["id".to_owned()],
            body: vec![Component::ResourceDetail {
                id: Some("$id".to_owned()),
                show_properties: Some(vec!["name".to_owned()]),
                title_property: Some("name".to_owned()),
                class: Some("Character".to_owned()),
                media: None,
                groups: vec![],
                narrative_for: vec![],
                incoming_links: None,
                head: None,
                navigator: false,
            }],
            figure_slots: Vec::new(),
        };
        let narrative = make_alice_narrative();
        let resource = make_alice_resource();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/CH_001",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(Some(resource))];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        assert_eq!(
            out.matches("breadcrumb__item").count(),
            3,
            "a class with no registered browse route must fall back to the \
             3-item trail (no dead class segment), got: {out}"
        );
    }

    /// The resource-detail control's href reflects
    /// the request's own load-time `?img=` state (`ctx.current_query` is
    /// the raw request query string; `compose_canonical_query`'s
    /// `ViewSurface::Record` branch keeps `img` when it's > 1) — no new
    /// wiring needed, this asserts the existing plumbing is correct.
    #[test]
    fn render_page_resource_detail_share_href_reflects_load_time_img() {
        let site = make_alice_site();
        let page = make_character_page();
        let narrative = make_alice_narrative();
        let resource = make_alice_resource();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/CH_001",
            route_params: &[],
            current_query: Some("img=3"),
            appearance: Appearance::Light,
            selected_image: 2,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(Some(resource))];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        assert!(
            out.contains("href=\"http://localhost:8080/alice-in-daschland/characters/CH_001?img=3\""),
            "share control href must carry the load-time ?img= state, got: {out}"
        );
        // Accessible name stays the bare prefix — record surfaces never
        // announce which state they caught.
        assert!(
            out.contains("aria-label=\"Copy link to this view\""),
            "record share control must have no accessible-name suffix, got: {out}"
        );
    }

    /// A REAL multi-image, all-IIIF
    /// `media_items` fixture drives `render_page`'s resource-detail branch
    /// through `media::has_sequence_pager` (true for this fixture — 3 images,
    /// every one IIIF-sequenceable) and asserts the resulting share-control
    /// `<a>` carries the reactive `data-attr="{'href': …}"` expression with
    /// the ACTUAL `img_prefix`/`static_url_no_img`/`base_url` composition
    /// (not a hand-written literal — contrast with
    /// `breadcrumb.rs`'s `share_control_reactive_href_attr_renders_data_attr_when_present`,
    /// which only checks that an already-composed literal string renders as
    /// `data-attr`). `ctx.current_query` carries an unrelated `lang=de` param
    /// so the "merge with other existing query params" path is exercised,
    /// not just the empty-query case. Mirrors
    /// `compose_canonical_query`'s own `ViewSurface::Record` `img`-omission
    /// rule (omitted when 1) so the two can never diverge silently.
    #[test]
    fn render_page_resource_detail_reactive_href_composes_real_multi_image_iiif_fixture() {
        use cpe_core::ir::{Representation, RepresentationKind, Resource as IrResource};

        let site = make_alice_site();
        let page = make_character_page();
        let narrative = make_alice_narrative();
        let resource = make_alice_resource();
        let lang = resolve(&["en".to_owned()], &site);

        fn iiif_image(id: &str) -> IrResource {
            IrResource {
                id: id.to_owned(),
                class: "Image".to_owned(),
                properties: vec![],
                representations: vec![Representation {
                    kind: RepresentationKind::Image,
                    uri: format!("https://iiif.demo.dasch.swiss:443/0854/{id}.jpx/full/max/0/default.jpg"),
                    width: Some(1000),
                    height: Some(1200),
                    alt: Some("An illustration".to_owned()),
                    attribution: Some("John Tenniel · Public Domain".to_owned()),
                    seq: 0,
                }],
            }
        }
        let media_items = vec![iiif_image("I_001"), iiif_image("I_002"), iiif_image("I_003")];
        assert!(
            media::has_sequence_pager(&media_items),
            "fixture sanity check: 3 all-IIIF images must satisfy has_sequence_pager"
        );

        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/CH_001",
            route_params: &[],
            current_query: Some("lang=de"),
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &media_items,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(Some(resource))];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();

        // Server-rendered static href keeps `lang=de`, omits `img` (no `img`
        // param on this request, so it's absent from `current_query` too —
        // nothing to omit, but the OTHER param must survive the merge).
        assert!(
            out.contains("href=\"http://localhost:8080/alice-in-daschland/characters/CH_001?lang=de\""),
            "static load-time href must preserve the unrelated lang= param, got: {out}"
        );

        // The reactive data-attr must be present (pager offered) and must be
        // canonical-by-construction: a JS array literal of ALL THREE
        // per-plate URLs, indexed by `$plate` — each one composed by the
        // REAL `compose_canonical_query` + `compose_absolute_url`,
        // which puts `img` BEFORE
        // `lang` (`?img=2&lang=de`, never
        // `?lang=de&img=2`).
        let raw_queries_per_index = ["lang=de&img=1", "lang=de&img=2", "lang=de&img=3"];
        let expected_urls: Vec<String> = raw_queries_per_index
            .iter()
            .map(|raw| {
                crate::share_link::compose_absolute_url(
                    "http://localhost:8080",
                    "/alice-in-daschland/characters/CH_001",
                    &crate::share_link::compose_canonical_query(
                        Some(raw),
                        &[],
                        crate::share_link::ViewSurface::Record,
                        None,
                        &[],
                    ),
                )
            })
            .collect();
        assert_eq!(
            expected_urls,
            vec![
                "http://localhost:8080/alice-in-daschland/characters/CH_001?lang=de",
                "http://localhost:8080/alice-in-daschland/characters/CH_001?img=2&lang=de",
                "http://localhost:8080/alice-in-daschland/characters/CH_001?img=3&lang=de",
            ],
            "each array entry must be byte-identical to compose_canonical_query's own output for that img index (img before lang — the canonical order)"
        );
        let expected_data_attr_unescaped = format!(
            "{{'href': ['{}','{}','{}'][$plate]}}",
            expected_urls[0], expected_urls[1], expected_urls[2]
        );
        let idx = out.find("data-attr=\"").expect("data-attr must be present");
        let after = &out[idx + "data-attr=\"".len()..];
        let end = after.find('"').expect("data-attr value must be closed");
        let unescaped = after[..end]
            .replace("&amp;", "&")
            .replace("&#x27;", "'")
            .replace("&quot;", "\"");
        assert_eq!(
            unescaped, expected_data_attr_unescaped,
            "share control must carry the reactive data-attr composed from the REAL \
             per-plate-index URL array, got page:\n{out}"
        );
    }

    /// A record page
    /// that ALSO carries a compound-viewer (the `/books/:id` shape) threads the
    /// RESOLVED reader member into both the static Copy-link href and every
    /// entry of the reactive `$plate` URL list — `img` · `page=<id>` · `lang`.
    /// The request typed a name (`page=Two`); only the resolved id `M_2`
    /// may appear. Member 1 carries no `page=` at all.
    #[test]
    fn render_record_breadcrumb_threads_reader_member_into_static_and_plate_hrefs() {
        use cpe_core::ir::{CompoundPage, MemberHeader, Representation, RepresentationKind, Resource as IrResource};

        let site = make_alice_site();
        let page = make_character_page();
        fn iiif_image(id: &str) -> IrResource {
            IrResource {
                id: id.to_owned(),
                class: "Image".to_owned(),
                properties: vec![],
                representations: vec![Representation {
                    kind: RepresentationKind::Image,
                    uri: format!("https://iiif.demo.dasch.swiss:443/0854/{id}.jpx/full/max/0/default.jpg"),
                    width: Some(1000),
                    height: Some(1200),
                    alt: None,
                    attribution: None,
                    seq: 0,
                }],
            }
        }
        let media_items = vec![iiif_image("I_001"), iiif_image("I_002")];
        let compound = |index: u32| CompoundPage {
            member: None,
            index,
            total: 2,
            members: vec![
                MemberHeader {
                    id: "M_1".to_owned(),
                    class: "Page".to_owned(),
                    name: "One".to_owned(),
                },
                MemberHeader {
                    id: "M_2".to_owned(),
                    class: "Page".to_owned(),
                    name: "Two".to_owned(),
                },
            ],
            member_media: vec![],
            stale_page: None,
            annotations: vec![],
            annotated_members: std::collections::BTreeSet::new(),
        };
        let base = "http://localhost:8080/alice-in-daschland/characters/CH_001";
        for (index, query, member_part) in [(2, "page=Two&lang=de", "page=M_2&"), (1, "page=One&lang=de", "")] {
            let ctx = RenderContext {
                base: &cpe_core::url::ROOT,
                lang: resolve(&["en".to_owned()], &site),
                project_id: "alice-in-daschland",
                current_path: "/alice-in-daschland/characters/CH_001",
                route_params: &[],
                current_query: Some(query),
                appearance: Appearance::Light,
                selected_image: 0,
                link_routes: &EMPTY_ROUTES,
                media_properties: &EMPTY_ROUTES,
                media_items: &media_items,
                strip_open: true,
                media_thumbs: &EMPTY_MEDIA_THUMBS,
                compound_members: &[],
                compound_parents: &EMPTY_PARENTS,
                class_list_routes: &EMPTY_ROUTES,
                public_origin: "http://localhost:8080",
                incoming_links: None,
                resource_links: &[],
            };
            let components = vec![
                ComponentData::Detail(Some(make_alice_resource())),
                ComponentData::Compound(compound(index)),
            ];
            let out = render_record_breadcrumb_for_page(&page, &ctx, &components, &site, &DataArks::default())
                .expect("a resource-detail page has a record crumb")
                .into_string();
            // `share_href_for_reader` is unconditionally anchor-free —
            // even a non-first member's static Copy-link href never carries
            // `#compound-viewer` any more (member paging is a URL-writing
            // pushState/popstate swap, not a same-document anchor scroll).
            // The `$plate`-reactive array below is a separate, untouched
            // mechanism (image paging within the resolved member) and
            // carries no anchor either.
            assert!(
                out.replace("&amp;", "&")
                    .contains(&format!("href=\"{base}?{member_part}lang=de\"")),
                "member {index}: static Copy-link href, got: {out}"
            );
            assert!(
                !out.contains("#compound-viewer"),
                "member {index}: Copy-link href must never carry the anchor, got: {out}"
            );
            let expected_attr =
                format!("{{'href': ['{base}?{member_part}lang=de','{base}?img=2&{member_part}lang=de'][$plate]}}");
            let idx = out.find("data-attr=\"").expect("data-attr must be present");
            let after = &out[idx + "data-attr=\"".len()..];
            let end = after.find('"').expect("data-attr value must be closed");
            let unescaped = after[..end]
                .replace("&amp;", "&")
                .replace("&#x27;", "'")
                .replace("&quot;", "\"");
            assert_eq!(unescaped, expected_attr, "member {index}: $plate URL list");
            assert!(
                !out.contains("page=Two") && !out.contains("page=One"),
                "member {index}: the typed name must never leak, got: {out}"
            );
        }
    }

    /// A resource-detail page with no resolved resource
    /// (not found) DOES render a breadcrumb — the not-found variant
    /// (`CPE › project › "Not found"`). `make_character_page`'s
    /// `resource-detail` has `class: None`, so no class crumb is expected
    /// here (see `render_page_not_found_breadcrumb_has_class_crumb_when_class_routed`
    /// for the class-crumb case).
    #[test]
    fn render_page_not_found_has_not_found_breadcrumb() {
        let site = make_alice_site();
        let page = make_character_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/NOPE",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(None)];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        assert!(
            out.contains("class=\"breadcrumb\""),
            "not-found page must have a breadcrumb (N3 variant), got: {out}"
        );
        assert!(
            out.contains("Not found"),
            "not-found breadcrumb current item must read 'Not found', got: {out}"
        );
        assert_eq!(
            out.matches("breadcrumb__item").count(),
            3,
            "class-less resource-detail must yield the 3-item trail (no class crumb), got: {out}"
        );
    }

    /// The not-found class crumb appears only when the page's `resource-detail`
    /// declares a `class` AND that class has a registered browse route in
    /// `ctx.class_list_routes` — a resolved record page
    /// never gains a class crumb (only the not-found variant does).
    #[test]
    fn render_page_not_found_breadcrumb_has_class_crumb_when_class_routed() {
        let site = make_alice_site();
        let page = cpe_core::ir::Page {
            id: "character".to_owned(),
            route: "/characters/:id".to_owned(),
            params: vec!["id".to_owned()],
            body: vec![Component::ResourceDetail {
                id: Some("$id".to_owned()),
                show_properties: Some(vec!["name".to_owned()]),
                title_property: Some("name".to_owned()),
                class: Some("Character".to_owned()),
                media: None,
                groups: vec![],
                narrative_for: vec![],
                incoming_links: None,
                head: None,
                navigator: false,
            }],
            figure_slots: Vec::new(),
        };
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let class_list_routes: HashMap<String, String> = [("Character".to_owned(), "/characters".to_owned())].into();
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/NOPE",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &class_list_routes,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(None)];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        assert_eq!(
            out.matches("breadcrumb__item").count(),
            4,
            "a routed class must yield the 4-item trail (class crumb present), got: {out}"
        );
        assert!(
            out.contains("href=\"/alice-in-daschland/characters\""),
            "class crumb must link to the registered browse route, got: {out}"
        );
    }

    // Breadcrumb: browse surfaces

    /// A faceted browse page: `FacetPanel` (real `Facet`s, rail order
    /// role → theme) targeting a `ResourceList` (contrast with `make_faceted_page`
    /// above, whose `FacetPanel.facets` is deliberately empty for the empty-facet-panel
    /// test).
    fn make_faceted_browse_page_with_facets() -> cpe_core::ir::Page {
        use cpe_core::ir::{Facet, FacetKind, Pagination};
        cpe_core::ir::Page {
            id: "characters".to_owned(),
            route: "/characters".to_owned(),
            params: vec![],
            body: vec![
                Component::FacetPanel {
                    target: "character-list".to_owned(),
                    facets: vec![
                        Facet { property: "role".to_owned(), kind: FacetKind::List },
                        Facet { property: "theme".to_owned(), kind: FacetKind::List },
                    ],
                },
                Component::ResourceList {
                    id: Some("character-list".to_owned()),
                    class: "Character".to_owned(),
                    wheres: vec![],
                    sorts: vec![],
                    pagination: Pagination::PerPage(24),
                    gloss: None,
                    chips: vec![],
                    media: None,
                    grid: false,
                    tile_ratio: None,
                    order_name_label: None,
                    order_name_desc_label: None,
                    order_default_label: None,
                    order_name_property: None,
                    order_options: vec![],
                },
            ],
            figure_slots: Vec::new(),
        }
    }

    /// A non-faceted browse page: a lone `ResourceList` with no sibling
    /// `FacetPanel`.
    fn make_nonfaceted_browse_page() -> cpe_core::ir::Page {
        cpe_core::ir::Page {
            id: "chapters".to_owned(),
            route: "/chapters".to_owned(),
            params: vec![],
            body: vec![Component::ResourceList {
                id: Some("chapter-list".to_owned()),
                class: "StoryChapter".to_owned(),
                wheres: vec![],
                sorts: vec![],
                pagination: cpe_core::ir::Pagination::PerPage(24),
                gloss: None,
                chips: vec![],
                media: None,
                grid: false,
                tile_ratio: None,
                order_name_label: None,
                order_name_desc_label: None,
                order_default_label: None,
                order_name_property: None,
                order_options: vec![],
            }],
            figure_slots: Vec::new(),
        }
    }

    fn make_browse_ctx<'a>(lang: Lang, current_path: &'a str, current_query: Option<&'a str>) -> RenderContext<'a> {
        RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path,
            route_params: &[],
            current_query,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        }
    }

    /// Refined, faceted (role + theme selected, active query): the
    /// accessible-name suffix matches this example
    /// verbatim ("3 of 54 <class plural> · Supporting · Fantasy · “hat”" —
    /// the count segment carries the class noun; this fixture
    /// declares no `ClassLabel`, so the noun humanizes to "Character") and the href
    /// carries `q`/`role`/`theme` in canonical order.
    #[test]
    fn browse_breadcrumb_faceted_refined_en() {
        use cpe_core::ir::{FacetGroup, FacetKind, FacetOption, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_browse_page_with_facets();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = make_browse_ctx(lang, "/alice-in-daschland/characters", Some("q=hat&role=R_01&theme=T_02"));
        let facets_data = vec![
            FacetGroup {
                property: "role".to_owned(),
                label: "Role".to_owned(),
                kind: FacetKind::List,
                options: vec![FacetOption {
                    value: "R_01".to_owned(),
                    label: "Supporting".to_owned(),
                    count: 3,
                    selected: true,
                }],
            },
            FacetGroup {
                property: "theme".to_owned(),
                label: "Theme".to_owned(),
                kind: FacetKind::List,
                options: vec![FacetOption {
                    value: "T_02".to_owned(),
                    label: "Fantasy".to_owned(),
                    count: 3,
                    selected: true,
                }],
            },
        ];
        let list_page = ResourceListPage {
            items: vec![],
            total: 3,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(54),
        };
        let components: Vec<ComponentData> = vec![ComponentData::Facets(facets_data), ComponentData::List(list_page)];
        let out = render_browse_breadcrumb_for_page(&page, &ctx, &components, &site)
            .expect("faceted, id-bearing ResourceList must yield a browse breadcrumb")
            .into_string();
        assert!(
            out.contains("aria-label=\"Copy link to this view — 3 of 54 Character · Supporting · Fantasy · \u{201C}hat\u{201D}\""),
            "accessible-name suffix must match the design doc example verbatim, got: {out}"
        );
        assert!(
            out.contains(
                "href=\"http://localhost:8080/alice-in-daschland/characters?q=hat&amp;role=R_01&amp;theme=T_02\""
            ),
            "href must compose q/role/theme in canonical order, got: {out}"
        );
        assert!(
            out.contains("id=\"share-character-list\""),
            "control id must be stable, per-list, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// Same fixture, DE — count text and quote marks localize.
    #[test]
    fn browse_breadcrumb_faceted_refined_de() {
        use cpe_core::ir::{FacetGroup, FacetKind, FacetOption, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_browse_page_with_facets();
        let lang = resolve(&["de".to_owned()], &site);
        let ctx = make_browse_ctx(lang, "/alice-in-daschland/characters", Some("q=hat&role=R_01&theme=T_02"));
        let facets_data = vec![
            FacetGroup {
                property: "role".to_owned(),
                label: "Rolle".to_owned(),
                kind: FacetKind::List,
                options: vec![FacetOption {
                    value: "R_01".to_owned(),
                    label: "Nebenrolle".to_owned(),
                    count: 3,
                    selected: true,
                }],
            },
            FacetGroup {
                property: "theme".to_owned(),
                label: "Thema".to_owned(),
                kind: FacetKind::List,
                options: vec![FacetOption {
                    value: "T_02".to_owned(),
                    label: "Fantasy".to_owned(),
                    count: 3,
                    selected: true,
                }],
            },
        ];
        let list_page = ResourceListPage {
            items: vec![],
            total: 3,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(54),
        };
        let components: Vec<ComponentData> = vec![ComponentData::Facets(facets_data), ComponentData::List(list_page)];
        let out = render_browse_breadcrumb_for_page(&page, &ctx, &components, &site)
            .expect("faceted, id-bearing ResourceList must yield a browse breadcrumb")
            .into_string();
        assert!(
            out.contains(
                "aria-label=\"Link zu dieser Ansicht kopieren — 3 von 54 Character · Nebenrolle · Fantasy · \u{201E}hat\u{201C}\""
            ),
            "DE accessible-name suffix must localize count text and quote marks, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// Unrefined, faceted: no query, nothing selected — the suffix falls
    /// back to the plain "N resources" count (no "of M", no facet/query
    /// segments) and the href is the bare path.
    #[test]
    fn browse_breadcrumb_faceted_unrefined_en() {
        use cpe_core::ir::{FacetGroup, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_browse_page_with_facets();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = make_browse_ctx(lang, "/alice-in-daschland/characters", None);
        let facets_data: Vec<FacetGroup> = vec![];
        let list_page = ResourceListPage {
            items: vec![],
            total: 54,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: None,
        };
        let components: Vec<ComponentData> = vec![ComponentData::Facets(facets_data), ComponentData::List(list_page)];
        let out = render_browse_breadcrumb_for_page(&page, &ctx, &components, &site)
            .expect("faceted, id-bearing ResourceList must yield a browse breadcrumb")
            .into_string();
        assert!(
            out.contains("aria-label=\"Copy link to this view — 54 Character\""),
            "unrefined suffix must be the bare count, got: {out}"
        );
        assert!(
            out.contains("href=\"http://localhost:8080/alice-in-daschland/characters\""),
            "unrefined href must be the bare path (no query), got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// Drift guard: the browse crumb's accessible name and the
    /// resource-list surf-head must carry the SAME class noun. They are two
    /// independent call sites of the same
    /// `chrome::count_resources_text`/`count_of_text` split (documented on
    /// `render_browse_breadcrumb_for_page` as "the EXACT same split"), so a
    /// noun resolved differently in one of them would silently desynchronize
    /// the visible head from what a screen reader announces. Note the crumb's
    /// own *current segment* stays the capitalized plural ("Characters") —
    /// only the COUNT segment is compared here.
    #[test]
    fn browse_crumb_count_noun_matches_the_surf_head_noun() {
        use cpe_components::component::resource_list::{render_resource_list, ResourceListCfg};
        use cpe_core::ir::{ClassLabel, FacetGroup, Pagination, ResourceListPage};

        let mut site = make_alice_site();
        site.class_labels.push(ClassLabel {
            class: "Character".to_owned(),
            translations: vec![
                ("en".to_owned(), "Character".to_owned()),
                ("de".to_owned(), "Figur".to_owned()),
            ],
            plural: vec![
                ("en".to_owned(), "characters".to_owned()),
                ("de".to_owned(), "Figuren".to_owned()),
            ],
        });
        let page = make_faceted_browse_page_with_facets();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = make_browse_ctx(lang, "/alice-in-daschland/characters", None);

        let make_list = || ResourceListPage {
            items: vec![],
            total: 8,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(54),
        };
        let components: Vec<ComponentData> = vec![
            ComponentData::Facets(Vec::<FacetGroup>::new()),
            ComponentData::List(make_list()),
        ];
        let crumb = render_browse_breadcrumb_for_page(&page, &ctx, &components, &site)
            .expect("faceted, id-bearing ResourceList must yield a browse breadcrumb")
            .into_string();

        let thumbs = std::collections::HashMap::new();
        let routes = std::collections::HashMap::new();
        let cfg = ResourceListCfg::with_links(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            &routes,
            "/alice-in-daschland/characters",
            None,
            Some("character-list"),
            None,
            &[],
            "Character",
            &site.class_labels,
            None,
            None,
            &thumbs,
            false,
            None,
            None,
            None,
            None,
            &[],
        );
        let list = make_list();
        let head = render_resource_list(&list, &cfg, &ctx.lang).into_string();

        assert!(
            crumb.contains("8 of 54 characters"),
            "the crumb's count segment must carry the class plural, got: {crumb}"
        );
        assert!(
            head.contains("8 of 54 characters"),
            "the surf-head's count must carry the same class plural, got: {head}"
        );
    }

    /// DE variant of the unrefined faceted case — count text localizes.
    #[test]
    fn browse_breadcrumb_faceted_unrefined_de() {
        use cpe_core::ir::{FacetGroup, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_browse_page_with_facets();
        let lang = resolve(&["de".to_owned()], &site);
        let ctx = make_browse_ctx(lang, "/alice-in-daschland/characters", None);
        let facets_data: Vec<FacetGroup> = vec![];
        let list_page = ResourceListPage {
            items: vec![],
            total: 54,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: None,
        };
        let components: Vec<ComponentData> = vec![ComponentData::Facets(facets_data), ComponentData::List(list_page)];
        let out = render_browse_breadcrumb_for_page(&page, &ctx, &components, &site)
            .expect("faceted, id-bearing ResourceList must yield a browse breadcrumb")
            .into_string();
        assert!(
            out.contains("aria-label=\"Link zu dieser Ansicht kopieren — 54 Character\""),
            "DE unrefined suffix must localize the plain count, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// Refined, non-faceted (`/chapters` shape — search only, no
    /// `FacetPanel` at all): the suffix is `<count of total> · "<q>"`, no
    /// facet segment.
    #[test]
    fn browse_breadcrumb_nonfaceted_refined_en() {
        use cpe_core::ir::{Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_nonfaceted_browse_page();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = make_browse_ctx(lang, "/alice-in-daschland/chapters", Some("q=looking"));
        let list_page = ResourceListPage {
            items: vec![],
            total: 2,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(13),
        };
        let components: Vec<ComponentData> = vec![ComponentData::List(list_page)];
        let out = render_browse_breadcrumb_for_page(&page, &ctx, &components, &site)
            .expect("non-faceted, id-bearing ResourceList must yield a browse breadcrumb")
            .into_string();
        assert!(
            out.contains("aria-label=\"Copy link to this view — 2 of 13 StoryChapter · \u{201C}looking\u{201D}\""),
            "non-faceted refined suffix must have no facet segment, got: {out}"
        );
        assert!(
            out.contains("href=\"http://localhost:8080/alice-in-daschland/chapters?q=looking\""),
            "href must carry q, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// DE variant of the non-faceted refined case.
    #[test]
    fn browse_breadcrumb_nonfaceted_refined_de() {
        use cpe_core::ir::{Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_nonfaceted_browse_page();
        let lang = resolve(&["de".to_owned()], &site);
        let ctx = make_browse_ctx(lang, "/alice-in-daschland/chapters", Some("q=looking"));
        let list_page = ResourceListPage {
            items: vec![],
            total: 2,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(13),
        };
        let components: Vec<ComponentData> = vec![ComponentData::List(list_page)];
        let out = render_browse_breadcrumb_for_page(&page, &ctx, &components, &site)
            .expect("non-faceted, id-bearing ResourceList must yield a browse breadcrumb")
            .into_string();
        assert!(
            out.contains(
                "aria-label=\"Link zu dieser Ansicht kopieren — 2 von 13 StoryChapter · \u{201E}looking\u{201C}\""
            ),
            "DE non-faceted refined suffix must localize, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// Unrefined, non-faceted: bare count, bare path.
    #[test]
    fn browse_breadcrumb_nonfaceted_unrefined_en() {
        use cpe_core::ir::{Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_nonfaceted_browse_page();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = make_browse_ctx(lang, "/alice-in-daschland/chapters", None);
        let list_page = ResourceListPage {
            items: vec![],
            total: 13,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: None,
        };
        let components: Vec<ComponentData> = vec![ComponentData::List(list_page)];
        let out = render_browse_breadcrumb_for_page(&page, &ctx, &components, &site)
            .expect("non-faceted, id-bearing ResourceList must yield a browse breadcrumb")
            .into_string();
        assert!(
            out.contains("aria-label=\"Copy link to this view — 13 StoryChapter\""),
            "unrefined non-faceted suffix must be the bare count, got: {out}"
        );
        assert!(
            out.contains("href=\"http://localhost:8080/alice-in-daschland/chapters\""),
            "unrefined href must be the bare path, got: {out}"
        );
        assert!(
            out.contains("breadcrumb__current"),
            "class label must be the current (unlinked) crumb item, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    /// `render_page` (not just the standalone helper) wires the browse
    /// breadcrumb — a page with a `ResourceList` and no `Narrative`/
    /// `ResourceDetail` gets the crumb line above `render_body`'s output,
    /// and `.page` on the `.browse` grid is untouched (the breadcrumb is a
    /// sibling, never a 5th grid child).
    #[test]
    fn render_page_browse_page_has_breadcrumb_above_the_browse_grid() {
        use cpe_core::ir::{FacetGroup, FacetKind, FacetOption, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_browse_page_with_facets();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = make_browse_ctx(lang, "/alice-in-daschland/characters", Some("role=R_01"));
        let facets_data = vec![FacetGroup {
            property: "role".to_owned(),
            label: "Role".to_owned(),
            kind: FacetKind::List,
            options: vec![FacetOption {
                value: "R_01".to_owned(),
                label: "Supporting".to_owned(),
                count: 3,
                selected: true,
            }],
        }];
        let list_page = ResourceListPage {
            items: vec![],
            total: 3,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(54),
        };
        let components: Vec<ComponentData> = vec![ComponentData::Facets(facets_data), ComponentData::List(list_page)];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        assert!(
            out.contains("class=\"breadcrumb\""),
            "browse page must have a breadcrumb, got: {out}"
        );
        assert!(
            out.contains("crumb-share"),
            "browse breadcrumb must carry the share control, got: {out}"
        );
        let breadcrumb_idx = out.find("class=\"breadcrumb\"").expect("breadcrumb present");
        let browse_idx = out.find("class=\"browse page\"").expect(".browse.page root present");
        assert!(
            breadcrumb_idx < browse_idx,
            "breadcrumb must render ABOVE the .browse grid, not inside/after it, got: {out}"
        );
    }

    /// A page combining a
    /// `Narrative` component AND an id-bearing `ResourceList` (a shape a
    /// config author could write) must render
    /// the BROWSE crumb id (`crumb-{list_id}`), never the narrative one —
    /// the host's `_facets`/`_sort`/`_search` fragment handlers only ever
    /// patch `#crumb-{list_id}`, so the full-page render must agree.
    #[test]
    fn render_page_prefers_browse_crumb_id_over_narrative_when_both_present() {
        use cpe_core::ir::{Pagination, ResourceListPage};

        let site = make_alice_site();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let page = cpe_core::ir::Page {
            id: "hybrid".to_owned(),
            route: "/hybrid".to_owned(),
            params: vec![],
            body: vec![
                Component::Narrative { file: "home.md".to_owned(), companion: None, side: None },
                Component::ResourceList {
                    id: Some("character-list".to_owned()),
                    class: "Character".to_owned(),
                    wheres: vec![],
                    sorts: vec![],
                    pagination: Pagination::PerPage(24),
                    gloss: None,
                    chips: vec![],
                    media: None,
                    grid: false,
                    tile_ratio: None,
                    order_name_label: None,
                    order_name_desc_label: None,
                    order_default_label: None,
                    order_name_property: None,
                    order_options: vec![],
                },
            ],
            figure_slots: Vec::new(),
        };
        let ctx = make_browse_ctx(lang, "/alice-in-daschland/hybrid", None);
        let list_page = ResourceListPage {
            items: vec![],
            total: 54,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: None,
        };
        let components: Vec<ComponentData> = vec![ComponentData::Other, ComponentData::List(list_page)];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        assert!(
            out.contains("id=\"crumb-character-list\""),
            "must render the browse crumb id (matching the fragment handlers' patch \
             target), got: {out}"
        );
        assert!(
            !out.contains("id=\"crumb-alice-in-daschland\""),
            "must NOT render the narrative crumb id when a browse list is also present, \
             got: {out}"
        );
    }

    // A11y: hidden-<h1> injection on headless pages

    /// A `ResourceList`-only page (no `Narrative`, no `ResourceDetail`) — the
    /// canonical "headless" archetype.
    fn make_browse_page() -> cpe_core::ir::Page {
        cpe_core::ir::Page {
            id: "characters".to_owned(),
            route: "/characters".to_owned(),
            params: vec![],
            body: vec![Component::ResourceList {
                id: Some("character-list".to_owned()),
                class: "Character".to_owned(),
                wheres: vec![],
                sorts: vec![],
                pagination: cpe_core::ir::Pagination::PerPage(24),
                gloss: None,
                chips: vec![],
                media: None,
                grid: false,
                tile_ratio: None,
                order_name_label: None,
                order_name_desc_label: None,
                order_default_label: None,
                order_name_property: None,
                order_options: vec![],
            }],
            figure_slots: Vec::new(),
        }
    }

    /// A headless resource-list page (no visible heading of its own) must get
    /// exactly one `<h1>`, injected as `h1.visually-hidden`.
    #[test]
    fn render_page_injects_hidden_h1_on_headless_page() {
        let site = make_alice_site();
        let page = make_browse_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert_eq!(out.matches("<h1").count(), 1, "exactly one <h1> must be present, got: {out}");
        assert!(
            out.contains("<h1 class=\"visually-hidden\">"),
            "a headless page must inject a visually-hidden h1, got: {out}"
        );
    }

    /// A narrative page whose first heading IS level 1 already has its own
    /// visible `<h1>` (via `editorial::render_editorial`) — no hidden one must
    /// be injected on top of it.
    #[test]
    fn render_page_omits_hidden_h1_when_narrative_has_visible_h1() {
        let site = make_alice_site();
        let page = make_alice_page(); // narrative file="home.md", route "/"
        let narrative = make_alice_narrative(); // "# Welcome" — H1
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();
        assert!(
            !out.contains(r#"<h1 class="visually-hidden""#),
            "a narrative page with its own visible h1 must NOT get a hidden one, got: {out}"
        );
        assert_eq!(out.matches("<h1").count(), 1, "exactly one <h1> must be present, got: {out}");
    }

    /// A resource-detail page (resolved resource) already has `h1.record__title`
    /// — no hidden h1 must be injected on top of it.
    #[test]
    fn render_page_omits_hidden_h1_on_resource_detail_page() {
        let site = make_alice_site();
        let page = make_character_page();
        let narrative = make_alice_narrative();
        let resource = make_alice_resource();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/CH_001",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(Some(resource))];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        assert!(
            !out.contains(r#"<h1 class="visually-hidden""#),
            "a resource-detail page already has h1.record__title; must not also get a \
             hidden one, got: {out}"
        );
        assert_eq!(out.matches("<h1").count(), 1, "exactly one <h1> must be present, got: {out}");
    }

    /// A resource-detail page with NO resolved resource still renders
    /// `render_not_found`'s own `h1.record__title` — the not-found branch is
    /// itself a `ResourceDetail` component, so no hidden h1 must be injected.
    #[test]
    fn render_page_not_found_still_has_exactly_one_h1() {
        let site = make_alice_site();
        let page = make_character_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/NOPE",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(None)];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        assert!(
            !out.contains(r#"<h1 class="visually-hidden""#),
            "not-found page already has h1.record__title; must not also get a hidden \
             one, got: {out}"
        );
        assert_eq!(out.matches("<h1").count(), 1, "exactly one <h1> must be present, got: {out}");
    }

    /// A headless `Narrative` whose first
    /// heading is H2 (no h1 anywhere) on a page whose route matches a
    /// declared `nav` entry must render exactly one `<h1>`, and its text
    /// must be the NAV LABEL — not the H2 heading text. Locks both the
    /// no-double-h1 property (`page_has_visible_h1`) and the
    /// nav-label-preferred `page_heading` rung (inserted specifically to
    /// avoid a hidden-h1/visible-h2 text duplication).
    #[test]
    fn snapshot_render_page_headless_narrative_h2_uses_nav_label_for_hidden_h1() {
        // make_alice_site's nav includes "/about" -> literal label "About"
        // (no matching chrome-string id, so nav_label falls back to it as-is).
        let site = make_alice_site();
        let page = cpe_core::ir::Page {
            id: "about".to_owned(),
            route: "/about".to_owned(),
            params: vec![],
            body: vec![Component::Narrative { file: "about.md".to_owned(), companion: None, side: None }],
            figure_slots: Vec::new(),
        };
        let narrative = {
            let pairs: Vec<(String, String)> =
                vec![("about.md".to_owned(), "## Our mission\n\nSome body text.".to_owned())];
            NarrativeStore::from_pairs(&pairs)
        };
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/about",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let out = render_page(&site, &page, &narrative, &ctx, &[], &DataArks::default()).into_string();

        assert_eq!(
            out.matches("<h1").count(),
            1,
            "exactly one <h1> must be present (no double-h1), got: {out}"
        );
        assert!(
            out.contains("<h1 class=\"visually-hidden\">About</h1>"),
            "the hidden h1 must read the nav label 'About', not the H2 \
             heading text, got: {out}"
        );
        // The H2 heading itself still renders normally in the body (via
        // editorial::render_editorial) — only the HIDDEN H1's text must not
        // be the H2's text.
        assert!(
            out.contains("<h2 id=\"sec-1\">Our mission</h2>"),
            "the narrative's own H2 heading must still render in the body, got: {out}"
        );
        assert!(
            !out.contains("<h1 class=\"visually-hidden\">Our mission</h1>"),
            "the H2 heading text must not leak into the hidden h1, got: {out}"
        );

        insta::assert_snapshot!(out);
    }

    // Layer-3: render_page with a resource-detail component

    /// Build a page whose body contains a single `ResourceDetail` component.
    fn make_character_page() -> cpe_core::ir::Page {
        cpe_core::ir::Page {
            id: "character".to_owned(),
            route: "/characters/:id".to_owned(),
            params: vec!["id".to_owned()],
            body: vec![Component::ResourceDetail {
                id: Some("$id".to_owned()),
                show_properties: Some(vec!["name".to_owned(), "description".to_owned(), "role".to_owned()]),
                title_property: Some("name".to_owned()),
                class: None,
                media: None,
                groups: vec![],
                narrative_for: vec![],
                incoming_links: None,
                head: None,
                navigator: false,
            }],
            figure_slots: Vec::new(),
        }
    }

    /// Build a `Resource` literal representing Alice (used in render_page tests).
    fn make_alice_resource() -> cpe_core::ir::Resource {
        use cpe_core::ir::{Property, PropertyValue, Representation, RepresentationKind, Resource};
        Resource {
            id: "CH_001".to_owned(),
            class: "Character".to_owned(),
            properties: vec![
                Property {
                    name: "name".to_owned(),
                    values: vec![
                        PropertyValue::Text { text: "Alice".to_owned(), lang: Some("en".to_owned()) },
                        PropertyValue::Text { text: "Alice".to_owned(), lang: Some("de".to_owned()) },
                    ],
                },
                Property {
                    name: "description".to_owned(),
                    values: vec![
                        PropertyValue::Richtext {
                            text: "A **curious** girl who falls into Wonderland.".to_owned(),
                            lang: Some("en".to_owned()),
                        },
                        PropertyValue::Richtext {
                            text: "Ein **neugieriges** Mädchen, das ins Wunderland fällt.".to_owned(),
                            lang: Some("de".to_owned()),
                        },
                    ],
                },
                Property {
                    name: "role".to_owned(),
                    values: vec![PropertyValue::ListValue {
                        node_id: "ROLE_001".to_owned(),
                        label: Some("Protagonist".to_owned()),
                    }],
                },
            ],
            representations: vec![Representation {
                kind: RepresentationKind::Image,
                uri: "https://example.org/iiif/alice.jpg".to_owned(),
                width: Some(800),
                height: Some(600),
                alt: None,
                attribution: None,
                seq: 0,
            }],
        }
    }

    /// A resource-detail page backed by a resolved `Resource` renders in full chrome (EN).
    #[test]
    fn snapshot_render_page_resource_detail_en() {
        let site = make_alice_site();
        let page = make_character_page();
        let narrative = make_alice_narrative();
        let resource = make_alice_resource();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/CH_001",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        // The components slice has exactly one entry (paired with the one ResourceDetail
        // component).
        let components: Vec<ComponentData> = vec![ComponentData::Detail(Some(resource))];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        insta::assert_snapshot!(out);
    }

    /// A resource-detail page backed by a resolved `Resource` renders in full chrome (DE).
    #[test]
    fn snapshot_render_page_resource_detail_de() {
        let site = make_alice_site();
        let page = make_character_page();
        let narrative = make_alice_narrative();
        let resource = make_alice_resource();
        let lang = resolve(&["de".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/CH_001",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(Some(resource))];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        insta::assert_snapshot!(out);
    }

    /// One linked (IIIF-shaped) Image resource for the record-viewer full-page
    /// snapshots — `data-appearance` is emitted once on `<html>` by
    /// `render_page`, not by the media renderer itself, so a dark-mode
    /// snapshot only needs coverage at this full-page level.
    fn make_media_items() -> Vec<Resource> {
        vec![Resource {
            id: "I_001".to_owned(),
            class: "Image".to_owned(),
            properties: vec![],
            representations: vec![Representation {
                kind: RepresentationKind::Image,
                uri: "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg".to_owned(),
                width: Some(800),
                height: Some(600),
                alt: None,
                attribution: None,
                seq: 0,
            }],
        }]
    }

    /// The "one dark" snapshot axis: a resource-detail page with a real
    /// linked image, rendered with `Appearance::Dark` — confirms
    /// `data-appearance="dark"` propagates onto the same page shell that
    /// wraps the record viewer.
    #[test]
    fn snapshot_render_page_resource_detail_dark_en() {
        let site = make_alice_site();
        let page = make_character_page();
        let narrative = make_alice_narrative();
        let resource = make_alice_resource();
        let media = make_media_items();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/CH_001",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Dark,
            selected_image: 1,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &media,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(Some(resource))];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        insta::assert_snapshot!(out);
    }

    /// A resource-detail slot with `None` (not found) renders `render_not_found` inside chrome.
    #[test]
    fn snapshot_render_page_resource_detail_not_found_en() {
        let site = make_alice_site();
        let page = make_character_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/NOPE",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        // Detail(None) → not-found body inside chrome.
        let components: Vec<ComponentData> = vec![ComponentData::Detail(None)];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        insta::assert_snapshot!(out);
    }

    /// A resource-detail slot with `None` (not found) renders the DE not-found body.
    #[test]
    fn snapshot_render_page_resource_detail_not_found_de() {
        let site = make_alice_site();
        let page = make_character_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["de".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters/NOPE",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Detail(None)];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        insta::assert_snapshot!(out);
    }

    // Layer-3: render_page with FacetPanel + ResourceList

    /// Build a page whose body has a FacetPanel + ResourceList (the canonical
    /// faceted browse page shape), each with a matching list id.
    fn make_faceted_page() -> cpe_core::ir::Page {
        use cpe_core::ir::{Pagination, Sort, SortDir, WhereClause};
        cpe_core::ir::Page {
            id: "characters".to_owned(),
            route: "/characters".to_owned(),
            params: vec![],
            body: vec![
                Component::FacetPanel { target: "character-list".to_owned(), facets: vec![] },
                Component::ResourceList {
                    id: Some("character-list".to_owned()),
                    class: "Character".to_owned(),
                    wheres: vec![WhereClause::Equals {
                        property: "name".to_owned(),
                        value: cpe_core::ir::Value::String("Alice".to_owned()),
                    }],
                    sorts: vec![Sort {
                        key: cpe_core::ir::SortKey::Property("name".to_owned()),
                        direction: SortDir::Asc,
                    }],
                    pagination: Pagination::PerPage(24),
                    gloss: None,
                    chips: vec![],
                    media: None,
                    grid: false,
                    tile_ratio: None,
                    order_name_label: None,
                    order_name_desc_label: None,
                    order_default_label: None,
                    order_name_property: None,
                    order_options: vec![],
                },
            ],
            figure_slots: Vec::new(),
        }
    }

    /// A page with a `FacetPanel` + `ResourceList` renders both components in
    /// full chrome. The panel carries the `facets-character-list` id; the list
    /// carries `character-list`. The rendered output is snapshot-tested so any
    /// future HTML changes require explicit acceptance.
    #[test]
    fn snapshot_render_page_with_facet_panel_and_list() {
        use cpe_core::ir::{FacetGroup, FacetKind, FacetOption, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters",
            route_params: &[],
            current_query: Some("lang=en&role=R_01"),
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };

        // Build minimal FacetGroup data (one group, two options, one selected).
        let facets_data = vec![FacetGroup {
            property: "role".to_owned(),
            label: "Role".to_owned(),
            kind: FacetKind::List,
            options: vec![
                FacetOption {
                    value: "R_01".to_owned(),
                    label: "Main character".to_owned(),
                    count: 12,
                    selected: true,
                },
                FacetOption {
                    value: "R_02".to_owned(),
                    label: "Secondary character".to_owned(),
                    count: 28,
                    selected: false,
                },
            ],
        }];

        // Build a minimal ResourceListPage (one item).
        let list_page = ResourceListPage {
            items: vec![make_alice_resource()],
            total: 1,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: None,
        };

        // components aligned 1:1 with page.body:
        // [0] FacetPanel → Facets(...)
        // [1] ResourceList → List(...)
        let components: Vec<ComponentData> = vec![ComponentData::Facets(facets_data), ComponentData::List(list_page)];

        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        insta::assert_snapshot!(out);

        // Regression: the language-switcher and appearance-toggle hrefs must
        // preserve the current request's other query params (here `role=R_01`
        // from the active facet filter) instead of dropping them — see the
        // header-controls query-string-drop bug fix. Scope each check to its
        // own anchor via a stable, param-order-independent marker (rather than
        // matching a full literal href string, which would be coupled to
        // `override_query_param`'s exact key ordering).
        let de_window = tag_containing(&out, r#"hreflang="de""#);
        assert!(
            de_window.contains("lang=de"),
            "DE lang switcher href should target lang=de: {de_window}"
        );
        assert!(
            de_window.contains("role=R_01"),
            "DE lang switcher href should preserve role=R_01: {de_window}"
        );

        let toggle_window = tag_containing(&out, r#"class="theme-toggle""#);
        assert!(
            toggle_window.contains("appearance=dark"),
            "appearance toggle href should target appearance=dark: {toggle_window}"
        );
        assert!(
            toggle_window.contains("lang=en") && toggle_window.contains("role=R_01"),
            "appearance toggle href should preserve lang=en&role=R_01: {toggle_window}"
        );
    }

    /// An empty facet panel: a `FacetPanel` targets this
    /// list but its RESOLVED groups are empty (`Component::FacetPanel.facets`
    /// is `vec![]` in `make_faceted_page` — the static config condition that
    /// makes the case reachable). The whole `.browse` grid/rail must be
    /// absent — the page degrades to the non-faceted layout (`/chapters`'
    /// shape) — and the list renders full-measure, standalone.
    #[test]
    fn snapshot_render_page_facet_panel_with_no_groups_renders_no_rail() {
        use cpe_core::ir::{FacetGroup, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };

        // The FacetPanel resolves to zero groups — the empty-facet-panel condition.
        let facets_data: Vec<FacetGroup> = vec![];

        let list_page = ResourceListPage {
            items: vec![make_alice_resource()],
            total: 1,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: None,
        };

        // components aligned 1:1 with page.body:
        // [0] FacetPanel → Facets(vec![]) (empty)
        // [1] ResourceList → List(...)
        let components: Vec<ComponentData> = vec![ComponentData::Facets(facets_data), ComponentData::List(list_page)];

        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();

        assert!(
            !out.contains(r#"class="browse page""#) && !out.contains("class=\"browse "),
            "E3b: no `.browse` wrapper must render when the facet-panel \
             resolves to zero groups, got: {out}"
        );
        assert!(
            !out.contains("facet-panel"),
            "E3b: no facet-panel rail markup (not even an empty section) \
             must appear, got: {out}"
        );
        assert!(
            !out.contains("filterbar"),
            "E3b: no filterbar chip row must appear either (nothing can be \
             selected on a rail with no groups), got: {out}"
        );
        assert!(
            out.contains(r#"class="rl-annot__item""#) || out.contains("resource-list-section"),
            "the list itself must still render standalone, got: {out}"
        );

        insta::assert_snapshot!(out);
    }

    /// Facet-only page (FacetPanel + ResourceList, NO SearchBar) narrowed to
    /// zero matches — the core regression
    /// guard: the OLD `cfg.search`-only gate could never show the
    /// narrowed-to-zero body here (there is no search-bar to have an
    /// "active query"), degrading to the quiet, wrong `p.resource-list__empty`
    /// "This class contains no resources" message even though a facet IS
    /// actively selected. The new `cfg.narrowed` field (built from
    /// `any_facet_selected` on the resolved `FacetGroup` data, independent of
    /// any search-bar) must render `.search-empty` with ONLY "Clear all
    /// filters" (no "Clear search", no hint — both gated on an active query,
    /// which does not exist on this page at all).
    #[test]
    fn snapshot_render_page_facet_only_narrowed_to_zero_no_search_bar() {
        use cpe_core::ir::{FacetGroup, FacetKind, FacetOption, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters",
            route_params: &[],
            current_query: Some("role=R_01"),
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };

        let facets_data = vec![FacetGroup {
            property: "role".to_owned(),
            label: "Role".to_owned(),
            kind: FacetKind::List,
            options: vec![FacetOption {
                value: "R_01".to_owned(),
                label: "Main character".to_owned(),
                count: 0,
                selected: true,
            }],
        }];

        let list_page = ResourceListPage {
            items: vec![],
            total: 0,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(54),
        };

        let components: Vec<ComponentData> = vec![ComponentData::Facets(facets_data), ComponentData::List(list_page)];

        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();

        assert!(
            out.contains(r#"class="search-empty""#),
            "a facet-only narrowing with NO search-bar must still render \
             .search-empty, got: {out}"
        );
        assert!(
            !out.contains("resource-list__empty"),
            "must NOT degrade to the genuinely-empty-class message (E1), got: {out}"
        );
        assert_eq!(
            out.matches(r#"class="search-empty__act""#).count(),
            1,
            "filters-only, no search-bar: exactly ONE action (Clear all \
             filters) must render, got: {out}"
        );
        assert!(
            !out.contains("search-empty__hint"),
            "no search-bar on this page → no search hint, got: {out}"
        );
        assert!(
            out.contains("0 of 54"),
            "the count must reflect the unfiltered total, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    // Layer-3: render_page with FacetPanel + ResourceList + SearchBar
    // (the 3-way `render_body` grouping)

    /// Build a page whose body has FacetPanel + ResourceList + SearchBar, all
    /// three bound to the same list id.
    fn make_faceted_search_page() -> cpe_core::ir::Page {
        use cpe_core::ir::{Pagination, Sort, SortDir, WhereClause};
        cpe_core::ir::Page {
            id: "characters".to_owned(),
            route: "/characters".to_owned(),
            params: vec![],
            body: vec![
                Component::FacetPanel { target: "character-list".to_owned(), facets: vec![] },
                Component::SearchBar {
                    target: "character-list".to_owned(),
                    searches: vec!["name".to_owned(), "description".to_owned()],
                    placeholder: None,
                    scope: Some("search-scope-note".to_owned()),
                },
                Component::ResourceList {
                    id: Some("character-list".to_owned()),
                    class: "Character".to_owned(),
                    wheres: vec![WhereClause::Equals {
                        property: "name".to_owned(),
                        value: cpe_core::ir::Value::String("Alice".to_owned()),
                    }],
                    sorts: vec![Sort {
                        key: cpe_core::ir::SortKey::Property("name".to_owned()),
                        direction: SortDir::Asc,
                    }],
                    pagination: Pagination::PerPage(24),
                    gloss: None,
                    chips: vec![],
                    media: None,
                    grid: false,
                    tile_ratio: None,
                    order_name_label: None,
                    order_name_desc_label: None,
                    order_default_label: None,
                    order_name_property: None,
                    order_options: vec![],
                },
            ],
            figure_slots: Vec::new(),
        }
    }

    /// A page with FacetPanel + ResourceList + SearchBar renders one
    /// `section.browse.page` wrapper containing the search row, the rail, the
    /// filterbar, and the list — the search row must NOT carry its own
    /// `.page` class (it's a flat grid child, mirroring `.filterbar`).
    #[test]
    fn snapshot_render_page_faceted_with_search_bar() {
        use cpe_core::ir::{FacetGroup, FacetKind, FacetOption, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_search_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters",
            route_params: &[],
            current_query: Some("q=rab&role=R_01"),
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };

        let facets_data = vec![FacetGroup {
            property: "role".to_owned(),
            label: "Role".to_owned(),
            kind: FacetKind::List,
            options: vec![FacetOption {
                value: "R_01".to_owned(),
                label: "Main character".to_owned(),
                count: 12,
                selected: true,
            }],
        }];
        let list_page = ResourceListPage {
            items: vec![make_alice_resource()],
            total: 1,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(20),
        };

        // components aligned 1:1 with page.body:
        // [0] FacetPanel → Facets(...)
        // [1] SearchBar → Other (no ComponentData variant of its own)
        // [2] ResourceList → List(...)
        let components: Vec<ComponentData> = vec![
            ComponentData::Facets(facets_data),
            ComponentData::Other,
            ComponentData::List(list_page),
        ];

        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        insta::assert_snapshot!(out);

        // The search row must be INSIDE section.browse.page.
        let browse_start = out.find(r#"class="browse page""#).expect("browse wrapper present");
        let browse_section = &out[browse_start..];
        assert!(
            browse_section.contains("search-row"),
            "the search row must render inside section.browse.page, got: {out}"
        );
        // ...but must NOT carry its own `.page` class (flat grid child, like
        // .filterbar) — a class-order-agnostic check: neither "search-row page"
        // nor "page search-row" may appear.
        assert!(
            !out.contains("search-row page") && !out.contains("page search-row"),
            "the faceted search row must NOT carry its own .page class, got: {out}"
        );
        // This fixture's `q=rab` is an ACTIVE, MATCHING
        // query (`list_page.items` non-empty) — the full `render_page` →
        // `render_body` → `SearchSurface` → `render_resource_list` pipeline
        // must produce the surf-head's `id="browse-count"` + scope note, and
        // must NOT fall into the no-match `.search-empty` branch.
        assert!(
            out.contains(r#"id="browse-count""#),
            "the surf-head count must carry id=\"browse-count\", got: {out}"
        );
        assert!(
            out.contains(r#"class="search-scope""#),
            "an active matching query must render the .search-scope note, got: {out}"
        );
        assert!(
            !out.contains("search-empty"),
            "a matching query must NOT render .search-empty, got: {out}"
        );
        // An active query + an active facet renders
        // BOTH chip kinds in the filterbar.
        assert!(
            out.contains(r#"class="filterchip filterchip--query""#),
            "the filterbar must render the query chip, got: {out}"
        );
        assert!(
            out.contains(r#"class="filterchip""#),
            "the filterbar must also render the facet chip, got: {out}"
        );
    }

    /// Active query that matches **nothing** (faceted shape): `.search-empty`
    /// replaces the list (msg + BOTH clear actions, since a facet is active)
    /// instead of the pre-existing `.resource-list__empty`. Proves the full
    /// `render_page` → `render_body` → `SearchSurface` → `render_resource_list`
    /// pipeline end-to-end — `resource_list.rs`'s own snapshot tests only
    /// exercise `render_resource_list` directly, never through `render_page`.
    #[test]
    fn snapshot_render_page_faceted_search_bar_no_match() {
        use cpe_core::ir::{FacetGroup, FacetKind, FacetOption, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_search_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters",
            route_params: &[],
            current_query: Some("q=xyzzy&role=R_01"),
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };

        let facets_data = vec![FacetGroup {
            property: "role".to_owned(),
            label: "Role".to_owned(),
            kind: FacetKind::List,
            options: vec![FacetOption {
                value: "R_01".to_owned(),
                label: "Main character".to_owned(),
                count: 12,
                selected: true,
            }],
        }];
        // Zero matches — the active query's no-match state.
        let list_page = ResourceListPage {
            items: vec![],
            total: 0,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(20),
        };

        let components: Vec<ComponentData> = vec![
            ComponentData::Facets(facets_data),
            ComponentData::Other,
            ComponentData::List(list_page),
        ];

        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        insta::assert_snapshot!(out);

        assert!(
            out.contains(r#"class="search-empty""#),
            "an active query with zero matches must render .search-empty, got: {out}"
        );
        assert!(
            !out.contains("resource-list__empty"),
            "the active-query no-match state must NOT fall back to \
             .resource-list__empty, got: {out}"
        );
        assert!(
            out.contains(">Clear search<"),
            "the search-empty state must offer a Clear search action, got: {out}"
        );
        assert!(
            out.contains(">Clear all filters<"),
            "with a facet active, the search-empty state must also offer \
             Clear all filters, got: {out}"
        );
        assert!(
            out.contains(r#"id="browse-count""#),
            "the surf-head count must still carry id=\"browse-count\" in the \
             no-match state, got: {out}"
        );
        // The two clear actions must drop DIFFERENT things: "Clear search"
        // drops only `q` (keeps the active `role=R_01` facet); "Clear all"
        // drops BOTH the facet (`role`) AND `q` ("Clear all clears
        // everything"), leaving the bare
        // path. `facet_keys` must come from the group's RESOLVED facet data
        // (`ComponentData::Facets`), not the config's (empty, in this
        // fixture) `Component::FacetPanel.facets` list — a real regression
        // this fixture is deliberately shaped to catch.
        assert!(
            out.contains(r#"href="/alice-in-daschland/characters?role=R_01">Clear search<"#),
            "Clear search must drop only q, preserving role=R_01, got: {out}"
        );
        assert!(
            out.contains(r#"href="/alice-in-daschland/characters">Clear all filters<"#),
            "Clear all filters must drop both the role=R_01 facet and q=xyzzy \
             (step 9), leaving the bare path, got: {out}"
        );
        // The filterbar itself must also render the query chip (query and
        // facets: both chip kinds side by side) — this fixture
        // has an active (though non-matching) query AND a selected facet.
        assert!(
            out.contains(r#"class="filterchip filterchip--query""#),
            "the filterbar must render the query chip alongside the facet \
             chip when both are active, got: {out}"
        );
    }

    /// Build a page whose body has ResourceList + SearchBar only (no
    /// FacetPanel).
    fn make_search_only_page() -> cpe_core::ir::Page {
        use cpe_core::ir::{Pagination, Sort, SortDir};
        cpe_core::ir::Page {
            id: "chapters".to_owned(),
            route: "/chapters".to_owned(),
            params: vec![],
            body: vec![
                Component::SearchBar {
                    target: "chapter-list".to_owned(),
                    searches: vec!["name".to_owned(), "description".to_owned()],
                    placeholder: None,
                    scope: Some("search-scope-note".to_owned()),
                },
                Component::ResourceList {
                    id: Some("chapter-list".to_owned()),
                    class: "StoryChapter".to_owned(),
                    wheres: vec![],
                    sorts: vec![Sort {
                        key: cpe_core::ir::SortKey::Property("name".to_owned()),
                        direction: SortDir::Asc,
                    }],
                    pagination: Pagination::PerPage(24),
                    gloss: None,
                    chips: vec![],
                    media: None,
                    grid: false,
                    tile_ratio: None,
                    order_name_label: None,
                    order_name_desc_label: None,
                    order_default_label: None,
                    order_name_property: None,
                    order_options: vec![],
                },
            ],
            figure_slots: Vec::new(),
        }
    }

    /// Build a page whose body has ResourceList + SearchBar only, searching
    /// over THREE properties incl. `full-text`
    /// (`searches "name" "description" "full-text"`: the
    /// "non-faceted full-text scope" state).
    fn make_search_only_page_fulltext() -> cpe_core::ir::Page {
        use cpe_core::ir::{Pagination, Sort, SortDir};
        cpe_core::ir::Page {
            id: "chapters".to_owned(),
            route: "/chapters".to_owned(),
            params: vec![],
            body: vec![
                Component::SearchBar {
                    target: "chapter-list".to_owned(),
                    searches: vec!["name".to_owned(), "description".to_owned(), "full-text".to_owned()],
                    placeholder: None,
                    scope: Some("search-scope-note-fulltext".to_owned()),
                },
                Component::ResourceList {
                    id: Some("chapter-list".to_owned()),
                    class: "StoryChapter".to_owned(),
                    wheres: vec![],
                    sorts: vec![Sort {
                        key: cpe_core::ir::SortKey::Property("name".to_owned()),
                        direction: SortDir::Asc,
                    }],
                    pagination: Pagination::PerPage(24),
                    gloss: None,
                    chips: vec![],
                    media: None,
                    grid: false,
                    tile_ratio: None,
                    order_name_label: None,
                    order_name_desc_label: None,
                    order_default_label: None,
                    order_name_property: None,
                    order_options: vec![],
                },
            ],
            figure_slots: Vec::new(),
        }
    }

    /// A page with ResourceList + SearchBar only (no FacetPanel) renders NO
    /// `.browse` wrapper at all — the search row (carrying its own `.page`)
    /// immediately followed by the list, both standalone.
    #[test]
    fn snapshot_render_page_non_faceted_with_search_bar() {
        use cpe_core::ir::{Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_search_only_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/chapters",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };

        let list_page = ResourceListPage {
            items: vec![make_alice_resource()],
            total: 1,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: None,
        };

        // components aligned 1:1 with page.body:
        // [0] SearchBar → Other
        // [1] ResourceList → List(...)
        let components: Vec<ComponentData> = vec![ComponentData::Other, ComponentData::List(list_page)];

        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        insta::assert_snapshot!(out);

        assert!(
            !out.contains(r#"class="browse page""#) && !out.contains(r#"class="browse"#),
            "a search-only (non-faceted) page must render NO .browse wrapper, got: {out}"
        );
        assert!(
            out.contains("search-row page") || out.contains("page search-row"),
            "the standalone search row must carry its own .page class, got: {out}"
        );
    }

    /// Active, matching query on the non-faceted shape — mirrors
    /// `snapshot_render_page_faceted_with_search_bar`'s active-match assertions
    /// for the case with no `.browse` wrapper at all.
    #[test]
    fn snapshot_render_page_non_faceted_search_bar_active_match() {
        use cpe_core::ir::{Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_search_only_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/chapters",
            route_params: &[],
            current_query: Some("q=rab"),
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };

        let list_page = ResourceListPage {
            items: vec![make_alice_resource()],
            total: 1,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(13),
        };

        let components: Vec<ComponentData> = vec![ComponentData::Other, ComponentData::List(list_page)];

        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        insta::assert_snapshot!(out);

        assert!(
            out.contains(r#"id="browse-count""#),
            "the surf-head count must carry id=\"browse-count\", got: {out}"
        );
        assert!(
            out.contains(r#"class="search-scope""#),
            "an active matching query must render the .search-scope note, got: {out}"
        );
        assert!(
            !out.contains("search-empty"),
            "a matching query must NOT render .search-empty, got: {out}"
        );
    }

    /// The non-faceted shape's no-match state must offer ONLY "Clear search"
    /// — no "Clear all" (that action exists only when a facet is
    /// selected). End-to-end companion to
    /// `snapshot_render_page_faceted_search_bar_no_match`.
    #[test]
    fn snapshot_render_page_non_faceted_search_bar_no_match() {
        use cpe_core::ir::{Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_search_only_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/chapters",
            route_params: &[],
            current_query: Some("q=xyzzy"),
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };

        let list_page = ResourceListPage {
            items: vec![],
            total: 0,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(13),
        };

        let components: Vec<ComponentData> = vec![ComponentData::Other, ComponentData::List(list_page)];

        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        insta::assert_snapshot!(out);

        assert!(
            out.contains(r#"class="search-empty""#),
            "an active query with zero matches must render .search-empty, got: {out}"
        );
        assert!(
            out.contains(">Clear search<"),
            "the search-empty state must offer a Clear search action, got: {out}"
        );
        assert!(
            !out.contains(">Clear all<"),
            "with no facet-panel, the search-empty state must NOT offer \
             Clear all, got: {out}"
        );
    }

    /// Fifth of the five search-bar states:
    /// non-faceted, full-text scope — an
    /// active matching query over THREE searched properties (name,
    /// description, full-text). Proves the scope note reads the distinct
    /// full-text copy, not the two-property "matching name and description"
    /// wording the characters/locations states use.
    #[test]
    fn snapshot_render_page_non_faceted_search_bar_full_text_scope() {
        use cpe_core::ir::{Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_search_only_page_fulltext();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/chapters",
            route_params: &[],
            current_query: Some("q=alice"),
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };

        let list_page = ResourceListPage {
            items: vec![make_alice_resource()],
            total: 1,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(13),
        };

        let components: Vec<ComponentData> = vec![ComponentData::Other, ComponentData::List(list_page)];

        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();
        insta::assert_snapshot!(out);

        assert!(
            out.contains("matching title, description and full text"),
            "the full-text scope note must read the distinct 3-property \
             copy, got: {out}"
        );
        assert!(
            !out.contains("matching Name and Description"),
            "the full-text state must NOT reuse the 2-property \
             characters/locations scope note, got: {out}"
        );
    }

    // Contract asserts

    /// `.browse.page` has exactly its four flat grid children, each present
    /// exactly once and in this order: the search row, the facet rail, the
    /// filterbar, the list. (String-substring evidence, not a DOM parser —
    /// see the module's "no HTML parser in this test suite" convention; each
    /// marker is chosen to be unambiguous and appear nowhere else in this
    /// fixture's output.)
    #[test]
    fn contract_browse_has_four_flat_children_in_order() {
        use cpe_core::ir::{FacetGroup, FacetKind, FacetOption, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_search_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters",
            route_params: &[],
            current_query: Some("q=rab&role=R_01"),
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let facets_data = vec![FacetGroup {
            property: "role".to_owned(),
            label: "Role".to_owned(),
            kind: FacetKind::List,
            options: vec![FacetOption {
                value: "R_01".to_owned(),
                label: "Main character".to_owned(),
                count: 12,
                selected: true,
            }],
        }];
        let list_page = ResourceListPage {
            items: vec![make_alice_resource()],
            total: 1,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(20),
        };
        let components: Vec<ComponentData> = vec![
            ComponentData::Facets(facets_data),
            ComponentData::Other,
            ComponentData::List(list_page),
        ];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();

        let browse_start = out.find(r#"class="browse page""#).expect("browse wrapper present");
        let browse_section = &out[browse_start..];

        // Each of the four markers must appear exactly once inside .browse.
        for (marker, label) in [
            (r#"class="search-row""#, "search row (flat, no .page)"),
            (r#"class="facet-panel page""#, "facet rail"),
            (r#"id="filters-"#, "filterbar"),
            (r#"class="resource-list-section page""#, "list"),
        ] {
            let count = browse_section.matches(marker).count();
            assert_eq!(
                count, 1,
                "{label} marker '{marker}' must appear exactly once inside \
                 .browse, got {count} in: {browse_section}"
            );
        }

        // And in this exact order (the row order: SEARCH, then
        // the rail/filterbar/list order).
        let search_pos = browse_section.find(r#"class="search-row""#).expect("search row present");
        let facet_pos = browse_section.find(r#"class="facet-panel page""#).expect("facet rail present");
        let filterbar_pos = browse_section.find(r#"id="filters-"#).expect("filterbar present");
        let list_pos = browse_section
            .find(r#"class="resource-list-section page""#)
            .expect("list present");
        assert!(
            search_pos < facet_pos && facet_pos < filterbar_pos && filterbar_pos < list_pos,
            "the four .browse children must appear in order search-row < \
             facet-panel < filterbar < list, got positions {search_pos}, \
             {facet_pos}, {filterbar_pos}, {list_pos}"
        );
    }

    /// The search row's OWN markup never contains either swap-target
    /// selector id (`filters-character-list`/`character-list`) — proving it
    /// sits structurally OUTSIDE both regions the `_facets`/`_search`
    /// fragment handlers ever patch, so a DataStar morph of either can never
    /// reach into (or disturb) the search input's caret/selection.
    #[test]
    fn contract_search_row_excludes_both_swap_target_ids() {
        use cpe_core::ir::{FacetGroup, FacetKind, FacetOption, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_search_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters",
            route_params: &[],
            current_query: Some("q=rab&role=R_01"),
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let facets_data = vec![FacetGroup {
            property: "role".to_owned(),
            label: "Role".to_owned(),
            kind: FacetKind::List,
            options: vec![FacetOption {
                value: "R_01".to_owned(),
                label: "Main character".to_owned(),
                count: 12,
                selected: true,
            }],
        }];
        let list_page = ResourceListPage {
            items: vec![make_alice_resource()],
            total: 1,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(20),
        };
        let components: Vec<ComponentData> = vec![
            ComponentData::Facets(facets_data),
            ComponentData::Other,
            ComponentData::List(list_page),
        ];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();

        let search_row_start = out.find(r#"<form class="search-row""#).expect("search row present");
        let search_row_end = out[search_row_start..]
            .find("</form>")
            .map(|i| search_row_start + i)
            .expect("search row must close with </form>");
        let search_row_markup = &out[search_row_start..search_row_end];

        assert!(
            !search_row_markup.contains("filters-character-list"),
            "the search row's own markup must not contain the filterbar's \
             swap-target id, got: {search_row_markup}"
        );
        assert!(
            !search_row_markup.contains(r#"id="character-list""#),
            "the search row's own markup must not contain the list's \
             swap-target id, got: {search_row_markup}"
        );
    }

    /// The search-bar's own additions — the search row, the query chip, and
    /// the no-match state — introduce NO new `aria-live` region beyond the
    /// pre-existing baseline: `.surf-head__count` + `.filterbar` (the filterbar is its own
    /// small live region) plus the ordering
    /// control's own `#orderctl-status` (a
    /// dedicated status span announcing the current ordering), for a total
    /// of THREE. Regression guard against a FOURTH live region creeping in
    /// (e.g. on the query chip or the search input itself, which would
    /// over-announce on every debounced keystroke).
    #[test]
    fn contract_no_third_aria_live_region_added_by_search_bar() {
        use cpe_core::ir::{FacetGroup, FacetKind, FacetOption, Pagination, ResourceListPage};

        let site = make_alice_site();
        let page = make_faceted_search_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/characters",
            route_params: &[],
            current_query: Some("q=rab&role=R_01"),
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let facets_data = vec![FacetGroup {
            property: "role".to_owned(),
            label: "Role".to_owned(),
            kind: FacetKind::List,
            options: vec![FacetOption {
                value: "R_01".to_owned(),
                label: "Main character".to_owned(),
                count: 12,
                selected: true,
            }],
        }];
        let list_page = ResourceListPage {
            items: vec![make_alice_resource()],
            total: 1,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: Some(20),
        };
        let components: Vec<ComponentData> = vec![
            ComponentData::Facets(facets_data),
            ComponentData::Other,
            ComponentData::List(list_page),
        ];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();

        let browse_start = out.find(r#"class="browse page""#).expect("browse wrapper present");
        let browse_section = &out[browse_start..];
        let aria_live_count = browse_section.matches("aria-live").count();
        assert_eq!(
            aria_live_count, 3,
            "exactly three aria-live regions (filterbar + surf-head__count + \
             orderctl-status) must exist inside .browse — search-bar adds \
             none of its own — got {aria_live_count} in: {browse_section}"
        );
        assert!(
            browse_section.contains(r#"surf-head__count" id="browse-count" aria-live"#),
            "surf-head__count must be one of the live regions, got: {browse_section}"
        );
        assert!(
            browse_section.contains(r#"class="filterbar" id="filters-character-list" aria-live"#),
            "the filterbar must be another of the live regions, got: {browse_section}"
        );
        assert!(
            browse_section.contains(r#"id="orderctl-status" aria-live"#),
            "the ordering control's status span must be the third live region, \
             got: {browse_section}"
        );
        // The search row itself (rendered before either live region in
        // browse_section) must carry no aria-live of its own.
        let search_row_start = browse_section.find(r#"<form class="search-row""#).expect("search row present");
        let search_row_end = browse_section[search_row_start..]
            .find("</form>")
            .map(|i| search_row_start + i)
            .expect("search row must close with </form>");
        assert!(
            !browse_section[search_row_start..search_row_end].contains("aria-live"),
            "the search row must not carry its own aria-live region, got: {}",
            &browse_section[search_row_start..search_row_end]
        );
    }

    /// No `<svg>` anywhere in the search row: search does not
    /// earn a second icon.
    #[test]
    fn contract_no_svg_in_search_row() {
        let site = make_alice_site();
        let page = make_search_only_page();
        let narrative = make_alice_narrative();
        let lang = resolve(&["en".to_owned()], &site);
        let ctx = RenderContext {
            base: &cpe_core::url::ROOT,
            lang,
            project_id: "alice-in-daschland",
            current_path: "/alice-in-daschland/chapters",
            route_params: &[],
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        };
        let components: Vec<ComponentData> = vec![ComponentData::Other, ComponentData::Other];
        let out = render_page(&site, &page, &narrative, &ctx, &components, &DataArks::default()).into_string();

        let search_row_start = out.find(r#"<form class="search-row"#).expect("search row present");
        let search_row_end = out[search_row_start..]
            .find("</form>")
            .map(|i| search_row_start + i)
            .expect("search row must close with </form>");
        let search_row_markup = &out[search_row_start..search_row_end];
        assert!(
            !search_row_markup.contains("<svg"),
            "the search row must contain no <svg> — no magnifier icon per \
             design-doc §1, got: {search_row_markup}"
        );
    }

    // A11y: ARIA + landmarks

    /// The ordering control's `<summary>` has an `aria-label` naming the
    /// current ordering, and every row button carries visible text —
    /// regression guard for the a11y audit's "every interactive control has
    /// an accessible name" call. See `resource_list::render_order_control`.
    #[test]
    fn order_control_has_accessible_label() {
        use cpe_core::ir::{Pagination, Resource, ResourceListPage};
        let cfg = resource_list::ResourceListCfg {
            links: None,
            current_path: "/alice-in-daschland/characters",
            current_query: None,
            list_id: Some("character-list"),
            gloss: None,
            chips: &[],
            class: "Character",
            class_labels: &[],
            search: None,
            narrowed: None,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            grid: false,
            tile_ratio: None,
            order_name_label: None,
            order_name_desc_label: None,
            order_default_label: None,
            order_options: &[],
        };
        let list = ResourceListPage {
            items: vec![Resource {
                id: "CH_001".to_owned(),
                class: "Character".to_owned(),
                properties: vec![],
                representations: vec![],
            }],
            total: 1,
            page: 1,
            pagination: Pagination::PerPage(24),
            unfiltered_total: None,
        };
        let lang = resolve(&["en".to_owned()], &make_alice_site());
        let out = resource_list::render_resource_list(&list, &cfg, &lang).into_string();
        assert!(
            out.contains(r#"aria-label="Order: As curated""#),
            "the summary must carry aria-label 'Order: <current>', got: {out}"
        );
        for label in ["As curated", "Name A\u{2013}Z", "Name Z\u{2013}A", "Identifier"] {
            assert!(
                out.contains(label),
                "every ordering row must have visible text (missing '{label}'), got: {out}"
            );
        }
    }

    // The crumb line's dataset panel (synthetic ARKs)

    const DATA_ARK: &str = "https://ark.dasch.swiss/ark:/72163/1/0803/";

    /// The data ARK map for these resource and annotation ids.
    fn data_arks_for(resources: &[&str], annotations: &[&str]) -> DataArks {
        let map = |ids: &[&str]| -> HashMap<String, String> {
            ids.iter().map(|id| ((*id).to_owned(), format!("{DATA_ARK}{id}="))).collect()
        };
        DataArks::from_stored(map(resources), map(annotations))
    }

    /// Alice's site with Incunabula's three class labels.
    fn data_panel_site() -> SiteConfig {
        let mut site = make_alice_site();
        for (class, de, en) in [
            ("Book", "Buch", "Book"),
            ("Page", "Seite", "Page"),
            ("BorderStrip", "Randleiste", "Border strip"),
        ] {
            site.class_labels.push(cpe_core::ir::ClassLabel {
                class: class.to_owned(),
                translations: vec![("de".to_owned(), de.to_owned()), ("en".to_owned(), en.to_owned())],
                plural: vec![],
            });
        }
        site
    }

    fn named(id: &str, class: &str, name: &str) -> Resource {
        Resource {
            id: id.to_owned(),
            class: class.to_owned(),
            properties: vec![cpe_core::ir::Property {
                name: "name".to_owned(),
                values: vec![cpe_core::ir::PropertyValue::Text { text: name.to_owned(), lang: None }],
            }],
            representations: vec![],
        }
    }

    fn data_region(id: &str, number: u32, remark: Option<&str>) -> cpe_core::ir::AnnotationEntry {
        cpe_core::ir::AnnotationEntry::Annotation {
            number,
            annotation: cpe_core::ir::Annotation {
                id: id.to_owned(),
                resource_id: "c1v".to_owned(),
                shape: cpe_core::ir::AnnotationShape::Rectangle,
                x: 0.1,
                y: 0.1,
                w: 0.2,
                h: 0.2,
                points: vec![],
                color: Some("#ff3333".to_owned()),
                remark: remark.map(str::to_owned),
                remark_lang: remark.map(|_| "de".to_owned()),
            },
            partners: vec![],
        }
    }

    /// The reader on page c1v of two, with two Regions.
    fn c1v_reader() -> cpe_core::ir::CompoundPage {
        cpe_core::ir::CompoundPage {
            member: Some(named("c1v", "Page", "c1v")),
            index: 2,
            total: 2,
            members: vec![
                cpe_core::ir::MemberHeader {
                    id: "c1r".to_owned(),
                    class: "Page".to_owned(),
                    name: "c1r".to_owned(),
                },
                cpe_core::ir::MemberHeader {
                    id: "c1v".to_owned(),
                    class: "Page".to_owned(),
                    name: "c1v".to_owned(),
                },
            ],
            member_media: vec![],
            stale_page: None,
            annotations: vec![
                data_region("r1", 1, Some("Narr mit Schellenkappe")),
                data_region("r2", 2, None),
            ],
            annotated_members: std::collections::BTreeSet::new(),
        }
    }

    fn data_ctx<'a>(
        site: &SiteConfig,
        code: &str,
        path: &'a str,
        route_params: &'a [(String, String)],
    ) -> RenderContext<'a> {
        RenderContext {
            base: &cpe_core::url::ROOT,
            lang: resolve(&[code.to_owned()], site),
            project_id: "incunabula",
            current_path: path,
            route_params,
            current_query: None,
            appearance: Appearance::Light,
            selected_image: 0,
            link_routes: &EMPTY_ROUTES,
            media_properties: &EMPTY_ROUTES,
            media_items: &EMPTY_MEDIA,
            strip_open: true,
            media_thumbs: &EMPTY_MEDIA_THUMBS,
            compound_members: &[],
            compound_parents: &EMPTY_PARENTS,
            class_list_routes: &EMPTY_ROUTES,
            public_origin: "http://localhost:8080",
            incoming_links: None,
            resource_links: &[],
        }
    }

    fn detail(class: &str, navigator: bool) -> Component {
        Component::ResourceDetail {
            head: None,
            id: Some("$id".to_owned()),
            show_properties: None,
            title_property: Some("name".to_owned()),
            class: Some(class.to_owned()),
            media: None,
            groups: vec![],
            narrative_for: vec![],
            incoming_links: None,
            navigator,
        }
    }

    fn reader_component(layout: cpe_core::ir::CompoundLayout, paired: bool) -> Component {
        Component::CompoundViewer {
            source: "$id".to_owned(),
            media: None,
            picker: None,
            layout,
            index: cpe_core::ir::MemberIndex::Off,
            paired,
            member_links: None,
            annotations: true,
        }
    }

    /// The book page: a Book record beside its `pages` reader on c1v.
    fn data_book_page() -> cpe_core::ir::Page {
        cpe_core::ir::Page {
            id: "book".to_owned(),
            route: "/books/:id".to_owned(),
            params: vec!["id".to_owned()],
            body: vec![
                detail("Book", true),
                reader_component(cpe_core::ir::CompoundLayout::Pages, true),
            ],
            figure_slots: Vec::new(),
        }
    }

    fn data_book_components() -> [ComponentData; 2] {
        [
            ComponentData::Detail(Some(named("narrenschiff-dt", "Book", "[Das] Narrenschiff (dt.)"))),
            ComponentData::Compound(c1v_reader()),
        ]
    }

    fn book_crumb(code: &str, data_arks: &DataArks) -> String {
        let site = data_panel_site();
        let params = [("id".to_owned(), "narrenschiff-dt".to_owned())];
        let ctx = data_ctx(&site, code, "/incunabula/books/narrenschiff-dt", &params);
        render_record_breadcrumb_for_page(&data_book_page(), &ctx, &data_book_components(), &site, data_arks)
            .expect("a record crumb")
            .into_string()
    }

    fn book_arks() -> DataArks {
        data_arks_for(&["narrenschiff-dt", "c1v"], &["r1", "r2"])
    }

    #[test]
    fn snapshot_data_panel_book_page_de() {
        insta::assert_snapshot!(book_crumb("de", &book_arks()));
    }

    #[test]
    fn snapshot_data_panel_book_page_en() {
        insta::assert_snapshot!(book_crumb("en", &book_arks()));
    }

    /// A record whose resource has no ARK keeps the frame and Copy link alone.
    #[test]
    fn snapshot_data_panel_record_without_ark_de() {
        let out = book_crumb("de", &DataArks::default());
        assert!(!out.contains("crumb-data"), "{out}");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_data_panel_strip_en() {
        let site = data_panel_site();
        let params = [("id".to_owned(), "l-1a".to_owned())];
        let ctx = data_ctx(&site, "en", "/incunabula/strips/l-1a", &params);
        let page = cpe_core::ir::Page {
            id: "strip".to_owned(),
            route: "/strips/:id".to_owned(),
            params: vec!["id".to_owned()],
            body: vec![detail("BorderStrip", false)],
            figure_slots: Vec::new(),
        };
        let components = [ComponentData::Detail(Some(named(
            "l-1a",
            "BorderStrip",
            "Randleiste l_1a",
        )))];
        let out = render_record_breadcrumb_for_page(&page, &ctx, &components, &site, &data_arks_for(&["l-1a"], &[]))
            .expect("a record crumb")
            .into_string();
        assert_eq!(out.matches("crumb-data__entry").count(), 1, "{out}");
        insta::assert_snapshot!(out);
    }

    /// A narrative reader page lists its shown member and that member's
    /// Regions; it has no record.
    #[test]
    fn snapshot_data_panel_narrative_reader_en() {
        let site = data_panel_site();
        let ctx = data_ctx(&site, "en", "/incunabula/reader", &[]);
        let page = cpe_core::ir::Page {
            id: "reader".to_owned(),
            route: "/reader".to_owned(),
            params: vec![],
            body: vec![
                Component::Narrative { file: "home.md".to_owned(), companion: None, side: None },
                reader_component(cpe_core::ir::CompoundLayout::Contents, false),
            ],
            figure_slots: Vec::new(),
        };
        let components = [ComponentData::Other, ComponentData::Compound(c1v_reader())];
        let out = render_narrative_breadcrumb_for_page(
            &page,
            &ctx,
            &make_alice_narrative(),
            &site,
            &components,
            &data_arks_for(&["c1v"], &["r1"]),
        )
        .expect("a narrative crumb")
        .into_string();
        insta::assert_snapshot!(out);
    }

    /// The `_compound` swap's crumb is the full page's, panel included.
    #[test]
    fn compound_fragment_crumb_matches_the_full_page_with_a_panel() {
        let site = data_panel_site();
        let params = [("id".to_owned(), "narrenschiff-dt".to_owned())];
        let ctx = data_ctx(&site, "de", "/incunabula/books/narrenschiff-dt", &params);
        let page = data_book_page();
        let narrative = make_alice_narrative();
        let components = data_book_components();
        let arks = book_arks();
        let full = render_page(&site, &page, &narrative, &ctx, &components, &arks).into_string();
        let crumb = render_compound_fragment(&site, &page, &narrative, &ctx, &components, &arks)
            .and_then(|f| f.crumb)
            .expect("a crumb")
            .into_string();
        assert!(crumb.contains("crumb-data-entry-4"), "{crumb}");
        assert!(full.contains(&crumb), "the full page differs from: {crumb}");
    }
}
