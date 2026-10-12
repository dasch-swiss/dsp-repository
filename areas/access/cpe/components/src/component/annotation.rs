//! The annotation register of a `pages` reader's page (`dsp-incubator
//! cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md`):
//! `div#cv-annotations`, the reader's second `_compound` patch target, holding
//! `section#annotations` and the stacked-width bookmark. The page's overlay markup (the shapes and
//! tags drawn on the scan) and the pure geometry and colour helpers live here too; nothing here
//! touches SQL.

use cpe_core::ir::{
    Annotation, AnnotationEntry, AnnotationLink, AnnotationPartner, AnnotationShape, CompoundPage, Resource,
};
use cpe_core::lang::Lang;
use maud::{html, Markup, PreEscaped};

use crate::chrome::{anno_arrival_text, anno_chip_name, anno_register_heading, ChromeMsg};
use crate::component::dom::{entry_id, HEADER_MENU_OPEN_EXPR, PANEL_ID};
use crate::component::links::LinkResolver;
use crate::component::values::member_text;

/// The id of the register's patch target. It renders on every page of a
/// reader with `annotations=#true` — empty on a page without a register —
/// so a `_compound` swap always has an element to patch.
pub const ANNOTATIONS_TARGET_ID: &str = "cv-annotations";

/// The hovered, pinned and arrival-emphasised ids (`''` = none; only
/// [`ARRIVAL_INIT_EXPR`] writes the arrival one) and the arrival note's text.
/// The one key set both the declaration and the reset use. Every annotation
/// signal is `_`-prefixed, so DataStar's default `filterSignals` keeps it out
/// of every request (`_compound`'s `?datastar=` among them). Each is a string
/// that is never assigned `undefined`, which would delete the signal: an
/// assignment from event or dataset data ends in `?? ''`.
const RESETTABLE_SIGNALS: &str = "_annoHover: '', _annoPin: '', _annoArrive: '', _annoNote: ''";

/// The `data-signals` declaration on `article.record--reader`: the
/// resettable signals plus `_annoShow` (the toggle, on by default). The
/// article is outside every `_compound` patch target, so no swap
/// re-declares the toggle and it survives a page turn.
pub(crate) fn annotation_signals_decl() -> String {
    format!("{{_annoShow: true, {RESETTABLE_SIGNALS}}}")
}

/// The `_compound` swap's reset of hover, pin and arrival, never `_annoShow`.
#[must_use]
pub fn annotation_reset_signals_json() -> String {
    format!("{{{RESETTABLE_SIGNALS}}}")
}

/// The arrival: a `data-init` on `article.record--reader`, which
/// no swap re-inserts, so it runs once per real load. A `#annotation-<id>`
/// hash for a numbered entry here emphasises its shape (no zoom). The note
/// needs a `navigate` load (the referrer survives a reload and Back/Forward)
/// and a same-origin referrer naming a partner page: the partner link's path,
/// with its `page=` id, or no `page=` on a book's first page. It shows the
/// link's `data-anno-note`; nothing is written to the URL or storage.
///
/// A `navigate` load also scrolls once (`dsp-incubator
/// cpe/docs/adr/0007-fragment-swaps-are-transient.md`, 2026-10-07 amendment): a sticky reader's
/// entry to the window's end, else a one-track (stacked) reader's head, the parent of `#cv-title`.
/// The layout is read from `record.css`'s computed style, so `#compound-viewer`'s sticky rule and
/// the `.record__tracks` grid are part of this contract. The scroll waits a frame:
/// by then the note, which sits above the entries, is in the DOM.
pub(crate) const ARRIVAL_INIT_EXPR: &str = "\
    const at = (() => { try { return document.getElementById(decodeURIComponent(location.hash.slice(1))); } \
        catch { return null; } })()?.closest('#annotations .anno__entry'); \
    if (at) { \
        if (!at.classList.contains('anno__entry--page')) $_annoArrive = at.id.slice('annotation-'.length); \
        const nav = performance.getEntriesByType('navigation')[0]; \
        const ref = nav?.type === 'navigate' && document.referrer ? new URL(document.referrer) : null; \
        if (ref?.origin === location.origin) { \
            const page = ref.searchParams.get('page'); \
            const from = [...at.querySelectorAll('a[data-anno-page]')].find((a) => \
                new URL(a.href).pathname === ref.pathname \
                && (page === null ? 'annoFirst' in a.dataset : page === a.dataset.annoPage)); \
            if (from) $_annoNote = from.dataset.annoNote ?? ''; \
        } \
        if (nav?.type === 'navigate') requestAnimationFrame(() => { \
            const reader = document.getElementById('compound-viewer'); \
            const tracks = el.querySelector('.record__tracks'); \
            if (reader && getComputedStyle(reader).position === 'sticky') at.scrollIntoView({block: 'end'}); \
            else if (tracks && getComputedStyle(tracks).gridTemplateColumns.split(' ').length === 1) \
                document.getElementById('cv-title')?.parentElement?.scrollIntoView({block: 'start'}); \
        }); \
    }";

/// Clears the arrival note: the toggle, a pin and a zoom away from fit each
/// end the arrival; a page turn resets it with the rest.
const NOTE_CLEAR: &str = "$_annoNote = ''";

/// The stage's `viewer-zoom` listener on a page with a register: a zoom away
/// from fit clears the arrival note, keeping the pin. Bind it debounced
/// (`__debounce.250ms`) so only the settled zoom counts: just after opening,
/// a layout change reports one `atHome: false` before fit again.
pub(crate) fn zoom_away_expr() -> String {
    format!("if (!evt.detail.atHome) {NOTE_CLEAR}")
}

// ── Interaction ─────────────────────────────────────

/// The id drawn emphasised, as a JS expression: the pin wins over a hover,
/// which wins over the arrival emphasis. The viewer's `setOverlayState`
/// focus and the register's emphasis class both read it, so the scan and
/// the list never emphasise different entries.
pub(crate) const FOCUS_EXPR: &str = "($_annoPin || $_annoHover || $_annoArrive)";

/// The reader's one `<image-viewer>` (`MultipleImageViewers` allows no
/// second), looked up when the handler runs: a page turn replaces it.
pub(crate) const READER_VIEWER_EXPR: &str = "document.querySelector('#compound-viewer image-viewer')";

/// Pin `id` and zoom onto its shape, or, when `id` is already pinned, unpin
/// and return to fit. A pin clears the arrival note. `id` and
/// `viewer` are JS expressions; the viewer calls are optional-chained, since
/// a page may have no mounted viewer (a non-IIIF scan) and still pins in the
/// register. Pass no zoom options: the padding and cap are the viewer's.
pub(crate) fn pin_toggle_expr(id: &str, viewer: &str) -> String {
    format!(
        "if ($_annoPin === {id}) {{ $_annoPin = ''; {viewer}?.goHome?.(); }} \
         else {{ $_annoPin = {id}; {NOTE_CLEAR}; {viewer}?.zoomToShape?.({id}); }}"
    )
}

/// Esc unpins and returns to fit, then returns, so a keydown
/// expression may go on after it for other keys. Bound twice: on the reader
/// and on the register, which sits outside `#compound-viewer`. Ignored in
/// an open `details`, and, wherever focus is, while the crumb line's dataset
/// panel, the header's language menu or the header menu panel is open:
/// their own Esc runs later (window-level, or the popover's close request),
/// so `defaultPrevented` cannot tell, and one Esc must close only that layer.
pub(crate) fn esc_unpin_expr() -> String {
    format!(
        "if (evt.key === 'Escape') {{ if ($_annoPin && !evt.target.closest('details[open]') \
         && !document.querySelector('#{PANEL_ID}[open]') \
         && !document.querySelector('.lang-menu[open]') && !{HEADER_MENU_OPEN_EXPR}) \
         {{ $_annoPin = ''; {READER_VIEWER_EXPR}?.goHome?.(); }} return; }}"
    )
}

