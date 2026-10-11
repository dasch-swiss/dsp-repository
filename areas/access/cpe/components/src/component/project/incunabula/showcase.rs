//! Incunabula's showcase figures (Slice 7): the woodcut crop cases of
//! `/woodcuts` and `/narrenschiff`, and `/narrenschiff`'s edition cards and
//! strip row, built to the Claude Design handoff
//! (`projects/incunabula/design/showcases/design/HANDOFF.md` §1–2).
//!
//! A case is one woodblock's impressions side by side at one height: each
//! crop is cut from its page scan by the annotation's box (IIIF `pct:`, see
//! `iiif::crop_region`) and links to its page in the reader with
//! that annotation addressed. A page case shows two whole pages joined by a
//! page-level link instead, each linking to that link's register entry.
//! Styles are the `SHOWCASE` block of `static/cpe/incunabula.css`; inline
//! `style=` carries only the crop's pixel aspect `--ar`.
//!
//! Every text from the store (titles, alts, link comments, excerpts) and
//! every authored remark is Maud-escaped and never enters a script
//! expression; the image directives are static constants.

use std::borrow::Cow;

use cpe_core::iiif::{
    crop_region, iiif_confined_url, iiif_region_url, iiif_strip_tile_url_clamped, CROP_FETCH_PX, CROP_MARGIN_PCT,
    DETAIL_FETCH_PX, PAGE_FETCH_PX, PLATE_CARD_FETCH_PX,
};
use cpe_core::ir::{
    AnnotationCrop, CaseLayout, ChromeString, Component, CropImage, DataArks, EditionCard, EditionCardsPayload,
    GlossPart, MediaThumb, MemberParent, PagePlate, ShowcaseCase, ShowcaseCrop, ShowcaseRemark, SiteConfig,
    StripRowPayload, WoodcutCasesPayload,
};
use cpe_core::labels::{chrome_text, resolve_class_label, resolve_parts};
use cpe_core::lang::Lang;
use cpe_core::util::is_safe_external_url;
use cpe_core::view_data::ComponentData;
use maud::{html, Markup};

use crate::chrome::{
    case_alt_fallback, case_detail_label, case_impressions_text, case_page_alt, case_pages_text, case_same_block_text,
    case_schramm_text, data_group_head, data_listed_role, strip_tile_name, ChromeMsg,
};
use crate::component::context::RenderContext;
use crate::component::data_entries::{DataEntry, DataGroup};
use crate::component::links::LinkResolver;
use crate::component::values::{composed_part_texts, resolve_title_str};

/// Marks a crop's bed failed when its image fails to load: the CSS then
/// hides the image and shows the label and the alt copy. `el` is the `img`.
pub(crate) const CROP_ERROR_EXPR: &str = "el.closest('.case__bed').classList.add('is-failed')";

/// The same mark for an image that failed before DataStar bound the error
/// handler (a cached failure). A lazy image not fetched yet is not
/// `complete`, so it is never marked.
pub(crate) const CROP_INIT_EXPR: &str =
    "el.complete && !el.naturalWidth && el.closest('.case__bed').classList.add('is-failed')";

/// The registered render function of `woodcut-cases` (row
/// `("incunabula", "woodcut-cases")`). Renders nothing for a slot without
/// its payload, the other data-carrying arms' defensive fallback.
pub(crate) fn render_registered(
    c: &Component,
    ctx: &RenderContext<'_>,
    data: &ComponentData,
    site: &SiteConfig,
) -> Markup {
    let (Component::WoodcutCases { layout, cases, .. }, ComponentData::WoodcutCases(payload)) = (c, data) else {
        return html! {};
    };
    let env = Env {
        links: &LinkResolver::new(ctx.project_id, ctx.link_routes),
        chrome: &site.chrome,
        lang: &ctx.lang,
    };
    render_woodcut_cases(*layout, cases, payload, &env)
}

/// The dataset-panel listing of `woodcut-cases` (its row's `data_listing`,
/// plan D12): one group per case, in render order, listing exactly what the
/// case renders. A crop case lists the Region of every crop and of its detail
/// (not in a compact block, which shows none), headed by its Schramm number,
/// else by its pages; a page case lists its two Pages, headed "Seitenebene"
/// and its pages. Each group head counts its listed entries; an entry without
/// a data ARK is left out, and a case with none lists nothing.
pub(crate) fn woodcut_cases_listing(
    c: &Component,
    data: &ComponentData,
    site: &SiteConfig,
    lang: &Lang,
    data_arks: &DataArks,
) -> Vec<DataGroup> {
    let (Component::WoodcutCases { layout, cases, .. }, ComponentData::WoodcutCases(payload)) = (c, data) else {
        return Vec::new();
    };
    // One listed entry, on the page `label` of the book `parent`.
    let entry = |kind: &str,
                 label: &str,
                 parent_title: Option<&str>,
                 parent: Option<&MemberParent>,
                 class: Option<&str>,
                 ark: &str| DataEntry {
        role: data_listed_role(kind, label),
        title: book_of(parent_title, parent.map(|p| p.id.as_str())).to_owned(),
        title_lang: None,
        number: None,
        class: class.map(str::to_owned),
        ark: ark.to_owned(),
    };
    let region = ChromeMsg::DataRegion.text(lang);
    cases
        .iter()
        .filter_map(|case| {
            let mut pages: Vec<&str> = Vec::new();
            let mut entries = Vec::new();
            let label = match case {
                ShowcaseCase::Crops { schramm, crops, detail, .. } => {
                    for crop in crops.iter().chain(shown_detail(*layout, detail.as_ref())) {
                        let Some(source) = payload.crops.iter().find(|s| s.annotation.id == crop.annotation) else {
                            continue;
                        };
                        let Some(ark) = data_arks.annotation(&source.annotation.id) else {
                            continue;
                        };
                        if !pages.contains(&source.resource_label.as_str()) {
                            pages.push(&source.resource_label);
                        }
                        entries.push(entry(
                            region,
                            &source.resource_label,
                            source.parent_title.as_deref(),
                            source.parent.as_ref(),
                            None,
                            ark,
                        ));
                    }
                    schramm.map(|s| case_schramm_text(s, lang))
                }
                ShowcaseCase::Pages { pages: ids, .. } => {
                    for id in ids {
                        let Some(plate) = payload.pages.iter().find(|p| &p.resource_id == id) else {
                            continue;
                        };
                        let Some(ark) = data_arks.resource(&plate.resource_id) else {
                            continue;
                        };
                        let kind = resolve_class_label(&site.class_labels, &plate.resource_class, lang);
                        pages.push(&plate.resource_label);
                        entries.push(entry(
                            &kind,
                            &plate.resource_label,
                            plate.parent_title.as_deref(),
                            plate.parent.as_ref(),
                            Some(&plate.resource_class),
                            ark,
                        ));
                    }
                    Some(data_listed_role(ChromeMsg::DataPageLevel.text(lang), &pages.join(" / ")))
                }
            };
            (!entries.is_empty()).then(|| DataGroup {
                head: data_group_head(&label.unwrap_or_else(|| pages.join(" / ")), entries.len()),
                entries,
            })
        })
        .collect()
}

/// What every case reads besides its own data: the reader routes, the
/// project's chrome strings (authored remarks) and the page language.
pub(crate) struct Env<'a> {
    pub(crate) links: &'a LinkResolver<'a>,
    pub(crate) chrome: &'a [ChromeString],
    pub(crate) lang: &'a Lang,
}

/// The block: `div.cases.cases--<layout>`, one `figure.case` per case. The
/// layout is a class only: the CSS sets the row heights and the columns.
pub(crate) fn render_woodcut_cases(
    layout: CaseLayout,
    cases: &[ShowcaseCase],
    payload: &WoodcutCasesPayload,
    env: &Env<'_>,
) -> Markup {
    html! {
        div class={ "cases cases--" (layout.as_str()) } {
            @for case in cases {
                @match case {
                    ShowcaseCase::Crops { schramm, title, remark, crops, detail } => {
                        ({
                            render_crop_case(
                                CaseView {
                                    key: case.key(),
                                    schramm: *schramm,
                                    title: title.as_deref(),
                                    remark: remark.as_ref(),
                                    crops,
                                },
                                shown_detail(layout, detail.as_ref()),
                                payload,
                                env,
                            )
                        })
                    }
                    ShowcaseCase::Pages { link, remark, pages } => {
                        (render_page_case(link, remark.as_ref(), pages, payload, env))
                    }
                }
            }
        }
    }
}

/// The detail crop a case shows: none in a compact block (boot-checked too).
/// The figure and the dataset listing both read it, so the listing lists
/// what the case renders.
fn shown_detail(layout: CaseLayout, detail: Option<&ShowcaseCrop>) -> Option<&ShowcaseCrop> {
    detail.filter(|_| layout != CaseLayout::Compact)
}

