//! Breadcrumb renderer for CPE pages.
//!
//! Emits `nav.breadcrumb[aria-label="Breadcrumb"] > div.breadcrumb__row`,
//! a flex row (`justify-content: space-between`; it wraps above 40rem) holding
//! `a.crumb-back` (the one link a phone shows in place of the path), the
//! `ol.breadcrumb__list` path on the left and, at the right edge, the
//! `div.crumb-actions` frame whose first row holds the "Copy link" share
//! control. Only the record and narrative variants can
//! carry a dataset panel ([`DataPanel`]) in that frame.
//! Four crumb-line SHAPES share the row/control
//! machinery:
//!
//! 1. [`render_breadcrumb`] — the resource-detail trail (`CPE › project › resource-title`), or `CPE
//!    › project › class › resource-title` when the resource's class has a registered browse route
//!    (`class_crumb: Some((label, href))`).
//! 2. [`render_browse_breadcrumb`] — `CPE › project › class-label`.
//! 3. [`render_narrative_breadcrumb`] — `CPE › project › page-title`, or `CPE › project` alone for
//!    the project home page (`page_title: None`).
//! 4. [`render_site_index_breadcrumb`] — the degenerate single-crumb line (`CPE`, current,
//!    unlinked) for the site index.
//!
//! Every variant ends with the same share control, built by the caller
//! via [`ShareControl`] — this module never composes the href itself
//! (that stays with `share_link::compose_canonical_query` +
//! `compose_absolute_url`, called by each surface's own renderer/handler).
//!
//! `aria-label` is localized via the engine chrome catalog
//! (`crate::chrome::ChromeMsg::BreadcrumbLabel`).

use cpe_core::lang::Lang;
use cpe_core::url::{ProjectUrls, StaticDir};
use cpe_core::util::js_string_escape;
use maud::{html, Markup};

use super::data_links::{render_data_panel, DataPanel};
use crate::chrome::ChromeMsg;

/// Parameters for the "Copy link" control at the right edge of the crumb
/// line.
///
/// Callers pre-compose `href` via `share_link::compose_canonical_query` +
/// `compose_absolute_url` — this module only renders markup, never touches
/// query-string/URL composition itself.
pub struct ShareControl<'a> {
    /// Stable per-region id, emitted as the control `<a>`'s `id` attribute
    /// so a fragment swap that replaces the crumb line does not orphan the
    /// keyboard focus ring.
    pub id: &'a str,
    /// The already-composed absolute URL to copy / follow.
    pub href: &'a str,
    /// State-in-words suffix appended after
    /// [`ChromeMsg::CopyLinkLongPrefix`] as ` — <suffix>` (browse surfaces
    /// only: on record/reader the control
    /// never announces which state it caught). The caller composes this suffix.
    pub accessible_name_suffix: Option<&'a str>,
    /// An OPTIONAL reactive `data-attr` object-literal expression
    /// (`"{'href': …}"`) that overrides `href` client-side once DataStar
    /// hydrates. `href` is a plain
    /// HTML attribute, not `data-*`-namespaced, so writing it reactively
    /// via `data-attr` is a sanctioned exception
    /// — it does NOT violate the "never reactively write a `data-*`
    /// attribute" rule (that rule is about DataStar's own `data-*`
    /// namespace, e.g. `data-signals`/`data-bind`).
    ///
    /// Only the record surface (a `resource-detail` page with more than
    /// one IIIF-sequenceable linked image, i.e. the record-viewer's
    /// `$plate` pager is present) ever sets this — every other surface
    /// passes `None`, keeping a plain static `href` with no reactive
    /// override. The static `href` field above always stays the no-JS /
    /// initial-load value; this expression only overrides it once JS runs.
    ///
    /// **Unverified in-browser**: whether
    /// the vendored DataStar bundle accepts a `data-attr` object
    /// literal whose value indexes a JS array literal (of server-precomposed,
    /// canonical, per-plate-index URLs) by `$plate`.
    pub reactive_href_attr: Option<&'a str>,
}

/// Render the breadcrumb trail for a resource-detail page.
///
/// Parameters:
/// - `project_id` — the kebab-case project id (URL segment).
/// - `project_title` — the human-readable site title for the project crumb.
/// - `class_crumb` — an OPTIONAL live-linked class segment inserted between the project and
///   resource-title crumbs (`CPE › project › class › resource`), `Some((label, href))`. `None`
///   yields the original 3-crumb trail unchanged — the fallback when the resource's class has no
///   registered browse route (no dead segment, no unlinked text). Mirrors the unknown-record
///   not-found breadcrumb's `class_crumb` resolution ([`render_not_found_breadcrumb`]) — same
///   `resolve_class_label` + `class_list_routes` seam, just on the resolved-record path instead of
///   the unresolved-id path.
/// - `resource_title` — the resolved resource title for the current-page crumb.
/// - `lang` — the resolved page language, for the localized `aria-label`.
/// - `share` — the share control parameters (right edge of the crumb row).
/// - `data` — the dataset panel under Copy link, or `None` for no panel row.
pub fn render_breadcrumb(
    urls: ProjectUrls<'_>,
    project_title: &str,
    class_crumb: Option<(&str, &str)>,
    resource_title: &str,
    lang: &Lang,
    share: &ShareControl<'_>,
    data: Option<&DataPanel>,
) -> Markup {
    let site_href = urls.site_home();
    let project_href = urls.home();
    let mut links = vec![cpe_crumb(&site_href), CrumbLink::new(&project_href, project_title)];
    if let Some((label, href)) = class_crumb {
        links.push(CrumbLink::new(href, label));
    }
    render_crumb_row(
        &links,
        resource_title,
        lang,
        share,
        data.map(|panel| (panel, urls.static_dir())),
    )
}

/// Render the browse crumb-line variant: `CPE › project › class-label`
/// (the class label is the current item — the page IS the browse listing
/// for that class, so it carries no link).
///
/// Wired at its production call site in `dsp-incubator cpe/web`'s `page` module's
/// `render_browse_breadcrumb_for_page` — any page
/// with a `ResourceList` component carrying a declared `id`.
pub fn render_browse_breadcrumb(
    urls: ProjectUrls<'_>,
    project_title: &str,
    class_label: &str,
    lang: &Lang,
    share: &ShareControl<'_>,
) -> Markup {
    let site_href = urls.site_home();
    let project_href = urls.home();
    let links = [cpe_crumb(&site_href), CrumbLink::new(&project_href, project_title)];
    render_crumb_row(&links, class_label, lang, share, None)
}