/// The stage's `data-effect`: hand the toggle and the focused id to the
/// page's own viewer. Naming `viewer_id` (`rv-<image id>`) makes the value
/// differ per page, so DataStar re-applies the effect after a swap; a morph
/// does not re-run an effect whose value is unchanged. The viewer keeps the
/// last state and applies it on `open`, which comes after the effect.
///
/// The signals are read as the argument of an outer call, never inside an
/// optional call: DataStar may run the effect before `image-viewer.js` has
/// defined the element, and a short-circuited `?.setOverlayState?.(…)` would
/// read no signal, leaving the effect subscribed to none, so it would never
/// run again. The state is handed over once the element is defined.
pub(crate) fn overlay_effect_expr(viewer_id: &str) -> String {
    format!(
        "((s) => customElements.whenDefined('image-viewer').then(() => \
         document.getElementById({})?.setOverlayState?.(s)))\
         ({{visible: $_annoShow, focus: {FOCUS_EXPR}}})",
        js_id(viewer_id)
    )
}

/// The stage's `data-on:viewer-overlay-hover`: a shape or tag hovered on
/// the scan (`id` null on leave).
pub(crate) const OVERLAY_HOVER_EXPR: &str = "$_annoHover = evt.detail.id ?? ''";

/// The stage's `data-on:viewer-overlay-click`: a shape or its tag clicked
/// pins and zooms, like its chip. The event's target is the viewer itself.
pub(crate) fn overlay_click_expr() -> String {
    pin_toggle_expr("(evt.detail.id ?? '')", "evt.target")
}

/// Whether `entries` hold a drawn shape: only those can be shown, hidden or
/// pinned.
pub(crate) fn has_shapes(entries: &[AnnotationEntry]) -> bool {
    entries.iter().any(|e| matches!(e, AnnotationEntry::Annotation { .. }))
}

/// `id` as a single-quoted JS string literal.
fn js_id(id: &str) -> String {
    format!("'{}'", cpe_core::util::js_string_escape(id))
}

/// The register section's id: the in-page target of the reader's
/// "N annotations" signpost.
pub(crate) const REGISTER_ID: &str = "annotations";

/// The annotation signposts' accent dot. Decorative: each signpost states
/// the fact in its accessible name.
pub(crate) const ANNO_DOT: PreEscaped<&str> = PreEscaped(r#"<span class="anno-dot" aria-hidden="true"></span>"#);

/// The register heading's id; the section is `aria-labelledby` it.
const CAPTION_ID: &str = "anno-cap";

/// The shown book's identity: repeated by the bookmark under the register,
/// and the book an annotation link's partner page is compared with to read
/// "this book".
#[doc(hidden)]
pub struct Bookmark<'a> {
    /// The book's class label ("Buch").
    #[doc(hidden)]
    pub class_label: &'a str,
    #[doc(hidden)]
    pub id: &'a str,
    #[doc(hidden)]
    pub title: &'a str,
}

/// Render `div#cv-annotations` for the reader's shown page: the register and
/// the bookmark when the page has entries, else the element alone with no
/// children (no whitespace either, so CSS `:empty` matches).
pub fn render_annotations_target(
    page: &CompoundPage,
    member_class_label: &str,
    bookmark: &Bookmark<'_>,
    links: &LinkResolver<'_>,
    lang: &Lang,
) -> Markup {
    let register = page.member.as_ref().filter(|_| !page.annotations.is_empty()).map(|member| {
        let label = register_page_label(page, member, lang);
        let partners = Partners { links, book: bookmark, lang };
        render_register(page, member_class_label, &label, &partners)
    });
    html! {
        div id=(ANNOTATIONS_TARGET_ID) {
            @if let Some(register) = register { (register) (render_bookmark(bookmark)) }
        }
    }
}

/// The shown page's name in the register heading: its `name`, else its
/// position. The dataset panel's group head names the page with it too.
pub(crate) fn register_page_label(page: &CompoundPage, member: &Resource, lang: &Lang) -> String {
    member_text(member, "name", lang).map_or_else(|| page.index.to_string(), str::to_owned)
}

/// `section#annotations`: a head repeating the reader's
/// identity line over the heading, then one `li` per entry in register order.
fn render_register(page: &CompoundPage, member_class_label: &str, page_label: &str, partners: &Partners<'_>) -> Markup {
    let lang = partners.lang;
    let drawn = has_shapes(&page.annotations);
    html! {
        section
            .anno
            id=(REGISTER_ID)
            aria-labelledby=(CAPTION_ID)
            data-on:keydown=[drawn.then(esc_unpin_expr)]
        {
            header.anno__head {
                div.anno__title {
                    p.eyebrow.anno__of {
                        ({
                            format!(
                                "{member_class_label} · {} / {}",
                                page.index,
                                page.total,
                            )
                        })
                    }
                    h3.anno__cap id=(CAPTION_ID) { (anno_register_heading(page_label, lang)) }
                }
                @if drawn {
                    // Revealed only with JS: `.vctl` sets `display`, so the
                    // paired `.vctl[hidden]` rule is what hides it. The
                    // string `aria-pressed` form: the `attr` plugin writes
                    // `true` as an empty attribute.
                    button
                        .vctl
                        .anno__toggle
                        type="button"
                        hidden
                        aria-pressed="true"
                        data-attr="{hidden: false, 'aria-pressed': $_annoShow ? 'true' : 'false'}"
                        data-on:click=(format!("$_annoShow = !$_annoShow; {NOTE_CLEAR}"))
                    { (ChromeMsg::AnnoShowOnScan.text(lang)) }
                }
            }
            // The arrival note, filled by `ARRIVAL_INIT_EXPR`;
            // empty, and so no height, otherwise.
            p.anno__arrival role="status" data-text="$_annoNote" {}
            // `role="list"`: `list-style: none` drops the list semantics in
            // Safari/VoiceOver.
            ol.anno__list role="list" {
                @for entry in &page.annotations {
                    @let item = match entry {
                        AnnotationEntry::Annotation { number, annotation, partners: p } => {
                            render_numbered_entry(
                                *number,
                                annotation,
                                &partners.lines(p, Some(*number)),
                                lang,
                            )
                        }
                        AnnotationEntry::Link { link, partners: p } => {
                            render_page_entry(link, &partners.lines(p, None), lang)
                        }
                    };
                    (item)
                }
            }
        }
    }
}

/// What an entry's annotation-link lines need: the resolver for the partner
/// hrefs, the shown book (a partner page in it reads "this book") and the
/// language.
struct Partners<'a> {
    links: &'a LinkResolver<'a>,
    book: &'a Bookmark<'a>,
    lang: &'a Lang,
}

/// One annotation-link line: a link name over the partners it joins the entry to.
struct LinkLine<'a> {
    name: &'a str,
    name_lang: Option<&'a str>,
    partners: Vec<&'a AnnotationPartner>,
}

/// Group link partners into lines: one line per link name, in first-seen
/// order, and within a line one partner per resource, in first-seen order.
/// When a resource comes up again on the same line, `replace(new, kept)`
/// decides whether `new` takes the kept partner's place. Each line is its
/// first-seen item (which carries the name and its language) and its
/// partners. Shared by the register's lines and the Apparat's links row.
pub(crate) fn group_by_link_name<'a, T>(
    items: &'a [T],
    name: impl Fn(&T) -> &str,
    resource: impl Fn(&T) -> &str,
    replace: impl Fn(&T, &T) -> bool,
) -> Vec<(&'a T, Vec<&'a T>)> {
    let mut lines: Vec<(&'a T, Vec<&'a T>)> = Vec::new();
    for item in items {
        let Some((_, partners)) = lines.iter_mut().find(|(first, _)| name(first) == name(item)) else {
            lines.push((item, vec![item]));
            continue;
        };
        match partners.iter_mut().find(|p| resource(p) == resource(item)) {
            Some(kept) if replace(item, kept) => *kept = item,
            Some(_) => {}
            None => partners.push(item),
        }
    }
    lines
}

