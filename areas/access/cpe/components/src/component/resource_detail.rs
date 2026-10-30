//! Renderer for the `resource-detail` component.
//!
//! Takes a domain `Resource` and renders an `<article class="record">` with
//! the record archetype: a `.record__head` (eyebrow, title, optional quote),
//! a `.record__body` grid (optional figure + property list), and per-value-type
//! treatments for every `PropertyValue` variant. The renderer is stateless and
//! DB-free — the store fetches the `Resource` (`Project::page_data`) before
//! `render_resource_detail` is called.
//!
//! ## Value-type vocabulary (mirrors `record.css`)
//! - `Text` URL → `a.ext-link` (https/http/relative; scheme allowlist rejects `javascript:`/`data:`
//!   → falls back to `span.v-text`).
//! - `Text` plain → `span.v-text`.
//! - `Richtext` → `div.v-richtext` wrapping the markdown fragment.
//! - `Integer`/`Decimal` → `span.v-number`.
//! - `Boolean` → `span.v-bool[data-bool]` + localised Yes/No | Ja/Nein.
//! - `Date` → `span.v-date`.
//! - `Link { target_label: Some }` → `<a class="int-link">` when the target class is bound in the
//!   `LinkResolver`; otherwise falls back to `span.v-text` plain.
//! - `Link { target_label: None, target_class: None }` (target does not exist) → `span.unresolved`
//!   with the mono UNRESOLVED mark.
//! - `Link { target_label: None, target_class: Some(_) }` (target exists, just has no `name`) →
//!   plain `code` — this value resolves.
//! - `ListValue` (multi) → `ul.keyword-set > li.keyword` (path + leaf split).
//! - `ListValue` (single) → `span.term`.

use std::collections::HashMap;

use cpe_core::ir::{
    DatePrecision, IncomingLink, IncomingLinks, MemberHeader, Property, PropertyValue, Resource, ResourceLink,
    SiteConfig,
};
use cpe_core::lang::Lang;
use cpe_core::util::is_safe_external_url;
use maud::{html, Markup};

use crate::chrome::{bool_labels, nf_browse_all_text, nf_home_text, nf_id_msg_text, nf_id_title_text, ChromeMsg};
use crate::component::links::LinkResolver;
use crate::component::markdown::render_richtext_fragment;
use crate::component::values::{
    composed_part_texts, format_date_text, pick_values, resolve_title_str, split_keyword_label, with_calendar_word,
};

// Config borrow struct

/// One resolved config property group.
#[derive(Debug, Clone, Copy)]
pub struct GroupCfg<'a> {
    /// The resolved caption (chrome id or `ChromeMsg::PropertyGroupTitle`).
    pub caption: &'a str,
    /// Member property names, in display order.
    pub properties: &'a [String],
    /// The resolved `empty=` sentence, shown under the caption when every
    /// member is absent on the record. `None` ⇒ an all-absent group renders
    /// nothing.
    pub empty: Option<&'a str>,
    /// The closing links row (`group links-label=`).
    /// `None` ⇒ no row: undeclared, or the record has no resource-level
    /// link partners.
    pub links: Option<GroupLinks<'a>>,
}

/// A group's closing row of resource-level annotation-link partners.
#[derive(Debug, Clone, Copy)]
pub struct GroupLinks<'a> {
    /// The resolved `links-label=` caption (chrome id, else
    /// `ChromeMsg::AnnoRelated`). Singular whatever the partner count.
    pub caption: &'a str,
    /// The partners (`ProjectReads::resource_links`), non-empty.
    pub links: &'a [ResourceLink],
}

/// A record's header slots ([`cpe_core::ir::HeadSlots`]) with the imprint's
/// `unit=` chrome ids resolved for one request language, as
/// `render_component` builds them.
#[doc(hidden)]
pub struct ResolvedHead<'a> {
    #[doc(hidden)]
    pub alt_title: Option<&'a str>,
    #[doc(hidden)]
    pub author: Option<&'a str>,
    #[doc(hidden)]
    pub imprint: Vec<cpe_core::labels::ResolvedGlossPart<'a>>,
}

impl ResolvedHead<'_> {
    /// Whether `name` is one of the slot properties.
    fn names(&self, name: &str) -> bool {
        self.alt_title == Some(name) || self.author == Some(name) || self.imprint.iter().any(|p| p.property == name)
    }
}

/// Borrowed slice of the IR fields needed by `render_resource_detail`.
///
/// Holds references into the validated `Component::ResourceDetail` variant,
/// so no IR cloning occurs in the renderer.
pub struct ResourceDetailCfg<'a> {
    pub id: &'a str,
    pub title_property: Option<&'a str>,
    pub show_properties: &'a [String],
    /// The class's media link property name,
    /// resolved by the caller from `RenderContext.media_properties`. When
    /// `Some`, excluded from the `.props` list (the record viewer renders it
    /// separately via `media_items`); `render_property_block` itself never
    /// learns the class.
    pub media_property: Option<&'a str>,
    /// Pre-hydrated Image resources linked via `media_property`,
    /// in seqnum order. Empty when `media_property` is `None` or
    /// the property has no values — the record viewer (`record--viewer`) is
    /// only rendered when non-empty.
    pub media_items: &'a [Resource],
    /// Selected image index (from `?img=<index>`, **1-based**). Renderer clamps
    /// to valid range.
    pub selected_image: usize,
    /// Thumb-strip fold state (`?strip=`, default open).
    pub strip_open: bool,
    /// Lightweight member headers for `resource`, when it is itself a compound
    /// parent (`.rv-contents`). Empty for
    /// every non-compound host (the common case — Alice's Characters,
    /// Locations, and StoryChapters are never compound parents), in which case
    /// `.rv-contents` is omitted entirely. Pre-hydrated by the store
    /// (`PageData.compound_members`); this renderer stays DB-free.
    pub compound_members: &'a [MemberHeader],
    /// URI path of the current request (e.g. `/alice-in-daschland/characters/CH_001`).
    /// Used to build the record viewer's prev/next `.pg` pager hrefs.
    pub current_path: &'a str,
    /// Raw query string of the current request (everything after `?`, no leading `?`).
    /// Used to build the record viewer's hrefs that preserve lang/appearance params.
    pub current_query: Option<&'a str>,
    /// Optional resolver for turning `Link` values into clickable `<a class="int-link">`.
    ///
    /// `None` → all links fall back to `span.v-text`.
    /// `Some(resolver)` → links whose target class is bound in the map become
    /// clickable; those without a binding still fall back to `span.v-text`.
    #[doc(hidden)]
    pub links: Option<LinkResolver<'a>>,
    /// Declared property labels (`labels { property "…" { … } }`), used to
    /// resolve each shown property's `div.prop__label` text. Passed to
    /// `resolve_property_label` for every rendered `.prop` block; a property
    /// with no matching declared label (or no translation for the current
    /// language) falls back to `humanize_property_name`. Empty slice ⇒ every
    /// label is humanized.
    pub property_labels: &'a [cpe_core::ir::PropertyLabel],
    /// Declared class labels (`labels { class "…" { … } }`), used to resolve
    /// the header eyebrow (`p.eyebrow`) text. Passed to `resolve_class_label`;
    /// a class with no matching declared label (or no translation for the
    /// current language) falls back to `humanize_property_name`. Empty slice
    /// ⇒ the eyebrow is humanized.
    pub class_labels: &'a [cpe_core::ir::ClassLabel],
    /// Config-declared property groups, in
    /// declaration order: see `GroupCfg` (resolved caption, member property
    /// names, optional resolved empty sentence). The
    /// caller resolves each `group label=` chrome id (falling back to
    /// `ChromeMsg::PropertyGroupTitle`). Each non-empty group renders as a
    /// `section.pgroup` after the flat `div.props`. Empty ⇒ the group-less behaviour
    /// exactly; non-empty with an empty `show_properties` ⇒ the flat list is
    /// EMPTY and `div.props` is omitted (not "every property").
    pub groups: &'a [GroupCfg<'a>],
    /// The record's rendered intro narrative (
    /// `narrative-for`), already language-resolved and rendered to a
    /// `div.prose` by `render_component`. The **one owned field** of this
    /// otherwise borrowed struct, deliberately: the HTML is produced in the
    /// caller's arm and must not become a `&str` over a buffer that dies
    /// there. `Some` ⇒ `div.record__intro` is the body's first child, the rest
    /// is wrapped in `div.record__apparat`, and the body gains
    /// `record__body--intro`; `None` ⇒ the body markup is unchanged.
    pub intro: Option<Markup>,
    /// Whether this record is paired with a same-id `compound-viewer` on its
    /// page — the boot fact `Component::ResourceDetail.navigator` (`dsp-incubator
    /// cpe/docs/adr/0012-one-member-navigator-declared-layout.md`). `true` ⇒ `.rv-contents` does
    /// not render (the navigator is the one member list); `false` ⇒ a compound
    /// parent lists its members in `.rv-contents`.
    pub navigator: bool,
    /// The paired `layout="pages"` reader, already rendered by `render_body`
    /// (`dsp-incubator cpe/docs/adr/0012-one-member-navigator-declared-layout.md`'s 2026-09-29
    /// amendment). `Some` ⇒ the article gains `record--reader` and wraps head, reader and body
    /// in `div.record__tracks`, in that DOM order: the stacked order is the
    /// mobile order, and the `book` container query (record.css) moves the
    /// reader into the right-hand track. `None` ⇒ the markup is unchanged.
    /// Owned, like `intro`, for the same reason.
    pub reader: Option<Markup>,
    /// The paired reader's annotation patch target (`div#cv-annotations`),
    /// already rendered by `render_body`. Placed right after `reader` inside
    /// `div.record__tracks`, so its DOM order follows the stacked order.
    /// Ignored when `reader` is `None`.
    pub annotations: Option<Markup>,
    /// The record's incoming-links list — see [`IncomingCfg`]. Renders as
    /// `section.pgroup.incoming` after the last config group. `None` ⇒
    /// nothing (no spec, or the lookup failed).
    #[doc(hidden)]
    pub incoming: Option<IncomingCfg<'a>>,
    /// `site.date_precision_word`: append
    /// the `span.prec` precision word to every date value this record
    /// renders. `false` ⇒ dates render exactly as before. In the imprint it
    /// adds only the Gregorian calendar marker, never the precision word.
    pub date_precision_word: bool,
    /// The header slots (alt title, author, imprint). `None` ⇒ the header
    /// renders as it does without slots.
    #[doc(hidden)]
    pub head: Option<ResolvedHead<'a>>,
}

/// Everything [`render_incoming_links`] needs, resolved by `render_component`
/// (one carrier, so neither the renderer nor its call site
/// grows an argument list).
///
/// Row options (`row-label`/`row-aside`) need no field here: the query layer
/// fills [`IncomingLink::label`]/[`IncomingLink::aside`] only when the spec
/// declares them, so the rows themselves carry the choice.
#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct IncomingCfg<'a> {
    /// The resolved caption (`chrome_lookup` of the spec's `label`, else the
    /// source class's capitalized plural).
    #[doc(hidden)]
    pub caption: &'a str,
    /// The rows the store fetched. `rows: vec![]` ⇒ ungrouped: the caption +
    /// the empty sentence; grouped: nothing at all.
    /// `total` is the uncapped match count — an "N of M" line renders only
    /// when `rows.len() < total`, in both modes.
    #[doc(hidden)]
    pub links: &'a IncomingLinks,
    /// The spec's declared link properties, in declaration order (the `via`
    /// group order and the `via`-marker suppression rule).
    #[doc(hidden)]
    pub properties: &'a [String],
    /// `Some` ⇒ the grouped list (`group-by=`); `None` ⇒ today's flat list.
    #[doc(hidden)]
    pub grouping: Option<&'a cpe_core::ir::IncomingGrouping>,
    /// The resolved `note-label` template for this language (contains `{n}`,
    /// substituted with the row count). Ignored when `grouping` is `None`.
    #[doc(hidden)]
    pub note: Option<&'a str>,
}

// Public renderer entry points

/// The header slots under the record's `h1`, in order: `p.record__alt`,
/// `p.record__author`, `p.record__imprint`, each only when it has text. The
/// alt title's and the author's language-picked values join with ", "; the
/// imprint is [`composed_part_texts`] joined with
/// " · ", its dates carrying the Gregorian marker when `calendar_marker`.
fn render_head_slots(resource: &Resource, head: &ResolvedHead<'_>, calendar_marker: bool, lang: &Lang) -> Markup {
    let text_of = |name: Option<&str>| -> Option<String> {
        let property = resource.properties.iter().find(|p| Some(p.name.as_str()) == name)?;
        let values: Vec<&str> = pick_values(property, lang)
            .into_iter()
            .filter_map(|v| match v {
                PropertyValue::Text { text, .. } | PropertyValue::Richtext { text, .. } if !text.trim().is_empty() => {
                    Some(text.as_str())
                }
                _ => None,
            })
            .collect();
        (!values.is_empty()).then(|| values.join(", "))
    };
    let alt = text_of(head.alt_title);
    let author = text_of(head.author);
    let imprint = composed_part_texts(resource, &head.imprint, calendar_marker, lang).join(" · ");
    html! {
        @if let Some(alt) = alt {
            p.record__alt { (alt) }
        }
        @if let Some(author) = author {
            p.record__author { (author) }
        }
        @if !imprint.is_empty() {
            p.record__imprint { (imprint) }
        }
    }
}

/// Render a `Resource` as an `<article class="record page">` element.
/// (`.page` supplies the shared max-width + gutter + centering box, same as the
/// editorial home; `.record` adds the archetype-specific layout/typography.)
///
/// Structure:
/// - `header.record__head`: eyebrow `.record__class` (humanised class name + `.record__id`),
///   `h1.record__title`, optional `blockquote.record__quote` (when the resource has a `quote`
///   property with a renderable value).
/// - `div.record__body` (`.record__body--noimg` modifier when no image Representation): the record
///   viewer then `div.props`.
/// - Each shown property → `div.prop > div.prop__label + div.prop__value`.
///
/// **No double-render:** the `title_property` and a `quote` property (if
/// present) are excluded from the `.props` list — they render in
/// `.record__head`.
///
/// Image representations render via the inline record viewer
/// (`component::media`), not a `.record__figure` on this component — a
/// resource with no `media` declared gets `.record__body--noimg` and no
/// figure slot at all. Audio, video, and document representations are
/// silently skipped.
pub fn render_resource_detail(resource: &Resource, cfg: ResourceDetailCfg<'_>, lang: &Lang) -> Markup {
    let title = resolve_title_str(resource, cfg.title_property, lang);

    // Determine the set of properties excluded from .props (title + quote).
    let title_prop_name: Option<&str> = cfg.title_property;

    // Find the quote property (if any) and extract its first renderable value
    // for the blockquote.
    let quote_text: Option<String> = resource.properties.iter().find(|p| p.name == "quote").and_then(|p| {
        let values = pick_values(p, lang);
        for v in values {
            match v {
                PropertyValue::Text { text, .. } if !text.is_empty() => {
                    return Some(text.clone());
                }
                PropertyValue::Richtext { text, .. } if !text.is_empty() => {
                    return Some(text.clone());
                }
                _ => {}
            }
        }
        None
    });

    // Determine property render order, excluding title_property, "quote", and
    // the media link property (the record viewer
    // renders it separately, never as a plain property row).
    let excluded = |name: &str| title_prop_name == Some(name) || name == "quote" || cfg.media_property == Some(name);
    // Once any `group` is declared, an absent/empty `show-properties`
    // means "no flat rows" — never "every property", or grouped members would
    // render twice (flat and under their cap).
    let ordered: Vec<&Property> = if cfg.show_properties.is_empty() && !cfg.groups.is_empty() {
        Vec::new()
    } else if cfg.show_properties.is_empty() {
        resource
            .properties
            .iter()
            .filter(|p| {
                title_prop_name.is_none_or(|n| p.name != n)
                    && p.name != "quote"
                    && cfg.media_property.is_none_or(|mp| p.name != mp)
                    && !cfg.head.as_ref().is_some_and(|h| h.names(&p.name))
            })
            .collect()
    } else {
        cfg.show_properties
            .iter()
            .filter(|name| {
                (title_prop_name != Some(name.as_str()))
                    && name.as_str() != "quote"
                    && cfg.media_property.is_none_or(|mp| name.as_str() != mp)
            })
            .filter_map(|name| resource.properties.iter().find(|p| &p.name == name))
            .collect()
    };

    // Config property groups: same title/quote/media exclusion as the
    // flat list; a group with no present member renders nothing.
    let groups_markup: Vec<Markup> = cfg
        .groups
        .iter()
        .map(|group| {
            let props: Vec<&Property> = group
                .properties
                .iter()
                .filter(|name| !excluded(name.as_str()))
                .filter_map(|name| resource.properties.iter().find(|p| &p.name == name))
                .collect();
            render_property_group(
                group,
                &props,
                lang,
                cfg.property_labels,
                cfg.links.as_ref(),
                cfg.date_precision_word,
            )
        })
        .collect();
    // With no groups the flat `div.props` is always emitted;
    // with groups it is omitted when it has no rows.
    let emit_flat_props = cfg.groups.is_empty() || !ordered.is_empty();

    let has_media = !cfg.media_items.is_empty();

    let body_class = if has_media {
        "record__body"
    } else {
        "record__body record__body--noimg"
    };
    // Automatic, scoped to the resource-detail
    // media column, no config attribute — every resource-detail with
    // media renders `record--viewer` (the deep-zoom viewer) unconditionally.
    // The viewer's own per-image `.fallback` markup (component::media) is the
    // sole zero-JS/pre-upgrade path.
    // A pages reader forbids `media=` on the same record
    // (`MultipleImageViewers`), so `record--reader` never meets
    // `record--viewer`.
    // The bare `record page` is the plain record: `grid.css` and `media.css`
    // select it as `.record:not(.record--reader, .record--viewer)`, so a
    // third variant must join that list or it is laid out as plain.
    let article_class = if cfg.reader.is_some() {
        "record page record--reader"
    } else if has_media {
        "record page record--viewer"
    } else {
        "record page"
    };
    // WIDER. The expand state is a CSS *class*
    // (`.is-wide`, toggled by DataStar from the `$expand` boolean signal), NOT a
    // reactive `data-expand` attribute — reactively writing a `data-*` attribute
    // collides with DataStar's own `data-*` MutationObserver and silently
    // derails the whole binding pass (see media.rs `render_media_viewer`). Gated on
    // `has_media`: only a viewer record carries the `$expand` signal (declared
    // in media.rs's `.rv` block) and the WIDER toggle.
    let wide_class: Option<&str> = has_media.then_some("{'is-wide': $expand}");
    // Static, never written reactively (the trap above is about reactive
    // writes). Only a viewer record has a frame to size.
    let tall_frame = (cfg.reader.is_none() && super::media::frame_is_tall(cfg.media_items)).then_some("tall");
    // The head + body are made GENUINELY inert while expanded, not merely
    // `display:none`'d in CSS — the binding is `media::HIDDEN_WHEN_WIDE_ATTR`,
    // shared with the compound viewer's `pages` reader so the two WIDER hosts
    // cannot drift (see its doc comment for the `'true'`/`false` rationale).
    let hidden_when_wide: Option<&str> = has_media.then_some(super::media::HIDDEN_WHEN_WIDE_ATTR);

    // Resolve the class label for the eyebrow: declared translation for
    // `lang` when `labels { class "…" { … } }` declares one, else the
    // humanized class name (unchanged fallback).
    let class_label = cpe_core::labels::resolve_class_label(cfg.class_labels, &resource.class, lang);

    // Build the record viewer for the resource's linked Image resources.
    let viewer_markup = if has_media {
        super::media::render_media_viewer(
            cfg.media_items,
            title,
            cfg.selected_image,
            cfg.current_path,
            cfg.current_query,
            cfg.strip_open,
            lang,
        )
    } else {
        html! {}
    };

    // The host's own rights (`.pgroup`) and its compound-member list
    // (`.rv-contents`) are both part of the `record__body` structure
    // — siblings of `.props`, never
    // nested inside it. Both degrade to nothing when the host carries no
    // rights properties / is not a compound parent. The host rights only
    // ever apply to a viewer record (a non-media resource-detail has no
    // picture whose rights the host's need to stay distinct from).
    let host_rights_markup = if has_media {
        render_host_rights_group(resource, &class_label, lang)
    } else {
        html! {}
    };
    // `.rv-contents` is the compound-member FALLBACK: it
    // renders iff no navigator shows the members on this page, with or
    // without media — a compound parent's members are listed somewhere,
    // exactly once.
    let contents_markup = if cfg.navigator {
        html! {}
    } else {
        render_compound_contents(cfg.compound_members, cfg.links.as_ref(), lang)
    };

    // Everything the body holds besides an intro — the flat rows, config
    // groups, host rights and contents. Emitted bare when there is no intro,
    // wrapped in `div.record__apparat` when there
    // is one.
    let apparat_markup = html! {
        @if emit_flat_props {
            div.props {
                @for prop in &ordered {
                    ({
                        render_property_block(
                            prop,
                            lang,
                            cfg.property_labels,
                            cfg.links.as_ref(),
                            cfg.date_precision_word,
                        )
                    })
                }
            }
        }
        @for group in &groups_markup { (group) }
        // The incoming-links list — record content, so it sits with the
        // record's own groups (after the LAST one), before the host-rights /
        // compound apparatus. Not gated on `has_media`.
        @if let Some(incoming) = cfg.incoming {
            ({
                render_incoming_links(
                    &incoming,
                    cfg.links.as_ref(),
                    cfg.property_labels,
                    lang,
                )
            })
        }
        (host_rights_markup)
        (contents_markup)
    };
    // An intro adds the `--intro` modifier (the two-column grid).
    let body_class: std::borrow::Cow<'static, str> = if cfg.intro.is_some() {
        format!("{body_class} record__body--intro").into()
    } else {
        body_class.into()
    };

    let head_markup = html! {
        header.record__head data-attr=[hidden_when_wide] {
            div.record__class {
                p.eyebrow { (class_label) }
                span.record__id { (resource.id) }
            }
            h1.record__title { (title) }
            @if let Some(ref head) = cfg.head {
                (render_head_slots(resource, head, cfg.date_precision_word, lang))
            }
            @if let Some(ref q) = quote_text {
                blockquote.record__quote {
                    (q)
                    cite.record__cite { (title) }
                }
            }
        }
    };
    let body_markup = html! {
        div class=(body_class) data-attr=[hidden_when_wide] {
            @if let Some(ref intro) = cfg.intro {
                // Exactly two grid items: the
                // intro, then ONE apparat wrapper around everything else.
                // No `.page` — the intro is not a component root.
                div.record__intro { (intro) }
                div.record__apparat { (apparat_markup) }
            } @else { (apparat_markup) }
        }
    };

    // The annotation signals and the arrival init sit on the record article
    // because no `_compound` swap replaces it: the toggle must survive a page
    // turn and the arrival must run once per real load. The init follows the
    // declaration: DataStar applies an element's attributes in order.
    let annotated_reader = cfg.reader.is_some() && cfg.annotations.is_some();
    let anno_signals = annotated_reader.then(super::annotation::annotation_signals_decl);
    let anno_arrival = annotated_reader.then_some(super::annotation::ARRIVAL_INIT_EXPR);

    html! {
        article
            class=(article_class)
            data-frame=[tall_frame]
            data-class=[wide_class]
            data-signals=[anno_signals]
            data-init=[anno_arrival]
        {
            @if let Some(ref reader) = cfg.reader {
                // One wrapper, because the article is the `book` query
                // container and a container query never styles its own
                // container. The DOM order is the stacked order (head →
                // reader → intro → Apparat); record.css places the reader
                // in the right-hand track at desk widths.
                div.record__tracks {
                    (head_markup)
                    (reader)
                    @if let Some(ref annotations) = cfg.annotations { (annotations) }
                    (body_markup)
                }
            } @else {
                (head_markup)
                // The viewer (`.rv-col`) MUST be a direct child of
                // `article.record--viewer` — `media.css`'s
                // `.record--viewer > .rv-col { grid-column: 2; grid-row: 1 / span 2;
                // position: sticky; … }` selector requires that exact parentage to
                // place it in the sticky right-hand column.
                // Nesting it inside `.record__body` (a grandchild)
                // silently drops it out of grid placement entirely.
                (viewer_markup)
                (body_markup)
            }
        }
    }
}