/// The parts of a `ShowcaseCase::Crops` its figure reads.
struct CaseView<'a> {
    key: &'a str,
    schramm: Option<u32>,
    title: Option<&'a str>,
    remark: Option<&'a ShowcaseRemark>,
    crops: &'a [ShowcaseCrop],
}

/// A case's DOM id: `case-<key>`; its id line is `case-<key>-id`.
pub(crate) fn case_dom_id(key: &str) -> String {
    format!("case-{key}")
}

/// One crop case, HANDOFF §1 with D18's two corrections: the hidden lead
/// follows `div.case__id` (which names the group, so the lead must stay out
/// of it), and the row carries `role="list"` (its `list-style: none` drops
/// list semantics in Safari). The root is `figure[role=group]` and must
/// never gain a `figcaption`: a role on `figure` is allowed only without one.
/// The detail sits under the row, not in it: the row means "same block".
fn render_crop_case(
    case: CaseView<'_>,
    detail: Option<&ShowcaseCrop>,
    payload: &WoodcutCasesPayload,
    env: &Env<'_>,
) -> Markup {
    let id = case_dom_id(case.key);
    let id_line = format!("{id}-id");
    let n = case.crops.len();
    let mut eyebrow: Vec<String> = case.schramm.map(|s| case_schramm_text(s, env.lang)).into_iter().collect();
    eyebrow.push(case_impressions_text(n, env.lang));
    let source = |annotation: &str| payload.crops.iter().find(|c| c.annotation.id == annotation);
    html! {
        figure.case role="group" id=(id) aria-labelledby=(id_line) {
            div.case__id id=(id_line) {
                p.eyebrow { (eyebrow.join(" · ")) }
                @if let Some(title) = case.title {
                    h3.case__title lang="de" { (title) }
                }
            }
            span.visually-hidden { (case_same_block_text(n, env.lang)) }
            ol.case__row role="list" {
                @for crop in case.crops {
                    @if let Some(source) = source(&crop.annotation) {
                        (render_crop(crop, source, env))
                    }
                }
            }
            @if let Some((detail, source)) = detail.and_then(|d| Some((d, source(&d.annotation)?))) {
                (render_detail(detail, source, env))
            }
            (render_remark(case.remark, payload, env))
        }
    }
}

/// One page case: two whole pages joined by the page-level link `link`, the
/// link's name as the title in „…“, and the page-level note.
fn render_page_case(
    link: &str,
    remark: Option<&ShowcaseRemark>,
    pages: &[String],
    payload: &WoodcutCasesPayload,
    env: &Env<'_>,
) -> Markup {
    let id = case_dom_id(link);
    let id_line = format!("{id}-id");
    let name = payload.links.iter().find(|l| l.id == link).map(|l| &l.name);
    html! {
        figure.case.case--page role="group" id=(id) aria-labelledby=(id_line) {
            div.case__id id=(id_line) {
                p.eyebrow { (case_pages_text(pages.len(), env.lang)) }
                @if let Some(name) = name {
                    h3.case__title lang="de" { "„" (name) "“" }
                }
            }
            span.visually-hidden { (ChromeMsg::CaseSamePages.text(env.lang)) }
            ol.case__row role="list" {
                @for page in pages {
                    @if let Some(plate) = payload.pages.iter().find(|p| &p.resource_id == page) {
                        (render_page_plate(plate, link, env))
                    }
                }
            }
            (render_remark(remark, payload, env))
            p.case__note { (ChromeMsg::CasePageNote.text(env.lang)) }
        }
    }
}

/// A case's remark, or nothing: a link's own comment or a description
/// excerpt is data, quoted and German in both languages; an authored chrome
/// string is unquoted, in the page language.
fn render_remark(remark: Option<&ShowcaseRemark>, payload: &WoodcutCasesPayload, env: &Env<'_>) -> Markup {
    let quoted = |text: &str| {
        html! {
            p.case__remark lang="de" { "„" (text) "“" }
        }
    };
    match remark {
        Some(ShowcaseRemark::Link(link)) => payload
            .links
            .iter()
            .find(|l| &l.id == link)
            .and_then(|l| l.comment.as_deref())
            .map(quoted)
            .unwrap_or_default(),
        Some(ShowcaseRemark::Excerpt(text)) => quoted(text),
        Some(ShowcaseRemark::Chrome(id)) => chrome_text(env.chrome, id, env.lang)
            .map(|text| {
                html! {
                    p.case__remark { (text) }
                }
            })
            .unwrap_or_default(),
        None => html! {},
    }
}

/// The book a crop or plate names in its caption: its display title, or its
/// id without one.
fn book_of<'a>(parent_title: Option<&'a str>, parent_id: Option<&'a str>) -> &'a str {
    parent_title.or(parent_id).unwrap_or_default()
}

/// The image's stored size, when it is safe to request and both sides are
/// known and positive.
fn sized(image: Option<&CropImage>) -> Option<(&CropImage, i64, i64)> {
    image
        .filter(|image| is_safe_external_url(&image.uri))
        .and_then(|image| Some((image, image.width?, image.height?)))
        .filter(|&(_, w, h)| w > 0 && h > 0)
}

/// What a crop (in the row, or the detail) shows and links to, with the
/// image fitted into `max_px`. An unsafe or unsized image has no `src` (its
/// bed renders failed) and an aspect of 1.
struct CropParts<'a> {
    book: &'a str,
    page: &'a str,
    alt: Cow<'a, str>,
    alt_lang: &'a str,
    src: Option<String>,
    ar: f64,
    href: Option<String>,
}

fn crop_parts<'a>(crop: &'a ShowcaseCrop, source: &'a AnnotationCrop, max_px: i64, env: &'a Env<'_>) -> CropParts<'a> {
    let a = &source.annotation;
    let book = book_of(source.parent_title.as_deref(), source.parent.as_ref().map(|p| p.id.as_str()));
    let page = source.resource_label.as_str();
    let (alt, alt_lang) = match &crop.alt {
        Some(alt) => (Cow::Borrowed(alt.as_str()), "de"),
        None => (Cow::Owned(case_alt_fallback(book, page, env.lang)), env.lang.as_str()),
    };
    let region = crop_region(a.x, a.y, a.w, a.h, CROP_MARGIN_PCT);
    let sized = sized(source.image.as_ref());
    let src = sized.and_then(|(image, w, h)| iiif_region_url(&image.uri, region, w, h, max_px));
    let ar = sized.and_then(|(_, w, h)| region.pixel_aspect(w, h)).unwrap_or(1.0);
    let href = source
        .parent
        .as_ref()
        .and_then(|p| env.links.reader_entry_href(&p.id, &p.class, &a.resource_id, &a.id));
    CropParts { book, page, alt, alt_lang, src, ar, href }
}

/// One crop: a link around the bed and the caption (book title, page label,
/// arrow). The link's accessible name reads alt, book, page and the hidden
/// "— open the page" (D18); a crop with no link drops that phrase. An image
/// URI that fails `is_safe_external_url` renders the failed bed.
fn render_crop(crop: &ShowcaseCrop, source: &AnnotationCrop, env: &Env<'_>) -> Markup {
    let parts = crop_parts(crop, source, CROP_FETCH_PX, env);
    render_item(
        Item {
            bed: render_bed(parts.src.as_deref(), &parts.alt, parts.alt_lang, env.lang),
            href: parts.href,
            ar: parts.ar,
            whole_page: false,
            book: parts.book,
            page: parts.page,
        },
        env.lang,
    )
}

/// One whole page of a page case, linking to the page-level link's register
/// entry on that page (`#annotation-<link>`). Its alt is always composed: a
/// page has no single description line that names it whole.
fn render_page_plate(plate: &PagePlate, link: &str, env: &Env<'_>) -> Markup {
    let book = book_of(plate.parent_title.as_deref(), plate.parent.as_ref().map(|p| p.id.as_str()));
    let page = plate.resource_label.as_str();
    let alt = case_page_alt(book, page, env.lang);
    let sized = sized(plate.image.as_ref());
    let src = sized.and_then(|(image, w, h)| iiif_confined_url(&image.uri, w, h, PAGE_FETCH_PX));
    // Image sizes are far below 2^52, so both are exact f64s.
    let ar = sized.map_or(1.0, |(_, w, h)| w as f64 / h as f64);
    let href = plate
        .parent
        .as_ref()
        .and_then(|p| env.links.reader_entry_href(&p.id, &p.class, &plate.resource_id, link));
    render_item(
        Item {
            href,
            ar,
            bed: render_bed(src.as_deref(), &alt, env.lang.as_str(), env.lang),
            whole_page: true,
            book,
            page,
        },
        env.lang,
    )
}

/// The parts of one row item, crop or whole page.
struct Item<'a> {
    href: Option<String>,
    ar: f64,
    bed: Markup,
    /// A whole page carries the "Ganze Seite" eyebrow above its caption.
    whole_page: bool,
    book: &'a str,
    page: &'a str,
}