/// An entry's partners as link lines: one line per link name, in
/// first-seen order, so the links of one woodcut read as one line; within a
/// line one partner per partner page, the page's lowest-numbered annotation
/// (a page partner, unnumbered, ranks last).
fn link_lines(partners: &[AnnotationPartner]) -> Vec<LinkLine<'_>> {
    let rank = |p: &AnnotationPartner| p.number.unwrap_or(u32::MAX);
    group_by_link_name(
        partners,
        |p| &p.link_name,
        |p| &p.resource_id,
        |new, kept| rank(new) < rank(kept),
    )
    .into_iter()
    .map(|(first, partners)| LinkLine {
        name: &first.link_name,
        name_lang: first.link_name_lang.as_deref(),
        partners,
    })
    .collect()
}

impl Partners<'_> {
    /// `div.anno__links` for an entry's partners: per line
    /// the link name, then each partner named by its book's title ("this
    /// book" for the shown one) and its page. Nothing when it has none.
    /// `number` is the entry's own (`None` on a page-level entry), for the
    /// arrival note.
    fn lines(&self, partners: &[AnnotationPartner], number: Option<u32>) -> Markup {
        let lines = link_lines(partners);
        html! {
            @if !lines.is_empty() {
                div.anno__links {
                    @for line in &lines {
                        div class=({
                                if line.partners.len() > 1 {
                                    "anno__link anno__link--multi"
                                } else {
                                    "anno__link"
                                }
                            })
                        {
                            span.anno__link-name lang=[line.name_lang] { (line.name) }
                            @for partner in &line.partners { (self.partner(partner, number)) }
                        }
                    }
                }
            }
        }
    }

    /// One partner: `a.anno__partner` to its register entry, or the same
    /// words as plain text when it has no route.
    ///
    /// A partner page in a reader also carries the arrival data that
    /// [`ARRIVAL_INIT_EXPR`] reads: its page id, whether it is its
    /// book's first page, and the finished note naming it. The names are
    /// `data-anno-*` on purpose: a `data-<name>` whose name is a DataStar
    /// plugin (`data-text`, `data-init`, …) would be executed.
    fn partner(&self, partner: &AnnotationPartner, number: Option<u32>) -> Markup {
        // A member reads "<book> · <page>"; any other resource is its own
        // title. The note names the shown book by its title, not "this book".
        let (title, note_title, page) = match &partner.parent {
            Some(parent) if parent.id == self.book.id => (
                ChromeMsg::AnnoThisBook.text(self.lang),
                self.book.title,
                Some(partner.resource_label.as_str()),
            ),
            // A parent with no name falls back to its id, never to the
            // page label, which would read "c8r · c8r".
            Some(parent) => {
                let title = partner.parent_title.as_deref().unwrap_or(&parent.id);
                (title, title, Some(partner.resource_label.as_str()))
            }
            None => (partner.resource_label.as_str(), "", None),
        };
        let words = html! {
            em.anno__book { (title) }
            @if let Some(page) = page {
                " "
                span.anno__page {
                    span aria-hidden="true" { "· " }
                    (page)
                }
            }
        };
        let note = page.map(|page| anno_arrival_text(note_title, page, number, self.lang));
        match self.links.partner_href(partner) {
            Some(href) => html! {
                a   .anno__partner
                    href=(href)
                    data-anno-page=[note.is_some().then_some(&partner.resource_id)]
                    data-anno-first[note.is_some() && partner.first_member]
                    data-anno-note=[note]
                {
                    (words)
                    " "
                    span aria-hidden="true" { "→" }
                }
            },
            None => html! {
                span.anno__partner { (words) }
            },
        }
    }
}

/// A drawn annotation's entry: its chip, the shape line (shape word, then
/// the swatch and colour name when it has a colour), its remark, then its
/// annotation-link `lines`. The colour reaches the entry as `--iv-c`/`--iv-on`,
/// the viewer overlay contract's names, so the swatch and the overlay read
/// one value.
fn render_numbered_entry(number: u32, annotation: &Annotation, lines: &Markup, lang: &Lang) -> Markup {
    let shape = shape_word(annotation.shape).text(lang);
    let colour = annotation.color.as_deref().and_then(|hex| {
        let term = colour_term(hex)?;
        Some((colour_vars(hex)?, term.text(lang)))
    });
    let id = js_id(&annotation.id);
    // Hovering or focusing the entry emphasises its shape; leaving clears
    // the hover only if it is still this entry's, so a hover that has moved
    // to the scan is not undone.
    let enter = format!("$_annoHover = {id}");
    let leave = format!("if ($_annoHover === {id}) $_annoHover = ''");
    html! {
        li  .anno__entry
            id=(entry_id(&annotation.id))
            style=[colour.as_ref().map(|(style, _)| style)]
            data-class=(format!("{{'is-em': {FOCUS_EXPR} === {id}}}"))
            data-on:pointerenter=(enter)
            data-on:pointerleave=(leave)
            data-on:focusin=(enter)
            data-on:focusout=(leave)
        {
            // A button with or without JS: the number stays visible; with JS
            // it pins and zooms. Inert without JS.
            button
                .anno__chip
                type="button"
                aria-pressed="false"
                aria-label=(anno_chip_name(number, shape, lang))
                data-attr=(format!("{{'aria-pressed': $_annoPin === {id} ? 'true' : 'false'}}"))
                data-on:click=(pin_toggle_expr(&id, READER_VIEWER_EXPR))
            { (number) }
            div.anno__body {
                p.anno__shape {
                    (shape)
                    @if let Some((_, name)) = &colour {
                        // The spaces keep the two words apart for assistive
                        // technology; the separator and swatch are hidden.
                        " "
                        span.anno__sep aria-hidden="true" { "·" }
                        " "
                        span.anno__swatch aria-hidden="true" {}
                        (name)
                    }
                }
                @if let Some(remark) = &annotation.remark {
                    p.anno__remark lang=[annotation.remark_lang.as_deref()] { (remark) }
                }
                (lines)
            }
        }
    }
}

/// A page-level entry (a link targeting the page itself): unnumbered, the
/// whole-page shape word, the link's own comment as its remark, then its
/// annotation-link `lines`.
fn render_page_entry(link: &AnnotationLink, lines: &Markup, lang: &Lang) -> Markup {
    html! {
        li.anno__entry.anno__entry--page id=(entry_id(&link.id)) {
            span.anno__nonum aria-hidden="true" { "—" }
            div.anno__body {
                p.anno__shape { (ChromeMsg::AnnoWholePage.text(lang)) }
                @if let Some(comment) = &link.comment {
                    p.anno__remark lang=[link.comment_lang.as_deref()] { (comment) }
                }
                (lines)
            }
        }
    }
}

/// The bookmark under the register at stacked widths: the
/// book's identity line over its title, before the book's intro resumes.
fn render_bookmark(bookmark: &Bookmark<'_>) -> Markup {
    html! {
        div.anno-bookmark {
            div.record__class {
                p.eyebrow { (bookmark.class_label) }
                span.record__id { (bookmark.id) }
            }
            p.anno-bookmark__title { (bookmark.title) }
        }
    }
}