/// Render one config-declared property group in the
/// existing `.pgroup` class family: `section.pgroup >
/// header.pgroup__head > span.pgroup__title` + a `div.props` of
/// `render_property_block` rows, in declaration order. Deliberately no
/// `span.pgroup__of` — that subject line belongs to the host-rights block.
///
/// `properties` is already filtered to the members present on the resource
/// (minus title/quote/media); empty, with no links row, ⇒ nothing at all,
/// never an empty cap, unless `empty` names a resolved sentence (caption +
/// `p.pgroup__empty`). A links row ends the group's `div.props`.
fn render_property_group(
    group: &GroupCfg<'_>,
    properties: &[&Property],
    lang: &Lang,
    property_labels: &[cpe_core::ir::PropertyLabel],
    links: Option<&LinkResolver<'_>>,
    date_precision_word: bool,
) -> Markup {
    let caption = group.caption;
    if properties.is_empty() && group.links.is_none() {
        // `empty=`: the caption plus a plain italic sentence.
        return match group.empty {
            Some(text) => html! {
                section.pgroup {
                    header.pgroup__head {
                        span.pgroup__title { (caption) }
                    }
                    p.pgroup__empty { (text) }
                }
            },
            None => html! {},
        };
    }
    html! {
        section.pgroup {
            header.pgroup__head {
                span.pgroup__title { (caption) }
            }
            div.props {
                @for prop in properties {
                    ({
                        render_property_block(
                            prop,
                            lang,
                            property_labels,
                            links,
                            date_precision_word,
                        )
                    })
                }
                @if let Some(row) = &group.links { (render_group_links(row, links)) }
            }
        }
    }
}