/// One row item: `li.case__crop[--ar] > a.case__link` (a `span` without a
/// route) around the bed, an optional eyebrow and the caption. The whole
/// page's eyebrow is hidden from the link's name: its composed alt already
/// opens with the same words.
fn render_item(item: Item<'_>, lang: &Lang) -> Markup {
    let linked = item.href.is_some();
    let inner = html! {
        (item.bed)
        @if item.whole_page {
            span.eyebrow.case__whole aria-hidden="true" { (ChromeMsg::CaseWholePage.text(lang)) }
        }
        span.case__cap {
            em { (item.book) }
            span.case__page {
                span {
                    span.visually-hidden { "· " }
                    (item.page)
                    @if linked {
                        span.visually-hidden { " " (ChromeMsg::CaseOpenPage.text(lang)) }
                    }
                }
                span.case__arrow aria-hidden="true" { "→" }
            }
        }
    };
    html! {
        li.case__crop style=(format!("--ar: {:.4}", item.ar)) {
            @if let Some(href) = item.href {
                a.case__link href=(href) { (inner) }
            } @else {
                span.case__link { (inner) }
            }
        }
    }
}

/// A bed: the image (absent when there is no URL, so the bed renders
/// failed), the failed-state label and a copy of the alt. Exactly one copy of
/// the alt is in the accessibility tree in each state (D10).
fn render_bed(src: Option<&str>, alt: &str, alt_lang: &str, lang: &Lang) -> Markup {
    html! {
        span class=(if src.is_some() { "case__bed" } else { "case__bed is-failed" }) {
            @if let Some(src) = src {
                img loading="lazy"
                    src=(src)
                    alt=(alt)
                    lang=(alt_lang)
                    data-on:error=(CROP_ERROR_EXPR)
                    data-init=(CROP_INIT_EXPR);
            }
            span.absent__label aria-hidden="true" { (ChromeMsg::CaseCropNotLoaded.text(lang)) }
            span.case__alt lang=(alt_lang) { (alt) }
        }
    }
}

/// The detail crop (HANDOFF §1.5): its own link under the row, a small bed
/// beside an eyebrow ("Detail · b3v") and the page label. The link's name
/// reads alt, eyebrow, book and the hidden "— open the page"; the visible
/// page label is hidden from it, since the eyebrow already names the page.
fn render_detail(detail: &ShowcaseCrop, source: &AnnotationCrop, env: &Env<'_>) -> Markup {
    let parts = crop_parts(detail, source, DETAIL_FETCH_PX, env);
    let style = format!("--ar: {:.4}", parts.ar);
    let inner = html! {
        (render_bed(parts.src.as_deref(), &parts.alt, parts.alt_lang, env.lang))
        span.case__detail-text {
            span.eyebrow { (case_detail_label(parts.page, env.lang)) }
            span.case__page {
                span {
                    @if parts.href.is_some() {
                        span.visually-hidden {
                            "· "
                            (parts.book)
                            " "
                            (ChromeMsg::CaseOpenPage.text(env.lang))
                        }
                    }
                    span aria-hidden="true" { (parts.page) }
                }
                span.case__arrow aria-hidden="true" { "→" }
            }
        }
    };
    html! {
        @if let Some(href) = &parts.href {
            a.case__detail href=(href) style=(style) { (inner) }
        } @else {
            span.case__detail style=(style) { (inner) }
        }
    }
}

// ── Edition cards and the strip row (HANDOFF §2, `/narrenschiff`) ────────────

/// The registered render function of `edition-cards` (row
/// `("incunabula", "edition-cards")`). Renders nothing for a slot without
/// its payload.
pub(crate) fn render_edition_cards_registered(
    c: &Component,
    ctx: &RenderContext<'_>,
    data: &ComponentData,
    site: &SiteConfig,
) -> Markup {
    let (Component::EditionCards { books, imprint, .. }, ComponentData::EditionCards(payload)) = (c, data) else {
        return html! {};
    };
    let links = LinkResolver::new(ctx.project_id, ctx.link_routes);
    render_edition_cards(books, imprint, payload, &links, site, &ctx.lang)
}

/// The cards: `ul.editions`, one `li.edition` per configured book that
/// exists, in config order. A card is one link to the book's reader around
/// its cover plate (decorative, `alt=""`: the title names the link), the
/// language eyebrow, the title, the imprint (the record header's, so the
/// line matches the book page) and "Lesen →".
pub(crate) fn render_edition_cards(
    cards: &[EditionCard],
    imprint: &[GlossPart],
    payload: &EditionCardsPayload,
    links: &LinkResolver<'_>,
    site: &SiteConfig,
    lang: &Lang,
) -> Markup {
    let parts = resolve_parts(site, imprint, lang);
    html! {
        ul.editions role="list" {
            @for card in cards {
                @if let Some(book) = payload.books.iter().find(|b| b.resource.id == card.book) {
                    @let resource = &book.resource;
                    @let imprint_line = composed_part_texts(
                            resource,
                            &parts,
                            site.date_precision_word,
                            lang,
                        )
                        .join(" · ");
                    @let src = sized(book.cover.as_ref())
                        .and_then(|(image, w, h)| {
                            iiif_confined_url(&image.uri, w, h, PLATE_CARD_FETCH_PX)
                        });
                    @let href = links.href(&resource.id, Some(&resource.class));
                    @let eyebrow = card
                        .lang_label
                        .as_deref()
                        .and_then(|id| chrome_text(&site.chrome, id, lang));
                    @let title = resolve_title_str(resource, Some("name"), lang);
                    @let inner = edition_card_inner(
                        src.as_deref(),
                        eyebrow,
                        title,
                        &imprint_line,
                        href.is_some(),
                        lang,
                    );
                    li.edition {
                        @if let Some(href) = href {
                            a.edition__link href=(href) { (inner) }
                        } @else {
                            div.edition__link { (inner) }
                        }
                    }
                }
            }
        }
    }
}

/// What an edition card holds, the same inside its link and inside the `div`
/// of a card without one.
fn edition_card_inner(
    src: Option<&str>,
    eyebrow: Option<&str>,
    title: &str,
    imprint_line: &str,
    linked: bool,
    lang: &Lang,
) -> Markup {
    html! {
        span.edition__plate {
            @if let Some(src) = src {
                img loading="lazy" src=(src) alt="";
            }
        }
        span.edition__text {
            @if let Some(label) = eyebrow {
                span.eyebrow { (label) }
            }
            span.edition__title { (title) }
            @if !imprint_line.is_empty() {
                span.edition__imprint { (imprint_line) }
            }
            // "Read" is a call to follow the link: a card
            // without one does not show it.
            @if linked {
                span.edition__read {
                    (ChromeMsg::EditionRead.text(lang))
                    span aria-hidden="true" { " →" }
                }
            }
        }
    }
}

/// The registered render function of `strip-row` (row
/// `("incunabula", "strip-row")`). Renders nothing for a slot without its
/// payload.
pub(crate) fn render_strip_row_registered(
    c: &Component,
    ctx: &RenderContext<'_>,
    data: &ComponentData,
    _site: &SiteConfig,
) -> Markup {
    let (Component::StripRow { .. }, ComponentData::StripRow(payload)) = (c, data) else {
        return html! {};
    };
    let links = LinkResolver::new(ctx.project_id, ctx.link_routes);
    render_strip_row(payload, &links, &ctx.lang)
}

/// The strip row's `{n}`: every resource of its class (the "Alle 38
/// Randleisten" more link), not the listed few.
pub(crate) fn strip_row_slot_n(data: &ComponentData) -> Option<usize> {
    match data {
        ComponentData::StripRow(payload) => Some(payload.class_total),
        _ => None,
    }
}

/// The row: `ul.strips`, one tile per strip in payload order, each a link to
/// the strip's page around its image (decorative, `alt=""`) and its mono id.
/// The link's name is "Randleiste {id}", which holds the visible id.
pub(crate) fn render_strip_row(payload: &StripRowPayload, links: &LinkResolver<'_>, lang: &Lang) -> Markup {
    html! {
        ul.strips role="list" {
            @for strip in &payload.strips {
                @let id = resolve_title_str(strip, Some("name"), lang);
                @let thumb = payload
                    .thumbs
                    .get(&strip.id)
                    .filter(|t| is_safe_external_url(&t.uri));
                @let inner = strip_tile_inner(thumb, id);
                li {
                    @if let Some(href) = links.href(&strip.id, Some(&strip.class)) {
                        a.strip href=(href) aria-label=(strip_tile_name(id, lang)) { (inner) }
                    } @else {
                        div.strip { (inner) }
                    }
                }
            }
        }
    }
}