pub(crate) fn shape_word(shape: AnnotationShape) -> ChromeMsg {
    match shape {
        AnnotationShape::Rectangle => ChromeMsg::AnnoShapeRectangle,
        AnnotationShape::Polygon => ChromeMsg::AnnoShapePolygon,
        AnnotationShape::Circle => ChromeMsg::AnnoShapeCircle,
    }
}

// ── Overlay ───────────────────────────────────────────────────────────────────

/// The zero-JS overlay's hide-while-off binding (media.css `.anno-off`): a
/// class, never a reactively written `data-*` attribute.
const OFF_CLASS_EXPR: &str = "{'anno-off': !$_annoShow}";

/// The page's drawn shapes and their number tags, in the overlay markup
/// contract of `cpe-image-viewer`'s README ("Overlay layer"): one
/// `svg[data-iv-overlay]` in a `0 0 100 100` box, then one tag per shape.
/// Shapes follow register order, so a nested smaller shape (numbered later)
/// paints on top and wins the hit. The tags carry `left`/`top` too, which
/// only the zero-JS copy reads. `None` when the page has no numbered entry.
pub(crate) fn render_overlay(entries: &[AnnotationEntry]) -> Option<Markup> {
    let shapes: Vec<(u32, &Annotation)> = entries
        .iter()
        .filter_map(|entry| match entry {
            AnnotationEntry::Annotation { number, annotation, .. } => Some((*number, annotation)),
            AnnotationEntry::Link { .. } => None,
        })
        .collect();
    if shapes.is_empty() {
        return None;
    }
    // The light-DOM copy hides while the toggle is off, so a page turn never
    // flashes shapes before the viewer opens. The viewer's clone drops the
    // class binding and takes its state from `setOverlayState` alone.
    Some(html! {
        svg data-iv-overlay
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
            aria-hidden="true"
            data-class=(OFF_CLASS_EXPR)
        {
            @for (_, a) in &shapes {
                g data-iv-shape=(a.id) style=[a.color.as_deref().and_then(colour_vars)] {
                    @for part in ["halo", "line"] { (render_geometry(a, part)) }
                }
            }
        }
        @for (number, a) in &shapes { (render_tag(*number, a)) }
    })
}

fn render_geometry(a: &Annotation, part: &str) -> Markup {
    match a.shape {
        AnnotationShape::Rectangle => html! {
            rect
                data-iv-part=(part)
                x=(view_box(a.x))
                y=(view_box(a.y))
                width=(view_box(a.w))
                height=(view_box(a.h)) {}
        },
        AnnotationShape::Circle => html! {
            ellipse
                data-iv-part=(part)
                cx=(view_box(a.x + a.w / 2.0))
                cy=(view_box(a.y + a.h / 2.0))
                rx=(view_box(a.w / 2.0))
                ry=(view_box(a.h / 2.0)) {}
        },
        AnnotationShape::Polygon => {
            let points = a
                .points
                .iter()
                .map(|&(x, y)| format!("{},{}", view_box(x), view_box(y)))
                .collect::<Vec<_>>()
                .join(" ");
            html! {
                polygon data-iv-part=(part) points=(points) {}
            }
        }
    }
}

fn render_tag(number: u32, a: &Annotation) -> Markup {
    let anchor = tag_anchor(a);
    let (x, y) = (view_box(anchor.x), view_box(anchor.y));
    let position = format!("left:{x}%;top:{y}%");
    let style = match a.color.as_deref().and_then(colour_vars) {
        Some(colour) => format!("{colour};{position}"),
        None => position,
    };
    let align = match anchor.align {
        TagAlign::Start => "start",
        TagAlign::Center => "center",
    };
    html! {
        span
            data-iv-tag=(a.id)
            data-iv-x=(x)
            data-iv-y=(y)
            data-iv-align=(align)
            data-iv-place=[anchor.below.then_some("below")]
            style=(style)
            aria-hidden="true"
            data-class=(OFF_CLASS_EXPR)
        { (number) }
    }
}

/// How a tag's box sits on its anchor point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TagAlign {
    /// Its left edge on the anchor.
    Start,
    /// Centred on the anchor.
    Center,
}

/// Where a shape's number tag hangs, in the image's 0..1 coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TagAnchor {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) align: TagAlign,
    /// The tag hangs below the anchor, inside the shape, instead of above it.
    pub(crate) below: bool,
}

/// An anchor above this fraction of the image's height hangs its tag below:
/// above it, the tag would sit in the letterbox.
const TAG_BELOW_MAX_Y: f64 = 0.05;

/// The tag anchor of a shape: a rectangle's top-left
/// corner (tag left-aligned), a circle's top point, or a polygon's topmost
/// vertex, leftmost on a tie (both centred). The two rules keep nested
/// shapes' tags apart.
pub(crate) fn tag_anchor(a: &Annotation) -> TagAnchor {
    let (x, y, align) = match a.shape {
        AnnotationShape::Rectangle => (a.x, a.y, TagAlign::Start),
        AnnotationShape::Circle => (a.x + a.w / 2.0, a.y, TagAlign::Center),
        AnnotationShape::Polygon => {
            // Boot validation guarantees at least three vertices; the box's
            // top centre stands in should that ever not hold.
            let (x, y) = a
                .points
                .iter()
                .copied()
                .min_by(|p, q| p.1.total_cmp(&q.1).then(p.0.total_cmp(&q.0)))
                .unwrap_or((a.x + a.w / 2.0, a.y));
            (x, y, TagAlign::Center)
        }
    };
    TagAnchor { x, y, align, below: y < TAG_BELOW_MAX_Y }
}

/// A 0..1 image coordinate in the overlay's `viewBox 0 0 100 100` space,
/// fixed at three decimals so the markup is stable across renders. Clamped
/// to 0..1 first: boot validation allows a 1e-9 overshoot either side, which
/// would otherwise print as `-0.000` (as would `-0.0`, which the clamp keeps;
/// `+ 0.0` turns it into `0.0`).
pub(crate) fn view_box(v: f64) -> String {
    format!("{:.3}", v.clamp(0.0, 1.0) * 100.0 + 0.0)
}

// ── Colour ────────────────────────────────────────────────────────────────────

/// The inline custom properties carrying an annotation's colour under the
/// viewer overlay contract's names, read by the overlay, the swatch and the
/// chip alike. `None` for a value that is not `#rrggbb`.
fn colour_vars(hex: &str) -> Option<String> {
    Some(format!("--iv-c:{hex};--iv-on:{}", on_colour(hex)?))
}