/// A trailing member segment on a "reader" (compound-viewer) narrative
/// page: the
/// crumb's last segment must name the ON-SCREEN member, not just the page.
/// When present, `page_title` (which would otherwise be the unlinked
/// current item) becomes a live link to `page_href` (the page's own path,
/// no `?member=` — real navigation back to member 1), and this segment's
/// `label` becomes the new unlinked current item.
pub struct NarrativeMemberSegment<'a> {
    /// The page's own path with no query string at all (not just
    /// `?member` stripped) — e.g. `/alice-in-daschland/wonderland`. A
    /// real `<a href>`, not a fragment-swap target.
    pub page_href: &'a str,
    /// The localized `"{class-label} · NN"` text (1-based, zero-padded to
    /// ≥2 digits) — the SAME convention `compound_viewer.rs`'s
    /// `cv-member__eyebrow` uses, resolved through the same
    /// `resolve_class_label` seam so the two can never drift.
    pub label: &'a str,
}

/// Render the narrative crumb-line variant: `CPE › project › page-title`,
/// or `CPE › project` alone when `page_title` is `None` (the project home
/// page — "project home: `CPE › Project`").
/// On a reader (compound-viewer) page, `member` adds a fourth
/// segment naming the on-screen member
/// — see [`NarrativeMemberSegment`]. `data` is the dataset panel under
/// Copy link, or `None` for no panel row.
///
/// Wired at its production call site in `dsp-incubator cpe/web`'s `page` module's `render_page`
/// — any page with a `Narrative` component and no
/// `ResourceDetail` component.
pub fn render_narrative_breadcrumb(
    urls: ProjectUrls<'_>,
    project_title: &str,
    page_title: Option<&str>,
    member: Option<&NarrativeMemberSegment<'_>>,
    lang: &Lang,
    share: &ShareControl<'_>,
    data: Option<&DataPanel>,
) -> Markup {
    let site_href = urls.site_home();
    let project_href = urls.home();
    let project = CrumbLink::new(&project_href, project_title);
    let (links, current) = match (page_title, member) {
        (None, _) => (vec![cpe_crumb(&site_href)], project_title),
        (Some(title), None) => (vec![cpe_crumb(&site_href), project], title),
        (Some(title), Some(member)) => (
            vec![cpe_crumb(&site_href), project, CrumbLink::new(member.page_href, title)],
            member.label,
        ),
    };
    render_crumb_row(&links, current, lang, share, data.map(|panel| (panel, urls.static_dir())))
}

/// Render the site-index crumb-line variant: the degenerate single "CPE"
/// crumb (current, unlinked).
///
/// Called by `cpe_web::page::render_site_index`, CPE's overview of its
/// projects.
pub fn render_site_index_breadcrumb(lang: &Lang, share: &ShareControl<'_>) -> Markup {
    render_crumb_row(&[], CPE_LABEL, lang, share, None)
}

/// Render the unknown-record not-found breadcrumb variant.
///
/// `CPE › project › class (live link) › "Not found"` — the class crumb is
/// present only when `class_crumb` is `Some((label, href))`, giving
/// `CPE › project › "Not found"` otherwise (class-less detail pages, or a
/// class with no registered browse route — see `render_not_found`'s and
/// `Project::class_list_routes`'s doc comments for the same fallback rule).
///
/// The current-item label is the localized `ChromeMsg::NfCrumb` string ("Not found"),
/// never the unknown identifier — the identifier is never restated anywhere
/// in that page's visible copy.
///
/// **No `div.breadcrumb__row`, on purpose:** below 40rem `shell.css` (THE
/// PHONE ROW) hides a row's trail, because a row has a back link in its
/// place. This variant has no back link, so its trail must stay outside a row.
///
/// **No share control:** this variant does
/// NOT gain one — the four crumb-line variants with one are
/// detail/browse/narrative/site-index; a 404 page is none of those, and
/// "copy a link to a page that doesn't resolve" is not a stated use case.
pub fn render_not_found_breadcrumb(
    urls: ProjectUrls<'_>,
    project_title: &str,
    class_crumb: Option<(&str, &str)>,
    lang: &Lang,
) -> Markup {
    let site_href = urls.site_home();
    let project_href = urls.home();
    html! {
        nav.breadcrumb aria-label=(ChromeMsg::BreadcrumbLabel.text(lang)) {
            ol.breadcrumb__list {
                li.breadcrumb__item {
                    a href=(site_href) { (CPE_LABEL) }
                }
                li.breadcrumb__sep aria-hidden="true" { "›" }
                li.breadcrumb__item {
                    a href=(project_href) { (project_title) }
                }
                @if let Some((label, href)) = class_crumb {
                    li.breadcrumb__sep aria-hidden="true" { "›" }
                    li.breadcrumb__item {
                        a href=(href) { (label) }
                    }
                }
                li.breadcrumb__sep aria-hidden="true" { "›" }
                li.breadcrumb__item.breadcrumb__current aria-current="page" {
                    (ChromeMsg::NfCrumb.text(lang))
                }
            }
        }
    }
}

/// Derive the crumb line's own stable element id from the share control's
/// id (the whole crumb line, not just the control
/// anchor, is a second DataStar patch target inside the `_facets`/`_sort`/
/// `_search` fragment responses, since both the composed href
/// AND the state-bearing accessible name can change on a refinement).
///
/// Mirrors `share.id`'s `share-<region>` shape with a `crumb-` prefix, so
/// both ids derive from the same region key (project id or list id)
/// without threading a second id parameter through every crumb-line
/// variant fn. `dsp-incubator cpe/bin/src/router.rs`'s fragment handlers call this directly
/// (never hardcode a `"crumb-…"` string literal) to compute the same
/// `PatchElements` selector the full-page render already emits.
pub fn crumb_element_id(share_id: &str) -> String {
    format!("crumb-{}", share_id.strip_prefix("share-").unwrap_or(share_id))
}

// Shared crumb-row + share-control rendering

/// A linked crumb of the trail: an ancestor of the current page.
#[derive(Clone, Copy)]
struct CrumbLink<'a> {
    href: &'a str,
    label: &'a str,
}

impl<'a> CrumbLink<'a> {
    fn new(href: &'a str, label: &'a str) -> Self {
        Self { href, label }
    }
}

const CPE_LABEL: &str = "CPE";

/// CPE's own start (`Base::home`), the first crumb of every trail.
fn cpe_crumb(site_href: &str) -> CrumbLink<'_> {
    CrumbLink::new(site_href, CPE_LABEL)
}