/// What a strip tile holds, the same inside its link and inside the `div` of
/// a tile without one.
fn strip_tile_inner(thumb: Option<&MediaThumb>, id: &str) -> Markup {
    html! {
        span.strip__bed {
            @if let Some(thumb) = thumb {
                img loading="lazy"
                    src=(iiif_strip_tile_url_clamped(&thumb.uri, thumb.width).as_ref())
                    alt="";
            }
        }
        span.strip__id { (id) }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cpe_core::ir::{Annotation, AnnotationLink, AnnotationShape};

    use super::*;

    fn lang(code: &str) -> Lang {
        cpe_core::lang::resolve_builtin(&[code.to_owned()])
    }

    fn crop_source(id: &str, page: &str, book: &str, title: &str) -> AnnotationCrop {
        AnnotationCrop {
            annotation: Annotation {
                id: id.to_owned(),
                resource_id: format!("{book}-{page}"),
                shape: AnnotationShape::Rectangle,
                x: 0.18505338078291814,
                y: 0.16203703703703703,
                w: 0.6334519572953737,
                h: 0.5578703703703705,
                points: Vec::new(),
                color: Some("#ff3333".to_owned()),
                remark: None,
                remark_lang: None,
            },
            resource_class: "Page".to_owned(),
            resource_label: page.to_owned(),
            parent: Some(MemberParent { id: book.to_owned(), class: "Book".to_owned(), seq: 3 }),
            parent_title: Some(title.to_owned()),
            image: Some(CropImage {
                uri: format!("https://iiif.dasch.swiss/0803/{page}.jp2/full/max/0/default.jpg"),
                width: Some(3322),
                height: Some(5113),
            }),
        }
    }

    /// E1 as configured: Schramm 1124, its title, the link comment, the
    /// first alt from the data and the second composed.
    fn e1() -> (Vec<ShowcaseCase>, WoodcutCasesPayload) {
        let cases = vec![ShowcaseCase::Crops {
            schramm: Some(1124),
            title: Some("Kap. 13 · Von der Buhlerei".to_owned()),
            remark: Some(ShowcaseRemark::Link("jwSBbhl4U_6dIifcGTn0Qg".to_owned())),
            crops: vec![
                ShowcaseCrop {
                    annotation: "bsX4cDKkW3GMC8_Ms2tOpw".to_owned(),
                    alt: Some("Holzschnitt zu Kap. 13: Von der Buhlerei.".to_owned()),
                },
                ShowcaseCrop { annotation: "gAfAiG_-U4itgN8Gdr9Ipw".to_owned(), alt: None },
            ],
            detail: None,
        }];
        let mut second = crop_source(
            "gAfAiG_-U4itgN8Gdr9Ipw",
            "c8r",
            "narrenschiff-lat-august",
            "[Das] Narrenschiff (lat.) [Aug. 1497]",
        );
        second.annotation.x = 0.08754208754208755;
        second.annotation.w = 0.5420875420875421;
        second.image = Some(CropImage {
            uri: "https://iiif.dasch.swiss/0803/c8r.jp2/full/max/0/default.jpg".to_owned(),
            width: Some(2000),
            height: Some(3000),
        });
        let payload = WoodcutCasesPayload {
            crops: vec![
                crop_source("bsX4cDKkW3GMC8_Ms2tOpw", "c1v", "narrenschiff-dt", "[Das] Narrenschiff (dt.)"),
                second,
            ],
            links: vec![AnnotationLink {
                id: "jwSBbhl4U_6dIifcGTn0Qg".to_owned(),
                name: "identischer Holzschnitt".to_owned(),
                name_lang: Some("de".to_owned()),
                comment: Some(
                    "Der Holzschnitt wird auf Seite c1v der deutschen Ausgabe des Narrenschiffs \
                     und c8r der lateinischen Ausgabe des Narrenschiffs verwendet."
                        .to_owned(),
                ),
                comment_lang: Some("de".to_owned()),
            }],
            pages: Vec::new(),
        };
        (cases, payload)
    }

    fn routes() -> HashMap<String, String> {
        HashMap::from([("Book".to_owned(), "/books/:id".to_owned())])
    }

    fn render(cases: &[ShowcaseCase], payload: &WoodcutCasesPayload, code: &str) -> String {
        render_as(CaseLayout::Edition, cases, payload, code)
    }

    fn render_as(layout: CaseLayout, cases: &[ShowcaseCase], payload: &WoodcutCasesPayload, code: &str) -> String {
        let routes = routes();
        let links = LinkResolver::new("incunabula", &routes);
        let chrome = chrome();
        let lang = lang(code);
        let env = Env { links: &links, chrome: &chrome, lang: &lang };
        render_woodcut_cases(layout, cases, payload, &env).into_string()
    }

    /// The project's authored remark of E2 (design draft).
    fn chrome() -> Vec<ChromeString> {
        vec![ChromeString {
            id: "showcase-remark-e2".to_owned(),
            translations: vec![
                (
                    "de".to_owned(),
                    "Ein Stock in allen drei Ausgaben: zwei Verknüpfungen führen von c3v der \
                     deutschen Ausgabe auf d1r beider lateinischer."
                        .to_owned(),
                ),
                (
                    "en".to_owned(),
                    "One block in all three editions: two links lead from c3v of the German \
                     edition to d1r of both Latin ones."
                        .to_owned(),
                ),
            ],
        }]
    }

    fn crop(annotation: &str, alt: Option<&str>) -> ShowcaseCrop {
        ShowcaseCrop {
            annotation: annotation.to_owned(),
            alt: alt.map(str::to_owned),
        }
    }

    fn link(id: &str, name: &str, comment: Option<&str>) -> AnnotationLink {
        AnnotationLink {
            id: id.to_owned(),
            name: name.to_owned(),
            name_lang: Some("de".to_owned()),
            comment: comment.map(str::to_owned),
            comment_lang: Some("de".to_owned()),
        }
    }

    /// E2 as configured: three editions of Kap. 14, an authored remark.
    fn e2() -> (Vec<ShowcaseCase>, WoodcutCasesPayload) {
        let alt = "Holzschnitt zu Kap. 14: Von falschen Vorstellungen über Gott.";
        let cases = vec![ShowcaseCase::Crops {
            schramm: Some(1125),
            title: Some("Kap. 14 · Von falschen Vorstellungen über Gott".to_owned()),
            remark: Some(ShowcaseRemark::Chrome("showcase-remark-e2".to_owned())),
            crops: vec![
                crop("2IqZQvJ8VjyKQUN16phyfg", Some(alt)),
                crop("xXOFHTwyUHeFuDoXLQk9Vw", Some(alt)),
                crop("WpNfSfnWUhmTwLQvByNxZQ", Some(alt)),
            ],
            detail: None,
        }];
        let payload = WoodcutCasesPayload {
            crops: vec![
                crop_source("2IqZQvJ8VjyKQUN16phyfg", "c3v", "narrenschiff-dt", "[Das] Narrenschiff (dt.)"),
                crop_source(
                    "xXOFHTwyUHeFuDoXLQk9Vw",
                    "d1r",
                    "narrenschiff-lat-maerz",
                    "[Das] Narrenschiff (lat.) [März 1497]",
                ),
                crop_source(
                    "WpNfSfnWUhmTwLQvByNxZQ",
                    "d1r",
                    "narrenschiff-lat-august",
                    "[Das] Narrenschiff (lat.) [Aug. 1497]",
                ),
            ],
            links: Vec::new(),
            pages: Vec::new(),
        };
        (cases, payload)
    }

    /// F4: four impressions in one row, the link comment with its double
    /// space.
    fn f4() -> (Vec<ShowcaseCase>, WoodcutCasesPayload) {
        let comment = "Der Holzschnitt wird viermal verwendet: In der lateinischen Fassung des \
                       Narrenschiffs auf den Seiten  c3r und m5r und in der deutschen Fassung des \
                       Narrenschiffs auf den Seiten b4v und o7v.";
        let ids = [
            ("Uimrq1KxV3uL8HCjEll9HA", "b4v", "narrenschiff-dt", "[Das] Narrenschiff (dt.)"),
            ("LPw7a0ubUDKfjSWAc4-ECg", "o7v", "narrenschiff-dt", "[Das] Narrenschiff (dt.)"),
            (
                "x3NCLnuhVVW4WEDYFzCxhQ",
                "c3r",
                "narrenschiff-lat-august",
                "[Das] Narrenschiff (lat.) [Aug. 1497]",
            ),
            (
                "OWvgOgSkU3WzmRHxXWJuCQ",
                "m5r",
                "narrenschiff-lat-august",
                "[Das] Narrenschiff (lat.) [Aug. 1497]",
            ),
        ];
        let cases = vec![ShowcaseCase::Crops {
            schramm: Some(1119),
            title: Some("Kap. 8 · Guten Rat verschmähen".to_owned()),
            remark: Some(ShowcaseRemark::Link("YLToxLUYXx62qcjRXHArNg".to_owned())),
            crops: ids.iter().map(|(id, ..)| crop(id, None)).collect(),
            detail: None,
        }];
        let payload = WoodcutCasesPayload {
            crops: ids
                .iter()
                .map(|(id, page, book, title)| crop_source(id, page, book, title))
                .collect(),
            links: vec![link("YLToxLUYXx62qcjRXHArNg", "identischer Holzschnitt", Some(comment))],
            pages: Vec::new(),
        };
        (cases, payload)
    }

    /// B1 and B2, a compact pair of cases inside one book; B2's second page
    /// has no woodcut line, so its alt is composed.
    fn compact_pair() -> (Vec<ShowcaseCase>, WoodcutCasesPayload) {
        let case = |schramm, link: &str, a: ShowcaseCrop, b: ShowcaseCrop| ShowcaseCase::Crops {
            schramm: Some(schramm),
            title: None,
            remark: Some(ShowcaseRemark::Link(link.to_owned())),
            crops: vec![a, b],
            detail: None,
        };
        let postilla = "Postilla super epistolas et evangelia";
        let bereitung = "Bereitung zu dem Heiligen Sakrament";
        let cases = vec![
            case(
                356,
                "00bnHlmDVIq_Blb4DvKGiQ",
                crop("ik1KkbHvUjSBN5mBGn3Vuw", Some("Jesus predigt den Jüngern")),
                crop("lp0ibqQ0VPiOmO2GPauECw", Some("Jesus predigt den Jüngern")),
            ),
            case(
                458,
                "fUyzGd_SUSqZkqdkeukZYw",
                crop("xhvat7JpVLOR7uRfjZmmcA", Some("Jesus vor Pilatus")),
                crop("cRDtdyM2V4y2Jivpcspcmg", None),
            ),
        ];
        let payload = WoodcutCasesPayload {
            crops: vec![
                crop_source("ik1KkbHvUjSBN5mBGn3Vuw", "e3r", "postilla", postilla),
                crop_source("lp0ibqQ0VPiOmO2GPauECw", "e4r", "postilla", postilla),
                crop_source("xhvat7JpVLOR7uRfjZmmcA", "p7r", "bereitung", bereitung),
                crop_source("cRDtdyM2V4y2Jivpcspcmg", "q4v", "bereitung", bereitung),
            ],
            links: vec![
                link(
                    "00bnHlmDVIq_Blb4DvKGiQ",
                    "identischer Holzschnitt",
                    Some("Derselbe Holzschnitt wird auf den Seiten e3r und e4r verwendet."),
                ),
                link(
                    "fUyzGd_SUSqZkqdkeukZYw",
                    "identischer Holzschnitt",
                    Some("Der Holzschnitt wird auf den Seiten p7r und q4v verwendet."),
                ),
            ],
            pages: Vec::new(),
        };
        (cases, payload)
    }

    fn plate(page: &str, book: &str, title: &str, w: i64, h: i64) -> PagePlate {
        PagePlate {
            resource_id: format!("{book}-{page}"),
            resource_class: "Page".to_owned(),
            resource_label: page.to_owned(),
            parent: Some(MemberParent { id: book.to_owned(), class: "Book".to_owned(), seq: 7 }),
            parent_title: Some(title.to_owned()),
            image: Some(CropImage {
                uri: format!("https://iiif.dasch.swiss/0803/{page}.jp2/full/max/0/default.jpg"),
                width: Some(w),
                height: Some(h),
            }),
        }
    }

    /// P3: the printer's mark on the last page of both Latin editions, the
    /// page-level link's own comment as the remark.
    fn p3() -> (Vec<ShowcaseCase>, WoodcutCasesPayload) {
        let link_id = "_j3T1ZACWzGNux_T6tlnCA";
        let cases = vec![ShowcaseCase::Pages {
            link: link_id.to_owned(),
            remark: Some(ShowcaseRemark::Link(link_id.to_owned())),
            pages: vec![
                "narrenschiff-lat-maerz-t1v".to_owned(),
                "narrenschiff-lat-august-x4v".to_owned(),
            ],
        }];
        let payload = WoodcutCasesPayload {
            crops: Vec::new(),
            links: vec![link(
                link_id,
                "gleiche Druckermarke",
                Some(
                    "gleiche Druckermarke: Johann Bergmann von Olpe (Schramm, Bd. 22, Abb. \
                     1270)",
                ),
            )],
            pages: vec![
                plate(
                    "x4v",
                    "narrenschiff-lat-august",
                    "[Das] Narrenschiff (lat.) [Aug. 1497]",
                    3646,
                    5202,
                ),
                plate(
                    "t1v",
                    "narrenschiff-lat-maerz",
                    "[Das] Narrenschiff (lat.) [März 1497]",
                    1976,
                    2593,
                ),
            ],
        };
        (cases, payload)
    }

    /// E5: two impressions and the detail under the row that backs the
    /// remark.
    fn e5() -> (Vec<ShowcaseCase>, WoodcutCasesPayload) {
        let remark = "Weiedervwendung des selben Holzschnittes. In der deutschen Ausgabe ist in \
                      der Illustration noch eine handschriftliche Bemerkung";
        let cases = vec![ShowcaseCase::Crops {
            schramm: Some(1118),
            title: None,
            remark: Some(ShowcaseRemark::Link("ggDWsmYKUd2RWy3w9PcD1w".to_owned())),
            crops: vec![
                crop("EDqDFUAsXtSuBreIXyuCiQ", None),
                crop("MJBQMyqjUEe8FjFwRUAL4Q", Some("Holzschnitt zu Kap. 7: Vom Zwietracht stiften.")),
            ],
            detail: Some(crop(
                "XTtkp_hEUDWO828r2pb2PA",
                Some("Handschriftliche Bemerkung in der Zeichnung"),
            )),
        }];
        let mut detail = crop_source("XTtkp_hEUDWO828r2pb2PA", "b3v", "narrenschiff-dt", "[Das] Narrenschiff (dt.)");
        detail.annotation.x = 0.21958121109224674;
        detail.annotation.y = 0.5930059523809523;
        detail.annotation.w = 0.3418222976796831;
        detail.annotation.h = 0.09114583333333337;
        let payload = WoodcutCasesPayload {
            crops: vec![
                crop_source("EDqDFUAsXtSuBreIXyuCiQ", "b3v", "narrenschiff-dt", "[Das] Narrenschiff (dt.)"),
                crop_source(
                    "MJBQMyqjUEe8FjFwRUAL4Q",
                    "c2r",
                    "narrenschiff-lat-maerz",
                    "[Das] Narrenschiff (lat.) [März 1497]",
                ),
                detail,
            ],
            links: vec![link("ggDWsmYKUd2RWy3w9PcD1w", "identischer Holzschnitt", Some(remark))],
            pages: Vec::new(),
        };
        (cases, payload)
    }

    #[test]
    fn snapshot_woodcut_case_three_editions_de() {
        let (cases, payload) = e2();
        insta::assert_snapshot!(render(&cases, &payload, "de"));
    }

    #[test]
    fn snapshot_woodcut_case_three_editions_en() {
        let (cases, payload) = e2();
        insta::assert_snapshot!(render(&cases, &payload, "en"));
    }

    #[test]
    fn snapshot_woodcut_case_four_crops_de() {
        let (cases, payload) = f4();
        insta::assert_snapshot!(render_as(CaseLayout::Four, &cases, &payload, "de"));
    }

    #[test]
    fn snapshot_woodcut_case_compact_pair_de() {
        let (cases, payload) = compact_pair();
        insta::assert_snapshot!(render_as(CaseLayout::Compact, &cases, &payload, "de"));
    }

    #[test]
    fn snapshot_woodcut_case_page_level_de() {
        let (cases, payload) = p3();
        insta::assert_snapshot!(render_as(CaseLayout::Compact, &cases, &payload, "de"));
    }

    #[test]
    fn snapshot_woodcut_case_page_level_en() {
        let (cases, payload) = p3();
        insta::assert_snapshot!(render_as(CaseLayout::Compact, &cases, &payload, "en"));
    }

    #[test]
    fn snapshot_woodcut_case_with_detail_de() {
        let (cases, payload) = e5();
        insta::assert_snapshot!(render(&cases, &payload, "de"));
    }

    #[test]
    fn snapshot_woodcut_case_with_detail_en() {
        let (cases, payload) = e5();
        insta::assert_snapshot!(render(&cases, &payload, "en"));
    }

    #[test]
    fn the_layout_is_the_block_class_only() {
        let (cases, payload) = f4();
        let four = render_as(CaseLayout::Four, &cases, &payload, "de");
        let edition = render_as(CaseLayout::Edition, &cases, &payload, "de");
        assert!(four.starts_with("<div class=\"cases cases--four\">"), "{four}");
        assert_eq!(
            four.replacen("cases--four", "cases--edition", 1),
            edition,
            "the markup inside the block does not depend on the layout"
        );
    }

    #[test]
    fn an_authored_remark_is_unquoted_in_the_page_language() {
        let (cases, payload) = e2();
        let en = render(&cases, &payload, "en");
        assert!(
            en.contains("<p class=\"case__remark\">One block in all three editions: two links lead"),
            "{en}"
        );
        assert!(!en.contains("„One block"), "{en}");
    }

    #[test]
    fn an_unknown_authored_remark_renders_no_paragraph() {
        let (mut cases, payload) = e2();
        if let ShowcaseCase::Crops { remark, .. } = &mut cases[0] {
            *remark = Some(ShowcaseRemark::Chrome("nope".to_owned()));
        }
        assert!(!render(&cases, &payload, "de").contains("case__remark"));
    }

    #[test]
    fn an_excerpt_remark_is_quoted_german_data() {
        let (mut cases, payload) = p3();
        if let ShowcaseCase::Pages { remark, .. } = &mut cases[0] {
            *remark = Some(ShowcaseRemark::Excerpt("Holzschnitt: Druckermarke.".to_owned()));
        }
        let en = render_as(CaseLayout::Compact, &cases, &payload, "en");
        assert!(
            en.contains("<p class=\"case__remark\" lang=\"de\">„Holzschnitt: Druckermarke.“</p>"),
            "{en}"
        );
    }

    #[test]
    fn a_page_case_shows_whole_pages_linked_to_the_link_entry() {
        let (cases, payload) = p3();
        let out = render_as(CaseLayout::Compact, &cases, &payload, "de");
        assert!(
            out.contains(
                "href=\"/incunabula/books/narrenschiff-lat-maerz?page=narrenschiff-lat-maerz-t1v\
                 #annotation-_j3T1ZACWzGNux_T6tlnCA\""
            ),
            "{out}"
        );
        // The whole scan, fitted into the 480 px box (t1v is 1976 × 2593).
        assert!(
            out.contains("src=\"https://iiif.dasch.swiss/0803/t1v.jp2/full/!480,480/0/default.jpg\""),
            "{out}"
        );
        assert!(out.contains("style=\"--ar: 0.7621\""), "{out}");
        assert!(
            out.contains("<h3 class=\"case__title\" lang=\"de\">„gleiche Druckermarke“</h3>"),
            "{out}"
        );
        assert!(out.contains("<p class=\"eyebrow\">2 Seiten</p>"), "{out}");
        assert_eq!(out.matches("case__whole").count(), 2, "{out}");
        assert!(
            out.contains("alt=\"Ganze Seite t1v, [Das] Narrenschiff (lat.) [März 1497]\" lang=\"de\""),
            "{out}"
        );
        assert!(
            out.ends_with(
                "<p class=\"case__note\">Verknüpfung auf Seitenebene — ohne markierte \
                 Region</p></figure></div>"
            ),
            "the note closes the case: {out}"
        );
        // Rows follow the configured page order, not the payload's.
        let maerz = out.find("t1v.jp2").expect("t1v");
        let august = out.find("x4v.jp2").expect("x4v");
        assert!(maerz < august, "{out}");
    }

    #[test]
    fn the_detail_sits_under_the_row_and_reads_alt_eyebrow_and_book() {
        let (cases, payload) = e5();
        let out = render(&cases, &payload, "de");
        let row_end = out.find("</ol>").expect("row");
        let detail = out.find("<a class=\"case__detail\"").expect("detail link");
        let remark = out.find("case__remark").expect("remark");
        assert!(row_end < detail && detail < remark, "{out}");
        assert_eq!(out.matches("<li ").count(), 2, "the detail is not in the row");
        assert!(
            out.contains(
                "href=\"/incunabula/books/narrenschiff-dt?page=narrenschiff-dt-b3v\
                 #annotation-XTtkp_hEUDWO828r2pb2PA\""
            ),
            "{out}"
        );
        // Padded region 20.46,57.80,37.18,12.11 of 3322 × 5113, fitted into
        // the 400 px detail box, not the 800 px crop box.
        assert!(out.contains("/!400,400/0/default.jpg"), "{out}");
        assert!(
            out.contains(
                "<span class=\"eyebrow\">Detail · b3v</span><span class=\"case__page\"><span>\
                 <span class=\"visually-hidden\">· [Das] Narrenschiff (dt.) — Seite öffnen\
                 </span><span aria-hidden=\"true\">b3v</span></span>"
            ),
            "{out}"
        );
    }

    #[test]
    fn a_page_with_a_zero_stored_size_renders_failed_at_a_finite_aspect() {
        let (cases, mut payload) = p3();
        if let Some(image) = payload.pages[1].image.as_mut() {
            image.height = Some(0);
        }
        let out = render_as(CaseLayout::Compact, &cases, &payload, "de");
        assert!(out.contains("<span class=\"case__bed is-failed\">"), "{out}");
        assert!(out.contains("style=\"--ar: 1.0000\""), "{out}");
        assert!(!out.contains("--ar: inf") && !out.contains("--ar: NaN"), "{out}");
    }

    #[test]
    fn a_compact_block_never_shows_a_detail() {
        let (cases, payload) = e5();
        let out = render_as(CaseLayout::Compact, &cases, &payload, "de");
        assert!(!out.contains("case__detail"), "{out}");
        assert!(out.contains("case__remark"), "{out}");
    }

    #[test]
    fn snapshot_woodcut_case_e1_de() {
        let (cases, payload) = e1();
        insta::assert_snapshot!(render(&cases, &payload, "de"));
    }

    #[test]
    fn snapshot_woodcut_case_e1_en() {
        let (cases, payload) = e1();
        insta::assert_snapshot!(render(&cases, &payload, "en"));
    }

    #[test]
    fn the_crop_is_cut_by_its_box_and_links_to_its_register_entry() {
        let (cases, payload) = e1();
        let out = render(&cases, &payload, "de");
        assert!(
            out.contains(
                "src=\"https://iiif.dasch.swiss/0803/c1v.jp2/pct:17.01,14.70,66.35,58.79/!800,800/0/default.jpg\""
            ),
            "{out}"
        );
        assert!(
            out.contains(
                "href=\"/incunabula/books/narrenschiff-dt?page=narrenschiff-dt-c1v#annotation-bsX4cDKkW3GMC8_Ms2tOpw\""
            ),
            "{out}"
        );
        assert!(!out.contains("dasch-link"), "a crop link stays in CPE: {out}");
    }

    #[test]
    fn each_crop_carries_its_own_pixel_aspect() {
        let (cases, payload) = e1();
        let out = render(&cases, &payload, "de");
        // c1v: (66.35 % × 3322) / (58.79 % × 5113); c8r: its own box on 2000 × 3000.
        assert!(out.contains("style=\"--ar: 0.7333\""), "{out}");
        assert_eq!(out.matches("style=\"--ar: ").count(), 2, "{out}");
    }

    #[test]
    fn the_id_line_omits_an_absent_schramm_number_and_title() {
        let (mut cases, payload) = e1();
        if let ShowcaseCase::Crops { schramm, title, .. } = &mut cases[0] {
            *schramm = None;
            *title = None;
        }
        let out = render(&cases, &payload, "de");
        assert!(out.contains("<p class=\"eyebrow\">2 Abdrucke</p>"), "{out}");
        assert!(!out.contains("Schramm"), "{out}");
        assert!(!out.contains("case__title"), "{out}");
    }

    #[test]
    fn a_data_remark_is_quoted_german_in_both_languages() {
        let (cases, payload) = e1();
        for code in ["de", "en"] {
            let out = render(&cases, &payload, code);
            assert!(
                out.contains("<p class=\"case__remark\" lang=\"de\">„Der Holzschnitt wird auf Seite c1v"),
                "{code}: {out}"
            );
        }
    }

    #[test]
    fn alts_are_german_from_the_data_or_composed_in_the_page_language() {
        let (cases, payload) = e1();
        let out = render(&cases, &payload, "en");
        assert!(
            out.contains("alt=\"Holzschnitt zu Kap. 13: Von der Buhlerei.\" lang=\"de\""),
            "{out}"
        );
        assert!(
            out.contains("alt=\"Woodcut, [Das] Narrenschiff (lat.) [Aug. 1497], c8r\" lang=\"en\""),
            "{out}"
        );
    }

    #[test]
    fn the_failed_state_hooks_are_in_every_bed() {
        let (cases, payload) = e1();
        let out = render(&cases, &payload, "de");
        // Maud escapes `&` in an attribute value, not `'`.
        assert_eq!(out.matches(&format!("data-on:error=\"{CROP_ERROR_EXPR}\"")).count(), 2, "{out}");
        assert_eq!(
            out.matches("data-init=\"el.complete &amp;&amp; !el.naturalWidth &amp;&amp; ")
                .count(),
            2,
            "{out}"
        );
        assert_eq!(
            out.matches("<span class=\"absent__label\" aria-hidden=\"true\">Ausschnitt nicht geladen</span>")
                .count(),
            2,
            "{out}"
        );
        assert!(
            out.contains("<span class=\"case__alt\" lang=\"de\">Holzschnitt zu Kap. 13"),
            "the alt copy is announced (no aria-hidden): {out}"
        );
        let lazy = out.find("loading=\"lazy\"").expect("lazy");
        let src = out.find(" src=\"").expect("src");
        assert!(lazy < src, "loading precedes src: {out}");
    }

    #[test]
    fn the_hidden_lead_follows_the_id_line_and_the_row_is_a_list() {
        let (cases, payload) = e1();
        let out = render(&cases, &payload, "de");
        let id_end = out.find("</div>").expect("id line");
        let lead = out
            .find("<span class=\"visually-hidden\">Derselbe Holzstock, 2 Abdrucke:</span>")
            .expect("lead");
        let row = out.find("<ol class=\"case__row\" role=\"list\">").expect("row");
        assert!(id_end < lead && lead < row, "{out}");
        assert!(out.contains("aria-labelledby=\"case-bsX4cDKkW3GMC8_Ms2tOpw-id\""), "{out}");
        assert!(!out.contains("figcaption"), "{out}");
    }

    #[test]
    fn a_crop_without_a_book_route_is_not_a_link() {
        let (cases, payload) = e1();
        let routes = HashMap::new();
        let links = LinkResolver::new("incunabula", &routes);
        let de = lang("de");
        let env = Env { links: &links, chrome: &[], lang: &de };
        let out = render_woodcut_cases(CaseLayout::Edition, &cases, &payload, &env).into_string();
        assert!(!out.contains("<a "), "{out}");
        assert_eq!(out.matches("<span class=\"case__link\">").count(), 2, "{out}");
        assert!(
            !out.contains(ChromeMsg::CaseOpenPage.text(&lang("de"))),
            "no link, no 'open the page': {out}"
        );
        let linked = render(&cases, &payload, "de");
        assert_eq!(linked.matches(ChromeMsg::CaseOpenPage.text(&lang("de"))).count(), 2, "{linked}");
    }

    #[test]
    fn a_crop_with_an_unsafe_image_uri_renders_its_bed_failed() {
        let (cases, mut payload) = e1();
        if let Some(image) = payload.crops[0].image.as_mut() {
            image.uri = "javascript:alert(1)//full/max/0/default.jpg".to_owned();
        }
        let out = render(&cases, &payload, "de");
        assert!(!out.contains("javascript:"), "{out}");
        assert!(out.contains("<span class=\"case__bed is-failed\">"), "{out}");
        assert_eq!(out.matches("<img").count(), 1, "{out}");
    }

    // ── Edition cards and the strip row ──────────────────────────────────────

    use cpe_core::ir::{DatePrecision, EditionBook, MediaThumb, Property, PropertyValue, Resource};

    fn text(name: &str, value: &str) -> Property {
        Property {
            name: name.to_owned(),
            values: vec![PropertyValue::Text { text: value.to_owned(), lang: None }],
        }
    }

    fn resource(id: &str, class: &str, properties: Vec<Property>) -> Resource {
        Resource {
            id: id.to_owned(),
            class: class.to_owned(),
            properties,
            representations: Vec::new(),
        }
    }

    /// The `/narrenschiff` site: the language eyebrows and the extent unit.
    fn narrenschiff_site() -> SiteConfig {
        let string = |id: &str, de: &str, en: &str| ChromeString {
            id: id.to_owned(),
            translations: vec![("de".to_owned(), de.to_owned()), ("en".to_owned(), en.to_owned())],
        };
        SiteConfig {
            title: "Incunabula".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["de".to_owned(), "en".to_owned()],
            default_language: "de".to_owned(),
            nav: Vec::new(),
            chrome: vec![
                string("edition-german", "Deutsch", "German"),
                string("edition-latin", "Latein", "Latin"),
                string("gloss-extent", "{n} Seiten", "{n} pages"),
            ],
            property_labels: Vec::new(),
            class_labels: Vec::new(),
            theme: cpe_core::theme::ThemeKnob::default(),
            date_precision_word: false,
            footer_attribution: None,
            not_found_browse: Vec::new(),
            narrative_companion: Default::default(),
            narrative_side: Default::default(),
            text_align: Default::default(),
        }
    }

    fn book(id: &str, name: &str, date: &str, display: &str, extent: i64) -> EditionBook {
        EditionBook {
            resource: resource(
                id,
                "Book",
                vec![
                    Property {
                        name: "extent".to_owned(),
                        values: vec![PropertyValue::Integer(extent)],
                    },
                    text("name", name),
                    Property {
                        name: "pub-date".to_owned(),
                        values: vec![PropertyValue::Date {
                            start: date.to_owned(),
                            end: None,
                            precision: DatePrecision::Day,
                            display: Some(display.to_owned()),
                            display_lang: Some("de".to_owned()),
                            calendar: None,
                        }],
                    },
                    text("pub-place", "Basel"),
                    text("publisher", "Johann Bergmann de Olpe"),
                ],
            ),
            cover: Some(CropImage {
                uri: format!("https://iiif.dasch.swiss/0803/{id}.jp2/full/max/0/default.jpg"),
                width: Some(2016),
                height: Some(2623),
            }),
        }
    }

    /// Two of the three `/narrenschiff` cards, März first, the configured
    /// order the reverse of the payload's.
    fn editions() -> (Vec<EditionCard>, Vec<GlossPart>, EditionCardsPayload) {
        let card = |book: &str, label: &str| EditionCard { book: book.to_owned(), lang_label: Some(label.to_owned()) };
        let part = |property: &str, unit: Option<&str>| GlossPart {
            property: property.to_owned(),
            unit: unit.map(str::to_owned),
        };
        (
            vec![
                card("narrenschiff-lat-maerz", "edition-latin"),
                card("narrenschiff-dt", "edition-german"),
            ],
            vec![
                part("pub-place", None),
                part("publisher", None),
                part("pub-date", None),
                part("extent", Some("gloss-extent")),
            ],
            EditionCardsPayload {
                books: vec![
                    book("narrenschiff-dt", "[Das] Narrenschiff (dt.)", "1495-03-03", "3. März 1495", 328),
                    book(
                        "narrenschiff-lat-maerz",
                        "[Das] Narrenschiff (lat.) [März 1497]",
                        "1497-03-01",
                        "1. März 1497",
                        302,
                    ),
                ],
            },
        )
    }

    fn render_editions(code: &str) -> String {
        let (cards, imprint, payload) = editions();
        let routes = routes();
        let links = LinkResolver::new("incunabula", &routes);
        render_edition_cards(&cards, &imprint, &payload, &links, &narrenschiff_site(), &lang(code)).into_string()
    }

    #[test]
    fn snapshot_edition_cards_de() {
        insta::assert_snapshot!(render_editions("de"));
    }

    #[test]
    fn snapshot_edition_cards_en() {
        insta::assert_snapshot!(render_editions("en"));
    }

    #[test]
    fn an_edition_card_links_to_the_reader_with_the_book_page_imprint() {
        let out = render_editions("de");
        assert!(
            out.contains("<a class=\"edition__link\" href=\"/incunabula/books/narrenschiff-lat-maerz\">"),
            "{out}"
        );
        assert!(
            out.contains(
                "<span class=\"edition__imprint\">Basel · Johann Bergmann de Olpe · 1. März 1497 · \
                 302 Seiten</span>"
            ),
            "{out}"
        );
        // The cover, fitted into the 480 px box (2016 × 2623), decorative.
        assert!(
            out.contains(
                "<img loading=\"lazy\" src=\"https://iiif.dasch.swiss/0803/narrenschiff-lat-maerz.jp2/full/!480,480/0/default.jpg\" alt=\"\">"
            ),
            "{out}"
        );
        assert!(out.contains("<span class=\"eyebrow\">Latein</span>"), "{out}");
        assert!(out.contains("Lesen<span aria-hidden=\"true\"> →</span>"), "{out}");
        let maerz = out.find("narrenschiff-lat-maerz\"").expect("März card");
        let dt = out.find("narrenschiff-dt\"").expect("dt card");
        assert!(maerz < dt, "cards follow the configured order: {out}");
    }

    #[test]
    fn an_edition_card_whose_book_is_missing_is_not_rendered() {
        let (cards, imprint, mut payload) = editions();
        payload.books.retain(|b| b.resource.id != "narrenschiff-dt");
        let routes = routes();
        let links = LinkResolver::new("incunabula", &routes);
        let out =
            render_edition_cards(&cards, &imprint, &payload, &links, &narrenschiff_site(), &lang("de")).into_string();
        assert_eq!(out.matches("<li class=\"edition\">").count(), 1, "{out}");
    }

    #[test]
    fn an_edition_card_without_a_book_route_is_not_a_link_and_offers_no_read() {
        let (cards, imprint, payload) = editions();
        let routes = HashMap::new();
        let links = LinkResolver::new("incunabula", &routes);
        let out =
            render_edition_cards(&cards, &imprint, &payload, &links, &narrenschiff_site(), &lang("de")).into_string();
        assert!(!out.contains("<a "), "{out}");
        assert_eq!(out.matches("<div class=\"edition__link\">").count(), 2, "{out}");
        assert!(!out.contains("edition__read"), "{out}");
    }

    fn strips() -> StripRowPayload {
        let strip = |id: &str, name: &str| resource(id, "BorderStrip", vec![text("name", name)]);
        let thumb = |id: &str| MediaThumb {
            uri: format!("https://iiif.dasch.swiss/0803/{id}.jp2/full/max/0/default.jpg"),
            width: Some(300),
            height: None,
        };
        StripRowPayload {
            strips: vec![strip("r-4", "r_4"), strip("l-3a", "l_3a")],
            thumbs: HashMap::from([("r-4".to_owned(), thumb("r-4"))]),
            class_total: 38,
        }
    }

    fn render_strips(code: &str) -> String {
        let routes = HashMap::from([("BorderStrip".to_owned(), "/strips/:id".to_owned())]);
        let links = LinkResolver::new("incunabula", &routes);
        render_strip_row(&strips(), &links, &lang(code)).into_string()
    }

    #[test]
    fn snapshot_strip_row_de() {
        insta::assert_snapshot!(render_strips("de"));
    }

    #[test]
    fn a_strip_tile_is_named_by_its_visible_id() {
        let de = render_strips("de");
        assert!(
            de.contains("<a class=\"strip\" href=\"/incunabula/strips/r-4\" aria-label=\"Randleiste r_4\">"),
            "{de}"
        );
        assert!(de.contains("<span class=\"strip__id\">r_4</span>"), "{de}");
        // The tile image is fetched at STRIP_TILE_FETCH_PX.
        assert!(
            de.contains("src=\"https://iiif.dasch.swiss/0803/r-4.jp2/full/192,/0/default.jpg\" alt=\"\""),
            "{de}"
        );
        // A strip with no image keeps an empty bed.
        assert_eq!(de.matches("<img").count(), 1, "{de}");
        let en = render_strips("en");
        assert!(en.contains("aria-label=\"Border strip l_3a\""), "{en}");
        let r4 = de.find("r_4").expect("r_4");
        let l3a = de.find("l_3a").expect("l_3a");
        assert!(r4 < l3a, "tiles keep the payload's order: {de}");
    }

    #[test]
    fn the_strip_rows_slot_count_is_the_whole_class() {
        assert_eq!(strip_row_slot_n(&ComponentData::StripRow(strips())), Some(38));
        assert_eq!(strip_row_slot_n(&ComponentData::Other), None);
    }

    #[test]
    fn a_crop_without_a_sized_image_renders_its_bed_failed() {
        let (cases, mut payload) = e1();
        payload.crops[0].image = None;
        let out = render(&cases, &payload, "de");
        assert!(out.contains("<span class=\"case__bed is-failed\">"), "{out}");
        assert_eq!(out.matches("<img").count(), 1, "{out}");
    }

    // ── Dataset-panel listing (plan D12) ─────────────────────────────────────

    const ARK: &str = "https://ark.dasch.swiss/ark:/72163/1/0803/";

    /// Every crop annotation and page plate of `payload` with an ARK, except
    /// the ids in `without`.
    fn arks_for(payload: &WoodcutCasesPayload, without: &[&str]) -> DataArks {
        let keep = |id: &String| !without.contains(&id.as_str());
        let ark = |id: &String| (id.clone(), format!("{ARK}{id}"));
        DataArks::from_stored(
            payload
                .pages
                .iter()
                .map(|p| &p.resource_id)
                .filter(|id| keep(id))
                .map(ark)
                .collect(),
            payload
                .crops
                .iter()
                .map(|c| &c.annotation.id)
                .filter(|id| keep(id))
                .map(ark)
                .collect(),
        )
    }

    fn listing_site() -> SiteConfig {
        let mut site = narrenschiff_site();
        site.class_labels = vec![cpe_core::ir::ClassLabel {
            class: "Page".to_owned(),
            translations: vec![
                ("de".to_owned(), "Seite".to_owned()),
                ("en".to_owned(), "Page".to_owned()),
            ],
            plural: vec![
                ("de".to_owned(), "Seiten".to_owned()),
                ("en".to_owned(), "pages".to_owned()),
            ],
        }];
        site
    }

    fn listing(
        layout: CaseLayout,
        (cases, payload): (Vec<ShowcaseCase>, WoodcutCasesPayload),
        without: &[&str],
        code: &str,
    ) -> Vec<DataGroup> {
        let c = Component::WoodcutCases { media: "image".to_owned(), layout, cases };
        let arks = arks_for(&payload, without);
        woodcut_cases_listing(&c, &ComponentData::WoodcutCases(payload), &listing_site(), &lang(code), &arks)
    }

    fn rows(groups: &[DataGroup]) -> Vec<(String, Vec<(String, String)>)> {
        groups
            .iter()
            .map(|g| {
                let entries = g.entries.iter().map(|e| (e.role.clone(), e.title.clone())).collect();
                (g.head.clone(), entries)
            })
            .collect()
    }

    fn row(role: &str, title: &str) -> (String, String) {
        (role.to_owned(), title.to_owned())
    }

    #[test]
    fn a_crop_case_lists_its_regions_and_its_detail_under_its_schramm_number() {
        let groups = listing(CaseLayout::Edition, e5(), &[], "de");
        let dt = "[Das] Narrenschiff (dt.)";
        assert_eq!(
            rows(&groups),
            [(
                "Schramm 1118 · 3".to_owned(),
                vec![
                    row("Region · b3v", dt),
                    row("Region · c2r", "[Das] Narrenschiff (lat.) [März 1497]"),
                    row("Region · b3v", dt),
                ]
            )]
        );
        let entry = &groups[0].entries[2];
        assert_eq!(entry.ark, format!("{ARK}XTtkp_hEUDWO828r2pb2PA"));
        assert_eq!((entry.class.as_deref(), entry.number), (None, None));
    }

    #[test]
    fn a_compact_block_lists_no_detail() {
        let groups = listing(CaseLayout::Compact, e5(), &[], "de");
        assert_eq!(groups[0].head, "Schramm 1118 · 2");
        assert_eq!(groups[0].entries.len(), 2);
    }

    #[test]
    fn a_case_without_a_schramm_number_is_headed_by_its_pages() {
        let (mut cases, payload) = compact_pair();
        if let ShowcaseCase::Crops { schramm, .. } = &mut cases[0] {
            *schramm = None;
        }
        let groups = listing(CaseLayout::Compact, (cases, payload), &[], "en");
        assert_eq!(groups[0].head, "e3r / e4r · 2");
        assert_eq!(groups[1].head, "Schramm 458 · 2");
    }

    #[test]
    fn a_page_case_lists_its_pages_at_page_level() {
        let groups = listing(CaseLayout::Compact, p3(), &[], "de");
        assert_eq!(
            rows(&groups),
            [(
                "Seitenebene · t1v / x4v · 2".to_owned(),
                vec![
                    row("Seite · t1v", "[Das] Narrenschiff (lat.) [März 1497]"),
                    row("Seite · x4v", "[Das] Narrenschiff (lat.) [Aug. 1497]"),
                ]
            )]
        );
        assert_eq!(groups[0].entries[0].class.as_deref(), Some("Page"));
        let en = listing(CaseLayout::Compact, p3(), &[], "en");
        assert_eq!(en[0].head, "Page level · t1v / x4v · 2");
        assert_eq!(en[0].entries[1].role, "Page · x4v");
    }

    /// The count is of listed entries, and a case left with none is no group.
    #[test]
    fn an_entry_without_an_ark_is_left_out_and_not_counted() {
        let groups = listing(CaseLayout::Edition, e5(), &["XTtkp_hEUDWO828r2pb2PA"], "de");
        assert_eq!(groups[0].head, "Schramm 1118 · 2");
        let none = listing(
            CaseLayout::Edition,
            e1(),
            &["bsX4cDKkW3GMC8_Ms2tOpw", "gAfAiG_-U4itgN8Gdr9Ipw"],
            "de",
        );
        assert!(none.is_empty(), "{none:?}");
    }

    #[test]
    fn other_data_lists_nothing() {
        let (cases, _) = e1();
        let c = Component::WoodcutCases {
            media: "image".to_owned(),
            layout: CaseLayout::Edition,
            cases,
        };
        let out = woodcut_cases_listing(&c, &ComponentData::Other, &listing_site(), &lang("de"), &DataArks::default());
        assert!(out.is_empty());
    }
}