/// `#rrggbb` → its three channels; `None` for anything else.
fn parse_hex(hex: &str) -> Option<[u8; 3]> {
    // `from_str_radix` alone would also take a leading `+`.
    let digits = hex
        .strip_prefix('#')
        .filter(|d| d.len() == 6 && d.bytes().all(|b| b.is_ascii_hexdigit()))?;
    let channel = |i: usize| u8::from_str_radix(digits.get(i..i + 2)?, 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// WCAG 2 relative luminance of an sRGB colour.
fn relative_luminance([r, g, b]: [u8; 3]) -> f64 {
    let lin = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// The text colour on `hex`: black or white, whichever has
/// the higher WCAG contrast against it; black on a tie. `None` for a value
/// that is not `#rrggbb`.
pub(crate) fn on_colour(hex: &str) -> Option<&'static str> {
    let l = relative_luminance(parse_hex(hex)?);
    let against_black = (l + 0.05) / 0.05;
    let against_white = 1.05 / (l + 0.05);
    Some(if against_black >= against_white { "#000" } else { "#fff" })
}

/// Lightness at or below which a colour is named black.
const BLACK_MAX_LIGHTNESS: f64 = 0.12;
/// Lightness at or above which a colour is named white.
const WHITE_MIN_LIGHTNESS: f64 = 0.92;
/// Saturation below which a colour is achromatic (grey, black or white).
const CHROMATIC_MIN_SATURATION: f64 = 0.15;

/// Hue buckets, in degrees: a hue below a bucket's upper bound (and not
/// below the previous one's) takes its term. Red wraps around 0°.
const HUE_BUCKETS: [(f64, ChromeMsg); 8] = [
    (15.0, ChromeMsg::ColourRed),
    (45.0, ChromeMsg::ColourOrange),
    (70.0, ChromeMsg::ColourYellow),
    (170.0, ChromeMsg::ColourGreen),
    (260.0, ChromeMsg::ColourBlue),
    (290.0, ChromeMsg::ColourViolet),
    (345.0, ChromeMsg::ColourPink),
    (360.0, ChromeMsg::ColourRed),
];

/// The colour term for `hex`: a lightness or saturation
/// extreme names black, white or grey, else the HSL hue's bucket names it.
/// Every `#rrggbb` gets a term, so no project needs a colour table. `None`
/// for a value that is not `#rrggbb`.
pub(crate) fn colour_term(hex: &str) -> Option<ChromeMsg> {
    let [r, g, b] = parse_hex(hex)?.map(|c| f64::from(c) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) / 2.0;
    if lightness <= BLACK_MAX_LIGHTNESS {
        return Some(ChromeMsg::ColourBlack);
    }
    if lightness >= WHITE_MIN_LIGHTNESS {
        return Some(ChromeMsg::ColourWhite);
    }
    let delta = max - min;
    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    if delta == 0.0 || saturation < CHROMATIC_MIN_SATURATION {
        return Some(ChromeMsg::ColourGrey);
    }
    let sector = if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    let hue = sector * 60.0;
    HUE_BUCKETS
        .iter()
        .find(|(upper, _)| hue < *upper)
        .map(|(_, term)| *term)
        .or(Some(ChromeMsg::ColourRed))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cpe_core::ir::{Property, PropertyValue, Resource};
    use cpe_core::lang::resolve_builtin;

    use super::*;

    fn lang(code: &str) -> Lang {
        resolve_builtin(&[code.to_owned()])
    }

    /// DataStar subscribes an effect to the signals its first run reads. That
    /// run can come before `<image-viewer>` is defined, so the signals are the
    /// outer call's argument, read on every run, and the state is handed over
    /// once the element is defined.
    #[test]
    fn overlay_effect_reads_its_signals_before_the_viewer_is_defined() {
        let expr = overlay_effect_expr("rv-I_P1");
        let (call, signals) = expr.rsplit_once(")(").expect("an outer call");
        assert_eq!(signals, format!("{{visible: $_annoShow, focus: {FOCUS_EXPR}}})"));
        assert!(!call.contains('$'), "a signal read inside the call may be skipped: {expr}");
        assert!(call.contains("customElements.whenDefined('image-viewer')"), "{expr}");
        assert!(
            call.contains("document.getElementById('rv-I_P1')?.setOverlayState?.(s)"),
            "{expr}"
        );
    }

    /// One Escape closes only the topmost layer, so the unpin skips
    /// while the header's language menu or its menu panel is open, as it
    /// does for the dataset panel; the panel test is guarded for engines
    /// without popovers.
    #[test]
    fn esc_unpin_skips_while_a_header_layer_is_open() {
        let escaped = |s: &str| {
            html! {
                (s)
            }
            .into_string()
        };
        let rendered = html! {
            div data-on:keydown=(esc_unpin_expr()) {}
        }
        .into_string();
        // Both skips sit inside the unpin's condition, before its body.
        let condition = rendered.split_once("{ $_annoPin = ''").expect("the unpin body").0;
        assert!(
            condition.contains(&escaped(" && !document.querySelector('.lang-menu[open]')")),
            "{rendered}"
        );
        assert!(
            condition.contains(&escaped(&format!(" && !{HEADER_MENU_OPEN_EXPR})"))),
            "{rendered}"
        );
        assert!(
            HEADER_MENU_OPEN_EXPR.contains(&format!("getElementById('{}')", cpe_core::markup_contract::HEADER_MENU_ID)),
            "the guard names the menu's id"
        );
    }

    #[test]
    fn the_datas_two_colours_get_their_terms_and_on_colours() {
        assert_eq!(colour_term("#ff3333"), Some(ChromeMsg::ColourRed));
        assert_eq!(colour_term("#3333ff"), Some(ChromeMsg::ColourBlue));
        assert_eq!(on_colour("#ff3333"), Some("#000"));
        assert_eq!(on_colour("#3333ff"), Some("#fff"));
        assert_eq!(ChromeMsg::ColourRed.text(&lang("de")), "Rot");
        assert_eq!(ChromeMsg::ColourBlue.text(&lang("de")), "Blau");
    }

    #[test]
    fn hue_bucket_boundaries() {
        // Fully saturated, mid-lightness colours at each boundary's two sides.
        let at = |hue: f64| {
            let h = hue / 60.0;
            let x = 1.0 - (h.rem_euclid(2.0) - 1.0).abs();
            let (r, g, b) = match h as u32 {
                0 => (1.0, x, 0.0),
                1 => (x, 1.0, 0.0),
                2 => (0.0, 1.0, x),
                3 => (0.0, x, 1.0),
                4 => (x, 0.0, 1.0),
                _ => (1.0, 0.0, x),
            };
            let byte = |c: f64| (c * 255.0).round() as u8;
            format!("#{:02x}{:02x}{:02x}", byte(r), byte(g), byte(b))
        };
        let cases = [
            (0.0, ChromeMsg::ColourRed),
            (14.0, ChromeMsg::ColourRed),
            (16.0, ChromeMsg::ColourOrange),
            (44.0, ChromeMsg::ColourOrange),
            (46.0, ChromeMsg::ColourYellow),
            (69.0, ChromeMsg::ColourYellow),
            (71.0, ChromeMsg::ColourGreen),
            (169.0, ChromeMsg::ColourGreen),
            (171.0, ChromeMsg::ColourBlue),
            (259.0, ChromeMsg::ColourBlue),
            (261.0, ChromeMsg::ColourViolet),
            (289.0, ChromeMsg::ColourViolet),
            (291.0, ChromeMsg::ColourPink),
            (344.0, ChromeMsg::ColourPink),
            (346.0, ChromeMsg::ColourRed),
            (359.0, ChromeMsg::ColourRed),
        ];
        for (hue, term) in cases {
            let hex = at(hue);
            assert_eq!(colour_term(&hex), Some(term), "hue {hue} ({hex})");
        }
        assert_eq!(colour_term("#33ff33"), Some(ChromeMsg::ColourGreen));
    }

    #[test]
    fn achromatic_and_extreme_lightness_terms() {
        assert_eq!(colour_term("#000000"), Some(ChromeMsg::ColourBlack));
        assert_eq!(colour_term("#1e0000"), Some(ChromeMsg::ColourBlack)); // l ≈ 0.06
        assert_eq!(colour_term("#ffffff"), Some(ChromeMsg::ColourWhite));
        assert_eq!(colour_term("#fff0f0"), Some(ChromeMsg::ColourWhite)); // l ≈ 0.97
        assert_eq!(colour_term("#808080"), Some(ChromeMsg::ColourGrey));
        // Saturation just under / over the chromatic threshold at l = 0.5.
        assert_eq!(colour_term("#8a7575"), Some(ChromeMsg::ColourGrey)); // s ≈ 0.08
        assert_eq!(colour_term("#a35c5c"), Some(ChromeMsg::ColourRed)); // s ≈ 0.28
    }

    #[test]
    fn malformed_hex_has_no_term_and_no_on_colour() {
        for bad in ["", "ff3333", "#ff333", "#ff33333", "#gg3333", "#+f+f+f"] {
            assert_eq!(colour_term(bad), None, "{bad}");
            assert_eq!(on_colour(bad), None, "{bad}");
        }
    }

    fn page_member(name: &str) -> Resource {
        Resource {
            id: format!("bk-{name}"),
            class: "Page".to_owned(),
            properties: vec![Property {
                name: "name".to_owned(),
                values: vec![PropertyValue::Text { text: name.to_owned(), lang: None }],
            }],
            representations: vec![],
        }
    }

    fn annotation(id: &str, shape: AnnotationShape, remark: Option<&str>) -> Annotation {
        Annotation {
            id: id.to_owned(),
            resource_id: "bk-c1v".to_owned(),
            shape,
            x: 0.1,
            y: 0.1,
            w: 0.5,
            h: 0.5,
            points: if shape == AnnotationShape::Polygon {
                vec![(0.1, 0.1), (0.6, 0.1), (0.3, 0.6)]
            } else {
                vec![]
            },
            color: Some("#ff3333".to_owned()),
            remark: remark.map(str::to_owned),
            remark_lang: remark.map(|_| "de".to_owned()),
        }
    }

    fn numbered(number: u32, annotation: Annotation) -> AnnotationEntry {
        AnnotationEntry::Annotation { number, annotation, partners: vec![] }
    }

    fn page_with(name: &str, annotations: Vec<AnnotationEntry>) -> CompoundPage {
        CompoundPage {
            member: Some(page_member(name)),
            index: 36,
            total: 328,
            members: vec![],
            member_media: vec![],
            stale_page: None,
            annotations,
            annotated_members: std::collections::BTreeSet::new(),
        }
    }

    const BOOKMARK: Bookmark<'static> = Bookmark {
        class_label: "Buch",
        id: "narrenschiff-dt",
        title: "Das Narrenschiff",
    };

    fn render_in(page: &CompoundPage, member_class_label: &str, lang: &Lang) -> String {
        let routes = HashMap::from([("Book".to_owned(), "/books/:id".to_owned())]);
        let links = LinkResolver::new(cpe_core::url::ROOT.project("incunabula"), &routes);
        render_annotations_target(page, member_class_label, &BOOKMARK, &links, lang).into_string()
    }

    fn render(page: &CompoundPage) -> String {
        render_in(page, "Seite", &lang("de"))
    }

    /// A partner on `page` of `book` (`None`: a resource outside any
    /// compound), through link `link_id` named `name`.
    fn partner(
        link_id: &str,
        name: &str,
        annotation: Option<(&str, u32)>,
        book: Option<(&str, &str)>,
        page: &str,
    ) -> AnnotationPartner {
        AnnotationPartner {
            link_id: link_id.to_owned(),
            link_name: name.to_owned(),
            link_name_lang: Some("de".to_owned()),
            annotation_id: annotation.map(|(id, _)| id.to_owned()),
            number: annotation.map(|(_, n)| n),
            resource_id: page.to_owned(),
            resource_class: if book.is_some() { "Page" } else { "Book" }.to_owned(),
            resource_label: page.rsplit('-').next().unwrap_or(page).to_owned(),
            parent: book.map(|(id, _)| cpe_core::ir::MemberParent {
                id: id.to_owned(),
                class: "Book".to_owned(),
                seq: 1,
            }),
            parent_title: book.map(|(_, title)| title.to_owned()),
            first_member: false,
        }
    }

    const LAT: Option<(&str, &str)> = Some(("narrenschiff-lat", "Narrenschiff (lat.)"));
    const DT: Option<(&str, &str)> = Some(("narrenschiff-dt", "Das Narrenschiff"));

    #[test]
    fn grouping_by_link_name_keeps_or_replaces_a_repeated_resource() {
        // (link name, resource, rank)
        let items = [("a", "r1", 2), ("b", "r2", 1), ("a", "r1", 1), ("a", "r3", 1)];
        let lines = |replace: bool| {
            group_by_link_name(&items, |i| i.0, |i| i.1, |new, kept| replace && new.2 < kept.2)
                .into_iter()
                .map(|(first, ps)| (first.0, ps.iter().map(|p| (p.1, p.2)).collect::<Vec<_>>()))
                .collect::<Vec<_>>()
        };
        assert_eq!(lines(false), vec![("a", vec![("r1", 2), ("r3", 1)]), ("b", vec![("r2", 1)])]);
        assert_eq!(lines(true), vec![("a", vec![("r1", 1), ("r3", 1)]), ("b", vec![("r2", 1)])]);
    }

    #[test]
    fn link_lines_group_by_name_and_keep_one_partner_per_page() {
        let partners = [
            partner("L1", "identisch", Some(("a2", 2)), LAT, "lat-c3r"),
            partner("L2", "ähnlich", Some(("b1", 1)), LAT, "lat-m5r"),
            partner("L3", "identisch", Some(("a1", 1)), LAT, "lat-c3r"),
            partner("L3", "identisch", None, LAT, "lat-c3r"),
            partner("L4", "identisch", Some(("c1", 1)), DT, "dt-o7v"),
        ];
        let lines = link_lines(&partners);
        let got: Vec<(&str, Vec<Option<&str>>)> = lines
            .iter()
            .map(|l| {
                let ids = l.partners.iter().map(|p| p.annotation_id.as_deref());
                (l.name, ids.collect())
            })
            .collect();
        assert_eq!(
            got,
            vec![
                ("identisch", vec![Some("a1"), Some("c1")]),
                ("ähnlich", vec![Some("b1")]),
            ]
        );
    }

    #[test]
    fn a_partner_book_without_a_name_reads_its_id() {
        let mut nameless = partner("L1", "identisch", Some(("a1", 1)), LAT, "lat-c3r");
        nameless.parent_title = None;
        let mut same = partner("L2", "identisch", Some(("a2", 1)), DT, "dt-o7v");
        same.parent_title = None;
        let page = page_with(
            "b4v",
            vec![AnnotationEntry::Annotation {
                number: 1,
                annotation: annotation("w", AnnotationShape::Rectangle, None),
                partners: vec![nameless, same],
            }],
        );
        let out = render(&page);
        assert!(out.contains(r#"<em class="anno__book">narrenschiff-lat</em>"#), "{out}");
        assert!(
            out.contains(r#"<em class="anno__book">diese Ausgabe</em>"#),
            "the shown book reads 'this book': {out}"
        );
    }

    /// Each partner link in a reader names its page, its first-page flag and
    /// the finished arrival note; the shown book's own pages are named by its
    /// title there, not "diese Ausgabe"; an unrouted partner carries nothing.
    #[test]
    fn partner_links_carry_the_arrival_data() {
        let mut first = partner("L1", "identisch", Some(("x1", 1)), LAT, "lat-a1r");
        first.first_member = true;
        let same = partner("L2", "identisch", Some(("x2", 2)), DT, "narrenschiff-dt-o7v");
        let mut unrouted = partner("L3", "identisch", None, None, "ms-12");
        unrouted.resource_class = "Manuscript".to_owned();
        let numbered_page = page_with(
            "b4v",
            vec![AnnotationEntry::Annotation {
                number: 3,
                annotation: annotation("w", AnnotationShape::Rectangle, None),
                partners: vec![first, same, unrouted],
            }],
        );
        let out = render(&numbered_page);
        assert!(
            out.contains(
                r#"data-anno-page="lat-a1r" data-anno-first data-anno-note="Von Narrenschiff (lat.) · a1r – Annotation 3 hervorgehoben.""#
            ),
            "{out}"
        );
        assert!(
            out.contains(
                r#"data-anno-page="narrenschiff-dt-o7v" data-anno-note="Von Das Narrenschiff · o7v – Annotation 3 hervorgehoben.""#
            ),
            "{out}"
        );
        assert_eq!(out.matches("data-anno-page=").count(), 2, "{out}");

        let page_level = page_with(
            "a3r",
            vec![AnnotationEntry::Link {
                link: AnnotationLink {
                    id: "L1".to_owned(),
                    name: "identisch".to_owned(),
                    name_lang: None,
                    comment: None,
                    comment_lang: None,
                },
                partners: vec![partner("L1", "identisch", None, LAT, "walfart-d6v")],
            }],
        );
        assert!(
            render(&page_level).contains(r#"data-anno-note="Von Narrenschiff (lat.) · d6v.""#),
            "a page-level entry's note names no annotation"
        );
    }

    /// The arrival expression, pinned as a string: it must read the entry
    /// ids `entry_id` writes and the partner data `partner` writes, and
    /// write neither the URL nor any storage.
    #[test]
    fn snapshot_arrival_init_expr() {
        // The register's id and the entry-id prefix are spelled as literals;
        // they must match what is rendered. (`READER_VIEWER_EXPR`'s reader id
        // is checked against a rendered reader in `compound_viewer`'s tests.)
        assert!(
            ARRIVAL_INIT_EXPR.contains(&format!("'#{REGISTER_ID} .anno__entry'")),
            "{ARRIVAL_INIT_EXPR}"
        );
        assert!(
            ARRIVAL_INIT_EXPR.contains(&format!("'{}'.length", entry_id(""))),
            "{ARRIVAL_INIT_EXPR}"
        );
        // Each scroll sits under the layout test that selects it, reading
        // the reader and the tracks the record renders.
        let title = crate::component::dom::PAGES_TITLE_ID;
        for branch in [
            "document.getElementById('compound-viewer')".to_owned(),
            "el.querySelector('.record__tracks')".to_owned(),
            "getComputedStyle(reader).position === 'sticky') at.scrollIntoView({block: 'end'})".to_owned(),
            format!(
                "gridTemplateColumns.split(' ').length === 1) \
                 document.getElementById('{title}')?.parentElement?.scrollIntoView({{block: 'start'}})"
            ),
        ] {
            assert!(ARRIVAL_INIT_EXPR.contains(&branch), "{branch}");
        }
        // Both scrolls come after the frame callback opens, after the note
        // has filled above the entries, and only on a `navigate` load.
        let frame = ARRIVAL_INIT_EXPR
            .find("if (nav?.type === 'navigate') requestAnimationFrame(() => {")
            .expect("the deferred scroll");
        assert!(ARRIVAL_INIT_EXPR.find("$_annoNote =").expect("the note") < frame);
        let scrolls: Vec<usize> = ARRIVAL_INIT_EXPR.match_indices("scrollIntoView").map(|(i, _)| i).collect();
        assert_eq!(scrolls.len(), 2, "{ARRIVAL_INIT_EXPR}");
        assert!(scrolls.iter().all(|&i| i > frame), "{ARRIVAL_INIT_EXPR}");
        for read in [
            "a[data-anno-page]",
            "a.dataset.annoPage",
            "'annoFirst' in a.dataset",
            "from.dataset.annoNote",
        ] {
            assert!(ARRIVAL_INIT_EXPR.contains(read), "{read}");
        }
        for never in ["history", "Storage", "location.hash =", "location.href =", "behavior"] {
            assert!(!ARRIVAL_INIT_EXPR.contains(never), "{never}");
        }
        insta::assert_snapshot!(ARRIVAL_INIT_EXPR);
    }

    /// The register carries the empty arrival note, bound to `$_annoNote`,
    /// and the toggle and a pin clear it.
    #[test]
    fn the_register_carries_the_arrival_note_and_clears_it() {
        let out = render(&page_with(
            "c1v",
            vec![numbered(1, annotation("a", AnnotationShape::Rectangle, None))],
        ));
        assert!(
            out.contains(r#"</header><p class="anno__arrival" role="status" data-text="$_annoNote"></p><ol"#),
            "{out}"
        );
        assert!(
            out.contains(r#"data-on:click="$_annoShow = !$_annoShow; $_annoNote = ''""#),
            "{out}"
        );
        assert!(
            pin_toggle_expr("'a'", "v").contains("$_annoPin = 'a'; $_annoNote = '';"),
            "a pin clears the note"
        );
    }

    #[test]
    fn a_page_partner_ranks_after_an_annotation_on_the_same_page() {
        let partners = [
            partner("L1", "identisch", None, LAT, "lat-c3r"),
            partner("L2", "identisch", Some(("a9", 9)), LAT, "lat-c3r"),
        ];
        let lines = link_lines(&partners);
        assert_eq!(lines[0].partners.len(), 1);
        assert_eq!(lines[0].partners[0].annotation_id.as_deref(), Some("a9"));
    }

    /// A woodcut used four times: one name over three partner pages, one of
    /// them in the shown book, the duplicate link to c3r collapsed.
    #[test]
    fn snapshot_register_four_times_woodcut() {
        let entry = AnnotationEntry::Annotation {
            number: 1,
            annotation: annotation("w", AnnotationShape::Rectangle, Some("Narr")),
            partners: vec![
                partner("L1", "identischer Holzschnitt", Some(("x1", 1)), LAT, "lat-c3r"),
                partner("L1", "identischer Holzschnitt", Some(("x2", 1)), LAT, "lat-m5r"),
                partner("L1", "identischer Holzschnitt", Some(("x3", 2)), DT, "narrenschiff-dt-o7v"),
                partner("L2", "identischer Holzschnitt", Some(("x1", 1)), LAT, "lat-c3r"),
            ],
        };
        insta::assert_snapshot!(render(&page_with("b4v", vec![entry])));
    }

    /// A page-level entry's partner lands on the partner page's own
    /// page-level entry (the link's id); a partner with no route is text.
    #[test]
    fn snapshot_register_page_level_entry_with_partners() {
        let mut unrouted = partner("L1", "identischer Holzschnitt", None, None, "ms-12");
        unrouted.resource_class = "Manuscript".to_owned();
        let page = page_with(
            "a3r",
            vec![AnnotationEntry::Link {
                link: AnnotationLink {
                    id: "L1".to_owned(),
                    name: "identischer Holzschnitt".to_owned(),
                    name_lang: Some("de".to_owned()),
                    comment: Some("Gleicher Holzschnitt wie d6v".to_owned()),
                    comment_lang: Some("de".to_owned()),
                },
                partners: vec![
                    partner("L1", "identischer Holzschnitt", None, LAT, "walfart-d6v"),
                    unrouted,
                ],
            }],
        );
        let out = render(&page);
        assert!(
            out.contains(r#"href="/incunabula/books/narrenschiff-lat?page=walfart-d6v#annotation-L1""#),
            "{out}"
        );
        assert!(
            out.contains(r#"<span class="anno__partner"><em class="anno__book">12</em></span>"#),
            "{out}"
        );
        insta::assert_snapshot!(out);
    }

    #[test]
    fn unannotated_page_renders_the_empty_target() {
        assert_eq!(render(&page_with("c2r", vec![])), r#"<div id="cv-annotations"></div>"#);
    }

    /// c1v-shaped: two nested shapes, the containing rectangle numbered first.
    #[test]
    fn snapshot_register_two_nested_shapes() {
        let page = page_with(
            "c1v",
            vec![
                numbered(1, annotation("outer", AnnotationShape::Rectangle, Some("Holzschnitt"))),
                numbered(2, annotation("inner", AnnotationShape::Polygon, Some("Engel"))),
            ],
        );
        insta::assert_snapshot!(render(&page));
    }

    /// o6r-shaped: a rectangle and a circle, one in blue, one without a remark.
    #[test]
    fn snapshot_register_rectangle_and_circle() {
        let mut circle = annotation("circle", AnnotationShape::Circle, None);
        circle.color = Some("#3333ff".to_owned());
        let page = page_with(
            "o6r",
            vec![
                numbered(1, annotation("rect", AnnotationShape::Rectangle, Some("Rot"))),
                numbered(2, circle),
            ],
        );
        insta::assert_snapshot!(render(&page));
    }

    #[test]
    fn snapshot_register_single_polygon_in_english() {
        let page = page_with(
            "b3v",
            vec![numbered(1, annotation("poly", AnnotationShape::Polygon, Some("Bird")))],
        );
        insta::assert_snapshot!(render_in(&page, "Page", &lang("en")));
    }

    #[test]
    fn snapshot_register_page_level_entry() {
        let page = page_with(
            "a3r",
            vec![AnnotationEntry::Link {
                link: AnnotationLink {
                    id: "L1".to_owned(),
                    name: "identischer Holzschnitt".to_owned(),
                    name_lang: Some("de".to_owned()),
                    comment: Some("Gleicher Holzschnitt wie d6v".to_owned()),
                    comment_lang: Some("de".to_owned()),
                },
                partners: vec![],
            }],
        );
        insta::assert_snapshot!(render(&page));
    }

    fn boxed(shape: AnnotationShape, (x, y, w, h): (f64, f64, f64, f64)) -> Annotation {
        Annotation { x, y, w, h, points: vec![], ..annotation("a", shape, None) }
    }

    fn polygon(id: &str, points: &[(f64, f64)]) -> Annotation {
        let (xs, ys): (Vec<f64>, Vec<f64>) = points.iter().copied().unzip();
        let min = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
        let max = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Annotation {
            x: min(&xs),
            y: min(&ys),
            w: max(&xs) - min(&xs),
            h: max(&ys) - min(&ys),
            points: points.to_vec(),
            ..annotation(id, AnnotationShape::Polygon, None)
        }
    }

    #[test]
    fn a_rectangle_tag_starts_on_its_top_left_corner() {
        let anchor = tag_anchor(&boxed(AnnotationShape::Rectangle, (0.17, 0.15, 0.4, 0.3)));
        assert_eq!(anchor, TagAnchor { x: 0.17, y: 0.15, align: TagAlign::Start, below: false });
    }

    #[test]
    fn a_circle_tag_centres_on_its_top_point() {
        let anchor = tag_anchor(&boxed(AnnotationShape::Circle, (0.2, 0.6, 0.1, 0.06)));
        assert!((anchor.x - 0.25).abs() < 1e-12, "{anchor:?}");
        assert_eq!((anchor.y, anchor.align), (0.6, TagAlign::Center));
    }

    #[test]
    fn a_polygon_tag_centres_on_its_topmost_vertex_leftmost_on_a_tie() {
        let a = polygon("p", &[(0.6, 0.3), (0.5, 0.14), (0.3, 0.2), (0.4, 0.14), (0.7, 0.5)]);
        let anchor = tag_anchor(&a);
        assert_eq!((anchor.x, anchor.y, anchor.align), (0.4, 0.14, TagAlign::Center));
    }

    #[test]
    fn a_tag_in_the_top_five_percent_hangs_below() {
        let at = |y: f64| tag_anchor(&boxed(AnnotationShape::Rectangle, (0.1, y, 0.2, 0.2))).below;
        assert!(at(0.0));
        assert!(at(0.049));
        assert!(!at(0.05));
        assert!(!at(0.5));
    }

    #[test]
    fn view_box_coordinates_have_three_decimals() {
        assert_eq!(view_box(0.17), "17.000");
        assert_eq!(view_box(0.123_456), "12.346");
        assert_eq!(view_box(0.0), "0.000");
        assert_eq!(view_box(1.0), "100.000");
        assert_eq!(view_box(-5e-10), "0.000");
        assert_eq!(view_box(-0.0), "0.000");
        assert_eq!(view_box(1.0 + 5e-10), "100.000");
    }

    #[test]
    fn a_page_without_numbered_entries_has_no_overlay() {
        assert!(render_overlay(&[]).is_none());
        let page_level = AnnotationEntry::Link {
            link: AnnotationLink {
                id: "L1".to_owned(),
                name: "identischer Holzschnitt".to_owned(),
                name_lang: None,
                comment: None,
                comment_lang: None,
            },
            partners: vec![],
        };
        assert!(render_overlay(&[page_level]).is_none());
    }

    fn overlay(entries: &[AnnotationEntry]) -> String {
        render_overlay(entries).expect("an overlay").into_string()
    }

    /// c1v-shaped: a rectangle holding a polygon; the containing shape (1)
    /// paints first, so the nested one wins the hit.
    #[test]
    fn snapshot_overlay_two_nested_shapes() {
        let outer = Annotation {
            id: "outer".to_owned(),
            ..boxed(AnnotationShape::Rectangle, (0.17, 0.15, 0.6, 0.5))
        };
        let inner = polygon("inner", &[(0.47, 0.18), (0.6, 0.3), (0.5, 0.45), (0.35, 0.32)]);
        let out = overlay(&[numbered(1, outer), numbered(2, inner)]);
        assert!(
            out.find(r#"data-iv-shape="outer""#) < out.find(r#"data-iv-shape="inner""#),
            "{out}"
        );
        insta::assert_snapshot!(out);
    }

    /// o6r-shaped: a red rectangle at the top edge (its tag hangs below) and
    /// a blue circle drawn as an ellipse.
    #[test]
    fn snapshot_overlay_rectangle_and_circle() {
        let rect = Annotation {
            id: "rect".to_owned(),
            ..boxed(AnnotationShape::Rectangle, (0.1, 0.02, 0.5, 0.3))
        };
        let circle = Annotation {
            id: "circle".to_owned(),
            color: Some("#3333ff".to_owned()),
            ..boxed(AnnotationShape::Circle, (0.6, 0.6, 0.092, 0.062))
        };
        insta::assert_snapshot!(overlay(&[numbered(1, rect), numbered(2, circle)]));
    }

    #[test]
    fn snapshot_overlay_single_polygon() {
        let poly = polygon("poly", &[(0.55, 0.5), (0.8, 0.58), (0.74, 0.82), (0.5, 0.76), (0.46, 0.6)]);
        insta::assert_snapshot!(overlay(&[numbered(1, poly)]));
    }

    #[test]
    fn a_colourless_shape_carries_only_its_position() {
        let mut bare = boxed(AnnotationShape::Rectangle, (0.1, 0.1, 0.2, 0.2));
        bare.color = None;
        let out = overlay(&[numbered(1, bare)]);
        assert!(
            !out.contains("--iv-c") && out.contains(r#"style="left:10.000%;top:10.000%""#),
            "{out}"
        );
    }

    #[test]
    fn a_colourless_annotation_has_no_swatch_and_no_style() {
        let mut bare = annotation("bare", AnnotationShape::Rectangle, None);
        bare.color = None;
        let out = render(&page_with("c1v", vec![numbered(1, bare)]));
        assert!(!out.contains("anno__swatch") && !out.contains("--iv-c"), "{out}");
    }

    #[test]
    fn an_unlabelled_page_is_named_by_its_position() {
        let mut page = page_with("c1v", vec![numbered(1, annotation("a", AnnotationShape::Rectangle, None))]);
        page.member = Some(Resource {
            id: "p".to_owned(),
            class: "Page".to_owned(),
            properties: vec![],
            representations: vec![],
        });
        assert!(render(&page).contains("Annotationen auf 36</h3>"));
    }
}