/// A group's links row: a `.prop` captioned by
/// `links-label=`, one line per link name, each name over the partners it
/// joins the record to, each its title and an arrow. A resource listed twice
/// on one line keeps its first listing: a resource link carries no number to
/// rank by, unlike the register's partners.
fn render_group_links(row: &GroupLinks<'_>, links: Option<&LinkResolver<'_>>) -> Markup {
    let lines = super::annotation::group_by_link_name(
        row.links,
        |l: &ResourceLink| &l.link_name,
        |l| &l.resource_id,
        |_, _| false,
    );
    html! {
        div.prop.prop--links {
            div.prop__label { (row.caption) }
            div.prop__value {
                @for (first, partners) in &lines {
                    div.prop__rel {
                        span.prop__rel-name lang=[first.link_name_lang.as_deref()] {
                            (first.link_name)
                        }
                        @for partner in partners {
                            @let href = links
                                .and_then(|l| {
                                    l.href(&partner.resource_id, Some(&partner.resource_class))
                                });
                            @match href {
                                Some(href) => a.prop__rel-to href=(href) {
                                    em { (partner.resource_label) }
                                    " "
                                    span aria-hidden="true" { "→" }
                                }
                                None => span.prop__rel-to {
                                    em { (partner.resource_label) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Render a record's incoming-links list in the
/// `.pgroup` caption idiom: `section.pgroup.incoming > header.pgroup__head >
/// span.pgroup__title { caption }` + `ul.incoming__list` of
/// `li.incoming__item` rows, each the source's name (a link, or a plain
/// `span.incoming__name`) and a faint `span.incoming__via` marker naming the
/// matched property (`resolve_property_label`).
///
/// Hrefs, first rung that resolves wins — generic, no project literal:
/// 1. the source's OWN detail page (`LinkResolver::href`);
/// 2. else, for a compound member whose PARENT's class is bound, the member inside its parent's
///    reader (`LinkResolver::member_href` — `?page=<id>`, no `#compound-viewer` anchor, never
///    carrying the current query);
/// 3. else a plain span (the engine's usual graceful degradation).
///
/// `rows` empty ⇒ the caption + `p.resource-list__empty`
/// (`ChromeMsg::IncomingLinksEmpty`) — "nothing links here" is a real
/// answer. The caller renders nothing at all for `None`.
///
/// **Row anatomy.** The link text is the row's
/// `row-label` value ([`IncomingLink::label`]) else its `name`; a
/// `row-aside` value ([`IncomingLink::aside`]) follows as a faint
/// `span.incoming__aside`. Both are `None` unless the spec declares them.
///
/// **Grouped mode** — `cfg.grouping` is `Some`: see
/// [`render_grouped_incoming`].
///
/// **Row cap.** `incoming.rows` is
/// already capped to the spec's `limit` by the query layer;
/// `incoming.total` is the uncapped match count. Only when `rows.len() <
/// total` (the list really is capped) a `p.incoming__count` line renders
/// after the `ul`, via `chrome::incoming_links_shown_text` — in grouped mode
/// too. No link, no pager, no show-all — see
/// [`cpe_core::ir::IncomingLinksSpec`]'s doc comment for the deferred design
/// work this defers.
fn render_incoming_links(
    cfg: &IncomingCfg<'_>,
    links: Option<&LinkResolver<'_>>,
    property_labels: &[cpe_core::ir::PropertyLabel],
    lang: &Lang,
) -> Markup {
    let incoming = cfg.links;
    let rows = &incoming.rows;
    let count_line = html! {
        @if rows.len() < incoming.total {
            p.incoming__count {
                ({
                    crate::chrome::incoming_links_shown_text(
                        rows.len(),
                        incoming.total,
                        lang,
                    )
                })
            }
        }
    };
    if let Some(grouping) = cfg.grouping {
        return render_grouped_incoming(cfg, grouping, count_line, links, property_labels, lang);
    }
    html! {
        section.pgroup.incoming {
            header.pgroup__head {
                span.pgroup__title { (cfg.caption) }
            }
            @if rows.is_empty() {
                p.resource-list__empty { (ChromeMsg::IncomingLinksEmpty.text(lang)) }
            } @else {
                ul.incoming__list {
                    @for row in rows {
                        (render_incoming_row(row, true, links, property_labels, lang))
                    }
                }
                (count_line)
            }
        }
    }
}

/// The grouped incoming-links list:
///
/// ```text
/// section.pgroup.incoming.incoming--grouped.incoming--{cards|columns}
///   header.pgroup__head > h2.pgroup__title { caption } [span.incoming__note]
///   div.incoming__groups
///     section.incoming__group            (one per non-empty group)
///       cards:   h3.incoming__card-head > span.incoming__card-label + " " + span.incoming__count-n
///       columns: h3.incoming__role { "Label · n" }
///       (no head element for the trailing headless group)
///       ul.incoming__list > li.incoming__item …
///   [p.incoming__count]
/// ```
///
/// The section cap is an `h2` (the ungrouped list keeps the `span`) so the
/// h3 group heads below it leave no outline gap. No rows renders nothing, not
/// even the caption. The `via` marker is dropped when grouping by `via` or
/// when only one link property is declared.
fn render_grouped_incoming(
    cfg: &IncomingCfg<'_>,
    grouping: &cpe_core::ir::IncomingGrouping,
    count_line: Markup,
    links: Option<&LinkResolver<'_>>,
    property_labels: &[cpe_core::ir::PropertyLabel],
    lang: &Lang,
) -> Markup {
    use cpe_core::ir::{IncomingGroupBy, IncomingLayout};

    let rows = &cfg.links.rows;
    if rows.is_empty() {
        return html! {};
    }
    let by_via = matches!(grouping.by, IncomingGroupBy::Via);
    let show_via = !(by_via || cfg.properties.len() == 1);
    let layout = grouping.layout;
    let section_class = match layout {
        IncomingLayout::Cards => "pgroup incoming incoming--grouped incoming--cards",
        IncomingLayout::Columns => "pgroup incoming incoming--grouped incoming--columns",
    };
    let note = cfg
        .note
        .map(|template| cpe_core::util::substitute_count(template, &crate::chrome::group_thousands(rows.len(), lang)));
    let groups = group_incoming(rows, grouping, cfg.properties);
    html! {
        section class=(section_class) {
            header.pgroup__head {
                h2.pgroup__title { (cfg.caption) }
                @if let Some(ref note) = note {
                    " "
                    span.incoming__note { (note) }
                }
            }
            div.incoming__groups {
                @for group in &groups {
                    @let head = group
                        .head
                        .as_deref()
                        .map(|h| {
                            if by_via {
                                cpe_core::labels::resolve_property_label(
                                    property_labels,
                                    h,
                                    lang,
                                )
                            } else {
                                h.to_owned()
                            }
                        });
                    @let count = crate::chrome::group_thousands(group.count, lang);
                    section.incoming__group {
                        @if let Some(ref head) = head {
                            @match layout {
                                IncomingLayout::Cards => {
                                    h3.incoming__card-head {
                                        span.incoming__card-label { (head) }
                                        " "
                                        span.incoming__count-n { (count) }
                                    }
                                }
                                IncomingLayout::Columns => {
                                    h3.incoming__role { (head) " · " (count) }
                                }
                            }
                        }
                        ul.incoming__list {
                            @for row in &group.rows {
                                ({
                                    render_incoming_row(
                                        row,
                                        show_via,
                                        links,
                                        property_labels,
                                        lang,
                                    )
                                })
                            }
                        }
                    }
                }
            }
            (count_line)
        }
    }
}

/// One `li.incoming__item` row — see [`render_incoming_links`]'s row
/// anatomy. `show_via` ⇒ the trailing faint `span.incoming__via` marker.
fn render_incoming_row(
    row: &IncomingLink,
    show_via: bool,
    links: Option<&LinkResolver<'_>>,
    property_labels: &[cpe_core::ir::PropertyLabel],
    lang: &Lang,
) -> Markup {
    let href = incoming_row_href(row, links);
    let text = row.label.as_deref().unwrap_or(&row.name);
    html! {
        li.incoming__item {
            @if let Some(ref href) = href {
                a.int-link.incoming__name href=(href) {
                    span.ref { (text) }
                }
            } @else {
                span.incoming__name { (text) }
            }
            @if let Some(ref aside) = row.aside {
                " "
                span.incoming__aside { (aside) }
            }
            @if show_via {
                " "
                span.incoming__via {
                    ({
                        cpe_core::labels::resolve_property_label(
                            property_labels,
                            &row.via,
                            lang,
                        )
                    })
                }
            }
        }
    }
}

/// The three-rung href of one incoming-links row — see
/// [`render_incoming_links`].
fn incoming_row_href(row: &IncomingLink, links: Option<&LinkResolver<'_>>) -> Option<String> {
    let links = links?;
    links
        .href(&row.id, Some(&row.class))
        .or_else(|| row.parent.as_ref().and_then(|p| links.member_href(&p.id, &p.class, &row.id)))
}

/// Render `.pgroup` — the HOST resource's own rights: rights holder ·
/// edition · licence, each a **separate row** (never merged into a run the way
/// the picture's bar does — "No known copyright restrictions" beside "Public
/// domain" is one fact stated twice for the picture, but the host's rights
/// holder / edition / licence are three independent claims that never
/// collapse). Unframed, in the properties flow, on the page surface — the
/// visual opposite of the image's legal band on the picture's own bed
/// (containment carries the distinction).
///
/// Reads three fixed EAV property names directly off `resource`
/// (`rights-holder`, `edition`, `license`) via `media::find_text_property` —
/// the same requested-lang → universal → any-present fallback chain every
/// other text property uses. This is a structural convention, not a
/// `show-properties`-gated one: a host's rights are never excluded from
/// `.pgroup` by an author's `show-properties` list, exactly like `quote`/
/// `title_property` are never excluded from the record header.
///
/// **Omitted entirely** when none of the three properties resolve to a
/// non-empty value (most Alice hosts carry no rights EAV of their own;
/// `.pgroup` degrades to nothing rather than an empty shell).
///
/// The header names the subject in words — the
/// resolved class label + the resource's own id, e.g. "Character · CH_001".
/// It is not "RIGHTS & ATTRIBUTION · THIS CHAPTER": a gendered "THIS <CLASS>"
/// demonstrative does not translate uniformly across en/de/fr/it without
/// per-class grammatical-gender data this project does not have, so the
/// subject is named as "{Class} · {id}" instead — still names the subject in
/// words plus its id, without the demonstrative.
fn render_host_rights_group(resource: &Resource, class_label: &str, lang: &Lang) -> Markup {
    let rights_holder = super::media::find_text_property(resource, "rights-holder", lang);
    let edition = super::media::find_text_property(resource, "edition", lang);
    let license = super::media::find_text_property(resource, "license", lang);

    if rights_holder.is_none() && edition.is_none() && license.is_none() {
        return html! {};
    }

    let subject = format!("{class_label} · {}", resource.id);

    html! {
        section.pgroup {
            header.pgroup__head {
                span.pgroup__title { (ChromeMsg::RvHostRightsTitle.text(lang)) }
                span.pgroup__of { (subject) }
            }
            dl.rf {
                @if let Some(v) = rights_holder {
                    div.rf__row {
                        dt { (ChromeMsg::RvFieldRightsHolder.text(lang)) }
                        dd { (v) }
                    }
                }
                @if let Some(v) = edition {
                    div.rf__row {
                        dt { (ChromeMsg::RvFieldEdition.text(lang)) }
                        dd { (v) }
                    }
                }
                @if let Some(v) = license {
                    div.rf__row {
                        dt { (ChromeMsg::RvFieldLicense.text(lang)) }
                        dd { (v) }
                    }
                }
            }
        }
    }
}

/// Render `.rv-contents` — the compound member list, in the reading column. Navigates the
/// *resource* (the host's own members), never the *picture* — the thumb strip
/// (`.em-col__strip`, in the viewer column) is the picture's own paging and is
/// a completely separate question; do not merge the two.
///
/// **Omitted entirely** when `members` is empty — the host is not a compound
/// parent, or has no members (no Alice Character/Location/StoryChapter is
/// ever a compound parent; only the two Story resources are, and neither has
/// a `resource-detail`/media page of its own, so this never renders in the
/// Alice demo today — the omission path is what's exercised).
///
/// Each member links to its own detail page via `links` (the same
/// `LinkResolver` `.pgroup`'s caller and `render_property_block`'s `Link`
/// values use) when the member's class has a bound route; otherwise the
/// member renders as plain text (graceful degradation, matching every other
/// `LinkResolver` call site). Unlike the compound-viewer's own contents rail
/// (`cv-toc`, which highlights the *currently shown* member because it pages
/// into one), no item here is ever `aria-current` — this page is the parent
/// resource, viewing its own member list, not paging through it.
fn render_compound_contents(members: &[MemberHeader], links: Option<&LinkResolver<'_>>, lang: &Lang) -> Markup {
    if members.is_empty() {
        return html! {};
    }

    let contents_label = ChromeMsg::CompoundContentsLabel.text(lang);

    html! {
        section.rv-contents {
            nav aria-label=(contents_label) {
                ol.rv-contents__list {
                    @for (i, m) in members.iter().enumerate() {
                        // 1-based ordinal from the 0-based enumerate index;
                        // bounded by the compound's own member count, so this
                        // always fits comfortably in a display string.
                        @let ordinal = i + 1;
                        @let href = links.and_then(|r| r.href(&m.id, Some(&m.class)));
                        li.rv-contents__item {
                            @if let Some(ref href) = href {
                                a.rv-contents__link href=(href) {
                                    span.rv-contents__no { (format!("{ordinal:02}")) }
                                    span { (m.name) }
                                }
                            } @else {
                                span.rv-contents__link {
                                    span.rv-contents__no { (format!("{ordinal:02}")) }
                                    span { (m.name) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Render the unknown-record not-found body — an unknown resource id on a matched,
/// project-prefixed `resource-detail` route.
///
/// Terse `.notfound--terse` anatomy: a mono `ChromeMsg::NfCode`
/// mark, an `h1.notfound__title` at `--size-6`, an optional `nf_id_msg_text`
/// paragraph, then one mono exits line (`nf_browse_all_text` + `ChromeMsg::NfProjectHome`),
/// then the unknown-path page's "Or browse:" line (`render_or_browse_line` — the same list
/// that page shows: `not-found { browse }` when declared, else the first three nav
/// items). The second line is deliberate.
///
/// - `class` is the `resource-detail`'s declared `class` attribute
///   (`Component::ResourceDetail.class`), if any. `Some` → the heading and message use the resolved
///   singular class label (`resolve_class_label` via `class_labels`); `None` (class-less detail
///   pages) → the heading falls back to `ChromeMsg::NfIdTitleFallback` and no `nf_id_msg_text`
///   paragraph is rendered (the message's copy structurally requires a class noun).
/// - `site` supplies the class labels and the "Or browse:" list.
/// - `class_list_routes` is the project's class→list-route map (`Project::class_list_routes`). The
///   "Browse all {class}" exit renders only when `class` is `Some` AND has an entry in this map (a
///   class with no browse page falls back to the Project home exit alone — see the map's own doc
///   comment for why this is advisory, not a hard-fail).
/// - The identifier is never restated in this body's copy — callers must NOT plumb the unknown id
///   in here.
///
/// All hrefs are built from `urls` (the project's home or one of its routes)
/// — config-derived and never path-derived, so they can never carry the
/// unknown identifier or leave the origin.
pub fn render_not_found(
    class: Option<&str>,
    urls: cpe_core::url::ProjectUrls<'_>,
    site: &SiteConfig,
    class_list_routes: &HashMap<String, String>,
    lang: &Lang,
) -> Markup {
    let code = ChromeMsg::NfCode.text(lang);
    let class_labels = &site.class_labels;

    // Heading + message. A class-less detail page can't form a singular noun
    // at all — fall back to the class-neutral heading and skip the message
    // (its copy is parameterized by the class noun; there's nothing to say
    // without one).
    let heading = not_found_heading(class, class_labels, lang);
    let message: Option<String> = class.map(|class| {
        let singular = cpe_core::labels::resolve_class_label(class_labels, class, lang);
        nf_id_msg_text(&singular, lang)
    });

    // "Browse all {class}" exit — only when the class has a registered
    // browse page (the map is advisory; see `build_class_list_routes`).
    let browse_all: Option<(String, String)> = class.and_then(|class| {
        class_list_routes.get(class).map(|route| {
            let plural = cpe_core::labels::resolve_class_plural(class_labels, class, lang);
            (nf_browse_all_text(&plural, lang), urls.page(route))
        })
    });

    let project_home_href = urls.home();

    let exits_line = html! {
        p.notfound__or {
            @if let Some((label, href)) = browse_all {
                a href=(href) { (label) }
            }
            a href=(project_home_href) { (ChromeMsg::NfProjectHome.text(lang)) }
        }
    };

    render_notfound_shell(
        true,
        code,
        &heading,
        message.as_deref(),
        None,
        Some(html! {
            (exits_line)
            (render_or_browse_line(site, urls, lang))
        }),
    )
}

/// The unknown-record page's `h1` text — "There is no {class} at this address." with the resolved
/// singular class label, or the class-neutral `ChromeMsg::NfIdTitleFallback` for a
/// class-less detail page. Shared by `render_not_found` (the body) and
/// `compose_page_title` (the `<title>`), so the two never diverge.
pub fn not_found_heading(class: Option<&str>, class_labels: &[cpe_core::ir::ClassLabel], lang: &Lang) -> String {
    match class {
        Some(class) => {
            let singular = cpe_core::labels::resolve_class_label(class_labels, class, lang);
            nf_id_title_text(&singular, lang)
        }
        None => ChromeMsg::NfIdTitleFallback.text(lang).to_owned(),
    }
}

/// The 404s' mono "Or browse:" line: the project's declared
/// `not-found { browse }` entries when there are any,
/// else the first three nav items. Shared by the unknown-path page
/// (`render_not_found_project_body`) and the unknown-record page (`render_not_found`).
fn render_or_browse_line(site: &SiteConfig, urls: cpe_core::url::ProjectUrls<'_>, lang: &Lang) -> Markup {
    let browse_links: Vec<(&str, &str)> = if site.not_found_browse.is_empty() {
        site.nav
            .iter()
            .take(3)
            .map(|item| (item.route.as_str(), cpe_core::labels::nav_label(site, item, lang)))
            .collect()
    } else {
        site.not_found_browse
            .iter()
            .map(|b| (b.route.as_str(), not_found_browse_label(site, b, lang)))
            .collect()
    };

    html! {
        p.notfound__or {
            span { (ChromeMsg::NfOrBrowse.text(lang)) }
            @for (route, label) in &browse_links {
                a href=(urls.page(route)) { (label) }
            }
        }
    }
}

/// Render the unknown-path not-found **body** — an unmatched, project-prefixed path
/// (router fallback with the raw first path segment matching a loaded
/// project id).
///
/// Full `.notfound` anatomy: mono `ChromeMsg::NfCode`, `h1` at
/// `--size-7` (`ChromeMsg::NfPathTitle`), `ChromeMsg::NfPathMsg`, then the one editorial
/// invitation row (`nf_home_text`, project title, accent `→`) to the project
/// home, and a mono `"Or browse:"` line with **at most three** of the
/// project's nav pages (`site.nav.iter().take(3)`).
///
/// This is the `.notfound__inner` body only — `render_not_found_project`
/// (`dsp-incubator cpe/web`'s `page` module) wraps it in the full bespoke page shell (chrome
/// deltas: no breadcrumb, no search box, no `aria-current`).
pub fn render_not_found_project_body(site: &SiteConfig, urls: cpe_core::url::ProjectUrls<'_>, lang: &Lang) -> Markup {
    let code = ChromeMsg::NfCode.text(lang);
    let heading = ChromeMsg::NfPathTitle.text(lang);
    let message = ChromeMsg::NfPathMsg.text(lang);

    let home_label = nf_home_text(cpe_core::labels::site_title(site, lang), lang);
    let home_href = urls.home();

    // The "Or browse:" links: `render_or_browse_line` (shared with the unknown-record page).
    render_notfound_shell(
        false,
        code,
        heading,
        Some(message),
        Some((&home_label, &home_href)),
        Some(render_or_browse_line(site, urls, lang)),
    )
}

/// Resolve one `not-found { browse }` entry's link text, first hit wins:
///
/// 1. an explicit `label=` — chrome-id-or-literal (`chrome_lookup(label, lang)`, else the literal
///    value; the `site.description` precedent);
/// 2. the label of the nav item with the **same** route, resolved exactly as the nav renders it
///    (`nav_label`);
/// 3. the route itself.
fn not_found_browse_label<'a>(site: &'a SiteConfig, browse: &'a cpe_core::ir::NotFoundBrowse, lang: &Lang) -> &'a str {
    if let Some(label) = &browse.label {
        return cpe_core::labels::chrome_lookup(site, label, lang).unwrap_or(label);
    }
    site.nav
        .iter()
        .find(|item| item.route == browse.route)
        .map_or(browse.route.as_str(), |item| cpe_core::labels::nav_label(site, item, lang))
}

/// Shared `.notfound` anatomy shell of the three not-found pages (unknown path, unknown
/// project, unknown record). `terse` selects the unknown-record page's `.notfound--terse`
/// scaled-down variant.
///
/// `way` is the one editorial invitation row (the unknown-path page's project-home link, the
/// unknown-project page's all-projects link) — `None` on the unknown-record page, which has
/// no invitation row at all. `or_line` is the trailing mono exits line (the unknown-path
/// page's `"Or browse:"` + nav pages, the unknown-record page's browse-all/project-home
/// pair) — `None` on the unknown-project page, which has no
/// "or browse" line (there is no project whose pages could be offered).
/// Both are independently optional; the unknown-path page renders both, the
/// unknown-project page only `way`, the unknown-record page only `or_line`.
pub fn render_notfound_shell(
    terse: bool,
    code: &str,
    heading: &str,
    message: Option<&str>,
    way: Option<(&str, &str)>,
    or_line: Option<Markup>,
) -> Markup {
    let root_class = if terse {
        "notfound notfound--terse page"
    } else {
        "notfound page"
    };
    html! {
        article class=(root_class) {
            div.notfound__inner {
                p.notfound__code { (code) }
                h1.notfound__title { (heading) }
                @if let Some(msg) = message {
                    p.notfound__msg { (msg) }
                }
                @if let Some((label, href)) = way {
                    p.notfound__way {
                        a href=(href) {
                            span { (label) }
                            span aria-hidden="true" { "→" }
                        }
                    }
                }
                @if let Some(ol) = or_line { (ol) }
            }
        }
    }
}

// Internal rendering helpers

/// Render one `Property` as a `.prop` block: label + value(s).
///
/// Multi-valued `ListValue` properties (more than one value) render as
/// `ul.keyword-set > li.keyword` inside the `.prop__value`. A multi-valued
/// all-`Link` property renders its values comma-separated inline (the
/// one case where a comma is unambiguous — link fragments are short and
/// name-like, never prose). Every other multi-valued property (`Text`,
/// `Richtext`, `Integer`, `Date`, …) renders as one `div.v-multi__item`
/// block per value, stacked — a comma join is ambiguous there because the
/// values themselves may contain commas (prose comments, citations), and an
/// invented separator could make two distinct values read as one run-on
/// sentence. Single-value properties delegate straight to `render_value`.
///
/// Exposed `pub(crate)` so sibling component modules (`compound_viewer`) can
/// reuse the same property-block layout without duplicating the logic.
///
/// `property_labels` resolves the `.prop__label` text via
/// `resolve_property_label` (declared translation, else humanized fallback).
/// `compound_viewer`'s call site deliberately passes `&[]` — compound-viewer
/// member property labels are out of scope for the config-translatable-labels
/// task (see that call site's comment).
pub(crate) fn render_property_block(
    property: &Property,
    lang: &Lang,
    property_labels: &[cpe_core::ir::PropertyLabel],
    links: Option<&LinkResolver<'_>>,
    date_precision_word: bool,
) -> Markup {
    let values_to_render = pick_values(property, lang);

    // Multi-valued ListValue → keyword-set list.
    let is_multi_list =
        values_to_render.len() > 1 && values_to_render.iter().all(|v| matches!(v, PropertyValue::ListValue { .. }));

    // Multi-valued Link property → comma-separated inline values. Links
    // render as short inline-flex `.int-link`/`.v-text` fragments (name-like,
    // never prose), so a comma read as "and" is unambiguous — this join predates
    // the stacked rendering below and is deliberately unchanged.
    let is_multi_link = values_to_render.len() > 1
        && !is_multi_list
        && values_to_render.iter().all(|v| matches!(v, PropertyValue::Link { .. }));

    // Every other multi-valued non-list property (Text, Richtext, Int,
    // Date, …) → one block per value, stacked. A comma join is ambiguous
    // here: the values themselves may contain commas (prose comments,
    // citations), so joining them with an invented ", " can make two
    // distinct values read as one run-on sentence (see
    // `render_property_block_multi_text_value_renders_stacked_blocks` and the richtext sibling
    // test below).
    let is_multi_stack = values_to_render.len() > 1 && !is_multi_list && !is_multi_link;

    let value_markup = if is_multi_list {
        html! {
            ul.keyword-set {
                @for value in &values_to_render { (render_keyword_item(value, lang)) }
            }
        }
    } else if is_multi_link {
        html! {
            @for (i, value) in values_to_render.iter().enumerate() {
                @if i > 0 { ", " }
                (render_value_opts(value, lang, links, date_precision_word))
            }
        }
    } else if is_multi_stack {
        html! {
            @for value in &values_to_render {
                div.v-multi__item { (render_value_opts(value, lang, links, date_precision_word)) }
            }
        }
    } else {
        html! {
            @for value in &values_to_render {
                (render_value_opts(value, lang, links, date_precision_word))
            }
        }
    };

    html! {
        div.prop {
            div.prop__label {
                ({
                    cpe_core::labels::resolve_property_label(
                        property_labels,
                        &property.name,
                        lang,
                    )
                })
            }
            div.prop__value { (value_markup) }
        }
    }
}

/// Render a single `ListValue` as a `li.keyword` with path and leaf spans.
///
/// Splits on the separator set `/\s+[–−-]\s+/` (space-padded U+2013 EN DASH,
/// U+2212 MINUS, U+002D HYPHEN). The last segment is the leaf; preceding
/// segments are path spans. A label with no separator → leaf only.
///
/// `label: None` → renders `span.unresolved` placeholder inline.
fn render_keyword_item(value: &PropertyValue, lang: &Lang) -> Markup {
    match value {
        PropertyValue::ListValue { label: Some(label), .. } => {
            let (path_parts, leaf) = split_keyword_label(label);
            html! {
                li.keyword {
                    @for part in &path_parts {
                        span.path { (part) }
                    }
                    span.leaf { (leaf) }
                }
            }
        }
        _ => {
            // Unresolved or wrong type: fall back to render_value for the placeholder.
            // No link resolver here — keyword items are list-typed, not links.
            html! {
                li { (render_value(value, lang, None)) }
            }
        }
    }
}

/// Render one `PropertyValue` to `Markup`.
///
/// - `Text` (URL) → `a.ext-link` with `target="_blank" rel="noopener noreferrer"`.
/// - `Text` (plain) → `span.v-text`.
/// - `Richtext` → `div.v-richtext` wrapping `render_richtext_fragment`.
/// - `Integer`/`Decimal` → `span.v-number`.
/// - `Boolean` → `span.v-bool[data-bool="true"|"false"]` + localised label.
/// - `Date` → `span.v-date`.
/// - `Link { target_label: Some }` → `<a class="int-link" href=…>` when the resolver binds the
///   target's class; otherwise `span.v-text` fallback.
/// - `Link { target_label: None, target_class: None }` (target does not exist) → `span.unresolved`
///   with the UNRESOLVED mark.
/// - `Link { target_label: None, target_class: Some(_) }` (target exists, no `name`) → plain `code`
///   — this value resolves, no mark.
/// - `ListValue { label: Some }` (single) → `span.term`.
/// - `ListValue { label: None }` → `span.unresolved` placeholder.
///
/// Dates render without the precision word; see `render_value_opts`.
pub fn render_value(value: &PropertyValue, lang: &Lang, links: Option<&LinkResolver<'_>>) -> Markup {
    render_value_opts(value, lang, links, false)
}

/// [`render_value`] with the site-level rendering options threaded through:
/// `date_precision_word` (`site { date-precision-word #true }`) appends the
/// `span.prec` precision word to a `Date` value. Every other variant ignores
/// it.
pub(crate) fn render_value_opts(
    value: &PropertyValue,
    lang: &Lang,
    links: Option<&LinkResolver<'_>>,
    date_precision_word: bool,
) -> Markup {
    match value {
        PropertyValue::Text { text, .. } => {
            if is_safe_external_url(text) {
                // Trim a display label: strip common scheme prefixes.
                let display = text.trim_start_matches("https://").trim_start_matches("http://");
                let display = display.trim_end_matches('/');
                // Deliberately NOT `PreEscaped` for the href: `text` is
                // reflected user/project data, so we pass it through Maud's
                // escaped attribute form (`href=(text)`), which HTML-escapes
                // it correctly for us. `PreEscaped` is reserved for markup we
                // constructed ourselves — never for reflected input like this.
                html! {
                    a.ext-link href=(text) target="_blank" rel="noopener noreferrer" { (display) }
                }
            } else {
                html! {
                    span.v-text { (text) }
                }
            }
        }
        PropertyValue::Richtext { text, .. } => {
            // render_richtext_fragment returns PreEscaped — safe here because
            // the markdown pipeline already strips raw HTML events and
            // scheme-filters link/image destinations.
            let inner = render_richtext_fragment(text);
            html! {
                div.v-richtext { (inner) }
            }
        }
        PropertyValue::Integer(n) => {
            html! {
                span.v-number { (n) }
            }
        }
        PropertyValue::Decimal(f) => {
            html! {
                span.v-number { (f) }
            }
        }
        PropertyValue::Boolean(b) => {
            let (yes, no) = bool_labels(lang);
            let label = if *b { yes } else { no };
            let data_bool = if *b { "true" } else { "false" };
            // Not `data-on`: Datastar reads any `data-on` as its event plugin
            // and throws `KeyRequired` on one without an event key.
            html! {
                span.v-bool data-bool=(data_bool) { (label) }
            }
        }
        PropertyValue::Date { .. } => format_date(value, lang, date_precision_word),
        PropertyValue::Link { target_label: Some(label), target_id, target_class } => {
            // Try to resolve a clickable href via the LinkResolver.
            // If the resolver is absent or the class has no binding, fall back
            // to the plain-text label.
            if let Some(url) = links.and_then(|r| r.href(target_id, target_class.as_deref())) {
                // Emit the .int-link anchor with the inner structure that record.css expects:
                // - `.ref` wraps the label text (styled as the link).
                // - `.rel` would carry a relation label — we have none here, so omit it.
                html! {
                    a.int-link href=(url) {
                        span.ref { (label) }
                    }
                }
            } else {
                html! {
                    span.v-text { (label) }
                }
            }
        }
        // Target exists but has no resolvable `name` — this value resolves
        // (it names a real resource), so it renders as a plain mono id, never
        // the UNRESOLVED mark. The mark's scope is a missing *target*, not a
        // missing *label* on an existing target.
        PropertyValue::Link { target_label: None, target_id, target_class: Some(_) } => {
            html! {
                code { (target_id) }
            }
        }
        // Target does not exist at all — the unresolved-link mark.
        PropertyValue::Link { target_label: None, target_id, target_class: None } => {
            let mark = ChromeMsg::ValueUnresolved.text(lang);
            let title = ChromeMsg::ValueUnresolvedTitle.text(lang);
            html! {
                span.unresolved title=(title) {
                    code { (target_id) }
                    span.unresolved__mark { (mark) }
                }
            }
        }
        PropertyValue::ListValue { label: Some(label), .. } => {
            // Single list-value → .term chip.
            html! {
                span.term { (label) }
            }
        }
        PropertyValue::ListValue { label: None, node_id } => {
            let placeholder = ChromeMsg::UnresolvedListPlaceholder.text(lang);
            html! {
                span.unresolved {
                    (placeholder)
                    " ("
                    code { (node_id) }
                    ")"
                }
            }
        }
    }
}

/// Render a date value's `span.v-date`: [`format_date_text`] plus, only when
/// `precision_word` (`site { date-precision-word #true }`), a
/// trailing `span.prec` holding the precision word (`Tag`/`Monat`/`Jahr`, or
/// `Zeitraum` for a range) and the calendar word per `values::calendar_word`
/// (Gregorian at day or month precision; Julian is unmarked).
fn format_date(value: &PropertyValue, lang: &Lang, precision_word: bool) -> Markup {
    let text = format_date_text(value, lang);
    let word = match value {
        PropertyValue::Date { start, end, precision, calendar, .. } if precision_word => {
            let msg = if cpe_core::ir::date_is_range(start, end.as_deref(), *precision) {
                ChromeMsg::DatePrecRange
            } else {
                match precision {
                    DatePrecision::Day => ChromeMsg::DatePrecDay,
                    DatePrecision::Month => ChromeMsg::DatePrecMonth,
                    DatePrecision::Year => ChromeMsg::DatePrecYear,
                }
            };
            let word = msg.text(lang);
            Some(with_calendar_word(word.to_owned(), *calendar, *precision, lang))
        }
        _ => None,
    };
    html! {
        span.v-date {
            (text)
            @if let Some(word) = word {
                " "
                span.prec { (word) }
            }
        }
    }
}

// Incoming-links grouping

/// One partition of an `incoming-links` list, produced by [`group_incoming`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IncomingGroup<'a> {
    /// For `group-by="via"` the raw link property name (the renderer
    /// resolves its label); otherwise the node label or text key. `None` is
    /// the trailing headless group.
    pub head: Option<String>,
    pub count: usize,
    /// In input order.
    pub rows: Vec<&'a IncomingLink>,
}

/// Partition `rows` by `grouping`; empty groups are omitted.
///
/// `Via` follows `declared_properties` order; `Property` follows list node
/// `seq` (ties by key) or case-insensitive alphabetical for text keys. Rows
/// with no key (or, for `Via`, an undeclared `via`) go to a trailing headless
/// group. A source linking through two properties is two rows and appears in
/// both `Via` groups.
pub(crate) fn group_incoming<'a>(
    rows: &'a [IncomingLink],
    grouping: &cpe_core::ir::IncomingGrouping,
    declared_properties: &[String],
) -> Vec<IncomingGroup<'a>> {
    use cpe_core::ir::{GroupKey, IncomingGroupBy};

    let mut groups: Vec<IncomingGroup<'a>> = Vec::new();
    let mut headless: Vec<&'a IncomingLink> = Vec::new();

    match &grouping.by {
        IncomingGroupBy::Via => {
            for prop in declared_properties {
                let members: Vec<&IncomingLink> = rows.iter().filter(|r| &r.via == prop).collect();
                if !members.is_empty() {
                    groups.push(IncomingGroup {
                        head: Some(prop.clone()),
                        count: members.len(),
                        rows: members,
                    });
                }
            }
            headless.extend(rows.iter().filter(|r| !declared_properties.iter().any(|p| p == &r.via)));
        }
        IncomingGroupBy::Property(_) => {
            // (sort key, head, rows) in first-seen order, sorted afterwards.
            let mut buckets: Vec<(GroupKey, Vec<&IncomingLink>)> = Vec::new();
            for row in rows {
                match &row.group {
                    None => headless.push(row),
                    Some(key) => match buckets.iter_mut().find(|(k, _)| same_group(k, key)) {
                        Some((_, members)) => members.push(row),
                        None => buckets.push((key.clone(), vec![row])),
                    },
                }
            }
            buckets.sort_by(|(a, _), (b, _)| match (a, b) {
                (GroupKey::List { seq: sa, key: ka, .. }, GroupKey::List { seq: sb, key: kb, .. }) => {
                    sa.cmp(sb).then_with(|| ka.cmp(kb))
                }
                (GroupKey::Text { key: a }, GroupKey::Text { key: b }) => {
                    a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b))
                }
                // Mixed kinds cannot occur (one property, one kind); keep
                // lists first for a total order.
                (GroupKey::List { .. }, GroupKey::Text { .. }) => std::cmp::Ordering::Less,
                (GroupKey::Text { .. }, GroupKey::List { .. }) => std::cmp::Ordering::Greater,
            });
            for (key, members) in buckets {
                let head = match key {
                    GroupKey::List { label, .. } => label,
                    GroupKey::Text { key } => key,
                };
                groups.push(IncomingGroup { head: Some(head), count: members.len(), rows: members });
            }
        }
    }

    if !headless.is_empty() {
        groups.push(IncomingGroup { head: None, count: headless.len(), rows: headless });
    }
    groups
}

/// Whether two group keys name the same group (list nodes by node id, text by
/// its already case-canonicalised key, first spelling wins).
fn same_group(a: &cpe_core::ir::GroupKey, b: &cpe_core::ir::GroupKey) -> bool {
    use cpe_core::ir::GroupKey;
    match (a, b) {
        (GroupKey::List { key: a, .. }, GroupKey::List { key: b, .. }) => a == b,
        (GroupKey::Text { key: a }, GroupKey::Text { key: b }) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod group_incoming_tests {
    use cpe_core::ir::{GroupKey, IncomingGroupBy, IncomingGrouping, IncomingLayout};

    use super::*;

    fn grouping(by: IncomingGroupBy) -> IncomingGrouping {
        IncomingGrouping { by, layout: IncomingLayout::Columns, note_label: None }
    }

    fn row(id: &str, via: &str, group: Option<GroupKey>) -> IncomingLink {
        IncomingLink {
            id: id.into(),
            class: "Performance".into(),
            name: id.into(),
            via: via.into(),
            parent: None,
            group,
            label: None,
            aside: None,
        }
    }

    fn list(key: &str, label: &str, seq: i64) -> Option<GroupKey> {
        Some(GroupKey::List { key: key.into(), label: label.into(), seq })
    }

    fn text(key: &str) -> Option<GroupKey> {
        Some(GroupKey::Text { key: key.into() })
    }

    fn ids<'a>(g: &'a IncomingGroup<'a>) -> Vec<&'a str> {
        g.rows.iter().map(|r| r.id.as_str()).collect()
    }

    fn props(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn empty_input_yields_no_groups() {
        let g = group_incoming(&[], &grouping(IncomingGroupBy::Via), &props(&["a", "b"]));
        assert!(g.is_empty());
        let g = group_incoming(&[], &grouping(IncomingGroupBy::Property("decade".into())), &[]);
        assert!(g.is_empty());
    }

    #[test]
    fn via_follows_declared_order_and_omits_empty_groups() {
        let rows = vec![
            row("w1", "composer", None),
            row("w2", "librettist", None),
            row("w3", "composer", None),
        ];
        let decl = props(&["librettist", "arranger", "composer"]);
        let g = group_incoming(&rows, &grouping(IncomingGroupBy::Via), &decl);
        let heads: Vec<_> = g.iter().map(|x| x.head.as_deref()).collect();
        assert_eq!(heads, vec![Some("librettist"), Some("composer")]);
        assert_eq!(g[1].count, 2);
        assert_eq!(ids(&g[1]), vec!["w1", "w3"]);
    }

    #[test]
    fn source_in_two_roles_appears_in_both_groups() {
        let rows = vec![row("w1", "composer", None), row("w1", "librettist", None)];
        let decl = props(&["composer", "librettist"]);
        let g = group_incoming(&rows, &grouping(IncomingGroupBy::Via), &decl);
        assert_eq!(g.len(), 2);
        assert_eq!(ids(&g[0]), vec!["w1"]);
        assert_eq!(ids(&g[1]), vec!["w1"]);
    }

    #[test]
    fn single_group() {
        let rows = vec![
            row("p1", "x", list("d1", "1850s", 1)),
            row("p2", "x", list("d1", "1850s", 1)),
        ];
        let g = group_incoming(&rows, &grouping(IncomingGroupBy::Property("decade".into())), &[]);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].head.as_deref(), Some("1850s"));
        assert_eq!(g[0].count, 2);
    }

    #[test]
    fn list_groups_order_by_seq_with_missing_node_last() {
        let rows = vec![
            row("p1", "x", list("d3", "1870s", 3)),
            row("p2", "x", list("dz", "orphan", i64::MAX)),
            row("p3", "x", list("d1", "1850s", 1)),
            row("p4", "x", list("d3", "1870s", 3)),
        ];
        let g = group_incoming(&rows, &grouping(IncomingGroupBy::Property("decade".into())), &[]);
        let heads: Vec<_> = g.iter().map(|x| x.head.as_deref().unwrap()).collect();
        assert_eq!(heads, vec!["1850s", "1870s", "orphan"]);
        assert_eq!(ids(&g[1]), vec!["p1", "p4"]);
    }

    #[test]
    fn text_groups_order_alphabetically_case_insensitive() {
        let rows = vec![
            row("p1", "x", text("beta")),
            row("p2", "x", text("Alpha")),
            row("p3", "x", text("Gamma")),
            row("p4", "x", text("beta")),
        ];
        let g = group_incoming(&rows, &grouping(IncomingGroupBy::Property("kind".into())), &[]);
        let heads: Vec<_> = g.iter().map(|x| x.head.as_deref().unwrap()).collect();
        assert_eq!(heads, vec!["Alpha", "beta", "Gamma"]);
        assert_eq!(g[1].count, 2);
    }

    #[test]
    fn missing_key_goes_to_trailing_headless_group() {
        let rows = vec![
            row("p1", "x", None),
            row("p2", "x", list("d1", "1850s", 1)),
            row("p3", "x", None),
        ];
        let g = group_incoming(&rows, &grouping(IncomingGroupBy::Property("decade".into())), &[]);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].head.as_deref(), Some("1850s"));
        assert_eq!(g[1].head, None);
        assert_eq!(g[1].count, 2);
        assert_eq!(ids(&g[1]), vec!["p1", "p3"]);
    }

    #[test]
    fn via_outside_declared_properties_is_not_dropped() {
        let rows = vec![row("w1", "composer", None), row("w2", "stray", None)];
        let g = group_incoming(&rows, &grouping(IncomingGroupBy::Via), &props(&["composer"]));
        assert_eq!(g.len(), 2);
        assert_eq!(g[1].head, None);
        assert_eq!(ids(&g[1]), vec!["w2"]);
    }
}

// Tests

#[cfg(test)]
mod tests {
    use cpe_core::ir::{
        AnnotationPartner, Calendar, DatePrecision, Property, PropertyValue, Representation, RepresentationKind,
        Resource,
    };
    use cpe_core::util::humanize_property_name;

    use super::*;

    #[test]
    fn format_date_collapses_a_range_with_equal_bounds() {
        let en = make_lang("en");
        let text = |start, end, precision| format_date_text(&date_value(start, end, precision, None, None, None), &en);
        assert_eq!(text("1491-07-28", Some("1491-07-28"), DatePrecision::Day), "1491-07-28");
        assert_eq!(text("1475-01-01", Some("1475-12-31"), DatePrecision::Year), "1475");
        assert_eq!(text("1475-01-01", Some("1499-12-31"), DatePrecision::Year), "1475\u{2013}1499");
    }

    // Date rendering

    fn date_value(
        start: &str,
        end: Option<&str>,
        precision: DatePrecision,
        display: Option<&str>,
        display_lang: Option<&str>,
        calendar: Option<Calendar>,
    ) -> PropertyValue {
        PropertyValue::Date {
            start: start.to_owned(),
            end: end.map(str::to_owned),
            precision,
            display: display.map(str::to_owned),
            display_lang: display_lang.map(str::to_owned),
            calendar,
        }
    }

    /// (a) A day-precise value stored with `end == start`
    /// renders ONE date, never `1471-03-21–1471-03-21`, and its precision
    /// word is the day's, not the range's.
    #[test]
    fn date_with_equal_bounds_renders_one_date_and_the_day_word() {
        let v = date_value("1471-03-21", Some("1471-03-21"), DatePrecision::Day, None, None, None);
        let de = make_lang("de");
        assert_eq!(format_date_text(&v, &de), "1471-03-21");
        let out = render_value_opts(&v, &de, None, true).into_string();
        assert_eq!(out, "<span class=\"v-date\">1471-03-21 <span class=\"prec\">Tag</span></span>");
    }

    /// (b) A real range renders `start–end` and the range word.
    #[test]
    fn date_real_range_renders_both_bounds_and_the_range_word() {
        let v = date_value("1475-01-01", Some("1499-12-31"), DatePrecision::Year, None, None, None);
        let de = make_lang("de");
        let out = render_value_opts(&v, &de, None, true).into_string();
        assert_eq!(
            out,
            "<span class=\"v-date\">1475\u{2013}1499 <span class=\"prec\">Zeitraum</span></span>"
        );
    }

    /// (c) A display string in the request language — or language-universal —
    /// renders verbatim.
    #[test]
    fn date_display_in_request_language_renders_verbatim() {
        let v = date_value(
            "1491-07-28",
            Some("1491-07-28"),
            DatePrecision::Day,
            Some("28. Juli 1491"),
            Some("de"),
            None,
        );
        assert_eq!(format_date_text(&v, &make_lang("de")), "28. Juli 1491");

        let universal = date_value("1491-07-28", None, DatePrecision::Day, Some("um 1491"), None, None);
        assert_eq!(format_date_text(&universal, &make_lang("en")), "um 1491");
        assert_eq!(format_date_text(&universal, &make_lang("de")), "um 1491");
    }

    /// (d) A display string in ANOTHER language composes the long
    /// form from the ISO bounds — never the other language's string, never
    /// ISO truncation.
    #[test]
    fn date_display_in_other_language_composes_the_long_form() {
        let de_display =
            |start, end, precision| date_value(start, end, precision, Some("Katalogtext"), Some("de"), None);
        let text = |v: &PropertyValue, code| format_date_text(v, &make_lang(code));

        let day = de_display("1491-02-19", Some("1491-02-19"), DatePrecision::Day);
        assert_eq!(text(&day, "en"), "19 February 1491");
        assert_eq!(text(&day, "fr"), "19 février 1491");
        assert_eq!(text(&day, "it"), "19 febbraio 1491");

        let first = de_display("1491-02-01", None, DatePrecision::Day);
        assert_eq!(text(&first, "fr"), "1er février 1491");
        assert_eq!(text(&first, "en"), "1 February 1491");

        let month = de_display("1491-02-01", None, DatePrecision::Month);
        assert_eq!(text(&month, "en"), "February 1491");

        let year = de_display("1489-01-01", Some("1489-12-31"), DatePrecision::Year);
        assert_eq!(text(&year, "en"), "1489");

        let range = de_display("1475-01-01", Some("1499-12-31"), DatePrecision::Year);
        assert_eq!(text(&range, "en"), "1475\u{2013}1499");

        let day_range = de_display("1491-02-19", Some("1491-03-03"), DatePrecision::Day);
        assert_eq!(text(&day_range, "en"), "19 February 1491\u{2013}3 March 1491");

        // The DE long form, for a display string written in English.
        let en_display = date_value(
            "1491-02-19",
            None,
            DatePrecision::Day,
            Some("19 February 1491"),
            Some("en"),
            None,
        );
        assert_eq!(text(&en_display, "de"), "19. Februar 1491");
    }

    /// (d′) No display string keeps today's ISO truncation in every language.
    #[test]
    fn date_without_display_keeps_iso_truncation() {
        let day = date_value("1865-07-04", None, DatePrecision::Day, None, None, None);
        let month = date_value("1865-07-04", None, DatePrecision::Month, None, None, None);
        for code in ["en", "de", "fr", "it"] {
            assert_eq!(format_date_text(&day, &make_lang(code)), "1865-07-04");
            assert_eq!(format_date_text(&month, &make_lang(code)), "1865-07");
        }
    }

    /// (e) The precision word is off unless the site asks for it:
    /// `render_value` (the thin wrapper) never emits `span.prec`.
    #[test]
    fn date_precision_word_is_off_by_default() {
        let v = date_value("1489-01-01", None, DatePrecision::Year, None, None, Some(Calendar::Gregorian));
        let out = render_value(&v, &make_lang("de"), None).into_string();
        assert_eq!(out, "<span class=\"v-date\">1489</span>");
        let off = render_value_opts(&v, &make_lang("de"), None, false).into_string();
        assert_eq!(off, out);
    }

    /// (f) Each precision word, the Gregorian calendar word, and NO calendar
    /// word for Julian or an absent calendar.
    #[test]
    fn date_precision_words_and_only_the_gregorian_calendar_word() {
        let prec = |precision, end: Option<&str>, calendar, code| {
            let v = date_value("1491-02-19", end, precision, None, None, calendar);
            let out = render_value_opts(&v, &make_lang(code), None, true).into_string();
            let start = out.find("<span class=\"prec\">").expect("span.prec present") + "<span class=\"prec\">".len();
            let end = out[start..].find("</span>").expect("span.prec closes") + start;
            out[start..end].to_owned()
        };
        assert_eq!(prec(DatePrecision::Day, None, None, "de"), "Tag");
        assert_eq!(prec(DatePrecision::Month, None, None, "de"), "Monat");
        assert_eq!(prec(DatePrecision::Year, None, None, "de"), "Jahr");
        assert_eq!(prec(DatePrecision::Year, Some("1499-12-31"), None, "de"), "Zeitraum");
        assert_eq!(prec(DatePrecision::Day, None, None, "en"), "day");
        assert_eq!(prec(DatePrecision::Year, Some("1499-12-31"), None, "en"), "range");
        // Year precision (ranges included) hides the Gregorian word.
        assert_eq!(prec(DatePrecision::Year, None, Some(Calendar::Gregorian), "de"), "Jahr");
        assert_eq!(
            prec(DatePrecision::Year, Some("1499-12-31"), Some(Calendar::Gregorian), "de"),
            "Zeitraum"
        );
        assert_eq!(
            prec(DatePrecision::Day, None, Some(Calendar::Gregorian), "de"),
            "Tag · gregorianisch"
        );
        assert_eq!(
            prec(DatePrecision::Month, None, Some(Calendar::Gregorian), "en"),
            "month · Gregorian"
        );
        assert_eq!(prec(DatePrecision::Year, None, Some(Calendar::Julian), "de"), "Jahr");
    }

    /// The flag reaches the `.props` row through `render_property_block`.
    #[test]
    fn render_property_block_threads_the_precision_word_flag() {
        let prop = Property {
            name: "pub-date".to_owned(),
            values: vec![date_value("1491-07-28", None, DatePrecision::Day, None, None, None)],
        };
        let de = make_lang("de");
        let on = render_property_block(&prop, &de, &[], None, true).into_string();
        assert!(on.contains("<span class=\"prec\">Tag</span>"), "{on}");
        let off = render_property_block(&prop, &de, &[], None, false).into_string();
        assert!(!off.contains("prec"), "{off}");
    }

    fn make_lang(code: &str) -> Lang {
        use cpe_core::ir::SiteConfig;
        // Include the requested code in the declared languages so `resolve`
        // returns it directly rather than falling back to the default.
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

    // render_value: Text

    #[test]
    fn render_value_text_plain_wraps_v_text() {
        let v = PropertyValue::Text {
            text: "Alice Liddell".to_owned(),
            lang: Some("en".to_owned()),
        };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"v-text\""),
            "plain text must be wrapped in span.v-text, got: {out}"
        );
        assert!(out.contains("Alice Liddell"), "plain text content must appear, got: {out}");
    }

    #[test]
    fn render_value_text_escapes_html() {
        let v = PropertyValue::Text { text: "<b>bold</b>".to_owned(), lang: Some("en".to_owned()) };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(out.contains("&lt;b&gt;"), "text value must be HTML-escaped, got: {out}");
        assert!(!out.contains("<b>"), "raw <b> must not appear in output, got: {out}");
    }

    #[test]
    fn render_value_text_https_url_renders_ext_link() {
        let v = PropertyValue::Text {
            text: "https://www.wikidata.org/wiki/Q4931".to_owned(),
            lang: None,
        };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"ext-link\""),
            "https URL must render as .ext-link, got: {out}"
        );
        assert!(
            out.contains("href=\"https://www.wikidata.org/wiki/Q4931\""),
            "ext-link must carry the full href, got: {out}"
        );
        assert!(
            out.contains("rel=\"noopener noreferrer\""),
            "ext-link must have rel=noopener noreferrer, got: {out}"
        );
        assert!(
            out.contains("target=\"_blank\""),
            "ext-link must have target=_blank, got: {out}"
        );
        assert!(!out.contains("class=\"v-text\""), "URL must not render as v-text, got: {out}");
    }

    #[test]
    fn render_value_text_http_url_renders_ext_link() {
        let v = PropertyValue::Text { text: "http://example.org/resource/1".to_owned(), lang: None };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"ext-link\""),
            "http URL must render as .ext-link, got: {out}"
        );
    }

    #[test]
    fn render_value_text_relative_url_renders_ext_link() {
        let v = PropertyValue::Text { text: "/some/path".to_owned(), lang: None };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"ext-link\""),
            "relative URL must render as .ext-link, got: {out}"
        );
    }

    #[test]
    fn render_value_text_javascript_scheme_renders_plain() {
        let v = PropertyValue::Text { text: "javascript:alert(1)".to_owned(), lang: None };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"v-text\""),
            "javascript: URL must render as plain v-text, got: {out}"
        );
        assert!(!out.contains("<a"), "javascript: URL must not produce an <a>, got: {out}");
    }

    #[test]
    fn render_value_text_data_scheme_renders_plain() {
        let v = PropertyValue::Text {
            text: "data:text/html,<script>evil</script>".to_owned(),
            lang: None,
        };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"v-text\""),
            "data: URL must render as plain v-text, got: {out}"
        );
        assert!(!out.contains("<a"), "data: URL must not produce an <a>, got: {out}");
    }

    // render_value: Richtext

    #[test]
    fn render_value_richtext_wraps_v_richtext() {
        let v = PropertyValue::Richtext { text: "**bold text**".to_owned(), lang: None };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"v-richtext\""),
            "richtext must be wrapped in div.v-richtext, got: {out}"
        );
        assert!(
            out.contains("<strong>bold text</strong>"),
            "richtext must render markdown, got: {out}"
        );
    }

    #[test]
    fn render_value_richtext_strips_raw_html() {
        let v = PropertyValue::Richtext { text: "<script>alert(1)</script>".to_owned(), lang: None };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(!out.contains("<script"), "richtext must strip raw HTML, got: {out}");
    }

    // render_value: Integer / Decimal

    #[test]
    fn render_value_integer_wraps_v_number() {
        let v = PropertyValue::Integer(42);
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"v-number\""),
            "integer must be wrapped in span.v-number, got: {out}"
        );
        assert!(out.contains("42"), "integer value must appear, got: {out}");
    }

    #[test]
    fn render_value_integer_negative() {
        let v = PropertyValue::Integer(-7);
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(out.contains("-7"), "negative integer output: {out}");
    }

    #[test]
    fn render_value_decimal_wraps_v_number() {
        let v = PropertyValue::Decimal(4.5);
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"v-number\""),
            "decimal must be wrapped in span.v-number, got: {out}"
        );
        assert!(out.contains("4.5"), "decimal value must appear: {out}");
    }

    // render_value: Boolean

    #[test]
    fn render_value_boolean_true_en_wraps_v_bool() {
        let v = PropertyValue::Boolean(true);
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"v-bool\""),
            "boolean must be wrapped in span.v-bool, got: {out}"
        );
        assert!(
            out.contains("data-bool=\"true\""),
            "boolean true must have data-bool=\"true\", got: {out}"
        );
        assert!(
            !out.contains("data-on="),
            "a key-less data-on throws Datastar's KeyRequired, got: {out}"
        );
        assert!(out.contains("Yes"), "EN true must show Yes, got: {out}");
    }

    #[test]
    fn render_value_boolean_false_en_has_data_bool_false() {
        let v = PropertyValue::Boolean(false);
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("data-bool=\"false\""),
            "boolean false must have data-bool=\"false\", got: {out}"
        );
        assert!(
            !out.contains("data-on="),
            "a key-less data-on throws Datastar's KeyRequired, got: {out}"
        );
        assert!(out.contains("No"), "EN false must show No, got: {out}");
    }

    #[test]
    fn render_value_boolean_true_de() {
        let v = PropertyValue::Boolean(true);
        let out = render_value(&v, &make_lang("de"), None).into_string();
        assert!(out.contains("Ja"), "DE true must show Ja, got: {out}");
        assert!(out.contains("data-bool=\"true\""), "must carry data-bool=true, got: {out}");
    }

    #[test]
    fn render_value_boolean_false_de() {
        let v = PropertyValue::Boolean(false);
        let out = render_value(&v, &make_lang("de"), None).into_string();
        assert!(out.contains("Nein"), "DE false must show Nein, got: {out}");
        assert!(out.contains("data-bool=\"false\""), "must carry data-bool=false, got: {out}");
    }

    #[test]
    fn render_value_boolean_true_fr() {
        let v = PropertyValue::Boolean(true);
        let out = render_value(&v, &make_lang("fr"), None).into_string();
        assert!(out.contains("Oui"), "FR true must show Oui, got: {out}");
        assert!(out.contains("data-bool=\"true\""), "must carry data-bool=true, got: {out}");
    }

    #[test]
    fn render_value_boolean_false_fr() {
        let v = PropertyValue::Boolean(false);
        let out = render_value(&v, &make_lang("fr"), None).into_string();
        assert!(out.contains("Non"), "FR false must show Non, got: {out}");
        assert!(out.contains("data-bool=\"false\""), "must carry data-bool=false, got: {out}");
    }

    #[test]
    fn render_value_boolean_true_it() {
        let v = PropertyValue::Boolean(true);
        let out = render_value(&v, &make_lang("it"), None).into_string();
        assert!(out.contains("Sì"), "IT true must show Sì, got: {out}");
        assert!(out.contains("data-bool=\"true\""), "must carry data-bool=true, got: {out}");
    }

    #[test]
    fn render_value_boolean_false_it() {
        // Note: Italian false renders as "No", which is identical to the English
        // fallback. This means this assertion alone cannot distinguish a missing
        // "it" arm from the EN fallback. The real guard for the Italian arm is
        // `render_value_boolean_true_it` above, which asserts the unique string
        // "Sì" — a value that can only come from a correctly-matched "it" branch.
        let v = PropertyValue::Boolean(false);
        let out = render_value(&v, &make_lang("it"), None).into_string();
        assert!(out.contains("No"), "IT false must show No, got: {out}");
        assert!(out.contains("data-bool=\"false\""), "must carry data-bool=false, got: {out}");
        // Belt-and-suspenders: confirm the class is present too, so this
        // assertion at least validates the structural output even if the label
        // is ambiguous with EN.
        assert!(
            out.contains("class=\"v-bool\""),
            "IT false must be wrapped in span.v-bool, got: {out}"
        );
    }

    // render_value: Date

    #[test]
    fn render_value_date_wraps_v_date() {
        let v = PropertyValue::Date {
            start: "1865-07-04".to_owned(),
            end: None,
            precision: DatePrecision::Day,
            display: None,
            display_lang: None,
            calendar: None,
        };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"v-date\""),
            "date must be wrapped in span.v-date, got: {out}"
        );
        assert!(out.contains("1865-07-04"), "date value must appear, got: {out}");
    }

    #[test]
    fn render_value_date_precision_year() {
        let v = PropertyValue::Date {
            start: "1865-07-04".to_owned(),
            end: None,
            precision: DatePrecision::Year,
            display: None,
            display_lang: None,
            calendar: None,
        };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(out.contains("1865"), "year precision: {out}");
    }

    #[test]
    fn render_value_date_range() {
        let v = PropertyValue::Date {
            start: "1865-07-04".to_owned(),
            end: Some("1865-12-31".to_owned()),
            precision: DatePrecision::Day,
            display: None,
            display_lang: None,
            calendar: None,
        };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(out.contains("1865-07-04\u{2013}1865-12-31"), "range with en-dash: {out}");
    }

    // render_value: Link

    #[test]
    fn render_value_link_resolved_label_wraps_v_text_no_anchor() {
        let v = PropertyValue::Link {
            target_id: "CH_002".to_owned(),
            target_label: Some("The White Rabbit".to_owned()),
            target_class: Some("Character".to_owned()),
        };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"v-text\""),
            "internal link renders in span.v-text, got: {out}"
        );
        assert!(
            !out.contains("<a"),
            "internal link without resolver must not render an <a> (falls back to v-text), got: {out}"
        );
        assert!(out.contains("The White Rabbit"), "label must appear, got: {out}");
    }

    #[test]
    fn render_value_link_unresolved_shows_mark() {
        // target_class is None → the target genuinely does not exist.
        let v = PropertyValue::Link {
            target_id: "CH_BROKEN".to_owned(),
            target_label: None,
            target_class: None,
        };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"unresolved\""),
            "unresolved target must render span.unresolved, got: {out}"
        );
        assert!(
            out.contains("class=\"unresolved__mark\""),
            "unresolved target must carry the mark span, got: {out}"
        );
        assert!(
            out.contains("unresolved"),
            "the mark text (lower-case, CSS uppercases) must appear, got: {out}"
        );
        assert!(out.contains("CH_BROKEN"), "broken link must show target_id, got: {out}");
        assert!(!out.contains("<a"), "never an <a>, got: {out}");
        assert!(
            out.contains("This reference names a resource that is not in the archive."),
            "root span must carry the title attribute, got: {out}"
        );
    }

    #[test]
    fn render_value_link_unresolved_de_mark() {
        let v = PropertyValue::Link {
            target_id: "CH_BROKEN".to_owned(),
            target_label: None,
            target_class: None,
        };
        let out = render_value(&v, &make_lang("de"), None).into_string();
        assert!(
            out.contains("nicht aufgelöst"),
            "broken link must show DE mark text, got: {out}"
        );
    }

    #[test]
    fn render_value_link_target_exists_but_no_label_is_plain_mono_id() {
        // target_class: Some(_) with target_label: None → the target
        // resolves (it names a real resource with no `name` property), so
        // it must NOT get the UNRESOLVED mark — the mark gates on the target
        // being genuinely missing.
        let v = PropertyValue::Link {
            target_id: "CH_NO_NAME".to_owned(),
            target_label: None,
            target_class: Some("Character".to_owned()),
        };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            !out.contains("unresolved"),
            "a target with a class (even label-less) must never get the unresolved mark, got: {out}"
        );
        assert!(out.contains("CH_NO_NAME"), "the bare id must still render, got: {out}");
        assert!(!out.contains("<a"), "no resolver passed, must not be an anchor, got: {out}");
    }

    // render_value: ListValue (single → .term)

    #[test]
    fn render_value_list_value_single_resolved_wraps_term() {
        let v = PropertyValue::ListValue {
            node_id: "ROLE_001".to_owned(),
            label: Some("Protagonist".to_owned()),
        };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("class=\"term\""),
            "single list-value must render as span.term, got: {out}"
        );
        assert!(out.contains("Protagonist"), "label must appear, got: {out}");
    }

    #[test]
    fn render_value_list_value_unresolved_shows_placeholder() {
        let v = PropertyValue::ListValue { node_id: "ROLE_MISSING".to_owned(), label: None };
        let out = render_value(&v, &make_lang("en"), None).into_string();
        assert!(
            out.contains("Unresolved list value"),
            "unresolved list value must show placeholder, got: {out}"
        );
        assert!(out.contains("ROLE_MISSING"), "unresolved list must show node_id, got: {out}");
    }

    // Keyword split

    #[test]
    fn render_keyword_item_multi_segment_has_path_and_leaf_spans() {
        let v = PropertyValue::ListValue {
            node_id: "KW_001".to_owned(),
            label: Some("Theme \u{2013} Fantasy \u{2013} Surrealism".to_owned()),
        };
        let out = render_keyword_item(&v, &make_lang("en")).into_string();
        assert!(out.contains("class=\"path\""), "keyword must have path spans, got: {out}");
        assert!(out.contains("class=\"leaf\""), "keyword must have leaf span, got: {out}");
        assert!(
            out.contains("class=\"keyword\""),
            "keyword must have .keyword class, got: {out}"
        );
    }

    #[test]
    fn render_keyword_item_single_segment_leaf_only() {
        let v = PropertyValue::ListValue {
            node_id: "KW_001".to_owned(),
            label: Some("Fantasy".to_owned()),
        };
        let out = render_keyword_item(&v, &make_lang("en")).into_string();
        assert!(
            !out.contains("class=\"path\""),
            "single-segment keyword must have no path span, got: {out}"
        );
        assert!(
            out.contains("class=\"leaf\""),
            "single-segment keyword must still have leaf span, got: {out}"
        );
    }

    #[test]
    fn render_keyword_item_none_label_renders_placeholder() {
        let v = PropertyValue::ListValue { node_id: "KW_MISSING".to_owned(), label: None };
        let out = render_keyword_item(&v, &make_lang("en")).into_string();
        assert!(
            out.contains("Unresolved list value"),
            "None label must render placeholder in li, got: {out}"
        );
    }

    // Multi-valued ListValue property → ul.keyword-set

    #[test]
    fn render_property_block_multi_list_value_renders_keyword_set() {
        let prop = Property {
            name: "keyword".to_owned(),
            values: vec![
                PropertyValue::ListValue {
                    node_id: "KW_001".to_owned(),
                    label: Some("Fantasy \u{2013} Surrealism".to_owned()),
                },
                PropertyValue::ListValue {
                    node_id: "KW_002".to_owned(),
                    label: Some("Adventure".to_owned()),
                },
            ],
        };
        let out = render_property_block(&prop, &make_lang("en"), &[], None, false).into_string();
        assert!(
            out.contains("class=\"keyword-set\""),
            "multi list-value property must render ul.keyword-set, got: {out}"
        );
        assert!(out.contains("class=\"keyword\""), "each value must be a li.keyword, got: {out}");
    }

    #[test]
    fn render_property_block_single_list_value_renders_term() {
        let prop = Property {
            name: "role".to_owned(),
            values: vec![PropertyValue::ListValue {
                node_id: "ROLE_001".to_owned(),
                label: Some("Protagonist".to_owned()),
            }],
        };
        let out = render_property_block(&prop, &make_lang("en"), &[], None, false).into_string();
        assert!(
            out.contains("class=\"term\""),
            "single list-value property must render span.term, got: {out}"
        );
        assert!(
            !out.contains("class=\"keyword-set\""),
            "single list-value must not render keyword-set, got: {out}"
        );
    }

    // No double-render: title + quote excluded from .props

    #[test]
    fn render_resource_detail_title_not_in_props() {
        // Use a unique title that won't appear elsewhere in the rendered output.
        let resource = Resource {
            id: "CH_001".to_owned(),
            class: "Character".to_owned(),
            properties: vec![
                Property {
                    name: "name".to_owned(),
                    values: vec![PropertyValue::Text { text: "Alice".to_owned(), lang: Some("en".to_owned()) }],
                },
                Property {
                    name: "description".to_owned(),
                    values: vec![PropertyValue::Text {
                        text: "A curious girl.".to_owned(),
                        lang: Some("en".to_owned()),
                    }],
                },
            ],
            representations: vec![],
        };
        let show: Vec<String> = vec!["name".to_owned(), "description".to_owned()];
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            head: None,
            id: "CH_001",
            title_property: Some("name"),
            show_properties: &show,
            media_property: None,
            media_items: &[],
            selected_image: 0,
            strip_open: true,
            compound_members: &[],
            current_path: "/test/path",
            current_query: None,
            links: None,
            property_labels: &[],
            class_labels: &[],
            groups: &[],
            intro: None,
            navigator: false,
            incoming: None,
            date_precision_word: false,
        };
        let out = render_resource_detail(&resource, cfg, &make_lang("en")).into_string();
        // Title "Alice" renders in h1.record__title.
        assert!(out.contains("record__title"), "title must appear in record__title, got: {out}");
        // "Alice" must appear EXACTLY ONCE in the output — it is pulled into the h1
        // title and must not also appear as a .prop__value in the props list.
        let alice_count = out.matches("Alice").count();
        assert_eq!(
            alice_count, 1,
            "title value 'Alice' must appear exactly once (in h1 only, not also as a prop value), \
             got {alice_count} occurrences in: {out}"
        );
        // "Name" prop label must NOT appear in .props (it's excluded).
        assert!(!out.contains(">Name<"), "Name prop label must not appear in props, got: {out}");
        // Description prop SHOULD appear.
        assert!(out.contains("Description"), "description prop must appear in props, got: {out}");
    }

    #[test]
    fn render_resource_detail_quote_not_in_props() {
        // Use a unique quote phrase that won't recur elsewhere in the output.
        let resource = Resource {
            id: "CH_001".to_owned(),
            class: "Character".to_owned(),
            properties: vec![
                Property {
                    name: "name".to_owned(),
                    values: vec![PropertyValue::Text { text: "Alice".to_owned(), lang: Some("en".to_owned()) }],
                },
                Property {
                    name: "quote".to_owned(),
                    values: vec![PropertyValue::Text {
                        text: "Curiouser and curiouser!".to_owned(),
                        lang: Some("en".to_owned()),
                    }],
                },
                Property {
                    name: "description".to_owned(),
                    values: vec![PropertyValue::Text {
                        text: "A curious girl.".to_owned(),
                        lang: Some("en".to_owned()),
                    }],
                },
            ],
            representations: vec![],
        };
        let show: Vec<String> = vec!["name".to_owned(), "quote".to_owned(), "description".to_owned()];
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            head: None,
            id: "CH_001",
            title_property: Some("name"),
            show_properties: &show,
            media_property: None,
            media_items: &[],
            selected_image: 0,
            strip_open: true,
            compound_members: &[],
            current_path: "/test/path",
            current_query: None,
            links: None,
            property_labels: &[],
            class_labels: &[],
            groups: &[],
            intro: None,
            navigator: false,
            incoming: None,
            date_precision_word: false,
        };
        let out = render_resource_detail(&resource, cfg, &make_lang("en")).into_string();
        // Quote renders in blockquote.record__quote.
        assert!(
            out.contains("class=\"record__quote\""),
            "quote must appear in record__quote, got: {out}"
        );
        // The quote text "Curiouser and curiouser!" must appear EXACTLY ONCE:
        // inside the <blockquote> header, and NOT again as a .prop__value in the
        // props list. Note: it also appears in <cite> within the same blockquote,
        // but "Curiouser and curiouser!" itself should only appear once as the quote.
        // The <cite> contains the title "Alice", not the quote text.
        let quote_count = out.matches("Curiouser and curiouser!").count();
        assert_eq!(
            quote_count, 1,
            "quote text must appear exactly once (in blockquote only, not also as a prop value), \
             got {quote_count} occurrences in: {out}"
        );
        // "Quote" must NOT appear as a prop label.
        assert!(
            !out.contains(">Quote<"),
            "Quote prop label must not appear in props, got: {out}"
        );
        // Description should still appear as a prop.
        assert!(out.contains("Description"), "description must still be in props, got: {out}");
    }
    // Body --noimg modifier

    #[test]
    fn render_resource_detail_noimg_modifier_when_no_image() {
        let resource = Resource {
            id: "LOC_001".to_owned(),
            class: "Location".to_owned(),
            properties: vec![Property {
                name: "name".to_owned(),
                values: vec![PropertyValue::Text { text: "Wonderland".to_owned(), lang: Some("en".to_owned()) }],
            }],
            representations: vec![],
        };
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            head: None,
            id: "LOC_001",
            title_property: Some("name"),
            show_properties: &[],
            media_property: None,
            media_items: &[],
            selected_image: 0,
            strip_open: true,
            compound_members: &[],
            current_path: "/test/path",
            current_query: None,
            links: None,
            property_labels: &[],
            class_labels: &[],
            groups: &[],
            intro: None,
            navigator: false,
            incoming: None,
            date_precision_word: false,
        };
        let out = render_resource_detail(&resource, cfg, &make_lang("en")).into_string();
        assert!(
            out.contains("record__body--noimg"),
            "no-image resource must have --noimg modifier, got: {out}"
        );
    }

    #[test]
    fn render_resource_detail_no_noimg_when_has_image() {
        // "Has an image" is driven by
        // `cfg.media_items` (pre-hydrated linked Image resources), not the
        // host's own `representations` field.
        let resource = Resource {
            id: "CH_001".to_owned(),
            class: "Character".to_owned(),
            properties: vec![],
            representations: vec![],
        };
        let media_items = vec![Resource {
            id: "I_001".to_owned(),
            class: "Image".to_owned(),
            properties: vec![],
            representations: vec![Representation {
                kind: RepresentationKind::Image,
                uri: "https://example.org/img.jpg".to_owned(),
                width: Some(100),
                height: Some(100),
                alt: None,
                attribution: None,
                seq: 0,
            }],
        }];
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            head: None,
            id: "CH_001",
            title_property: None,
            show_properties: &[],
            media_property: Some("image"),
            media_items: &media_items,
            selected_image: 1,
            strip_open: true,
            compound_members: &[],
            current_path: "/test/path",
            current_query: None,
            links: None,
            property_labels: &[],
            class_labels: &[],
            groups: &[],
            intro: None,
            navigator: false,
            incoming: None,
            date_precision_word: false,
        };
        let out = render_resource_detail(&resource, cfg, &make_lang("en")).into_string();
        assert!(
            !out.contains("record__body--noimg"),
            "resource with media items must not have --noimg modifier, got: {out}"
        );
        assert!(
            out.contains("record--viewer"),
            "resource with media items must carry the record--viewer class \
             (record-viewer plan step 4 — record--media is no longer reachable \
             from render_resource_detail), got: {out}"
        );
        // This fixture's URI (`https://example.org/img.jpg`) is not
        // IIIF-shaped, so the viewer must degrade to the plain `.rv__stand`
        // fallback and never mount `<image-viewer>` at all — asserted explicitly here
        // (not just incidentally via the snapshot tests below).
        assert!(
            !out.contains("<image-viewer"),
            "a non-IIIF representation must never mount <image-viewer>, got: {out}"
        );
        assert!(
            out.contains("rv__stand"),
            "a non-IIIF representation must degrade to the .rv__stand fallback, got: {out}"
        );
    }

    #[test]
    fn render_resource_detail_wider_wires_expand_class_and_inert_head_body() {
        // WIDER: a media-bearing record's
        // article toggles `.is-wide` from `$expand` via `data-class` (NOT a
        // reactive `data-expand` attribute — that would collide with DataStar's
        // `data-*` namespace), and its head + body become genuinely inert +
        // aria-hidden while expanded.
        let resource = Resource {
            id: "CH_001".to_owned(),
            class: "Character".to_owned(),
            properties: vec![],
            representations: vec![],
        };
        let media_items = vec![Resource {
            id: "I_001".to_owned(),
            class: "Image".to_owned(),
            properties: vec![],
            representations: vec![Representation {
                kind: RepresentationKind::Image,
                uri: "https://example.org/img.jpg".to_owned(),
                width: Some(100),
                height: Some(100),
                alt: None,
                attribution: None,
                seq: 0,
            }],
        }];
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            head: None,
            id: "CH_001",
            title_property: None,
            show_properties: &[],
            media_property: Some("image"),
            media_items: &media_items,
            selected_image: 1,
            strip_open: true,
            compound_members: &[],
            current_path: "/test/path",
            current_query: None,
            links: None,
            property_labels: &[],
            class_labels: &[],
            groups: &[],
            intro: None,
            navigator: false,
            incoming: None,
            date_precision_word: false,
        };
        let out = render_resource_detail(&resource, cfg, &make_lang("en")).into_string();
        assert!(
            !out.contains("data-expand"),
            "the old static data-expand attribute must be gone (CSS keys on .is-wide now), got: {out}"
        );
        assert!(
            out.contains(r#"data-class="{'is-wide': $expand}""#),
            "the article must toggle .is-wide from $expand via data-class, got: {out}"
        );
        assert!(
            out.contains(
                r#"data-attr="{'inert': $expand ? 'true' : false, 'aria-hidden': $expand ? 'true' : 'false'}""#
            ),
            "the head + body must become inert + aria-hidden while expanded, got: {out}"
        );
        // Both the head AND the body carry the inert binding (two occurrences).
        assert_eq!(
            out.matches("'inert': $expand").count(),
            2,
            "exactly the head and the body must carry the inert binding, got: {out}"
        );
    }

    #[test]
    fn render_resource_detail_no_media_has_no_wider_wiring() {
        // A record with no media carries no viewer, so no WIDER signal, class
        // toggle, or inert binding — the head/body attrs are omitted entirely.
        let resource = Resource {
            id: "CH_001".to_owned(),
            class: "Character".to_owned(),
            properties: vec![],
            representations: vec![],
        };
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            head: None,
            id: "CH_001",
            title_property: None,
            show_properties: &[],
            media_property: None,
            media_items: &[],
            selected_image: 0,
            strip_open: false,
            compound_members: &[],
            current_path: "/test/path",
            current_query: None,
            links: None,
            property_labels: &[],
            class_labels: &[],
            groups: &[],
            intro: None,
            navigator: false,
            incoming: None,
            date_precision_word: false,
        };
        let out = render_resource_detail(&resource, cfg, &make_lang("en")).into_string();
        assert!(
            !out.contains("is-wide") && !out.contains("$expand") && !out.contains("data-expand"),
            "a media-less record must carry no WIDER wiring at all, got: {out}"
        );
    }

    // humanize_property_name

    #[test]
    fn humanize_property_name_titlecases_and_splits() {
        assert_eq!(humanize_property_name("name"), "Name");
        assert_eq!(humanize_property_name("wikidata-link"), "Wikidata Link");
        assert_eq!(humanize_property_name("location-type"), "Location Type");
        assert_eq!(humanize_property_name("full_text"), "Full Text");
        assert_eq!(humanize_property_name("Date-Published"), "Date Published");
        assert_eq!(humanize_property_name("a--b__c"), "A B C");
    }

    // Language pick

    #[test]
    fn pick_values_exact_lang_match() {
        let prop = Property {
            name: "description".to_owned(),
            values: vec![
                PropertyValue::Text { text: "English".to_owned(), lang: Some("en".to_owned()) },
                PropertyValue::Text { text: "Deutsch".to_owned(), lang: Some("de".to_owned()) },
            ],
        };
        let picked = pick_values(&prop, &make_lang("en"));
        assert_eq!(picked.len(), 1);
        if let PropertyValue::Text { text, .. } = picked[0] {
            assert_eq!(text, "English");
        } else {
            panic!("unexpected variant");
        }
    }

    #[test]
    fn pick_values_non_text_renders_all() {
        let prop = Property {
            name: "count".to_owned(),
            values: vec![PropertyValue::Integer(1), PropertyValue::Integer(2)],
        };
        let picked = pick_values(&prop, &make_lang("en"));
        assert_eq!(picked.len(), 2, "all integer values rendered");
    }

    // Title resolution

    // The Alice fixture's
    // `name` value is identical in EN/DE ("Alice"), so the snapshot tests never
    // exercised the DE branch of `resolve_title_str`'s language pick. This
    // fixture uses EN/DE values that genuinely differ so a wrong-language bug
    // would fail the assertion below rather than passing by coincidence.
    #[test]
    fn title_derivation_picks_de_value_when_present() {
        let resource = Resource {
            id: "CH_002".to_owned(),
            class: "Character".to_owned(),
            properties: vec![Property {
                name: "name".to_owned(),
                values: vec![
                    PropertyValue::Text {
                        text: "The White Rabbit".to_owned(),
                        lang: Some("en".to_owned()),
                    },
                    PropertyValue::Text {
                        text: "Das weiße Kaninchen".to_owned(),
                        lang: Some("de".to_owned()),
                    },
                ],
            }],
            representations: vec![],
        };

        let de_title = resolve_title_str(&resource, Some("name"), &make_lang("de"));
        assert_eq!(
            de_title, "Das weiße Kaninchen",
            "DE lang must pick the DE-tagged value, got: {de_title}"
        );

        let en_title = resolve_title_str(&resource, Some("name"), &make_lang("en"));
        assert_eq!(
            en_title, "The White Rabbit",
            "EN lang must pick the EN-tagged value, got: {en_title}"
        );
    }

    // Layer-3: insta snapshots

    fn make_alice_resource() -> Resource {
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
                    name: "quote".to_owned(),
                    values: vec![
                        PropertyValue::Text {
                            text: "Curiouser and curiouser!".to_owned(),
                            lang: Some("en".to_owned()),
                        },
                        PropertyValue::Text {
                            text: "Immer neugieriger!".to_owned(),
                            lang: Some("de".to_owned()),
                        },
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
                Property {
                    name: "keyword".to_owned(),
                    values: vec![
                        PropertyValue::ListValue {
                            node_id: "KW_001".to_owned(),
                            label: Some("Fiction \u{2013} Fantasy".to_owned()),
                        },
                        PropertyValue::ListValue {
                            node_id: "KW_002".to_owned(),
                            label: Some("Adventure".to_owned()),
                        },
                    ],
                },
                Property {
                    name: "wikidata-link".to_owned(),
                    values: vec![PropertyValue::Text {
                        text: "https://www.wikidata.org/wiki/Q179515".to_owned(),
                        lang: None,
                    }],
                },
                Property {
                    name: "related".to_owned(),
                    values: vec![PropertyValue::Link {
                        target_id: "CH_002".to_owned(),
                        target_label: Some("The White Rabbit".to_owned()),
                        target_class: Some("Character".to_owned()),
                    }],
                },
            ],
            representations: vec![],
        }
    }

    /// The Alice resource's linked media (an
    /// image is its own Resource; a host embeds it via a `link` property, so
    /// `make_alice_resource` itself carries no `representations` row).
    fn make_alice_media() -> Vec<Resource> {
        vec![Resource {
            id: "I_001".to_owned(),
            class: "Image".to_owned(),
            properties: vec![],
            representations: vec![Representation {
                kind: RepresentationKind::Image,
                uri: "https://example.org/iiif/alice.jpg".to_owned(),
                width: Some(800),
                height: Some(600),
                alt: None,
                attribution: None,
                seq: 0,
            }],
        }]
    }

    fn make_alice_cfg<'a>(
        id: &'a str,
        title_property: Option<&'a str>,
        show_properties: &'a [String],
        media_items: &'a [Resource],
    ) -> ResourceDetailCfg<'a> {
        ResourceDetailCfg {
            reader: None,
            annotations: None,
            head: None,
            id,
            title_property,
            show_properties,
            media_property: (!media_items.is_empty()).then_some("image"),
            media_items,
            selected_image: 1,
            strip_open: true,
            compound_members: &[],
            current_path: "/test/path",
            current_query: None,
            links: None,
            property_labels: &[],
            class_labels: &[],
            groups: &[],
            intro: None,
            navigator: false,
            incoming: None,
            date_precision_word: false,
        }
    }

    // v-date snapshots

    /// A Book with a title, a publication date and a place — the book-page
    /// row shape the date renderer's snapshots pin.
    fn make_dated_book(date: PropertyValue) -> Resource {
        Resource {
            id: "narrenschiff-dt".to_owned(),
            class: "Book".to_owned(),
            properties: vec![
                Property {
                    name: "name".to_owned(),
                    values: vec![PropertyValue::Text { text: "Das Narrenschiff".to_owned(), lang: None }],
                },
                Property { name: "pub-date".to_owned(), values: vec![date] },
                Property {
                    name: "place".to_owned(),
                    values: vec![PropertyValue::Text { text: "Basel".to_owned(), lang: None }],
                },
            ],
            representations: vec![],
        }
    }

    fn render_dated_book(date: PropertyValue, precision_word: bool, lang: &str) -> String {
        let resource = make_dated_book(date);
        let show_props: Vec<String> = vec!["pub-date".to_owned(), "place".to_owned()];
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            date_precision_word: precision_word,
            ..make_alice_cfg("narrenschiff-dt", Some("name"), &show_props, &[])
        };
        render_resource_detail(&resource, cfg, &make_lang(lang)).into_string()
    }

    /// A day-precise date stored as `start == end`, with a DE display string
    /// and the precision word on: one date (verbatim), then `span.prec Tag`.
    #[test]
    fn snapshot_render_resource_detail_date_day_word_de() {
        let date = date_value(
            "1494-02-11",
            Some("1494-02-11"),
            DatePrecision::Day,
            Some("11. Februar 1494"),
            Some("de"),
            Some(Calendar::Julian),
        );
        insta::assert_snapshot!(render_dated_book(date, true, "de"));
    }

    /// A year-precise real range with the precision word on and the
    /// Gregorian calendar: `1475–1499` + `span.prec Zeitraum` (no calendar word at year precision).
    #[test]
    fn snapshot_render_resource_detail_date_range_word_de() {
        let date = date_value(
            "1475-01-01",
            Some("1499-12-31"),
            DatePrecision::Year,
            None,
            None,
            Some(Calendar::Gregorian),
        );
        insta::assert_snapshot!(render_dated_book(date, true, "de"));
    }

    /// The flag off (every project that does not opt in): the EN visitor
    /// gets the composed long form and no `span.prec` at all.
    #[test]
    fn snapshot_render_resource_detail_date_flag_off_en() {
        let date = date_value(
            "1494-02-11",
            Some("1494-02-11"),
            DatePrecision::Day,
            Some("11. Februar 1494"),
            Some("de"),
            Some(Calendar::Gregorian),
        );
        insta::assert_snapshot!(render_dated_book(date, false, "en"));
    }

    #[test]
    fn snapshot_render_resource_detail_en() {
        let resource = make_alice_resource();
        let show_props: Vec<String> = vec![
            "name".to_owned(),
            "quote".to_owned(),
            "description".to_owned(),
            "role".to_owned(),
            "keyword".to_owned(),
            "wikidata-link".to_owned(),
            "related".to_owned(),
        ];
        let media = make_alice_media();
        let cfg = make_alice_cfg("CH_001", Some("name"), &show_props, &media);
        let out = render_resource_detail(&resource, cfg, &make_lang("en")).into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_render_resource_detail_de() {
        let resource = make_alice_resource();
        let show_props: Vec<String> = vec![
            "name".to_owned(),
            "quote".to_owned(),
            "description".to_owned(),
            "role".to_owned(),
            "keyword".to_owned(),
            "wikidata-link".to_owned(),
            "related".to_owned(),
        ];
        let media = make_alice_media();
        let cfg = make_alice_cfg("CH_001", Some("name"), &show_props, &media);
        let out = render_resource_detail(&resource, cfg, &make_lang("de")).into_string();
        insta::assert_snapshot!(out);
    }

    /// With `property_labels`/`class_labels`
    /// declared, the header eyebrow and the `role` property's `div.prop__label`
    /// must show the DECLARED translation, not the humanized fallback
    /// (`make_alice_cfg`'s empty-slice tests above lock the fallback path;
    /// this test locks the declared-translation path).
    #[test]
    fn snapshot_render_resource_detail_with_labels_en() {
        use cpe_core::ir::{ClassLabel, PropertyLabel};

        let resource = make_alice_resource();
        let show_props: Vec<String> = vec!["name".to_owned(), "role".to_owned()];
        let property_labels = vec![PropertyLabel {
            property: "role".to_owned(),
            translations: vec![
                ("en".to_owned(), "Character role".to_owned()),
                ("de".to_owned(), "Rolle der Figur".to_owned()),
            ],
        }];
        let class_labels = vec![ClassLabel {
            class: "Character".to_owned(),
            translations: vec![
                ("en".to_owned(), "Cast member".to_owned()),
                ("de".to_owned(), "Besetzungsmitglied".to_owned()),
            ],
            plural: vec![],
        }];
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            head: None,
            id: "CH_001",
            title_property: Some("name"),
            show_properties: &show_props,
            media_property: None,
            media_items: &[],
            selected_image: 0,
            strip_open: true,
            compound_members: &[],
            current_path: "/test/path",
            current_query: None,
            links: None,
            property_labels: &property_labels,
            class_labels: &class_labels,
            groups: &[],
            intro: None,
            navigator: false,
            incoming: None,
            date_precision_word: false,
        };
        let out = render_resource_detail(&resource, cfg, &make_lang("en")).into_string();
        assert!(
            out.contains("Cast member"),
            "declared class label must render in the eyebrow, got: {out}"
        );
        assert!(
            out.contains("Character role"),
            "declared property label must render as the role prop__label, got: {out}"
        );
        assert!(
            !out.contains(">Role<"),
            "humanized fallback 'Role' must NOT appear when a label is declared, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_render_not_found_en() {
        let routes: HashMap<String, String> = [("Character".to_owned(), "/characters".to_owned())].into();
        let out = render_not_found(
            Some("Character"),
            cpe_core::url::ROOT.project("alice-in-daschland"),
            &make_alice_not_found_site(),
            &routes,
            &make_lang("en"),
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_render_not_found_de() {
        let routes: HashMap<String, String> = [("Character".to_owned(), "/characters".to_owned())].into();
        let out = render_not_found(
            Some("Character"),
            cpe_core::url::ROOT.project("alice-in-daschland"),
            &make_alice_not_found_site(),
            &routes,
            &make_lang("de"),
        )
        .into_string();
        insta::assert_snapshot!(out);
    }

    /// An Alice-shaped `SiteConfig` for the project-404 body: the real
    /// project title and its real six-item nav (chrome-id labels, EN/DE), so
    /// the `site.nav.iter().take(3)` cap is exercised — the baseline that
    /// stays byte-identical when no `not-found {}` block
    /// is declared.
    fn make_alice_not_found_site() -> SiteConfig {
        use cpe_core::ir::{ChromeString, NavItem};
        let nav = [
            ("nav-home", "/", "Home", "Startseite"),
            ("nav-wonderland", "/wonderland", "Wonderland", "Wunderland"),
            ("nav-looking-glass", "/looking-glass", "Looking-Glass", "Durch den Spiegel"),
            ("nav-characters", "/characters", "Characters", "Figuren"),
            ("nav-locations", "/locations", "Locations", "Orte"),
            ("nav-chapters", "/chapters", "Chapters", "Kapitel"),
        ];
        SiteConfig {
            title: "Alice in DaSCHland".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["en".to_owned(), "de".to_owned()],
            default_language: "en".to_owned(),
            nav: nav
                .iter()
                .map(|(id, route, _, _)| NavItem { label: (*id).to_owned(), route: (*route).to_owned() })
                .collect(),
            chrome: nav
                .iter()
                .map(|(id, _, en, de)| ChromeString {
                    id: (*id).to_owned(),
                    translations: vec![("en".to_owned(), (*en).to_owned()), ("de".to_owned(), (*de).to_owned())],
                })
                .collect(),
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

    /// Baseline: the Alice project-404 body, EN.
    #[test]
    fn snapshot_render_not_found_project_body_alice_en() {
        let site = make_alice_not_found_site();
        let out =
            render_not_found_project_body(&site, cpe_core::url::ROOT.project("alice-in-daschland"), &make_lang("en"))
                .into_string();
        insta::assert_snapshot!(out);
    }

    /// Baseline, DE — see `snapshot_render_not_found_project_body_alice_en`.
    #[test]
    fn snapshot_render_not_found_project_body_alice_de() {
        let site = make_alice_not_found_site();
        let out =
            render_not_found_project_body(&site, cpe_core::url::ROOT.project("alice-in-daschland"), &make_lang("de"))
                .into_string();
        insta::assert_snapshot!(out);
    }

    /// `make_alice_not_found_site()` with a `not-found { browse }` block: the
    /// three entries exercise the three label rungs — nav label (`/characters`),
    /// explicit chrome-id label (`/chapters`), and route fallback (`/story`,
    /// no nav item; the nav-route check is a boot concern, not the renderer's).
    fn make_alice_not_found_site_with_browse() -> SiteConfig {
        use cpe_core::ir::{ChromeString, NotFoundBrowse};
        let mut site = make_alice_not_found_site();
        site.chrome.push(ChromeString {
            id: "nf-browse-chapters".to_owned(),
            translations: vec![
                ("en".to_owned(), "All chapters".to_owned()),
                ("de".to_owned(), "Alle Kapitel".to_owned()),
            ],
        });
        site.not_found_browse = vec![
            NotFoundBrowse { route: "/characters".to_owned(), label: None },
            NotFoundBrowse {
                route: "/chapters".to_owned(),
                label: Some("nf-browse-chapters".to_owned()),
            },
            NotFoundBrowse { route: "/story".to_owned(), label: None },
        ];
        site
    }

    /// The `(href, text)` pairs of the project-404 body's "Or browse:" links.
    fn nf_browse_links(out: &str) -> Vec<(String, String)> {
        let start = out.find("<p class=\"notfound__or\">").expect("N1 body renders the or-line");
        let end = start + out[start..].find("</p>").expect("or-line closes");
        out[start..end]
            .split("<a href=\"")
            .skip(1)
            .map(|chunk| {
                let (href, rest) = chunk.split_once("\">").expect("anchor href closes");
                let text = rest.split_once("</a>").expect("anchor closes").0;
                (href.to_owned(), text.to_owned())
            })
            .collect()
    }

    #[test]
    fn not_found_browse_empty_falls_back_to_first_three_nav_items() {
        let out = render_not_found_project_body(
            &make_alice_not_found_site(),
            cpe_core::url::ROOT.project("alice-in-daschland"),
            &make_lang("en"),
        )
        .into_string();
        assert_eq!(
            nf_browse_links(&out),
            vec![
                ("/alice-in-daschland/".to_owned(), "Home".to_owned()),
                ("/alice-in-daschland/wonderland".to_owned(), "Wonderland".to_owned()),
                ("/alice-in-daschland/looking-glass".to_owned(), "Looking-Glass".to_owned()),
            ]
        );
    }

    #[test]
    fn not_found_browse_labels_resolve_nav_then_explicit_then_route() {
        let site = make_alice_not_found_site_with_browse();
        let en =
            render_not_found_project_body(&site, cpe_core::url::ROOT.project("alice-in-daschland"), &make_lang("en"))
                .into_string();
        assert_eq!(
            nf_browse_links(&en),
            vec![
                ("/alice-in-daschland/characters".to_owned(), "Characters".to_owned()),
                ("/alice-in-daschland/chapters".to_owned(), "All chapters".to_owned()),
                ("/alice-in-daschland/story".to_owned(), "/story".to_owned()),
            ]
        );
        let de =
            render_not_found_project_body(&site, cpe_core::url::ROOT.project("alice-in-daschland"), &make_lang("de"))
                .into_string();
        let de_labels: Vec<String> = nf_browse_links(&de).into_iter().map(|(_, t)| t).collect();
        assert_eq!(de_labels, vec!["Figuren", "Alle Kapitel", "/story"]);
    }

    /// An explicit `label=` that is not a declared chrome id renders as
    /// literal text (the chrome-id-or-literal rule), beating the nav label.
    #[test]
    fn not_found_browse_literal_label_beats_nav_label() {
        let mut site = make_alice_not_found_site();
        site.not_found_browse = vec![cpe_core::ir::NotFoundBrowse {
            route: "/characters".to_owned(),
            label: Some("Everyone in the book".to_owned()),
        }];
        let out =
            render_not_found_project_body(&site, cpe_core::url::ROOT.project("alice-in-daschland"), &make_lang("de"))
                .into_string();
        assert_eq!(
            nf_browse_links(&out),
            vec![("/alice-in-daschland/characters".to_owned(), "Everyone in the book".to_owned())]
        );
    }

    /// A chrome-id `site.title` resolves per language in the project-404 body's home invitation
    /// row; the raw id never renders.
    #[test]
    fn render_not_found_project_localizes_site_title() {
        let mut site = make_alice_not_found_site();
        site.title = "site-title".to_owned();
        site.chrome.push(cpe_core::ir::ChromeString {
            id: "site-title".to_owned(),
            translations: vec![
                ("en".to_owned(), "The Looking-Glass Archive".to_owned()),
                ("de".to_owned(), "Das Spiegelarchiv".to_owned()),
            ],
        });
        let en =
            render_not_found_project_body(&site, cpe_core::url::ROOT.project("alice-in-daschland"), &make_lang("en"))
                .into_string();
        assert!(
            en.contains("The Looking-Glass Archive — project home"),
            "N1 home row must use the EN site title: {en}"
        );
        let de =
            render_not_found_project_body(&site, cpe_core::url::ROOT.project("alice-in-daschland"), &make_lang("de"))
                .into_string();
        assert!(
            de.contains("Das Spiegelarchiv — Projektstartseite"),
            "N1 home row must use the DE site title: {de}"
        );
        assert!(!de.contains("site-title") && !en.contains("site-title"));
    }

    /// Overridden-copy sibling of `snapshot_render_not_found_project_body_alice_en`.
    #[test]
    fn snapshot_render_not_found_project_body_alice_browse_override_en() {
        let site = make_alice_not_found_site_with_browse();
        let out =
            render_not_found_project_body(&site, cpe_core::url::ROOT.project("alice-in-daschland"), &make_lang("en"))
                .into_string();
        insta::assert_snapshot!(out);
    }

    /// Overridden-copy sibling of `snapshot_render_not_found_project_body_alice_de`.
    #[test]
    fn snapshot_render_not_found_project_body_alice_browse_override_de() {
        let site = make_alice_not_found_site_with_browse();
        let out =
            render_not_found_project_body(&site, cpe_core::url::ROOT.project("alice-in-daschland"), &make_lang("de"))
                .into_string();
        insta::assert_snapshot!(out);
    }

    /// Class-less detail page (`Component::ResourceDetail.class == None`):
    /// falls back to `ChromeMsg::NfIdTitleFallback`, no `nf_id_msg_text` paragraph, no
    /// browse-all exit (project home only).
    #[test]
    fn render_not_found_with_no_class_falls_back_to_class_neutral_copy() {
        let routes: HashMap<String, String> = HashMap::new();
        let out = render_not_found(
            None,
            cpe_core::url::ROOT.project("alice-in-daschland"),
            &make_alice_not_found_site(),
            &routes,
            &make_lang("en"),
        )
        .into_string();
        assert!(
            out.contains("There is nothing at this address."),
            "class-less page must use the fallback heading, got: {out}"
        );
        assert!(
            !out.contains("class=\"notfound__msg\""),
            "class-less page must not render the id_msg paragraph, got: {out}"
        );
        assert!(
            !out.contains("Browse all"),
            "class-less page must have no browse-all exit, got: {out}"
        );
        assert!(
            out.contains("Project home"),
            "project home exit must still be present, got: {out}"
        );
    }

    /// A class with no registered browse page (absent from `class_list_routes`)
    /// omits the browse-all exit — Project home remains the only exit.
    #[test]
    fn render_not_found_with_unbrowsed_class_omits_browse_all() {
        let routes: HashMap<String, String> = HashMap::new();
        let out = render_not_found(
            Some("Character"),
            cpe_core::url::ROOT.project("alice-in-daschland"),
            &make_alice_not_found_site(),
            &routes,
            &make_lang("en"),
        )
        .into_string();
        assert!(
            !out.contains("Browse all"),
            "no browse-all exit without a registered list route, got: {out}"
        );
        assert!(
            out.contains("Project home"),
            "project home exit must still be present, got: {out}"
        );
    }

    /// Every `p.notfound__or` paragraph in `out`, in document order.
    fn notfound_or_lines(out: &str) -> Vec<&str> {
        out.match_indices("<p class=\"notfound__or\">")
            .map(|(start, _)| {
                let end = start + out[start..].find("</p>").expect("or-line closes") + 4;
                &out[start..end]
            })
            .collect()
    }

    /// The unknown-record page keeps its class-specific exits line and then ends with the
    /// project-404 body's own "Or browse:" line — byte-identical to it for the same site.
    #[test]
    fn render_not_found_ends_with_n1_or_browse_line() {
        let site = make_alice_not_found_site();
        let routes: HashMap<String, String> = [("Character".to_owned(), "/characters".to_owned())].into();
        for code in ["en", "de"] {
            let n3 = render_not_found(
                Some("Character"),
                cpe_core::url::ROOT.project("alice-in-daschland"),
                &site,
                &routes,
                &make_lang(code),
            )
            .into_string();
            let n1 = render_not_found_project_body(
                &site,
                cpe_core::url::ROOT.project("alice-in-daschland"),
                &make_lang(code),
            )
            .into_string();
            let n3_lines = notfound_or_lines(&n3);
            assert_eq!(n3_lines.len(), 2, "N3 has exits + or-browse lines: {n3}");
            assert!(n3_lines[0].contains("/alice-in-daschland/characters"));
            assert_eq!(
                n3_lines[1],
                notfound_or_lines(&n1)[0],
                "N3's second line is N1's or-browse line ({code})"
            );
            assert!(
                n3.ends_with("</p></div></article>"),
                "the or-browse line is the body's last element: {n3}"
            );
        }
    }

    #[test]
    fn render_not_found_or_browse_honours_declared_browse() {
        let site = make_alice_not_found_site_with_browse();
        let out = render_not_found(
            Some("Character"),
            cpe_core::url::ROOT.project("alice-in-daschland"),
            &site,
            &HashMap::new(),
            &make_lang("en"),
        )
        .into_string();
        let last = *notfound_or_lines(&out).last().expect("or-browse line");
        assert_eq!(
            nf_browse_links(last),
            vec![
                ("/alice-in-daschland/characters".to_owned(), "Characters".to_owned()),
                ("/alice-in-daschland/chapters".to_owned(), "All chapters".to_owned()),
                ("/alice-in-daschland/story".to_owned(), "/story".to_owned()),
            ]
        );
    }

    #[test]
    fn not_found_heading_names_the_class_or_falls_back() {
        assert_eq!(
            not_found_heading(Some("Character"), &[], &make_lang("en")),
            "There is no Character at this address."
        );
        assert_eq!(
            not_found_heading(None, &[], &make_lang("en")),
            "There is nothing at this address."
        );
    }

    // LinkResolver unit tests

    fn make_resolver<'a>(
        project_id: &'a str,
        routes: &'a std::collections::HashMap<String, String>,
    ) -> LinkResolver<'a> {
        LinkResolver::new(cpe_core::url::ROOT.project(project_id), routes)
    }

    fn make_routes(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn make_partner(annotation_id: Option<&str>, parent: Option<&str>) -> AnnotationPartner {
        AnnotationPartner {
            link_id: "link-1".to_owned(),
            link_name: "identisch".to_owned(),
            link_name_lang: None,
            annotation_id: annotation_id.map(str::to_owned),
            number: annotation_id.map(|_| 1),
            resource_id: "bk-c8r".to_owned(),
            resource_class: "Page".to_owned(),
            resource_label: "c8r".to_owned(),
            parent: parent.map(|class| cpe_core::ir::MemberParent {
                id: "bk".to_owned(),
                class: class.to_owned(),
                seq: 3,
            }),
            parent_title: None,
            first_member: false,
        }
    }

    #[test]
    fn partner_href_lands_on_the_partners_register_entry() {
        let routes = make_routes(&[("Book", "/books/:id"), ("Page", "/pages/:id")]);
        let resolver = make_resolver("incunabula", &routes);
        // An annotation partner: its own entry; a page partner: the link's.
        assert_eq!(
            resolver.partner_href(&make_partner(Some("a b"), Some("Book"))),
            Some("/incunabula/books/bk?page=bk-c8r#annotation-a%20b".to_owned())
        );
        assert_eq!(
            resolver.partner_href(&make_partner(None, Some("Book"))),
            Some("/incunabula/books/bk?page=bk-c8r#annotation-link-1".to_owned())
        );
        // No reader to land in: the partner's own page, without a fragment.
        assert_eq!(
            resolver.partner_href(&make_partner(Some("a1"), Some("Shelf"))),
            Some("/incunabula/pages/bk-c8r".to_owned())
        );
        assert_eq!(
            resolver.partner_href(&make_partner(None, None)),
            Some("/incunabula/pages/bk-c8r".to_owned())
        );
        let unbound = make_routes(&[]);
        let resolver = make_resolver("incunabula", &unbound);
        assert_eq!(resolver.partner_href(&make_partner(Some("a1"), Some("Book"))), None);
    }

    #[test]
    fn reader_entry_href_addresses_the_member_and_its_entry() {
        let routes = make_routes(&[("Book", "/books/:id")]);
        let resolver = make_resolver("incunabula", &routes);
        assert_eq!(
            resolver.reader_entry_href("bk", "Book", "bk-c1v", "x_Y-1"),
            Some("/incunabula/books/bk?page=bk-c1v#annotation-x_Y-1".to_owned())
        );
        assert_eq!(resolver.reader_entry_href("bk", "Shelf", "bk-c1v", "x"), None);
    }

    #[test]
    fn link_resolver_href_bound_class_returns_prefixed_substituted_url() {
        let routes = make_routes(&[("Character", "/characters/:id")]);
        let resolver = make_resolver("alice-in-daschland", &routes);
        let href = resolver.href("CH_002", Some("Character"));
        assert_eq!(
            href,
            Some("/alice-in-daschland/characters/CH_002".to_owned()),
            "bound class must produce prefixed substituted URL, got: {href:?}"
        );
    }

    #[test]
    fn link_resolver_href_unknown_class_returns_none() {
        let routes = make_routes(&[("Character", "/characters/:id")]);
        let resolver = make_resolver("alice-in-daschland", &routes);
        let href = resolver.href("LOC_001", Some("Location"));
        assert_eq!(href, None, "class absent from map must return None, got: {href:?}");
    }

    #[test]
    fn link_resolver_href_target_class_none_returns_none() {
        let routes = make_routes(&[("Character", "/characters/:id")]);
        let resolver = make_resolver("alice-in-daschland", &routes);
        let href = resolver.href("CH_001", None);
        assert_eq!(
            href, None,
            "target_class=None must return None (cannot route without class), got: {href:?}"
        );
    }

    #[test]
    fn link_resolver_href_url_encodes_id() {
        // An id with a space should be percent-encoded as %20.
        let routes = make_routes(&[("Item", "/items/:id")]);
        let resolver = make_resolver("my-project", &routes);
        let href = resolver.href("hello world", Some("Item"));
        assert_eq!(
            href,
            Some("/my-project/items/hello%20world".to_owned()),
            "space in id must be percent-encoded as %20, got: {href:?}"
        );
    }

    #[test]
    fn link_resolver_href_standard_ids_pass_through_unencoded() {
        // CH_001-style ids (alphanumeric + underscore) must not be percent-encoded.
        let routes = make_routes(&[("Character", "/characters/:id")]);
        let resolver = make_resolver("alice-in-daschland", &routes);
        let href = resolver.href("CH_001", Some("Character"));
        assert_eq!(
            href,
            Some("/alice-in-daschland/characters/CH_001".to_owned()),
            "standard id CH_001 must not be encoded, got: {href:?}"
        );
    }

    // render_value Link arm: with resolver

    #[test]
    fn render_value_link_with_resolver_bound_class_emits_int_link_anchor() {
        let routes = make_routes(&[("Character", "/characters/:id")]);
        let resolver = LinkResolver::new(cpe_core::url::ROOT.project("alice-in-daschland"), &routes);
        let v = PropertyValue::Link {
            target_id: "CH_002".to_owned(),
            target_label: Some("The White Rabbit".to_owned()),
            target_class: Some("Character".to_owned()),
        };
        let out = render_value(&v, &make_lang("en"), Some(&resolver)).into_string();
        assert!(
            out.contains("class=\"int-link\""),
            "bound class must emit <a class=\"int-link\">, got: {out}"
        );
        assert!(
            out.contains("href=\"/alice-in-daschland/characters/CH_002\""),
            "int-link must carry the resolved href, got: {out}"
        );
        assert!(out.contains("class=\"ref\""), "int-link must contain .ref span, got: {out}");
        assert!(
            out.contains("The White Rabbit"),
            "label must appear inside the anchor, got: {out}"
        );
        assert!(
            !out.contains("class=\"v-text\""),
            "int-link must not emit span.v-text, got: {out}"
        );
    }

    #[test]
    fn render_value_link_with_resolver_unbound_class_falls_back_to_v_text() {
        // Resolver has Character bound, but the link is to a Location.
        let routes = make_routes(&[("Character", "/characters/:id")]);
        let resolver = LinkResolver::new(cpe_core::url::ROOT.project("alice-in-daschland"), &routes);
        let v = PropertyValue::Link {
            target_id: "LOC_001".to_owned(),
            target_label: Some("Wonderland".to_owned()),
            target_class: Some("Location".to_owned()),
        };
        let out = render_value(&v, &make_lang("en"), Some(&resolver)).into_string();
        assert!(
            out.contains("class=\"v-text\""),
            "unbound class must fall back to span.v-text, got: {out}"
        );
        assert!(
            !out.contains("class=\"int-link\""),
            "unbound class must NOT emit int-link, got: {out}"
        );
        assert!(out.contains("Wonderland"), "label must still appear, got: {out}");
    }

    #[test]
    fn render_value_link_unresolved_with_resolver_stays_unresolved() {
        // target_label: None → span.unresolved regardless of resolver.
        let routes = make_routes(&[("Character", "/characters/:id")]);
        let resolver = LinkResolver::new(cpe_core::url::ROOT.project("alice-in-daschland"), &routes);
        let v = PropertyValue::Link {
            target_id: "CH_BROKEN".to_owned(),
            target_label: None,
            target_class: None,
        };
        let out = render_value(&v, &make_lang("en"), Some(&resolver)).into_string();
        assert!(
            out.contains("class=\"unresolved\""),
            "None label must show the unresolved mark even with resolver, got: {out}"
        );
        assert!(out.contains("CH_BROKEN"), "placeholder must still show target_id, got: {out}");
        assert!(
            !out.contains("class=\"int-link\""),
            "unresolved link must not emit int-link, got: {out}"
        );
    }

    // Multi-valued Link property → all values render as distinct int-link anchors

    /// A property with TWO Link values (each with a distinct target) must render
    /// BOTH as separate `<a class="int-link">` anchors when the resolver binds
    /// the target class.  Guards against an implementation that renders only the
    /// first value.
    #[test]
    fn render_property_block_multi_link_renders_all_int_link_anchors() {
        let prop = Property {
            name: "related".to_owned(),
            values: vec![
                PropertyValue::Link {
                    target_id: "CH_002".to_owned(),
                    target_label: Some("The White Rabbit".to_owned()),
                    target_class: Some("Character".to_owned()),
                },
                PropertyValue::Link {
                    target_id: "CH_003".to_owned(),
                    target_label: Some("The Cheshire Cat".to_owned()),
                    target_class: Some("Character".to_owned()),
                },
            ],
        };
        let routes = make_routes(&[("Character", "/characters/:id")]);
        let resolver = LinkResolver::new(cpe_core::url::ROOT.project("alice-in-daschland"), &routes);
        let out = render_property_block(&prop, &make_lang("en"), &[], Some(&resolver), false).into_string();

        // Both anchors must be present.
        let anchor_count = out.matches("class=\"int-link\"").count();
        assert_eq!(
            anchor_count, 2,
            "multi-valued link property must emit exactly 2 int-link anchors (one per value), got {anchor_count}: {out}"
        );

        // Each distinct href must appear.
        assert!(
            out.contains("href=\"/alice-in-daschland/characters/CH_002\""),
            "first link must have its resolved href, got: {out}"
        );
        assert!(
            out.contains("href=\"/alice-in-daschland/characters/CH_003\""),
            "second link must have its resolved href, got: {out}"
        );

        // Both labels must appear.
        assert!(out.contains("The White Rabbit"), "first link label must appear, got: {out}");
        assert!(out.contains("The Cheshire Cat"), "second link label must appear, got: {out}");

        // No span.v-text fallbacks — both classes are bound.
        assert!(
            !out.contains("class=\"v-text\""),
            "no v-text fallback expected when both classes are bound, got: {out}"
        );

        // Adjacent links must be separated (comma) so they don't butt together.
        assert!(
            out.contains("</a>, <a"),
            "multi-valued links must be comma-separated, got: {out}"
        );
    }

    #[test]
    fn render_property_block_multi_link_mixed_resolution_separates_each() {
        // One bound (Character → route), one unbound class (Place, no route).
        // Both still comma-separated; bound → int-link, unbound → v-text.
        let prop = Property {
            name: "related".to_owned(),
            values: vec![
                PropertyValue::Link {
                    target_id: "CH_002".to_owned(),
                    target_label: Some("The White Rabbit".to_owned()),
                    target_class: Some("Character".to_owned()),
                },
                PropertyValue::Link {
                    target_id: "PL_001".to_owned(),
                    target_label: Some("Wonderland".to_owned()),
                    target_class: Some("Place".to_owned()),
                },
            ],
        };
        let routes = make_routes(&[("Character", "/characters/:id")]);
        let resolver = LinkResolver::new(cpe_core::url::ROOT.project("alice-in-daschland"), &routes);
        let out = render_property_block(&prop, &make_lang("en"), &[], Some(&resolver), false).into_string();

        // Bound value → int-link; unbound value → plain v-text; comma between.
        assert!(
            out.contains("class=\"int-link\""),
            "bound link must render as int-link, got: {out}"
        );
        assert!(
            out.contains("class=\"v-text\""),
            "unbound-class link must fall back to v-text, got: {out}"
        );
        assert!(out.contains(", "), "the two values must be comma-separated, got: {out}");
    }

    /// A multi-valued Text property (e.g. a Book's two publishers, one
    /// property row per value) must render its values as separate
    /// stacked `div.v-multi__item` blocks, NOT comma-joined and NOT
    /// concatenated with no separator ("Johann AmerbachJohann Petri de
    /// Langendorff"). A comma join is ambiguous when the values themselves
    /// may contain commas, so the generic multi-value path stacks blocks
    /// instead of inventing a separator character.
    #[test]
    fn render_property_block_multi_text_value_renders_stacked_blocks() {
        let prop = Property {
            name: "publisher".to_owned(),
            values: vec![
                PropertyValue::Text { text: "Johann Amerbach".to_owned(), lang: None },
                PropertyValue::Text { text: "Johann Petri de Langendorff".to_owned(), lang: None },
            ],
        };
        let out = render_property_block(&prop, &make_lang("en"), &[], None, false).into_string();

        assert_eq!(
            out.matches("class=\"v-multi__item\"").count(),
            2,
            "multi-valued text values must render as two stacked v-multi__item blocks, got: {out}"
        );
        assert!(
            !out.contains("</span>, <span"),
            "text values must not be comma-joined, got: {out}"
        );
        assert!(
            !out.contains("AmerbachJohann"),
            "values must not be concatenated with no separator, got: {out}"
        );
    }

    /// Two multi-valued Richtext values (e.g. a Page's `comment` property
    /// carrying several prose rows) must render as two separate
    /// `div.v-multi__item` blocks, never joined by an invented ", " — the
    /// values already contain commas (real prose), so a comma join makes two
    /// distinct comments read as one run-on sentence (`bereitung-o3r`'s two
    /// comments — "Es fehlen … weiter." and "Schramm, Bd. 22, Abb. 454." —
    /// would read "…weiter., Schramm, Bd. 22, Abb. 454.").
    #[test]
    fn render_property_block_multi_richtext_comments_render_as_separate_blocks() {
        let prop = Property {
            name: "comment".to_owned(),
            values: vec![
                PropertyValue::Richtext {
                    text: "Es fehlen die Holzschnitte, Fragment weiter.".to_owned(),
                    lang: None,
                },
                PropertyValue::Richtext { text: "Schramm, Bd. 22, Abb. 454.".to_owned(), lang: None },
            ],
        };
        let out = render_property_block(&prop, &make_lang("en"), &[], None, false).into_string();

        assert_eq!(
            out.matches("class=\"v-multi__item\"").count(),
            2,
            "two prose comments must render as two stacked v-multi__item blocks, got: {out}"
        );
        assert!(
            out.contains("weiter.") && out.contains("Schramm, Bd. 22, Abb. 454."),
            "both comment values must be present, got: {out}"
        );
        assert!(
            !out.contains("weiter., Schramm"),
            "comments must not run together with an invented comma separator, got: {out}"
        );
    }

    // LinkResolver::href — special-char encoding + non-trailing :param

    /// An empty `target_id` yields `None` (the guard prevents a trailing-slash
    /// URL like `/my-project/items/`).
    #[test]
    fn link_resolver_href_empty_target_id_returns_none() {
        let routes = make_routes(&[("Item", "/items/:id")]);
        let resolver = make_resolver("my-project", &routes);
        assert_eq!(
            resolver.href("", Some("Item")),
            None,
            "empty target_id must yield None, not a trailing-slash URL"
        );
    }

    /// A target_id containing `/` must be percent-encoded as `%2F`.
    #[test]
    fn link_resolver_href_encodes_slash_in_id() {
        let routes = make_routes(&[("Item", "/items/:id")]);
        let resolver = make_resolver("my-project", &routes);
        let href = resolver.href("part/sub", Some("Item"));
        assert_eq!(
            href,
            Some("/my-project/items/part%2Fsub".to_owned()),
            "/ in id must be percent-encoded as %2F, got: {href:?}"
        );
    }

    /// A target_id containing `#` must be percent-encoded as `%23`.
    #[test]
    fn link_resolver_href_encodes_hash_in_id() {
        let routes = make_routes(&[("Item", "/items/:id")]);
        let resolver = make_resolver("my-project", &routes);
        let href = resolver.href("id#anchor", Some("Item"));
        assert_eq!(
            href,
            Some("/my-project/items/id%23anchor".to_owned()),
            "# in id must be percent-encoded as %23, got: {href:?}"
        );
    }

    /// A route whose `:param` is NOT the last segment (e.g. `"/chars/:id/detail"`)
    /// must still substitute correctly, producing the literal segments around it
    /// unchanged.
    #[test]
    fn link_resolver_href_param_not_last_segment_substitutes_correctly() {
        let routes = make_routes(&[("Character", "/chars/:id/detail")]);
        let resolver = make_resolver("my-project", &routes);
        let href = resolver.href("CH_007", Some("Character"));
        assert_eq!(
            href,
            Some("/my-project/chars/CH_007/detail".to_owned()),
            ":param not at the end must still substitute correctly, got: {href:?}"
        );
    }

    // Layer-3 snapshot: clickable link path

    /// Snapshot of render_resource_detail with a LinkResolver that binds the
    /// "Character" class, exercising the clickable <a class="int-link"> path.
    /// The resource has a `related` property that is a Link to CH_002.
    #[test]
    fn snapshot_render_resource_detail_with_int_link() {
        let resource = make_alice_resource(); // has `related` Link to CH_002 (Character class)
        let show_props: Vec<String> = vec!["related".to_owned()];
        let routes = make_routes(&[("Character", "/characters/:id")]);
        let resolver = LinkResolver::new(cpe_core::url::ROOT.project("alice-in-daschland"), &routes);
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            head: None,
            id: "CH_001",
            title_property: Some("name"),
            show_properties: &show_props,
            media_property: None,
            media_items: &[],
            selected_image: 0,
            strip_open: true,
            compound_members: &[],
            current_path: "/alice-in-daschland/characters/CH_001",
            current_query: None,
            links: Some(resolver),
            property_labels: &[],
            class_labels: &[],
            groups: &[],
            intro: None,
            navigator: false,
            incoming: None,
            date_precision_word: false,
        };
        let out = render_resource_detail(&resource, cfg, &make_lang("en")).into_string();
        // The clickable link must be present.
        assert!(
            out.contains("class=\"int-link\""),
            "snapshot: int-link must be present when resolver binds the class, got: {out}"
        );
        assert!(
            out.contains("href=\"/alice-in-daschland/characters/CH_002\""),
            "snapshot: int-link href must be the resolved URL, got: {out}"
        );
        insta::assert_snapshot!(out);
    }

    // Layer-3 snapshot: unresolved link

    /// Snapshot of render_resource_detail with an unresolved `related` Link
    /// value (target does not exist: `target_label: None`, `target_class:
    /// None`) — the UNRESOLVED mark, not the resolved `.int-link`/`.v-text`
    /// path exercised by `snapshot_render_resource_detail_en` (link-with-label)
    /// and `snapshot_render_resource_detail_with_int_link`.
    #[test]
    fn snapshot_render_resource_detail_with_unresolved_link() {
        let mut resource = make_alice_resource();
        for prop in &mut resource.properties {
            if prop.name == "related" {
                prop.values = vec![PropertyValue::Link {
                    target_id: "CH_BROKEN".to_owned(),
                    target_label: None,
                    target_class: None,
                }];
            }
        }
        let show_props: Vec<String> = vec!["related".to_owned()];
        let cfg = make_alice_cfg("CH_001", Some("name"), &show_props, &[]);
        let out = render_resource_detail(&resource, cfg, &make_lang("en")).into_string();
        assert!(
            out.contains("class=\"unresolved\""),
            "snapshot: unresolved mark must be present, got: {out}"
        );
        assert!(!out.contains("<a "), "snapshot: never an <a> anchor, got: {out}");
        insta::assert_snapshot!(out);
    }

    // Config property groups

    fn text_prop(name: &str, text: &str) -> Property {
        Property {
            name: name.to_owned(),
            values: vec![PropertyValue::Text { text: text.to_owned(), lang: None }],
        }
    }

    /// A synthetic, project-agnostic record for the group tests — never a real
    /// project config.
    fn make_grouped_resource() -> Resource {
        Resource {
            id: "R_001".to_owned(),
            class: "Thing".to_owned(),
            properties: vec![
                text_prop("name", "Example thing"),
                text_prop("quote", "A quoted line."),
                text_prop("creator", "Some Maker"),
                text_prop("date", "1492"),
                text_prop("place", "Basel"),
                text_prop("note", "A loose note."),
            ],
            representations: vec![],
        }
    }

    fn grp<'a>(caption: &'a str, properties: &'a [String]) -> GroupCfg<'a> {
        GroupCfg { caption, properties, empty: None, links: None }
    }

    fn grouped_cfg<'a>(show_properties: &'a [String], groups: &'a [GroupCfg<'a>]) -> ResourceDetailCfg<'a> {
        ResourceDetailCfg {
            reader: None,
            annotations: None,
            groups,
            ..make_alice_cfg("R_001", Some("name"), show_properties, &[])
        }
    }

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_owned()).collect()
    }

    #[test]
    fn group_renders_members_in_declaration_order() {
        let resource = make_grouped_resource();
        let members = strings(&["place", "creator", "date"]);
        let groups = [grp("Details", members.as_slice())];
        let out = render_resource_detail(&resource, grouped_cfg(&[], &groups), &make_lang("en")).into_string();
        let place = out.find("Basel").expect("place rendered");
        let creator = out.find("Some Maker").expect("creator rendered");
        let date = out.find("1492").expect("date rendered");
        assert!(
            place < creator && creator < date,
            "group members must follow declaration order: {out}"
        );
        assert!(
            out.contains(r#"<section class="pgroup"><header class="pgroup__head"><span class="pgroup__title">Details</span></header><div class="props">"#),
            "group must use the .pgroup family with no .pgroup__of: {out}"
        );
        assert!(!out.contains("pgroup__of"), "no subject line: {out}");
    }

    #[test]
    fn groups_without_show_properties_render_no_flat_props_and_no_duplicates() {
        let resource = make_grouped_resource();
        let members = strings(&["creator", "date"]);
        let groups = [grp("Details", members.as_slice())];
        let out = render_resource_detail(&resource, grouped_cfg(&[], &groups), &make_lang("en")).into_string();
        assert_eq!(
            out.matches(r#"<div class="props">"#).count(),
            1,
            "only the group's own div.props — the flat one is omitted: {out}"
        );
        assert_eq!(out.matches("Some Maker").count(), 1, "no member twice: {out}");
        assert!(
            !out.contains("Basel") && !out.contains("A loose note."),
            "ungrouped properties must NOT fall back to 'render all': {out}"
        );
    }

    #[test]
    fn no_groups_no_show_properties_still_renders_every_property() {
        let resource = make_grouped_resource();
        let out = render_resource_detail(&resource, grouped_cfg(&[], &[]), &make_lang("en")).into_string();
        for v in ["Some Maker", "1492", "Basel", "A loose note."] {
            assert!(out.contains(v), "absent show-properties means all ({v}): {out}");
        }
        assert!(!out.contains("pgroup"), "no group markup: {out}");
    }

    #[test]
    fn group_with_all_members_absent_renders_nothing() {
        let resource = make_grouped_resource();
        let flat = strings(&["note"]);
        let members = strings(&["missing-a", "missing-b"]);
        let groups = [grp("Details", members.as_slice())];
        let out = render_resource_detail(&resource, grouped_cfg(&flat, &groups), &make_lang("en")).into_string();
        assert!(
            !out.contains("pgroup") && !out.contains("Details"),
            "an all-absent group renders no cap at all: {out}"
        );
        assert!(out.contains("A loose note."), "flat list unaffected: {out}");
    }

    #[test]
    fn all_absent_groups_without_show_properties_omit_flat_props_too() {
        let resource = make_grouped_resource();
        let members = strings(&["missing"]);
        let groups = [grp("Details", members.as_slice())];
        let out = render_resource_detail(&resource, grouped_cfg(&[], &groups), &make_lang("en")).into_string();
        assert!(!out.contains(r#"class="props""#), "no empty div.props: {out}");
    }

    #[test]
    fn all_absent_group_with_empty_renders_caption_and_sentence() {
        let resource = make_grouped_resource();
        let members = strings(&["missing"]);
        let groups = [GroupCfg {
            empty: Some("Nobody recorded."),
            ..grp("People", members.as_slice())
        }];
        let out = render_resource_detail(&resource, grouped_cfg(&[], &groups), &make_lang("en")).into_string();
        assert!(
            out.contains(r#"<section class="pgroup"><header class="pgroup__head"><span class="pgroup__title">People</span></header><p class="pgroup__empty">Nobody recorded.</p></section>"#),
            "{out}"
        );
        assert!(!out.contains(r#"class="props""#), "{out}");
    }

    #[test]
    fn all_absent_group_without_empty_renders_nothing() {
        let resource = make_grouped_resource();
        let members = strings(&["missing"]);
        let groups = [grp("People", members.as_slice())];
        let out = render_resource_detail(&resource, grouped_cfg(&[], &groups), &make_lang("en")).into_string();
        assert!(!out.contains("People"), "{out}");
        assert!(!out.contains("pgroup__empty"), "{out}");
    }

    fn resource_link(link_id: &str, name: &str, id: &str, label: &str) -> ResourceLink {
        ResourceLink {
            link_id: link_id.to_owned(),
            link_name: name.to_owned(),
            link_name_lang: Some("de".to_owned()),
            resource_id: id.to_owned(),
            resource_class: "Book".to_owned(),
            resource_label: label.to_owned(),
        }
    }

    /// The Apparat's related-book row: one name over two
    /// partners, a second name over one, and a link to the same partner
    /// twice collapsed.
    #[test]
    fn snapshot_group_links_row() {
        let resource = make_grouped_resource();
        let members = strings(&["creator"]);
        let partners = [
            resource_link("L1", "im selben Band gebunden", "orationes", "Orationes"),
            resource_link("L1", "im selben Band gebunden", "lob", "Lob der Glieder"),
            resource_link("L2", "Übersetzung", "lat", "Narrenschiff (lat.)"),
            resource_link("L3", "Übersetzung", "lat", "Narrenschiff (lat.)"),
        ];
        let groups = [GroupCfg {
            links: Some(GroupLinks { caption: "Verwandtes Buch", links: &partners }),
            ..grp("Apparat", members.as_slice())
        }];
        let routes = make_routes(&[("Book", "/books/:id")]);
        let cfg = ResourceDetailCfg {
            links: Some(LinkResolver::new(cpe_core::url::ROOT.project("incunabula"), &routes)),
            ..grouped_cfg(&[], &groups)
        };
        let out = render_resource_detail(&resource, cfg, &make_lang("de")).into_string();
        let row = &out[out.find(r#"<div class="prop prop--links">"#).expect("a links row")..];
        assert_eq!(row.matches(r#"href="/incunabula/books/lat""#).count(), 1);
        insta::assert_snapshot!(&row[..row.find("</section>").expect("group closes")]);
    }

    #[test]
    fn group_with_only_a_links_row_still_renders() {
        let resource = make_grouped_resource();
        let members = strings(&["missing"]);
        let partners = [resource_link("L1", "Übersetzung", "lat", "Narrenschiff (lat.)")];
        let groups = [GroupCfg {
            links: Some(GroupLinks { caption: "Verwandtes Buch", links: &partners }),
            ..grp("Apparat", members.as_slice())
        }];
        let out = render_resource_detail(&resource, grouped_cfg(&[], &groups), &make_lang("de")).into_string();
        assert!(
            out.contains(r#"<span class="pgroup__title">Apparat</span></header><div class="props"><div class="prop prop--links">"#),
            "{out}"
        );
        assert!(
            out.contains(r#"<span class="prop__rel-to"><em>Narrenschiff (lat.)</em></span>"#),
            "no resolver: plain text, {out}"
        );
    }

    #[test]
    fn present_group_with_empty_never_shows_the_sentence() {
        let resource = make_grouped_resource();
        let members = strings(&["missing", "creator"]);
        let groups = [GroupCfg {
            empty: Some("Nobody recorded."),
            ..grp("People", members.as_slice())
        }];
        let out = render_resource_detail(&resource, grouped_cfg(&[], &groups), &make_lang("en")).into_string();
        assert!(out.contains("Some Maker"), "{out}");
        assert!(!out.contains("Nobody recorded."), "{out}");
    }

    #[test]
    fn group_members_exclude_title_and_quote() {
        let resource = make_grouped_resource();
        let members = strings(&["name", "quote", "creator"]);
        let groups = [grp("Details", members.as_slice())];
        let out = render_resource_detail(&resource, grouped_cfg(&[], &groups), &make_lang("en")).into_string();
        assert!(
            !out.contains(r#"prop__label">Name<"#),
            "title stays in the head, never a group row: {out}"
        );
        assert!(
            !out.contains(r#"prop__label">Quote<"#),
            "quote stays in the blockquote, never a group row: {out}"
        );
        assert_eq!(
            out.matches("A quoted line.").count(),
            1,
            "quote renders once (blockquote): {out}"
        );
        assert!(out.contains("Some Maker"), "{out}");
    }

    #[test]
    fn snapshot_render_resource_detail_grouped() {
        let resource = make_grouped_resource();
        let details = strings(&["creator", "date"]);
        let provenance = strings(&["place", "note"]);
        let groups = [
            grp("Details", details.as_slice()),
            grp("Provenance", provenance.as_slice()),
        ];
        let out = render_resource_detail(&resource, grouped_cfg(&[], &groups), &make_lang("en")).into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_render_resource_detail_grouped_and_flat() {
        let resource = make_grouped_resource();
        let flat = strings(&["note"]);
        let details = strings(&["creator", "date", "place"]);
        let groups = [grp("Details", details.as_slice())];
        let out = render_resource_detail(&resource, grouped_cfg(&flat, &groups), &make_lang("en")).into_string();
        insta::assert_snapshot!(out);
    }

    // The navigator-keyed record

    fn make_members() -> Vec<MemberHeader> {
        vec![
            MemberHeader {
                id: "p-a1r".to_owned(),
                class: "Page".to_owned(),
                name: "a1r".to_owned(),
            },
            MemberHeader {
                id: "p-a1v".to_owned(),
                class: "Page".to_owned(),
                name: "a1v".to_owned(),
            },
        ]
    }

    /// A no-media record cfg over `members`, with the given navigator flag,
    /// query and intro.
    fn navigator_cfg<'a>(
        members: &'a [MemberHeader],
        navigator: bool,
        current_query: Option<&'a str>,
        intro: Option<Markup>,
    ) -> ResourceDetailCfg<'a> {
        ResourceDetailCfg {
            reader: None,
            annotations: None,
            compound_members: members,
            current_path: "/incunabula/books/b1",
            current_query,
            intro,
            navigator,
            ..make_alice_cfg("CH_001", Some("name"), &[], &[])
        }
    }

    fn render_navigator(
        members: &[MemberHeader],
        navigator: bool,
        current_query: Option<&str>,
        intro: Option<Markup>,
    ) -> String {
        render_resource_detail(
            &make_alice_resource(),
            navigator_cfg(members, navigator, current_query, intro),
            &make_lang("de"),
        )
        .into_string()
    }

    fn make_intro() -> Markup {
        html! {
            div.prose {
                p { "Eine Einführung." }
            }
        }
    }

    /// A stand-in for the paired pages reader `render_body` injects.
    fn make_reader() -> Markup {
        html! {
            article #compound-viewer.compound-viewer.page { "READER" }
        }
    }

    /// The navigator record with the paired reader injected.
    fn render_with_reader(intro: Option<Markup>) -> String {
        let members = make_members();
        let cfg = ResourceDetailCfg {
            reader: Some(make_reader()),
            annotations: None,
            ..navigator_cfg(&members, true, Some("lang=de"), intro)
        };
        render_resource_detail(&make_alice_resource(), cfg, &make_lang("de")).into_string()
    }

    /// No start-reading link: a navigator record
    /// with members renders no link, and no link-only left column.
    #[test]
    fn navigator_record_renders_no_start_reading_link() {
        let members = make_members();
        let out = render_navigator(&members, true, None, None);
        assert!(!out.contains("record__start"), "{out}");
        assert!(!out.contains("record__intro"), "{out}");
        assert!(!out.contains("record__body--intro"), "{out}");
    }

    /// `reader: Some` ⇒ `article.record--reader > div.record__tracks >
    /// header.record__head + reader + div.record__body`, the reader once.
    #[test]
    fn reader_sits_between_head_and_body_inside_the_tracks() {
        let out = render_with_reader(None);
        assert!(
            out.starts_with(
                r#"<article class="record page record--reader"><div class="record__tracks"><header class="record__head">"#
            ),
            "{out}"
        );
        let head_end = out.find("</header>").expect("head");
        let reader_at = out.find("READER").expect("reader");
        let body_at = out.find(r#"<div class="record__body"#).expect("body");
        assert!(head_end < reader_at && reader_at < body_at, "{out}");
        assert!(out.ends_with("</div></div></article>"), "{out}");
        assert_eq!(out.matches("READER").count(), 1, "{out}");
    }

    /// An annotated reader's record article declares the annotation signals,
    /// then runs the arrival (a reader without annotations has neither,
    /// above).
    #[test]
    fn annotated_reader_declares_the_annotation_signals_on_the_article() {
        let members = make_members();
        let cfg = ResourceDetailCfg {
            reader: Some(make_reader()),
            annotations: Some(html! {
                div #cv-annotations {}
            }),
            ..navigator_cfg(&members, true, Some("lang=de"), None)
        };
        let out = render_resource_detail(&make_alice_resource(), cfg, &make_lang("de")).into_string();
        assert!(
            out.starts_with(
                r#"<article class="record page record--reader" data-signals="{_annoShow: true, _annoHover: '', _annoPin: '', _annoArrive: '', _annoNote: ''}" data-init=""#
            ),
            "{out}"
        );
        let tag = &out[..out.find('>').expect("the article tag closes")];
        let init = maud::html! {
            (super::super::annotation::ARRIVAL_INIT_EXPR)
        }
        .into_string();
        assert!(tag.contains(&format!(r#"data-init="{init}""#)), "{tag}");
    }

    /// With an intro, the body still holds intro then Apparat (the stacked
    /// companion); the reader stays outside the body.
    #[test]
    fn reader_with_intro_keeps_intro_then_apparat_in_the_body() {
        let out = render_with_reader(Some(make_intro()));
        let reader_at = out.find("READER").expect("reader");
        let intro_at = out.find(r#"<div class="record__intro">"#).expect("intro");
        let apparat_at = out.find(r#"<div class="record__apparat">"#).expect("apparat");
        assert!(reader_at < intro_at && intro_at < apparat_at, "{out}");
        assert!(out.contains("record__body--intro"), "{out}");
    }

    /// `reader: None` keeps the untracked markup (no wrapper, no modifier).
    #[test]
    fn reader_none_renders_no_tracks() {
        let members = make_members();
        let out = render_navigator(&members, true, None, Some(make_intro()));
        assert!(!out.contains("record__tracks"), "{out}");
        assert!(!out.contains("record--reader"), "{out}");
        assert!(out.starts_with(r#"<article class="record page"><header"#), "{out}");
    }

    #[test]
    fn rv_contents_renders_without_navigator_even_without_media() {
        let members = make_members();
        let out = render_navigator(&members, false, None, None);
        assert!(out.contains(r#"<section class="rv-contents">"#), "{out}");
    }

    #[test]
    fn rv_contents_absent_with_navigator() {
        let members = make_members();
        let out = render_navigator(&members, true, None, None);
        assert!(!out.contains("rv-contents"), "{out}");
    }

    #[test]
    fn snapshot_render_resource_detail_navigator_with_intro() {
        insta::assert_snapshot!(render_with_reader(Some(make_intro())));
    }

    #[test]
    fn snapshot_render_resource_detail_navigator_without_intro() {
        insta::assert_snapshot!(render_with_reader(None));
    }

    /// The structure the plain-record rules in `grid.css` and `media.css`
    /// select on: with an intro the body holds exactly the intro and one
    /// Apparat wrapper, and props and groups are the wrapper's children.
    #[test]
    fn plain_record_with_intro_holds_intro_then_apparat_with_props_and_groups() {
        let resource = make_grouped_resource();
        let members = strings(&["place", "creator"]);
        let groups = [grp("Details", members.as_slice())];
        let flat = strings(&["date"]);
        let cfg = ResourceDetailCfg { intro: Some(make_intro()), ..grouped_cfg(&flat, &groups) };
        let out = render_resource_detail(&resource, cfg, &make_lang("en")).into_string();
        assert!(
            out.starts_with(r#"<article class="record page"><header class="record__head">"#),
            "{out}"
        );
        let body = &out[out.find("</header>").expect("head") + "</header>".len()..];
        assert!(
            body.starts_with(concat!(
                r#"<div class="record__body record__body--noimg record__body--intro">"#,
                r#"<div class="record__intro"><div class="prose"><p>Eine Einführung.</p></div></div>"#,
                r#"<div class="record__apparat"><div class="props">"#,
            )),
            "{body}"
        );
        assert!(
            body.contains(r#"</div><section class="pgroup">"#),
            "the group follows the props inside the Apparat: {body}"
        );
        assert!(
            body.ends_with("</section></div></div></article>"),
            "the Apparat is the body's last child and holds the group: {body}"
        );
        assert_eq!(body.matches("record__apparat").count(), 1, "{body}");
    }

    // The tall frame: `data-frame="tall"` on a viewer record

    fn sized_image(id: &str, width: Option<i64>, height: Option<i64>) -> Resource {
        Resource {
            id: id.to_owned(),
            class: "Image".to_owned(),
            properties: vec![],
            representations: vec![Representation {
                kind: RepresentationKind::Image,
                uri: "https://example.org/img.jpg".to_owned(),
                width,
                height,
                alt: None,
                attribution: None,
                seq: 0,
            }],
        }
    }

    /// The record article's opening tag over `media_items`.
    fn article_tag(media_items: &[Resource]) -> String {
        let cfg = make_alice_cfg("CH_001", Some("name"), &[], media_items);
        let out = render_resource_detail(&make_alice_resource(), cfg, &make_lang("en")).into_string();
        out[..=out.find('>').expect("the article tag closes")].to_owned()
    }

    #[test]
    fn every_image_tall_gives_the_tall_frame() {
        let items = [
            sized_image("I_1", Some(360), Some(4264)),
            sized_image("I_2", Some(300), Some(2000)),
        ];
        assert!(article_tag(&items).contains(r#" data-frame="tall""#), "{}", article_tag(&items));
    }

    #[test]
    fn an_image_at_exactly_one_to_three_is_tall() {
        let at = [sized_image("I_1", Some(100), Some(300))];
        assert!(article_tag(&at).contains(r#"data-frame="tall""#));
        let under = [sized_image("I_1", Some(100), Some(299))];
        assert!(!article_tag(&under).contains("data-frame"));
    }

    #[test]
    fn a_mixed_set_keeps_the_default_frame() {
        let items = [
            sized_image("I_1", Some(360), Some(4264)),
            sized_image("I_2", Some(1000), Some(1200)),
        ];
        assert!(!article_tag(&items).contains("data-frame"), "{}", article_tag(&items));
    }

    #[test]
    fn an_unknown_dimension_keeps_the_default_frame() {
        for (w, h) in [(None, Some(4264)), (Some(360), None), (None, None)] {
            let alone = [sized_image("I_1", w, h)];
            assert!(!article_tag(&alone).contains("data-frame"), "{w:?} × {h:?}");
            let beside_a_tall_one = [sized_image("I_1", Some(360), Some(4264)), sized_image("I_2", w, h)];
            assert!(!article_tag(&beside_a_tall_one).contains("data-frame"), "{w:?} × {h:?}");
        }
    }

    #[test]
    fn one_tall_image_gives_the_tall_frame() {
        let items = [sized_image("I_1", Some(360), Some(4264))];
        assert_eq!(
            article_tag(&items),
            r#"<article class="record page record--viewer" data-frame="tall" data-class="{'is-wide': $expand}">"#
        );
    }

    #[test]
    fn a_record_without_an_image_is_no_viewer_record_and_has_no_frame() {
        assert_eq!(article_tag(&[]), r#"<article class="record page">"#);
    }

    // The incoming-links list
    //
    // A synthetic bag, not a project's data: a `Band` target, sources of two
    // classes — `Folio` (own detail binding) and `Leaf` (no binding; a member
    // of a bound `Volume`, or of nothing).

    fn incoming_routes() -> HashMap<String, String> {
        HashMap::from([
            ("Folio".to_owned(), "/folios/:id".to_owned()),
            ("Volume".to_owned(), "/volumes/:id".to_owned()),
        ])
    }

    fn incoming_labels() -> Vec<cpe_core::ir::PropertyLabel> {
        vec![cpe_core::ir::PropertyLabel {
            property: "band-left".to_owned(),
            translations: vec![
                ("en".to_owned(), "Left band".to_owned()),
                ("de".to_owned(), "Band links".to_owned()),
            ],
        }]
    }

    fn make_band() -> Resource {
        Resource {
            id: "BAND_01".to_owned(),
            class: "Band".to_owned(),
            properties: vec![
                Property {
                    name: "name".to_owned(),
                    values: vec![PropertyValue::Text { text: "Band one".to_owned(), lang: None }],
                },
                Property {
                    name: "width".to_owned(),
                    values: vec![PropertyValue::Integer(42)],
                },
            ],
            representations: vec![],
        }
    }

    fn row(id: &str, class: &str, via: &str, parent: Option<(&str, &str)>) -> IncomingLink {
        IncomingLink {
            id: id.to_owned(),
            class: class.to_owned(),
            name: format!("Name {id}"),
            via: via.to_owned(),
            parent: parent.map(|(pid, pclass)| cpe_core::ir::MemberParent {
                id: pid.to_owned(),
                class: pclass.to_owned(),
                seq: 1,
            }),
            group: None,
            label: None,
            aside: None,
        }
    }

    /// One row per href rung: own binding · parent binding · neither.
    fn three_rung_rows() -> Vec<IncomingLink> {
        vec![
            row("F_1", "Folio", "band-left", None),
            row("L_2", "Leaf", "band-right", Some(("VOL_1", "Volume"))),
            row("L_3", "Leaf", "band-left", None),
        ]
    }

    /// `total` overrides `rows.len()` when `Some`
    /// — the count line renders only when the given total exceeds
    /// the row count. `None` ⇒ uncapped (`total == rows.len()`).
    fn render_incoming_with_total(
        rows: Option<&[IncomingLink]>,
        total: Option<usize>,
        media: &[Resource],
        current_query: Option<&str>,
        lang: &str,
    ) -> String {
        let properties = strings(&["band-left", "band-right"]);
        render_incoming_full(rows, total, &properties, None, None, media, current_query, lang)
    }

    /// The general harness behind every incoming-links test: `grouping`
    /// `None` ⇒ the ungrouped list, `properties` the spec's declared link
    /// properties, `note` the already-resolved `note-label` template.
    #[allow(clippy::too_many_arguments)]
    fn render_incoming_full(
        rows: Option<&[IncomingLink]>,
        total: Option<usize>,
        properties: &[String],
        grouping: Option<&cpe_core::ir::IncomingGrouping>,
        note: Option<&str>,
        media: &[Resource],
        current_query: Option<&str>,
        lang: &str,
    ) -> String {
        let routes = incoming_routes();
        let labels = incoming_labels();
        let incoming_links =
            rows.map(|r| cpe_core::ir::IncomingLinks { rows: r.to_vec(), total: total.unwrap_or(r.len()) });
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            current_path: "/demo/bands/BAND_01",
            current_query,
            links: Some(LinkResolver::new(cpe_core::url::ROOT.project("demo"), &routes)),
            property_labels: &labels,
            incoming: incoming_links.as_ref().map(|il| IncomingCfg {
                caption: "Folios",
                links: il,
                properties,
                grouping,
                note,
            }),
            ..make_alice_cfg("BAND_01", Some("name"), &[], media)
        };
        render_resource_detail(&make_band(), cfg, &make_lang(lang)).into_string()
    }

    fn render_incoming(
        rows: Option<&[IncomingLink]>,
        media: &[Resource],
        current_query: Option<&str>,
        lang: &str,
    ) -> String {
        render_incoming_with_total(rows, None, media, current_query, lang)
    }

    /// The `<li>` whose name is `name`, as a string slice.
    fn item_for<'o>(out: &'o str, name: &str) -> &'o str {
        let name_at = out.find(name).expect("row present");
        let start = out[..name_at].rfind("<li").expect("inside an li");
        let end = name_at + out[name_at..].find("</li>").expect("li closes");
        &out[start..end]
    }

    #[test]
    fn incoming_rung_1_links_to_the_sources_own_detail_page() {
        let rows = three_rung_rows();
        let out = render_incoming(Some(&rows), &[], None, "en");
        let li = item_for(&out, "Name F_1");
        assert!(
            li.contains(r#"<a class="int-link incoming__name" href="/demo/folios/F_1"><span class="ref">"#),
            "{li}"
        );
    }

    #[test]
    fn incoming_rung_2_links_into_the_parents_reader_without_the_current_query() {
        let rows = three_rung_rows();
        let out = render_incoming(Some(&rows), &[], Some("lang=de&img=2&member=3"), "en");
        let li = item_for(&out, "Name L_2");
        assert!(
            li.contains(r#"href="/demo/volumes/VOL_1?page=L_2""#),
            "the parent's reader, member selected, fresh query: {li}"
        );
        for leaked in ["lang=", "img=", "member="] {
            assert!(!li.contains(leaked), "{leaked} leaked: {li}");
        }
    }

    #[test]
    fn incoming_rung_2_percent_encodes_the_member_id() {
        let rows = vec![row("L&2#x", "Leaf", "band-left", Some(("VOL_1", "Volume")))];
        let out = render_incoming(Some(&rows), &[], None, "en");
        assert!(out.contains(r#"href="/demo/volumes/VOL_1?page=L%262%23x""#), "{out}");
    }

    #[test]
    fn incoming_rung_3_is_a_plain_span() {
        let rows = three_rung_rows();
        let out = render_incoming(Some(&rows), &[], None, "en");
        let li = item_for(&out, "Name L_3");
        assert!(li.contains(r#"<span class="incoming__name">Name L_3</span>"#), "{li}");
        assert!(!li.contains("href"), "{li}");
    }

    #[test]
    fn incoming_rung_2_needs_a_bound_parent_class() {
        let rows = vec![row("L_4", "Leaf", "band-left", Some(("X_1", "Unbound")))];
        let out = render_incoming(Some(&rows), &[], None, "en");
        let li = item_for(&out, "Name L_4");
        assert!(!li.contains("href"), "unbound parent ⇒ rung 3: {li}");
    }

    #[test]
    fn incoming_via_marker_uses_the_declared_label_else_humanized() {
        let rows = three_rung_rows();
        let out = render_incoming(Some(&rows), &[], None, "de");
        assert!(
            item_for(&out, "Name F_1").contains(r#"<span class="incoming__via">Band links</span>"#),
            "{out}"
        );
        assert!(
            item_for(&out, "Name L_2").contains(r#"<span class="incoming__via">Band Right</span>"#),
            "{out}"
        );
    }

    #[test]
    fn incoming_none_renders_nothing() {
        let out = render_incoming(None, &[], None, "en");
        assert!(!out.contains("incoming"), "{out}");
        assert!(!out.contains("Nothing links here."), "{out}");
    }

    #[test]
    fn incoming_some_empty_renders_caption_and_empty_sentence() {
        let out = render_incoming(Some(&[]), &[], None, "en");
        assert!(
            out.contains(
                r#"<section class="pgroup incoming"><header class="pgroup__head"><span class="pgroup__title">Folios</span></header><p class="resource-list__empty">Nothing links here.</p></section>"#
            ),
            "{out}"
        );
        assert!(!out.contains("incoming__list"), "{out}");
    }

    #[test]
    fn incoming_sits_after_the_last_group_and_before_host_rights() {
        let mut band = make_band();
        band.properties.push(Property {
            name: "license".to_owned(),
            values: vec![PropertyValue::Text { text: "CC0".to_owned(), lang: None }],
        });
        band.properties.push(Property {
            name: "date".to_owned(),
            values: vec![PropertyValue::Text { text: "1493".to_owned(), lang: None }],
        });
        let routes = incoming_routes();
        let rows = three_rung_rows();
        let incoming_links = cpe_core::ir::IncomingLinks { total: rows.len(), rows };
        let members = strings(&["date"]);
        let groups = [grp("Details", members.as_slice())];
        let declared = strings(&["band-left", "band-right"]);
        let media = make_alice_media();
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            links: Some(LinkResolver::new(cpe_core::url::ROOT.project("demo"), &routes)),
            groups: &groups,
            incoming: Some(IncomingCfg {
                caption: "Folios",
                links: &incoming_links,
                properties: &declared,
                grouping: None,
                note: None,
            }),
            ..make_alice_cfg("BAND_01", Some("name"), &[], &media)
        };
        let out = render_resource_detail(&band, cfg, &make_lang("en")).into_string();
        let group_at = out.find(">Details<").expect("group");
        let incoming_at = out.find("pgroup incoming").expect("incoming");
        let rights_at = out.find("CC0").expect("host rights");
        assert!(group_at < incoming_at && incoming_at < rights_at, "{out}");
    }

    #[test]
    fn incoming_links_capped_renders_count_line() {
        let rows = vec![
            row("F_1", "Folio", "band-left", None),
            row("L_2", "Leaf", "band-right", None),
        ];
        let out = render_incoming_with_total(Some(&rows), Some(5), &[], None, "en");
        assert!(out.contains(r#"class="incoming__count""#), "{out}");
        assert!(out.contains("Showing 2 of 5"), "{out}");
    }

    #[test]
    fn incoming_links_uncapped_renders_no_count_line() {
        let rows = vec![
            row("F_1", "Folio", "band-left", None),
            row("L_2", "Leaf", "band-right", None),
        ];
        let out = render_incoming_with_total(Some(&rows), Some(2), &[], None, "en");
        assert!(!out.contains("incoming__count"), "{out}");
    }

    #[test]
    fn incoming_links_empty_still_renders_empty_sentence() {
        let out = render_incoming_with_total(Some(&[]), Some(0), &[], None, "en");
        assert!(out.contains("Nothing links here."), "{out}");
        assert!(!out.contains("incoming__count"), "{out}");
    }

    #[test]
    fn snapshot_render_resource_detail_incoming_en() {
        let rows = three_rung_rows();
        let media = make_alice_media();
        insta::assert_snapshot!(render_incoming(Some(&rows), &media, None, "en"));
    }

    #[test]
    fn snapshot_render_resource_detail_incoming_de() {
        let rows = three_rung_rows();
        let media = make_alice_media();
        insta::assert_snapshot!(render_incoming(Some(&rows), &media, None, "de"));
    }

    #[test]
    fn snapshot_render_resource_detail_incoming_empty_en() {
        let media = make_alice_media();
        insta::assert_snapshot!(render_incoming(Some(&[]), &media, None, "en"));
    }

    #[test]
    fn snapshot_render_resource_detail_incoming_empty_de() {
        let media = make_alice_media();
        insta::assert_snapshot!(render_incoming(Some(&[]), &media, None, "de"));
    }

    #[test]
    fn snapshot_render_resource_detail_incoming_noimg_en() {
        let rows = three_rung_rows();
        insta::assert_snapshot!(render_incoming(Some(&rows), &[], None, "en"));
    }

    #[test]
    fn snapshot_render_resource_detail_incoming_noimg_de() {
        let rows = three_rung_rows();
        insta::assert_snapshot!(render_incoming(Some(&rows), &[], None, "de"));
    }

    // Grouped incoming links + row anatomy

    /// The ungrouped `section.pgroup.incoming` for [`three_rung_rows`] in
    /// `en`: the markup the ungrouped mode must keep emitting (the
    /// `/strips/:id` shape: three rungs, a `via` marker per row, no aside).
    const UNGROUPED_THREE_RUNG_EN: &str = concat!(
        r#"<section class="pgroup incoming"><header class="pgroup__head"><span class="pgroup__title">Folios</span></header>"#,
        r#"<ul class="incoming__list">"#,
        r#"<li class="incoming__item"><a class="int-link incoming__name" href="/demo/folios/F_1"><span class="ref">Name F_1</span></a> <span class="incoming__via">Left band</span></li>"#,
        r#"<li class="incoming__item"><a class="int-link incoming__name" href="/demo/volumes/VOL_1?page=L_2"><span class="ref">Name L_2</span></a> <span class="incoming__via">Band Right</span></li>"#,
        r#"<li class="incoming__item"><span class="incoming__name">Name L_3</span> <span class="incoming__via">Left band</span></li>"#,
        r#"</ul></section>"#,
    );

    #[test]
    fn ungrouped_list_is_byte_identical_to_slice_5() {
        let rows = three_rung_rows();
        let out = render_incoming(Some(&rows), &[], None, "en");
        assert!(out.contains(UNGROUPED_THREE_RUNG_EN), "{out}");
    }

    #[test]
    fn ungrouped_capped_list_is_byte_identical_to_slice_5() {
        let rows = three_rung_rows();
        let out = render_incoming_with_total(Some(&rows), Some(7), &[], None, "en");
        let expected = UNGROUPED_THREE_RUNG_EN.replace(
            "</ul></section>",
            r#"</ul><p class="incoming__count">Showing 3 of 7</p></section>"#,
        );
        assert!(out.contains(&expected), "{out}");
    }

    fn grouping(
        by: cpe_core::ir::IncomingGroupBy,
        layout: cpe_core::ir::IncomingLayout,
    ) -> cpe_core::ir::IncomingGrouping {
        cpe_core::ir::IncomingGrouping { by, layout, note_label: None }
    }

    fn decade(key: &str, label: &str, seq: i64) -> Option<cpe_core::ir::GroupKey> {
        Some(cpe_core::ir::GroupKey::List { key: key.to_owned(), label: label.to_owned(), seq })
    }

    /// Two Folios by decade, one Folio with no decade (the headless group).
    fn decade_rows() -> Vec<IncomingLink> {
        let mut a = row("F_1", "Folio", "band-left", None);
        a.group = decade("d2", "1840s", 2);
        let mut b = row("F_2", "Folio", "band-right", None);
        b.group = decade("d1", "1830s", 1);
        let c = row("F_3", "Folio", "band-left", None);
        vec![a, b, c]
    }

    fn render_grouped(
        rows: &[IncomingLink],
        total: Option<usize>,
        properties: &[&str],
        grouping: &cpe_core::ir::IncomingGrouping,
        note: Option<&str>,
        lang: &str,
    ) -> String {
        render_incoming_full(Some(rows), total, &strings(properties), Some(grouping), note, &[], None, lang)
    }

    #[test]
    fn ungrouped_row_label_and_aside_replace_name_and_keep_via() {
        let mut r = row("F_1", "Folio", "band-left", None);
        r.label = Some("28.11.1838".to_owned());
        r.aside = Some("IB".to_owned());
        let out = render_incoming(Some(&[r]), &[], None, "en");
        assert!(
            out.contains(concat!(
                r#"<li class="incoming__item"><a class="int-link incoming__name" href="/demo/folios/F_1"><span class="ref">28.11.1838</span></a>"#,
                r#" <span class="incoming__aside">IB</span> <span class="incoming__via">Left band</span></li>"#,
            )),
            "{out}"
        );
        assert!(!out.contains("Name F_1"), "row-label wins over name: {out}");
        assert!(!out.contains("incoming--grouped"), "{out}");
    }

    #[test]
    fn grouped_layout_classes_on_the_wrapper() {
        use cpe_core::ir::{IncomingGroupBy, IncomingLayout};
        let rows = decade_rows();
        let by = || IncomingGroupBy::Property("decade".into());
        let cards = render_grouped(
            &rows,
            None,
            &["band-left", "band-right"],
            &grouping(by(), IncomingLayout::Cards),
            None,
            "en",
        );
        assert!(
            cards.contains(
                r#"<section class="pgroup incoming incoming--grouped incoming--cards"><header class="pgroup__head"><h2 class="pgroup__title">Folios</h2></header><div class="incoming__groups">"#
            ),
            "{cards}"
        );
        let columns = render_grouped(
            &rows,
            None,
            &["band-left", "band-right"],
            &grouping(by(), IncomingLayout::Columns),
            None,
            "en",
        );
        assert!(
            columns.contains(r#"class="pgroup incoming incoming--grouped incoming--columns""#),
            "{columns}"
        );
    }

    #[test]
    fn grouped_cards_heads_are_label_plus_count_and_headless_group_has_none() {
        use cpe_core::ir::{IncomingGroupBy, IncomingLayout};
        let rows = decade_rows();
        let out = render_grouped(
            &rows,
            None,
            &["band-left", "band-right"],
            &grouping(IncomingGroupBy::Property("decade".into()), IncomingLayout::Cards),
            None,
            "en",
        );
        let first = out
            .find(r#"<span class="incoming__card-label">1830s</span>"#)
            .expect("1830s card");
        let second = out
            .find(r#"<span class="incoming__card-label">1840s</span>"#)
            .expect("1840s card");
        assert!(first < second, "seq order: {out}");
        assert!(
            out.contains(concat!(
                r#"<section class="incoming__group"><h3 class="incoming__card-head">"#,
                r#"<span class="incoming__card-label">1830s</span> <span class="incoming__count-n">1</span></h3>"#,
            )),
            "{out}"
        );
        // The trailing headless group: a group section whose first child is
        // the list, no head element.
        assert_eq!(out.matches(r#"<section class="incoming__group">"#).count(), 3);
        assert!(
            out.contains(r#"<section class="incoming__group"><ul class="incoming__list"><li class="incoming__item"><a class="int-link incoming__name" href="/demo/folios/F_3">"#),
            "{out}"
        );
        assert_eq!(out.matches("incoming__card-head").count(), 2, "{out}");
    }

    #[test]
    fn grouped_by_via_heads_are_property_labels_and_drop_the_via_marker() {
        use cpe_core::ir::{IncomingGroupBy, IncomingLayout};
        let rows = three_rung_rows();
        let out = render_grouped(
            &rows,
            None,
            &["band-left", "band-right"],
            &grouping(IncomingGroupBy::Via, IncomingLayout::Columns),
            None,
            "de",
        );
        assert!(
            out.contains(r#"<h3 class="incoming__role">Band links · 2</h3>"#),
            "declared label resolves through the property-label seam: {out}"
        );
        assert!(
            out.contains(r#"<h3 class="incoming__role">Band Right · 1</h3>"#),
            "undeclared ⇒ humanized: {out}"
        );
        assert!(!out.contains("incoming__via"), "{out}");
    }

    #[test]
    fn grouped_by_property_with_one_declared_property_drops_the_via_marker() {
        use cpe_core::ir::{IncomingGroupBy, IncomingLayout};
        let rows = decade_rows();
        let out = render_grouped(
            &rows,
            None,
            &["band-left"],
            &grouping(IncomingGroupBy::Property("decade".into()), IncomingLayout::Cards),
            None,
            "en",
        );
        assert!(!out.contains("incoming__via"), "{out}");
    }

    #[test]
    fn grouped_by_property_with_several_properties_keeps_the_via_marker() {
        use cpe_core::ir::{IncomingGroupBy, IncomingLayout};
        let rows = decade_rows();
        let out = render_grouped(
            &rows,
            None,
            &["band-left", "band-right"],
            &grouping(IncomingGroupBy::Property("decade".into()), IncomingLayout::Cards),
            None,
            "en",
        );
        assert_eq!(out.matches(r#"class="incoming__via""#).count(), 3, "{out}");
        assert!(out.contains(r#"<span class="incoming__via">Band Right</span>"#), "{out}");
    }

    #[test]
    fn grouped_note_renders_on_the_section_head_with_the_row_count() {
        use cpe_core::ir::{IncomingGroupBy, IncomingLayout};
        let rows = decade_rows();
        let out = render_grouped(
            &rows,
            Some(50),
            &["band-left", "band-right"],
            &grouping(IncomingGroupBy::Property("decade".into()), IncomingLayout::Cards),
            Some("Aufführungen · {n}"),
            "de",
        );
        assert!(
            out.contains(
                r#"<header class="pgroup__head"><h2 class="pgroup__title">Folios</h2> <span class="incoming__note">Aufführungen · 3</span></header>"#
            ),
            "{out}"
        );
    }

    #[test]
    fn ungrouped_ignores_a_note() {
        let rows = three_rung_rows();
        let properties = strings(&["band-left", "band-right"]);
        let out = render_incoming_full(Some(&rows), None, &properties, None, Some("Entries · {n}"), &[], None, "en");
        assert!(!out.contains("incoming__note"), "{out}");
        assert!(out.contains(UNGROUPED_THREE_RUNG_EN), "{out}");
    }

    #[test]
    fn grouped_capped_list_renders_the_count_line() {
        use cpe_core::ir::{IncomingGroupBy, IncomingLayout};
        let rows = decade_rows();
        let out = render_grouped(
            &rows,
            Some(1200),
            &["band-left", "band-right"],
            &grouping(IncomingGroupBy::Via, IncomingLayout::Columns),
            None,
            "en",
        );
        assert!(
            out.contains(r#"</div><p class="incoming__count">Showing 3 of 1,200</p></section>"#),
            "{out}"
        );
        let uncapped = render_grouped(
            &rows,
            Some(3),
            &["band-left", "band-right"],
            &grouping(IncomingGroupBy::Via, IncomingLayout::Columns),
            None,
            "en",
        );
        assert!(!uncapped.contains("incoming__count"), "{uncapped}");
    }

    #[test]
    fn grouped_empty_list_renders_nothing() {
        use cpe_core::ir::{IncomingGroupBy, IncomingLayout};
        let out = render_grouped(
            &[],
            Some(0),
            &["band-left", "band-right"],
            &grouping(IncomingGroupBy::Via, IncomingLayout::Columns),
            Some("Entries · {n}"),
            "en",
        );
        assert!(!out.contains("incoming"), "{out}");
        assert!(!out.contains("Folios"), "{out}");
        assert!(!out.contains("Nothing links here."), "{out}");
    }

    // Record header slots

    fn texts_prop(name: &str, values: &[&str]) -> Property {
        Property {
            name: name.to_owned(),
            values: values
                .iter()
                .map(|t| PropertyValue::Text { text: (*t).to_owned(), lang: None })
                .collect(),
        }
    }

    /// A Book carrying every header-slot source: alt title, author, place,
    /// publisher(s), date and extent, plus a shelfmark for the flat list.
    fn make_slot_book(publishers: &[&str], date: PropertyValue) -> Resource {
        let mut properties = vec![
            texts_prop("name", &["[Das] Narrenschiff (dt.)"]),
            texts_prop("alt-title", &["Das Narren Schyff"]),
            texts_prop("creator", &["Sebastian Brant"]),
            texts_prop("pub-place", &["Basel"]),
            Property { name: "pub-date".to_owned(), values: vec![date] },
            Property {
                name: "extent".to_owned(),
                values: vec![PropertyValue::Integer(328)],
            },
            texts_prop("shelfmark", &["AN V 3"]),
        ];
        if !publishers.is_empty() {
            properties.push(texts_prop("publisher", publishers));
        }
        Resource {
            id: "narrenschiff-dt".to_owned(),
            class: "Book".to_owned(),
            properties,
            representations: vec![],
        }
    }

    fn day_date() -> PropertyValue {
        date_value(
            "1495-03-03",
            Some("1495-03-03"),
            DatePrecision::Day,
            Some("3. März 1495"),
            Some("de"),
            Some(Calendar::Julian),
        )
    }

    /// The Narrenschiff imprint: place · printer · date · extent.
    fn slot_parts(unit: Option<&str>) -> Vec<cpe_core::labels::ResolvedGlossPart<'_>> {
        ["pub-place", "publisher", "pub-date", "extent"]
            .into_iter()
            .map(|property| cpe_core::labels::ResolvedGlossPart {
                property,
                unit: (property == "extent").then_some(unit).flatten(),
            })
            .collect()
    }

    fn render_slots(resource: &Resource, precision_word: bool, lang: &str) -> String {
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            head: Some(ResolvedHead {
                alt_title: Some("alt-title"),
                author: Some("creator"),
                imprint: slot_parts(Some("{n} Seiten")),
            }),
            date_precision_word: precision_word,
            ..make_alice_cfg("narrenschiff-dt", Some("name"), &[], &[])
        };
        render_resource_detail(resource, cfg, &make_lang(lang)).into_string()
    }

    #[test]
    fn head_slots_render_under_the_title_in_order() {
        let out = render_slots(&make_slot_book(&["Johann Bergmann de Olpe"], day_date()), false, "de");
        assert!(
            out.contains(r#"</h1><p class="record__alt">Das Narren Schyff</p><p class="record__author">Sebastian Brant</p><p class="record__imprint">Basel · Johann Bergmann de Olpe · 3. März 1495 · 328 Seiten</p></header>"#),
            "got: {out}"
        );
    }

    /// Every slot property leaves the implicit flat list; the rest stays.
    #[test]
    fn head_slot_properties_leave_the_implicit_list() {
        let out = render_slots(&make_slot_book(&["Johann Bergmann de Olpe"], day_date()), false, "de");
        let props = &out[out.find(r#"<div class="props">"#).expect("props")..];
        for gone in ["Schyff", "Sebastian", "Basel", "Bergmann", "März", "328"] {
            assert!(!props.contains(gone), "{gone} must not be a row, got: {props}");
        }
        assert!(props.contains("AN V 3"), "got: {props}");
    }

    #[test]
    fn head_slots_join_multiple_publishers_and_skip_an_empty_part() {
        let two = render_slots(&make_slot_book(&["Johann Amerbach", "Johann Petri"], day_date()), false, "de");
        assert!(
            two.contains(">Basel · Johann Amerbach, Johann Petri · 3. März 1495 · 328 Seiten</p>"),
            "got: {two}"
        );
        let none = render_slots(&make_slot_book(&[], day_date()), false, "de");
        assert!(none.contains(">Basel · 3. März 1495 · 328 Seiten</p>"), "got: {none}");
    }

    #[test]
    fn head_imprint_writes_a_year_range_at_its_precision() {
        let range = date_value("1475-01-01", Some("1499-12-31"), DatePrecision::Year, None, None, None);
        let out = render_slots(&make_slot_book(&[], range), true, "de");
        assert!(
            out.contains(">Basel · 1475–1499 · 328 Seiten</p>"),
            "no precision word in the imprint, got: {out}"
        );
    }

    /// The imprint drops the precision word but keeps the
    /// Gregorian marker, and only under the site flag.
    #[test]
    fn head_imprint_carries_the_gregorian_marker_under_the_flag() {
        let gregorian = || {
            date_value(
                "1495-03-03",
                Some("1495-03-03"),
                DatePrecision::Day,
                Some("3. März 1495"),
                Some("de"),
                Some(Calendar::Gregorian),
            )
        };
        let on = render_slots(&make_slot_book(&[], gregorian()), true, "de");
        assert!(
            on.contains(">Basel · 3. März 1495 · gregorianisch · 328 Seiten</p>"),
            "got: {on}"
        );
        assert!(!on.contains(">Tag<"), "no precision word, got: {on}");
        let off = render_slots(&make_slot_book(&[], gregorian()), false, "de");
        assert!(off.contains(">Basel · 3. März 1495 · 328 Seiten</p>"), "got: {off}");
    }

    /// An imprint whose every part is empty emits no `p.record__imprint`;
    /// an absent alt title / author emits no line either.
    #[test]
    fn head_slots_with_no_text_emit_nothing() {
        let bare = Resource {
            id: "b".to_owned(),
            class: "Book".to_owned(),
            properties: vec![texts_prop("name", &["Bare"])],
            representations: vec![],
        };
        let out = render_slots(&bare, true, "de");
        assert!(out.contains(r#"<h1 class="record__title">Bare</h1></header>"#), "got: {out}");
        assert!(
            !out.contains("record__alt") && !out.contains("record__author") && !out.contains("record__imprint"),
            "got: {out}"
        );
    }

    #[test]
    fn snapshot_render_resource_detail_record_head_slots_de() {
        insta::assert_snapshot!(render_slots(
            &make_slot_book(&["Johann Bergmann de Olpe"], day_date()),
            true,
            "de"
        ));
    }

    #[test]
    fn snapshot_render_resource_detail_record_head_slots_en() {
        let cfg = ResourceDetailCfg {
            reader: None,
            annotations: None,
            head: Some(ResolvedHead {
                alt_title: Some("alt-title"),
                author: Some("creator"),
                imprint: slot_parts(Some("{n} pages")),
            }),
            date_precision_word: true,
            ..make_alice_cfg("narrenschiff-dt", Some("name"), &[], &[])
        };
        let book = make_slot_book(&["Johann Bergmann de Olpe"], day_date());
        insta::assert_snapshot!(render_resource_detail(&book, cfg, &make_lang("en")).into_string());
    }
}