/// Build the shared `nav.breadcrumb > div.breadcrumb__row > (a.crumb-back,
/// ol.breadcrumb__list, div.crumb-actions)` shape from a trail: its linked
/// crumbs (`links`, outermost first) and the unlinked current item.
///
/// `a.crumb-back` is the phone row's one link, shown in place of the trail
/// below 40rem (`shell.css`). It goes to the last linked crumb, read from
/// the same `links` the trail renders, so the two cannot name different
/// parents; a trail with no link (the site index) has no back link. The link
/// has no id and no `aria-label`: its name is the visually hidden
/// [`ChromeMsg::CrumbBackPrefix`] followed by the visible label, and the
/// label needs its own span because `text-overflow` does not reach the bare
/// text of an `inline-flex` box.
///
/// `div.crumb-actions` is the framed action block at the row's right edge:
/// its first row, `div.crumb-actions__row`, holds Copy link, and the
/// dataset panel row (`data`) goes under it inside the same frame.
///
/// The `nav.breadcrumb` root carries its own stable `id` ([`crumb_element_id`])
/// so a refinement fragment response can target the
/// whole crumb line — not just the share control's own `<a>` — with a
/// `datastar-patch-elements` event.
fn render_crumb_row(
    links: &[CrumbLink<'_>],
    current: &str,
    lang: &Lang,
    share: &ShareControl<'_>,
    data: Option<(&DataPanel, StaticDir)>,
) -> Markup {
    let crumb_id = crumb_element_id(share.id);
    html! {
        nav.breadcrumb id=(crumb_id) aria-label=(ChromeMsg::BreadcrumbLabel.text(lang)) {
            div.breadcrumb__row {
                @if let Some(parent) = links.last() {
                    a.crumb-back href=(parent.href) {
                        span.visually-hidden { (ChromeMsg::CrumbBackPrefix.text(lang)) }
                        " "
                        span aria-hidden="true" { "‹" }
                        span.crumb-back__label { (parent.label) }
                    }
                }
                ol.breadcrumb__list {
                    @for link in links {
                        li.breadcrumb__item {
                            a href=(link.href) { (link.label) }
                        }
                        li.breadcrumb__sep aria-hidden="true" { "›" }
                    }
                    li.breadcrumb__item.breadcrumb__current aria-current="page" { (current) }
                }
                div.crumb-actions {
                    div.crumb-actions__row { (render_share_control(share, lang)) }
                    @if let Some((panel, statics)) = &data {
                        (render_data_panel(panel, lang, statics))
                    }
                }
            }
        }
    }
}

/// Render the "Copy link" share control.
///
/// One `<a>` carrying the composed `href`, `title`/`aria-label` =
/// [`ChromeMsg::CopyLinkLongPrefix`] (plus `share.accessible_name_suffix`
/// when present), the chain-glyph SVG +
/// copied-state dot, two label spans, and an adjacent visually-hidden
/// `aria-live="polite"` status span.
///
/// The label is two sibling spans, a long and a short resting label, of
/// which CSS shows one: `data-text` replaces a span's whole content, so the
/// pair cannot be nested inside one labelled span. Both keep
/// `.crumb-share__label` and both read "Copied" while the receipt shows.
/// Each `data-text` is a value expression and must hold no `;`.
///
/// `data-signals` `{copied:false}` is
/// scoped to the control's own wrapper; the receipt swap (glyph → dot,
/// label → "Copied") is driven by `data-class`/`data-text` reading
/// `$copied` — never a reactively-written `data-*` attribute, and never
/// `hidden` (see `dsp-incubator cpe/CLAUDE.md`'s DataStar gotchas).
///
/// **Timeout idiom not yet browser-verified**:
/// the 2.4s reset is a plain `setTimeout` inside the `data-on:click`
/// expression. If the vendored DataStar bundle cannot schedule a JS
/// timeout from inside a directive expression, this needs a
/// fallback (a one-line `data-on:click` calling a tiny inline
/// helper).
fn render_share_control(share: &ShareControl<'_>, lang: &Lang) -> Markup {
    let prefix = ChromeMsg::CopyLinkLongPrefix.text(lang);
    let long_name = match share.accessible_name_suffix {
        Some(suffix) => format!("{prefix} — {suffix}"),
        None => prefix.to_owned(),
    };
    let label_long = ChromeMsg::CopyLinkLabel.text(lang);
    let label_short = ChromeMsg::CopyLinkShort.text(lang);
    let label_copied = ChromeMsg::CopyLinkCopied.text(lang);
    let live_text = ChromeMsg::CopyLinkLive.text(lang);
    let label_expr = |resting: &str| {
        format!(
            "$copied ? '{}' : '{}'",
            js_string_escape(label_copied),
            js_string_escape(resting)
        )
    };
    let live_expr = format!("$copied ? '{}' : ''", js_string_escape(live_text));

    html! {
        span.crumb-share data-signals="{copied:false}" {
            a   .crumb-share__link
                id=(share.id)
                href=(share.href)
                data-attr=[share.reactive_href_attr]
                title=(long_name)
                aria-label=(long_name)
                data-class="{'is-copied': $copied}"
                data-on:click__prevent="navigator.clipboard.writeText(el.href); $copied = true; setTimeout(() => { $copied = false }, 2400)"
            {
                span.crumb-share__mark aria-hidden="true" {
                    span.crumb-share__glyph { (chain_glyph()) }
                    span.crumb-share__dot {}
                }
                span.crumb-share__label.crumb-label--long data-text=(label_expr(label_long)) {
                    (label_long)
                }
                span.crumb-share__label.crumb-label--short data-text=(label_expr(label_short)) {
                    (label_short)
                }
            }
            span.crumb-share__status aria-live="polite" data-text=(live_expr) {}
        }
    }
}

/// The chain glyph SVG (14×14,
/// viewBox `0 0 16 16`, stroke-only, `currentColor`).
fn chain_glyph() -> Markup {
    html! {
        svg width="14"
            height="14"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            stroke-width="1.1"
            stroke-linecap="round"
            aria-hidden="true"
        {
            path d="M6.6 9.4 9.4 6.6" {}
            path d="M7.6 4.6 8.9 3.3a2.4 2.4 0 0 1 3.4 3.4L11 8" {}
            path d="M8.4 11.4 7.1 12.7a2.4 2.4 0 0 1-3.4-3.4L5 8" {}
        }
    }
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lang(code: &str) -> Lang {
        use cpe_core::ir::SiteConfig;
        let site = SiteConfig {
            title: "Test".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["en".to_owned(), "de".to_owned(), "fr".to_owned(), "it".to_owned()],
            default_language: "en".to_owned(),
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
        cpe_core::lang::resolve(&[code.to_owned()], &site)
    }

    fn share<'a>(id: &'a str, href: &'a str) -> ShareControl<'a> {
        ShareControl {
            id,
            href,
            accessible_name_suffix: None,
            reactive_href_attr: None,
        }
    }

    #[test]
    fn breadcrumb_contains_cpe_link_to_root() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/alice-in-daschland/characters/CH_001"),
            None,
        )
        .into_string();
        assert!(out.contains("href=\"/\""), "CPE crumb must link to /, got: {out}");
        assert!(out.contains("CPE"), "CPE label must appear, got: {out}");
    }

    /// Under a base, the crumbs and the dataset panel's files start with it.
    #[test]
    fn breadcrumb_under_a_base_writes_it_in_front_of_every_url() {
        let lang = make_lang("en");
        let base = cpe_core::url::Base::parse("/cpe").expect("/cpe is a base");
        let panel = DataPanel {
            entries: vec![super::super::data_entries::DataEntry {
                role: "Book".to_owned(),
                title: "R".to_owned(),
                title_lang: None,
                number: None,
                class: Some("Book".to_owned()),
                ark: "https://ark.dasch.swiss/ark:/72163/1/0803/x".to_owned(),
            }],
            annotations: None,
            listing: None,
        };
        let out = render_breadcrumb(
            base.project("p"),
            "P",
            Some(("Books", "/cpe/p/books")),
            "R",
            &lang,
            &share("share-p", "https://repository.example/cpe/p/books/r"),
            Some(&panel),
        )
        .into_string();
        assert!(out.contains(r#"href="/cpe/""#), "the CPE crumb: {out}");
        assert!(out.contains(r#"href="/cpe/p/""#), "the project crumb: {out}");
        assert!(out.contains(r#"href="/cpe/p/books""#), "the class crumb: {out}");
        assert!(out.contains(r#"src="/cpe/p/_static/"#), "a file of the dataset panel: {out}");
        assert!(
            out.contains(r#"href="https://repository.example/cpe/p/books/r""#),
            "the share link: {out}"
        );
        assert!(!out.contains("/cpe/cpe"), "the base is written once: {out}");
        for path in out.split('"').filter(|text| text.starts_with('/') && !text.starts_with("//")) {
            assert!(path.starts_with("/cpe/"), "{path} leaves the base: {out}");
        }
    }

    #[test]
    fn breadcrumb_contains_project_link() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            out.contains("href=\"/alice-in-daschland/\""),
            "project crumb must link to /alice-in-daschland/, got: {out}"
        );
        assert!(out.contains("Alice in DaSCHland"), "project title must appear, got: {out}");
    }

    #[test]
    fn breadcrumb_current_item_has_aria_current_and_no_link() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            out.contains("aria-current=\"page\""),
            "current item must have aria-current=page, got: {out}"
        );
        assert!(
            out.contains("breadcrumb__current"),
            "current item must have breadcrumb__current class, got: {out}"
        );
        // The resource title "Alice" must appear without being inside an <a>.
        // We check by verifying the breadcrumb__current li contains the title
        // but the li itself has no href (is not an <a>).
        // Simple: the class=breadcrumb__current item must contain "Alice".
        let current_idx = out.find("breadcrumb__current").expect("breadcrumb__current not found");
        let after_current = &out[current_idx..];
        assert!(
            after_current.contains("Alice"),
            "resource title must appear after breadcrumb__current, got: {after_current}"
        );
    }

    #[test]
    fn breadcrumb_has_nav_with_aria_label() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("my-project"),
            "My Project",
            None,
            "A Resource",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            out.contains("aria-label=\"Breadcrumb\""),
            "nav must have aria-label=Breadcrumb, got: {out}"
        );
        assert!(
            out.contains("class=\"breadcrumb\""),
            "must have breadcrumb class on nav, got: {out}"
        );
        assert!(
            out.contains("class=\"breadcrumb__list\""),
            "must have breadcrumb__list on ol, got: {out}"
        );
    }

    #[test]
    fn breadcrumb_separators_are_aria_hidden() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        // The sep elements must have aria-hidden="true".
        assert!(
            out.contains("class=\"breadcrumb__sep\" aria-hidden=\"true\""),
            "separators must be aria-hidden, got: {out}"
        );
    }

    /// The breadcrumb `aria-label` is localized via the chrome catalog.
    #[test]
    fn breadcrumb_aria_label_localizes_for_de_fr_it() {
        let lang = make_lang("de");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            out.contains("aria-label=\"Brotkrümelnavigation\""),
            "DE breadcrumb aria-label must be localized, got: {out}"
        );

        let lang = make_lang("fr");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            out.contains("aria-label=\"Fil d&#x27;Ariane\"") || out.contains("aria-label=\"Fil d'Ariane\""),
            "FR breadcrumb aria-label must be localized, got: {out}"
        );

        let lang = make_lang("it");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            out.contains("aria-label=\"Percorso di navigazione\""),
            "IT breadcrumb aria-label must be localized, got: {out}"
        );
    }

    // Share control

    #[test]
    fn share_control_has_href_and_stable_id() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share(
                "share-alice-in-daschland",
                "https://example.com/alice-in-daschland/characters/CH_001",
            ),
            None,
        )
        .into_string();
        assert!(
            out.contains("id=\"share-alice-in-daschland\""),
            "control must carry the caller's stable id, got: {out}"
        );
        assert!(
            out.contains("href=\"https://example.com/alice-in-daschland/characters/CH_001\""),
            "control href must be the composed absolute URL, got: {out}"
        );
    }

    #[test]
    fn share_control_label_is_copy_link_at_rest() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(out.contains("Copy link"), "resting label must be 'Copy link', got: {out}");
    }

    /// Both label spans keep the base class, and each swaps its own resting
    /// label for the same "Copied".
    #[test]
    fn share_control_has_a_long_and_a_short_label_span() {
        let lang = make_lang("de");
        let out = render_site_index_breadcrumb(&lang, &share("share-x", "https://example.com/x")).into_string();
        assert!(
            out.contains(
                "<span class=\"crumb-share__label crumb-label--long\" \
                 data-text=\"$copied ? 'Kopiert' : 'Link kopieren'\">Link kopieren</span>\
                 <span class=\"crumb-share__label crumb-label--short\" \
                 data-text=\"$copied ? 'Kopiert' : 'Link'\">Link</span></a>"
            ),
            "{out}"
        );
    }

    /// Every `data-text` value of the share control, HTML-unescaped.
    fn data_text_values(out: &str) -> Vec<String> {
        out.split(" data-text=\"")
            .skip(1)
            .map(|rest| {
                let raw = rest.split_once('"').expect("closing quote").0;
                raw.replace("&quot;", "\"")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&amp;", "&")
            })
            .collect()
    }

    /// DataStar splits a value expression on `;`, and `$el` is a signal
    /// lookup, not the element: neither may reach a `data-text`, whose
    /// strings come from the chrome catalog at run time.
    #[test]
    fn share_control_data_text_values_hold_no_semicolon_and_no_dollar_el() {
        for code in cpe_core::lang::BUILTIN_LANGUAGES {
            let lang = make_lang(code);
            let out = render_site_index_breadcrumb(&lang, &share("share-x", "https://example.com/x")).into_string();
            let values = data_text_values(&out);
            assert_eq!(values.len(), 3, "{code}: two label spans and the live span, got: {values:?}");
            for value in &values {
                assert!(!value.contains(';'), "{code}: {value}");
                assert!(!value.contains("$el"), "{code}: {value}");
            }
        }
    }

    #[test]
    fn share_control_data_signals_scoped_to_copied() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            out.contains("data-signals=\"{copied:false}\""),
            "share control must scope its own {{copied:false}} signal, got: {out}"
        );
    }

    #[test]
    fn share_control_uses_data_class_never_bare_hidden() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            out.contains("data-class=\"{&#x27;is-copied&#x27;: $copied}\"")
                || out.contains("data-class=\"{'is-copied': $copied}\""),
            "copied-state class must swap via data-class, got: {out}"
        );
        assert!(
            !out.contains(" hidden"),
            "share control must never use the bare hidden attribute, got: {out}"
        );
    }

    #[test]
    fn share_control_click_writes_clipboard_and_sets_copied_signal() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            out.contains("navigator.clipboard.writeText(el.href)"),
            "click handler must write el.href to the clipboard, got: {out}"
        );
        assert!(
            out.contains("$copied = true"),
            "click handler must set the copied signal, got: {out}"
        );
    }

    #[test]
    fn share_control_has_scoped_aria_live_status_span() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            out.contains("class=\"crumb-share__status\" aria-live=\"polite\""),
            "status span must be scoped (small element), not the crumb row root, got: {out}"
        );
    }

    #[test]
    fn share_control_accessible_name_suffix_is_appended() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &ShareControl {
                id: "share-detail",
                href: "https://example.com/x",
                accessible_name_suffix: Some("3 of 54 · Supporting · Fantasy"),
                reactive_href_attr: None,
            },
            None,
        )
        .into_string();
        assert!(
            out.contains("Copy link to this view — 3 of 54"),
            "suffix must be appended after the long-prefix text, got: {out}"
        );
    }

    #[test]
    fn share_control_accessible_name_has_no_suffix_when_none() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            out.contains("aria-label=\"Copy link to this view\""),
            "with no suffix, aria-label must be the bare long-prefix text, got: {out}"
        );
        assert!(
            !out.contains("Copy link to this view —"),
            "with no suffix, no em-dash suffix separator must appear, got: {out}"
        );
    }

    #[test]
    fn share_control_localizes_for_de() {
        let lang = make_lang("de");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(out.contains("Link kopieren"), "DE resting label must be localized, got: {out}");
        assert!(
            out.contains("aria-label=\"Link zu dieser Ansicht kopieren\""),
            "DE aria-label must be localized, got: {out}"
        );
    }

    // Crumb row layout

    #[test]
    fn crumb_row_wraps_list_and_control() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            out.contains("class=\"breadcrumb__row\""),
            "crumb line must be wrapped in the flex row, got: {out}"
        );
        // The row must contain both the path list and the share control.
        let row_idx = out.find("breadcrumb__row").expect("row not found");
        let after_row = &out[row_idx..];
        assert!(after_row.contains("breadcrumb__list"));
        assert!(after_row.contains("crumb-share"));
    }

    // crumb-actions frame

    /// Copy link sits in the frame's first row, and the frame follows the
    /// path list inside the flex row.
    fn assert_framed(out: &str, variant: &str) {
        assert!(
            out.contains(
                "<div class=\"crumb-actions\"><div class=\"crumb-actions__row\">\
                 <span class=\"crumb-share\""
            ),
            "{variant}: Copy link must open the frame's first row, got: {out}"
        );
        assert_eq!(
            out.matches("class=\"crumb-actions\"").count(),
            1,
            "{variant}: exactly one frame, got: {out}"
        );
        let list = out.find("breadcrumb__list").expect("path list");
        let frame = out.find("class=\"crumb-actions\"").expect("frame");
        assert!(list < frame, "{variant}: the frame follows the path list");
    }

    #[test]
    fn every_copy_link_crumb_variant_carries_the_frame() {
        let lang = make_lang("en");
        let s = share("share-x", "https://example.com/x");
        let record = render_breadcrumb(cpe_core::url::ROOT.project("p"), "P", None, "R", &lang, &s, None).into_string();
        assert_framed(&record, "record");
        let browse = render_browse_breadcrumb(cpe_core::url::ROOT.project("p"), "P", "Things", &lang, &s).into_string();
        assert_framed(&browse, "browse");
        let narrative =
            render_narrative_breadcrumb(cpe_core::url::ROOT.project("p"), "P", Some("Page"), None, &lang, &s, None)
                .into_string();
        assert_framed(&narrative, "narrative");
        let site_index = render_site_index_breadcrumb(&lang, &s).into_string();
        assert_framed(&site_index, "site index");
    }

    /// The record and narrative variants put a given panel in the frame,
    /// under Copy link's row; browse and the site index take none at all.
    #[test]
    fn only_record_and_narrative_crumbs_carry_the_dataset_panel() {
        let lang = make_lang("en");
        let s = share("share-x", "https://example.com/x");
        let panel = DataPanel {
            entries: vec![super::super::data_entries::DataEntry {
                role: "Book".to_owned(),
                title: "R".to_owned(),
                title_lang: None,
                number: None,
                class: Some("Book".to_owned()),
                ark: "https://ark.dasch.swiss/ark:/72163/1/0803/x".to_owned(),
            }],
            annotations: None,
            listing: None,
        };
        let record =
            render_breadcrumb(cpe_core::url::ROOT.project("p"), "P", None, "R", &lang, &s, Some(&panel)).into_string();
        let narrative = render_narrative_breadcrumb(
            cpe_core::url::ROOT.project("p"),
            "P",
            Some("Page"),
            None,
            &lang,
            &s,
            Some(&panel),
        )
        .into_string();
        for (variant, out) in [("record", &record), ("narrative", &narrative)] {
            assert!(
                out.contains("</span></div><details class=\"crumb-data\" id=\"crumb-data\""),
                "{variant}: the panel follows Copy link's row, got: {out}"
            );
            assert!(out.ends_with("</details></div></div></nav>"), "{variant}: {out}");
        }
        let browse = render_browse_breadcrumb(cpe_core::url::ROOT.project("p"), "P", "Things", &lang, &s).into_string();
        let site_index = render_site_index_breadcrumb(&lang, &s).into_string();
        for (variant, out) in [("browse", &browse), ("site index", &site_index)] {
            assert!(!out.contains("crumb-data"), "{variant}: {out}");
        }
    }

    #[test]
    fn not_found_breadcrumb_has_no_frame() {
        let lang = make_lang("en");
        let out =
            render_not_found_breadcrumb(cpe_core::url::ROOT.project("p"), "P", Some(("Things", "/p/things")), &lang)
                .into_string();
        assert!(!out.contains("crumb-actions"), "N3 has no Copy link, so no frame, got: {out}");
    }

    // the back link (the phone row's one link)

    /// The back link's `(href, visible label)`, read from the markup.
    fn back_link(out: &str) -> Option<(&str, &str)> {
        let rest = out.split_once("<a class=\"crumb-back\" href=\"")?.1;
        let (href, rest) = rest.split_once('"')?;
        let rest = rest.split_once("<span class=\"crumb-back__label\">")?.1;
        Some((href, rest.split_once("</span></a>")?.0))
    }

    #[test]
    fn record_back_link_goes_to_the_class_crumb_or_else_the_project() {
        let lang = make_lang("en");
        let s = share("share-x", "https://example.com/x");
        let with_class = render_breadcrumb(
            cpe_core::url::ROOT.project("p"),
            "P",
            Some(("Things", "/p/things")),
            "R",
            &lang,
            &s,
            None,
        )
        .into_string();
        assert_eq!(back_link(&with_class), Some(("/p/things", "Things")));
        let without =
            render_breadcrumb(cpe_core::url::ROOT.project("p"), "P", None, "R", &lang, &s, None).into_string();
        assert_eq!(back_link(&without), Some(("/p/", "P")));
    }

    #[test]
    fn browse_back_link_goes_to_the_project() {
        let lang = make_lang("en");
        let s = share("share-x", "https://example.com/x");
        let out = render_browse_breadcrumb(cpe_core::url::ROOT.project("p"), "P", "Things", &lang, &s).into_string();
        assert_eq!(back_link(&out), Some(("/p/", "P")));
    }

    #[test]
    fn project_home_back_link_goes_to_the_site_index() {
        let lang = make_lang("en");
        let s = share("share-x", "https://example.com/x");
        let out = render_narrative_breadcrumb(cpe_core::url::ROOT.project("p"), "P", None, None, &lang, &s, None)
            .into_string();
        assert_eq!(back_link(&out), Some(("/", "CPE")));
    }

    #[test]
    fn narrative_page_back_link_goes_to_the_project() {
        let lang = make_lang("en");
        let s = share("share-x", "https://example.com/x");
        let out =
            render_narrative_breadcrumb(cpe_core::url::ROOT.project("p"), "P", Some("Page"), None, &lang, &s, None)
                .into_string();
        assert_eq!(back_link(&out), Some(("/p/", "P")));
    }

    #[test]
    fn narrative_member_back_link_goes_to_the_page() {
        let lang = make_lang("en");
        let s = share("share-x", "https://example.com/x");
        let member = NarrativeMemberSegment { page_href: "/p/page", label: "Chapter · 07" };
        let out = render_narrative_breadcrumb(
            cpe_core::url::ROOT.project("p"),
            "P",
            Some("Page"),
            Some(&member),
            &lang,
            &s,
            None,
        )
        .into_string();
        assert_eq!(back_link(&out), Some(("/p/page", "Page")));
    }

    #[test]
    fn site_index_has_no_back_link() {
        let lang = make_lang("en");
        let out = render_site_index_breadcrumb(&lang, &share("share-x", "https://example.com/x")).into_string();
        assert!(!out.contains("crumb-back"), "{out}");
    }

    /// The unknown-record breadcrumb is not built through the row: no back link, and its trail
    /// directly under the `nav`, which the phone rule hiding a row's trail relies on.
    #[test]
    fn not_found_breadcrumb_has_no_back_link_and_no_row() {
        let lang = make_lang("en");
        let out =
            render_not_found_breadcrumb(cpe_core::url::ROOT.project("p"), "P", Some(("Things", "/p/things")), &lang)
                .into_string();
        assert!(!out.contains("crumb-back"), "{out}");
        assert!(
            out.starts_with("<nav class=\"breadcrumb\" aria-label=\"Breadcrumb\"><ol class=\"breadcrumb__list\">"),
            "{out}"
        );
    }

    /// The back link is the row's first child, and its accessible name is the
    /// hidden prefix, a space, then the visible label: the glyph is hidden
    /// from the name and nothing overrides it with an `aria-label`.
    #[test]
    fn back_link_is_named_by_its_prefix_and_visible_label() {
        for (code, prefix) in [
            ("en", "Back:"),
            ("de", "Zurück:"),
            ("fr", "Retour :"),
            ("it", "Indietro:"),
        ] {
            let lang = make_lang(code);
            let s = share("share-x", "https://example.com/x");
            let out =
                render_browse_breadcrumb(cpe_core::url::ROOT.project("p"), "P", "Things", &lang, &s).into_string();
            let link = format!(
                "<div class=\"breadcrumb__row\"><a class=\"crumb-back\" href=\"/p/\">\
                 <span class=\"visually-hidden\">{prefix}</span> \
                 <span aria-hidden=\"true\">‹</span>\
                 <span class=\"crumb-back__label\">P</span></a>\
                 <ol class=\"breadcrumb__list\">"
            );
            assert!(out.contains(&link), "{code}: {out}");
        }
    }

    // render_browse_breadcrumb

    #[test]
    fn browse_breadcrumb_has_three_crumbs_class_label_current() {
        let lang = make_lang("en");
        let out = render_browse_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            "Characters",
            &lang,
            &share("share-browse-characters", "https://example.com/alice-in-daschland/characters"),
        )
        .into_string();
        assert_eq!(out.matches("breadcrumb__item").count(), 3);
        let current_idx = out.find("breadcrumb__current").expect("breadcrumb__current not found");
        assert!(out[current_idx..].contains("Characters"));
        assert!(out.contains("href=\"/alice-in-daschland/\""));
    }

    #[test]
    fn snapshot_browse_breadcrumb_en() {
        let lang = make_lang("en");
        let out = render_browse_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            "Characters",
            &lang,
            &share("share-browse-characters", "https://example.com/alice-in-daschland/characters"),
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_browse_breadcrumb_de() {
        let lang = make_lang("de");
        let out = render_browse_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            "Figuren",
            &lang,
            &share("share-browse-characters", "https://example.com/alice-in-daschland/characters"),
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    // render_narrative_breadcrumb

    #[test]
    fn narrative_breadcrumb_with_page_title_has_three_crumbs() {
        let lang = make_lang("en");
        let out = render_narrative_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            Some("Wonderland"),
            None,
            &lang,
            &share("share-wonderland", "https://example.com/alice-in-daschland/wonderland"),
            None,
        )
        .into_string();
        assert_eq!(out.matches("breadcrumb__item").count(), 3);
        let current_idx = out.find("breadcrumb__current").expect("breadcrumb__current not found");
        assert!(out[current_idx..].contains("Wonderland"));
    }

    #[test]
    fn narrative_breadcrumb_project_home_has_two_crumbs_project_current() {
        let lang = make_lang("en");
        let out = render_narrative_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            None,
            &lang,
            &share("share-home", "https://example.com/alice-in-daschland/"),
            None,
        )
        .into_string();
        assert_eq!(out.matches("breadcrumb__item").count(), 2);
        let current_idx = out.find("breadcrumb__current").expect("breadcrumb__current not found");
        assert!(out[current_idx..].contains("Alice in DaSCHland"));
        // The project title must NOT also appear as a link when it's the
        // current item.
        assert!(!out.contains("href=\"/alice-in-daschland/\""));
    }

    #[test]
    fn snapshot_narrative_breadcrumb_with_title_en() {
        let lang = make_lang("en");
        let out = render_narrative_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            Some("Wonderland"),
            None,
            &lang,
            &share("share-wonderland", "https://example.com/alice-in-daschland/wonderland"),
            None,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_narrative_breadcrumb_with_title_de() {
        let lang = make_lang("de");
        let out = render_narrative_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            Some("Wunderland"),
            None,
            &lang,
            &share("share-wonderland", "https://example.com/alice-in-daschland/wonderland"),
            None,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_narrative_breadcrumb_project_home_en() {
        let lang = make_lang("en");
        let out = render_narrative_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            None,
            &lang,
            &share("share-home", "https://example.com/alice-in-daschland/"),
            None,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_narrative_breadcrumb_project_home_de() {
        let lang = make_lang("de");
        let out = render_narrative_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            None,
            &lang,
            &share("share-home", "https://example.com/alice-in-daschland/"),
            None,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    // render_narrative_breadcrumb — reader member segment

    #[test]
    fn narrative_breadcrumb_with_member_has_four_crumbs_title_links_member_current() {
        let lang = make_lang("en");
        let member = NarrativeMemberSegment {
            page_href: "/alice-in-daschland/wonderland",
            label: "Chapter · 07",
        };
        let out = render_narrative_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            Some("Wonderland"),
            Some(&member),
            &lang,
            &share(
                "share-wonderland",
                "https://example.com/alice-in-daschland/wonderland?page=SCH_007",
            ),
            None,
        )
        .into_string();
        assert_eq!(
            out.matches("breadcrumb__item").count(),
            4,
            "CPE, project, page-title (now linked), member: four items, got: {out}"
        );
        // The page-title segment is a live link to the page path (no
        // ?member).
        assert!(
            out.contains("href=\"/alice-in-daschland/wonderland\">Wonderland</a>"),
            "page-title must become a link to the bare page path, got: {out}"
        );
        // The member segment is the new current item.
        let current_idx = out.find("breadcrumb__current").expect("breadcrumb__current not found");
        assert!(
            out[current_idx..].contains("Chapter · 07"),
            "member label must be the current item, got: {}",
            &out[current_idx..]
        );
        // "Wonderland" must NOT itself be inside the current (unlinked) item.
        assert!(
            !out[current_idx..].contains("Wonderland"),
            "page title must not also be the current item once a member segment is present, got: {}",
            &out[current_idx..]
        );
    }

    #[test]
    fn snapshot_narrative_breadcrumb_with_member_en() {
        let lang = make_lang("en");
        let member = NarrativeMemberSegment {
            page_href: "/alice-in-daschland/wonderland",
            label: "Chapter · 07",
        };
        let out = render_narrative_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            Some("Wonderland"),
            Some(&member),
            &lang,
            &share(
                "share-wonderland",
                "https://example.com/alice-in-daschland/wonderland?page=SCH_007",
            ),
            None,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_narrative_breadcrumb_with_member_de() {
        let lang = make_lang("de");
        let member = NarrativeMemberSegment {
            page_href: "/alice-in-daschland/wonderland",
            label: "Kapitel · 07",
        };
        let out = render_narrative_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            Some("Wunderland"),
            Some(&member),
            &lang,
            &share(
                "share-wonderland",
                "https://example.com/alice-in-daschland/wonderland?page=SCH_007",
            ),
            None,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    // render_site_index_breadcrumb

    #[test]
    fn site_index_breadcrumb_is_single_current_cpe_crumb() {
        let lang = make_lang("en");
        let out = render_site_index_breadcrumb(&lang, &share("share-site-index", "https://example.com/")).into_string();
        assert_eq!(out.matches("breadcrumb__item").count(), 1);
        assert!(out.contains("breadcrumb__current"));
        assert!(out.contains("aria-current=\"page\""));
        let current_idx = out.find("breadcrumb__current").expect("breadcrumb__current not found");
        assert!(out[current_idx..].contains("CPE"));
    }

    #[test]
    fn snapshot_site_index_breadcrumb_en() {
        let lang = make_lang("en");
        let out = render_site_index_breadcrumb(&lang, &share("share-site-index", "https://example.com/")).into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_site_index_breadcrumb_de() {
        let lang = make_lang("de");
        let out = render_site_index_breadcrumb(&lang, &share("share-site-index", "https://example.com/")).into_string();
        insta::assert_snapshot!(out);
    }

    // Snapshots (existing detail variant)

    #[test]
    fn snapshot_breadcrumb_en() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share(
                "share-alice-in-daschland",
                "https://cpe-pbjdzenira-oa.a.run.app/alice-in-daschland/characters/CH_001",
            ),
            None,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    // Share control — record `?img=` client tracking

    #[test]
    fn share_control_reactive_href_attr_renders_data_attr_when_present() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &ShareControl {
                id: "share-alice-in-daschland",
                href: "https://example.com/alice-in-daschland/characters/CH_001",
                accessible_name_suffix: None,
                reactive_href_attr: Some(
                    "{'href': ($plate + 1) === 1 ? 'https://example.com/x' : 'https://example.com/x?img=' + ($plate + 1)}",
                ),
            }, None,
        )
        .into_string();
        assert!(
            out.contains("data-attr=\"{&#x27;href&#x27;: ($plate + 1) === 1")
                || out.contains("data-attr=\"{'href': ($plate + 1) === 1"),
            "reactive href expression must render as a data-attr directive, got: {out}"
        );
    }

    #[test]
    fn share_control_no_reactive_href_attr_when_none() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert!(
            !out.contains("'href':"),
            "with no plate pager, the control must carry no reactive href data-attr, got: {out}"
        );
    }

    #[test]
    fn snapshot_breadcrumb_record_with_reactive_href_en() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &ShareControl {
                id: "share-alice-in-daschland",
                href: "https://cpe-pbjdzenira-oa.a.run.app/alice-in-daschland/characters/CH_001",
                accessible_name_suffix: None,
                reactive_href_attr: Some(
                    "{'href': ($plate + 1) === 1 ? 'https://cpe-pbjdzenira-oa.a.run.app/alice-in-daschland/characters/CH_001' : 'https://cpe-pbjdzenira-oa.a.run.app/alice-in-daschland/characters/CH_001?img=' + ($plate + 1)}",
                ),
            }, None,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_breadcrumb_de() {
        let lang = make_lang("de");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "Alice",
            &lang,
            &share(
                "share-alice-in-daschland",
                "https://cpe-pbjdzenira-oa.a.run.app/alice-in-daschland/characters/CH_001",
            ),
            None,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    // render_breadcrumb — class crumb (breadcrumb-class-segment task)

    #[test]
    fn breadcrumb_class_crumb_inserted_between_project_and_current() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            Some(("Characters", "/alice-in-daschland/characters")),
            "White Rabbit",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert_eq!(
            out.matches("breadcrumb__item").count(),
            4,
            "a class crumb must yield exactly four items, got: {out}"
        );
        assert!(
            out.contains("href=\"/alice-in-daschland/characters\">Characters</a>"),
            "class crumb must be a live link to the class's browse route, got: {out}"
        );
        let current_idx = out.find("breadcrumb__current").expect("breadcrumb__current not found");
        assert!(
            out[current_idx..].contains("White Rabbit"),
            "resource title must remain the current item, got: {}",
            &out[current_idx..]
        );
        assert!(
            !out[current_idx..].contains("Characters"),
            "class crumb label must not also appear inside the current item, got: {}",
            &out[current_idx..]
        );
    }

    #[test]
    fn breadcrumb_no_class_crumb_when_none() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            "White Rabbit",
            &lang,
            &share("share-detail", "https://example.com/x"),
            None,
        )
        .into_string();
        assert_eq!(
            out.matches("breadcrumb__item").count(),
            3,
            "with no class crumb, the original 3-item trail must be unchanged, got: {out}"
        );
    }

    #[test]
    fn snapshot_breadcrumb_with_class_crumb_en() {
        let lang = make_lang("en");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            Some(("Character", "/alice-in-daschland/characters")),
            "White Rabbit",
            &lang,
            &share(
                "share-alice-in-daschland",
                "https://cpe-pbjdzenira-oa.a.run.app/alice-in-daschland/characters/CH_002",
            ),
            None,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_breadcrumb_with_class_crumb_de() {
        let lang = make_lang("de");
        let out = render_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            Some(("Figur", "/alice-in-daschland/characters")),
            "Weißes Kaninchen",
            &lang,
            &share(
                "share-alice-in-daschland",
                "https://cpe-pbjdzenira-oa.a.run.app/alice-in-daschland/characters/CH_002",
            ),
            None,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    // render_not_found_breadcrumb

    #[test]
    fn not_found_breadcrumb_without_class_crumb_has_three_items() {
        let lang = make_lang("en");
        let out = render_not_found_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            &lang,
        )
        .into_string();
        assert_eq!(
            out.matches("breadcrumb__item").count(),
            3, // CPE, project, current — no class crumb
            "no class crumb must yield exactly three items, got: {out}"
        );
        assert!(
            out.contains("Not found"),
            "current item must read the localized nf.crumb string, got: {out}"
        );
    }

    #[test]
    fn not_found_breadcrumb_with_class_crumb_has_four_items_and_live_link() {
        let lang = make_lang("en");
        let out = render_not_found_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            Some(("Characters", "/alice-in-daschland/characters")),
            &lang,
        )
        .into_string();
        assert_eq!(
            out.matches("breadcrumb__item").count(),
            4,
            "a class crumb must yield exactly four items, got: {out}"
        );
        assert!(
            out.contains("href=\"/alice-in-daschland/characters\""),
            "class crumb must be a live link to the browse route, got: {out}"
        );
        assert!(out.contains("Characters"), "class crumb label must appear, got: {out}");
    }

    #[test]
    fn not_found_breadcrumb_current_item_has_aria_current_and_no_link() {
        let lang = make_lang("en");
        let out = render_not_found_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            &lang,
        )
        .into_string();
        assert!(
            out.contains("aria-current=\"page\""),
            "current item must have aria-current=page, got: {out}"
        );
        let current_idx = out.find("breadcrumb__current").expect("breadcrumb__current not found");
        let after_current = &out[current_idx..];
        assert!(
            after_current.contains("Not found"),
            "not-found label must appear after breadcrumb__current, got: {after_current}"
        );
    }

    #[test]
    fn snapshot_not_found_breadcrumb_with_class_crumb_en() {
        let lang = make_lang("en");
        let out = render_not_found_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            Some(("Characters", "/alice-in-daschland/characters")),
            &lang,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_not_found_breadcrumb_without_class_crumb_en() {
        let lang = make_lang("en");
        let out = render_not_found_breadcrumb(
            cpe_core::url::ROOT.project("alice-in-daschland"),
            "Alice in DaSCHland",
            None,
            &lang,
        )
        .into_string();
        insta::assert_snapshot!(out);
    }
}
