//! The crumb line's "Show in dataset" panel: the page's resources that carry a
//! DSP data ARK, in the order the panel lists them.
//!
//! Only the record and narrative crumb lines carry a panel; the browse and
//! site-index crumb lines never do, and the unknown-record not-found crumb has no frame at all.
//!
//! A narrative page with figure blocks lists what they render in the
//! panel's listing form: a count line, one fold per page section when there
//! is more than one, case groups and compact two-line entries. The record and
//! reader panels do not use it.

use cpe_core::ir::{AnnotationEntry, ClassLabel, Component, CompoundPage, DataArks, Page, Resource, SiteConfig};
use cpe_core::labels::{resolve_class_label, resolve_class_plural};
use cpe_core::lang::Lang;
use cpe_core::view_data::ComponentData;
use maud::{html, Markup};

use super::annotation::{register_page_label, shape_word};
use super::data_entries::{DataEntry, DataGroup};
use super::dom::{PANEL_ID, PICKER_ESCAPE_EXPR, PICKER_FOCUSOUT_EXPR};
use super::values::{member_text, resolve_title_str};
use crate::chrome::{
    anno_register_heading, count_resources_text, data_copied_live, data_copy_name, data_count_line, data_entries_text,
    data_entry_name, data_open_name, data_region_role, data_regions_text, data_summary_name, ChromeMsg,
};
use crate::component::context::NarrativeStore;

/// The dataset panel of one crumb line. Only the engine builds one, from
/// [`cpe_core::ir::DataArks`], so every `ark` it carries passed `MalformedDataArk`.
///
/// A panel with no entries at all is never built: the crumb line then has no
/// panel row, so a caller passes `None` rather than an empty `DataPanel`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPanel {
    /// The record and the reader's shown member, each listed only when it has
    /// an ARK.
    pub(crate) entries: Vec<DataEntry>,
    /// The shown member's numbered annotations under their group head, or
    /// `None` when none of them is listed.
    pub(crate) annotations: Option<DataGroup>,
    /// What the page's figure blocks render, or `None` when none of them
    /// lists anything.
    pub(crate) listing: Option<DataListing>,
}

/// The listing form of a panel: what the page's figure blocks render, by page
/// section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DataListing {
    /// The counted parts ("32 Regionen", "6 Seiten"), in listing order: the
    /// summary's accessible name lists them.
    pub(crate) counts: Vec<String>,
    /// The count line under the panel head, built from `counts`.
    pub(crate) count_line: String,
    /// In page order; never empty, and no section is empty.
    pub(crate) sections: Vec<DataSection>,
}

/// One page section of a listing: the figure blocks of one `## … {#id}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DataSection {
    /// The section heading's text in the page language.
    pub(crate) heading: String,
    /// The blocks' case groups in render order; none is empty.
    pub(crate) groups: Vec<DataGroup>,
}

impl DataSection {
    /// The number of listed entries in the section.
    fn len(&self) -> usize {
        self.groups.iter().map(|g| g.entries.len()).sum()
    }
}

/// What the narrative crumb line gives the panel collector to resolve a
/// figure block's listing hook and its section heading. The record crumb line
/// passes none: a record page has no figure blocks.
#[doc(hidden)]
pub struct ListingSource<'a> {
    #[doc(hidden)]
    pub project_id: &'a str,
    #[doc(hidden)]
    pub site: &'a SiteConfig,
    #[doc(hidden)]
    pub narrative: &'a NarrativeStore,
    #[doc(hidden)]
    pub default_lang: &'a Lang,
}

// ── Client behaviour ──────────────────────────────────────────────────────────
//
// Every expression is static and identical on every entry: it reads the entry
// from the DOM (`el.closest('li')`), so no store data ever enters a script.
// DataStar splits a value expression (`data-signals`, `data-class`,
// `data-text`, `data-attr`) on `;`, so those must stay `;`-free; only the
// `data-on` click is a statement body.

/// The panel's receipt signal: `''`, or the copied entry's `li` id.
/// `copied_signal_reset_event` (`dsp-incubator cpe/bin/src/router.rs`) resets the same key on
/// every swap, since `data-signals` never resets an existing signal; keep the
/// two in step. The `_` prefix keeps it out of every `@get`.
pub(crate) const DATA_SIGNALS: &str = "{_dataCopied: ''}";

/// The copy button's click: write the entry's ARK (its Open link's `href`, the
/// one source of it) and, only once the write succeeds, show the receipt for
/// 2.4s. A rejected write shows nothing.
pub(crate) const COPY_PID_EXPR: &str = "navigator.clipboard.writeText(el.closest('li').querySelector('a.dasch-link').href).then(() => { $_dataCopied = el.closest('li').id; setTimeout(() => { $_dataCopied = '' }, 2400) }, () => {})";

/// The entry's copied state (the dot and the "Copied" tooltip are CSS on it).
pub(crate) const COPIED_CLASS_EXPR: &str = "{'is-copied': $_dataCopied === el.id}";

/// The panel's one live region reads the copied entry's hidden long form.
pub(crate) const LIVE_TEXT_EXPR: &str =
    "$_dataCopied && document.getElementById($_dataCopied)?.querySelector('.crumb-data__live-text')?.textContent || ''";

/// Reveals the copy button, rendered `hidden` so no-JS has no dead control.
pub(crate) const REVEAL_EXPR: &str = "{hidden: false}";

/// Render the crumb line's dataset row: `details#crumb-data`, closed (the
/// server never writes `open`), its summary over the panel.
pub(crate) fn render_data_panel(panel: &DataPanel, lang: &Lang) -> Markup {
    let groups = panel.annotations.iter().flat_map(|g| &g.entries);
    // A listing is named by its counts, never entry by entry: it can hold
    // dozens of entries.
    let summary_items: Vec<String> = panel
        .entries
        .iter()
        .chain(groups)
        .map(|e| match e.number {
            Some(n) => format!("{} {n}", ChromeMsg::DataRegion.text(lang)),
            None => format!("{} {}", e.role, e.title),
        })
        .chain(panel.listing.iter().flat_map(|l| l.counts.iter().cloned()))
        .collect();
    let first_group_id = panel.entries.len() + 1;
    let first_listing_id = first_group_id + panel.annotations.as_ref().map_or(0, |g| g.entries.len());
    html! {
        details
            .crumb-data
            id=(PANEL_ID)
            data-signals=(DATA_SIGNALS)
            data-on:keydown__window=(PICKER_ESCAPE_EXPR)
            data-on:click__outside="el.open = false"
            data-on:focusout=(PICKER_FOCUSOUT_EXPR)
        {
            summary.crumb-data__summary aria-label=(data_summary_name(&summary_items, lang)) {
                span.dasch-mark aria-hidden="true" {}
                span.crumb-data__label.crumb-label--long { (ChromeMsg::DataSummary.text(lang)) }
                span.crumb-data__label.crumb-label--short { (ChromeMsg::DataSummaryShort.text(lang)) }
                span.crumb-data__caret aria-hidden="true" { "▾" }
            }
            div.crumb-data__panel role="group" aria-label=(ChromeMsg::DataCaption.text(lang)) {
                div.crumb-data__head {
                    img .dasch-logo__img
                        .dasch-logo__img--light
                        src="/_static/cpe/dasch-logo-colour.svg"
                        alt=(ChromeMsg::DaschAriaLabel.text(lang))
                        width="2147"
                        height="621";
                    img .dasch-logo__img
                        .dasch-logo__img--dark
                        src="/_static/cpe/dasch-logo-white.svg"
                        alt=(ChromeMsg::DaschAriaLabel.text(lang))
                        width="2148"
                        height="622";
                    p.crumb-data__lede { (ChromeMsg::DataLede.text(lang)) }
                }
                @if let Some(listing) = &panel.listing {
                    p.crumb-data__count { (listing.count_line) }
                }
                // `role="list"`: `list-style: none` drops the list semantics
                // in Safari/VoiceOver.
                ol.crumb-data__list role="list" {
                    @for (i, entry) in panel.entries.iter().enumerate() {
                        (render_entry(entry, i + 1, false, lang))
                    }
                    @if let Some(group) = &panel.annotations {
                        li.crumb-data__group { (group.head) }
                        @for (i, entry) in group.entries.iter().enumerate() {
                            (render_entry(entry, first_group_id + i, false, lang))
                        }
                    }
                    @if let Some(listing) = &panel.listing {
                        (render_listing(listing, first_listing_id, lang))
                    }
                }
                span.crumb-data__live.visually-hidden aria-live="polite" data-text=(LIVE_TEXT_EXPR) {}
            }
        }
    }
}

/// The listing after the panel's own entries: with more than one section,
/// one `li > details.crumb-data__fold` per section (the first `open`), each
/// holding its own `ol[role=list]`, since an `ol` may hold only `li`; with
/// one section, its groups flat in the panel's list. Entry positions
/// continue from `first_id` across the folds.
fn render_listing(listing: &DataListing, first_id: usize, lang: &Lang) -> Markup {
    if let [section] = listing.sections.as_slice() {
        return render_groups(&section.groups, first_id, lang);
    }
    let starts = running_starts(first_id, listing.sections.iter().map(DataSection::len));
    html! {
        @for ((k, section), start) in listing.sections.iter().enumerate().zip(starts) {
            li.crumb-data__section {
                details.crumb-data__fold open[k == 0] {
                    summary.crumb-data__fold-summary {
                        span.crumb-data__fold-title { (section.heading) }
                        // The digit alone would be read without a noun.
                        span.crumb-data__fold-count {
                            span aria-hidden="true" { (section.len()) }
                            span.visually-hidden { ", " (data_entries_text(section.len(), lang)) }
                        }
                        span.crumb-data__fold-caret aria-hidden="true" {}
                    }
                    ol.crumb-data__sublist role="list" {
                        (render_groups(&section.groups, start, lang))
                    }
                }
            }
        }
    }
}

/// A listing's case groups: each head row, then its compact entries, the
/// positions running on from `first_id`.
fn render_groups(groups: &[DataGroup], first_id: usize, lang: &Lang) -> Markup {
    let starts = running_starts(first_id, groups.iter().map(|g| g.entries.len()));
    html! {
        @for (group, start) in groups.iter().zip(starts) {
            li.crumb-data__group { (group.head) }
            @for (entry, position) in group.entries.iter().zip(start..) {
                (render_entry(entry, position, true, lang))
            }
        }
    }
}

/// The first position of each run of `lengths`, counting on from `first`.
fn running_starts(first: usize, lengths: impl Iterator<Item = usize>) -> impl Iterator<Item = usize> {
    lengths.scan(first, |next, len| {
        let start = *next;
        *next += len;
        Some(start)
    })
}

/// One entry, `li#crumb-data-entry-<position>` (1-based over the entries,
/// the group heads not counted): its role and title, the PID with Copy PID,
/// the hidden receipt text, then Open in DSP. Copy PID precedes Open.
///
/// `compact` is the listing's two-line form: role and title on the first
/// line; the PID, Copy PID and Open on the second, the PID's label for
/// screen readers only.
fn render_entry(entry: &DataEntry, position: usize, compact: bool, lang: &Lang) -> Markup {
    let name = data_entry_name(&entry.role, &entry.title, lang);
    // The PID is shown from `ark:/` on; the resolver host is the boot-checked
    // prefix every ARK shares.
    let pid = entry.ark.find("ark:/").map_or(entry.ark.as_str(), |at| &entry.ark[at..]);
    let pid_box = html! {
        div.crumb-data__pid {
            code.crumb-data__code { (pid) }
            span.crumb-data__copy-wrap {
                button
                    .crumb-data__copy
                    type="button"
                    hidden
                    aria-label=(data_copy_name(&name, lang))
                    data-attr=(REVEAL_EXPR)
                    data-on:click=(COPY_PID_EXPR)
                {
                    (copy_glyph())
                    span.crumb-data__dot aria-hidden="true" {}
                }
                span.crumb-data__tip aria-hidden="true" {
                    span.crumb-data__tip-rest { (ChromeMsg::DataCopyPid.text(lang)) }
                    span.crumb-data__tip-done { (ChromeMsg::CopyLinkCopied.text(lang)) }
                }
            }
        }
    };
    let live_text = html! {
        span.crumb-data__live-text hidden { (data_copied_live(&name, lang)) }
    };
    let open = html! {
        a.dasch-link href=(entry.ark) aria-label=(data_open_name(&name, lang)) {
            span.dasch-link__mark aria-hidden="true" {}
            span.dasch-link__text { (ChromeMsg::DataOpen.text(lang)) }
        }
    };
    html! {
        li  class=({
                if compact {
                    "crumb-data__entry crumb-data__entry--compact"
                } else {
                    "crumb-data__entry"
                }
            })
            id=(format!("crumb-data-entry-{position}"))
            data-class=(COPIED_CLASS_EXPR)
        {
            div.crumb-data__id {
                span.crumb-data__role { (entry.role) }
                span
                    class=({
                        if entry.number.is_some() {
                            "crumb-data__title crumb-data__title--clamp"
                        } else {
                            "crumb-data__title"
                        }
                    })
                    lang=[entry.title_lang.as_deref()]
                { (entry.title) }
            }
            @if compact {
                div.crumb-data__line {
                    span.visually-hidden { (ChromeMsg::DataPidLabel.text(lang)) }
                    (pid_box)
                    (live_text)
                    (open)
                }
            } @else {
                div.crumb-data__pid-block {
                    span.crumb-data__pid-label { (ChromeMsg::DataPidLabel.text(lang)) }
                    (pid_box)
                }
                (live_text)
                (open)
            }
        }
    }
}

/// The copy glyph (two offset rectangles): 14×14, viewBox `0 0 16 16`, stroke-only,
/// `currentColor`.
fn copy_glyph() -> Markup {
    html! {
        svg aria-hidden="true"
            width="14"
            height="14"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            stroke-width="1.1"
            stroke-linecap="round"
            stroke-linejoin="round"
        {
            rect x="5.5" y="5.5" width="8" height="8" rx="0.5" {}
            path d="M10.5 5.5V3a.5.5 0 0 0-.5-.5H3a.5.5 0 0 0-.5.5v7a.5.5 0 0 0 .5.5h2.5" {}
        }
    }
}

/// The page's dataset panel: the first resolved `resource-detail` record, the
/// reader's shown member, then that member's numbered annotations under their
/// group head, then what the page's figure blocks list (`collect_listing`,
/// only with a `listing` source), each only when it has a data ARK. Nothing
/// else the page renders is listed: list rows, timeline markers, incoming
/// links, link targets and page-level annotation links never are. `None` when
/// nothing is listed.
pub fn page_data_panel(
    page: &Page,
    components: &[ComponentData],
    class_labels: &[ClassLabel],
    lang: &Lang,
    data_arks: &DataArks,
    listing: Option<&ListingSource<'_>>,
) -> Option<DataPanel> {
    let listing = listing.and_then(|src| collect_listing(page, components, src, lang, data_arks));
    match collect_page_entries(page, components, class_labels, lang, data_arks) {
        Some(panel) => Some(DataPanel { listing, ..panel }),
        None => listing.map(|listing| DataPanel {
            entries: Vec::new(),
            annotations: None,
            listing: Some(listing),
        }),
    }
}

/// What the page's figure blocks list: for every slotted component whose
/// registry row has a listing hook, its non-empty case groups, under the
/// heading of the section it renders in, read from the narrative it renders
/// into in the request's language (the narrative's 3-rung fallback, so the
/// heading matches the page). Blocks naming the same section share one; the
/// sections follow the narratives' heading order.
/// `None` when nothing is listed; a section with no listed entry is left out.
fn collect_listing(
    page: &Page,
    components: &[ComponentData],
    src: &ListingSource<'_>,
    lang: &Lang,
    data_arks: &DataArks,
) -> Option<DataListing> {
    // Each host narrative's part headings, parsed once.
    let mut parts: std::collections::HashMap<usize, Vec<cpe_core::narrative_outline::SectionHeading>> =
        std::collections::HashMap::new();
    // Keyed by (host, part position, section id).
    let mut sections: Vec<((usize, usize, &str), DataSection)> = Vec::new();
    for (i, c) in page.body.iter().enumerate() {
        let (Some(slot), Some(host)) = (page.figure_slot(i), page.figure_host(i)) else {
            continue;
        };
        let Some(Component::Narrative { file, .. }) = page.body.get(host) else {
            continue;
        };
        let Some(hook) = crate::component::project::data_listing(src.project_id, c) else {
            continue;
        };
        let data = components.get(i).unwrap_or(&ComponentData::Other);
        let groups: Vec<DataGroup> = hook(c, data, src.site, lang, data_arks)
            .into_iter()
            .filter(|g| !g.entries.is_empty())
            .collect();
        if groups.is_empty() {
            continue;
        }
        let headings = parts.entry(host).or_insert_with(|| {
            super::narrative::resolve_narrative_content(file, lang, src.default_lang, src.narrative)
                .map(cpe_core::narrative_outline::section_headings)
                .unwrap_or_default()
                .into_iter()
                .filter(|h| !h.holds_invite_list)
                .collect()
        });
        // Boot's `UnknownFigureSection` makes a miss unreachable.
        let (position, heading) = headings
            .iter()
            .enumerate()
            .find(|(_, h)| h.id == slot.section)
            .map_or((usize::MAX, slot.section.clone()), |(k, h)| (k, h.text.clone()));
        let key = (host, position, slot.section.as_str());
        match sections.iter_mut().find(|(k, _)| *k == key) {
            Some((_, section)) => section.groups.extend(groups),
            None => sections.push((key, DataSection { heading, groups })),
        }
    }
    if sections.is_empty() {
        return None;
    }
    sections.sort_by_key(|(key, _)| *key);
    let sections: Vec<DataSection> = sections.into_iter().map(|(_, s)| s).collect();
    let counts = listing_counts(&sections, &src.site.class_labels, lang);
    Some(DataListing {
        count_line: data_count_line(&counts, sections.len(), lang),
        counts,
        sections,
    })
}

/// A listing's counted parts: its Regions, then each listed class in the
/// order it first appears, by its declared label and plural.
fn listing_counts(sections: &[DataSection], class_labels: &[ClassLabel], lang: &Lang) -> Vec<String> {
    let entries = || sections.iter().flat_map(|s| &s.groups).flat_map(|g| &g.entries);
    let regions = entries().filter(|e| e.class.is_none()).count();
    let mut classes: Vec<(&str, usize)> = Vec::new();
    for class in entries().filter_map(|e| e.class.as_deref()) {
        match classes.iter_mut().find(|(c, _)| *c == class) {
            Some((_, n)) => *n += 1,
            None => classes.push((class, 1)),
        }
    }
    let regions = (regions > 0).then(|| data_regions_text(regions, lang));
    regions
        .into_iter()
        .chain(classes.into_iter().map(|(class, n)| {
            count_resources_text(
                n,
                &resolve_class_label(class_labels, class, lang),
                &resolve_class_plural(class_labels, class, lang),
                lang,
            )
        }))
        .collect()
}

/// The panel's entry part: the record, the reader's shown member and its
/// annotations.
fn collect_page_entries(
    page: &Page,
    components: &[ComponentData],
    class_labels: &[ClassLabel],
    lang: &Lang,
    data_arks: &DataArks,
) -> Option<DataPanel> {
    let record = page.body.iter().zip(components).find_map(|(c, slot)| match (c, slot) {
        (Component::ResourceDetail { title_property, .. }, ComponentData::Detail(Some(r))) => {
            Some((r, title_property.as_deref()))
        }
        _ => None,
    });
    let compound = components.iter().find_map(|c| match c {
        ComponentData::Compound(cp) => Some(cp),
        _ => None,
    });
    collect(record, compound, class_labels, lang, data_arks)
}

fn collect(
    record: Option<(&Resource, Option<&str>)>,
    compound: Option<&CompoundPage>,
    class_labels: &[ClassLabel],
    lang: &Lang,
    data_arks: &DataArks,
) -> Option<DataPanel> {
    let resource_entry = |resource: &Resource, title: &str| {
        data_arks.resource(&resource.id).map(|ark| DataEntry {
            role: resolve_class_label(class_labels, &resource.class, lang),
            title: title.to_owned(),
            title_lang: None,
            number: None,
            class: Some(resource.class.clone()),
            ark: ark.to_owned(),
        })
    };
    let mut entries: Vec<DataEntry> = record
        .and_then(|(resource, title_property)| {
            resource_entry(resource, resolve_title_str(resource, title_property, lang))
        })
        .into_iter()
        .collect();

    let mut annotations = None;
    if let Some((cp, member)) = compound.and_then(|cp| cp.member.as_ref().map(|m| (cp, m))) {
        let name = member_text(member, "name", lang);
        entries.extend(resource_entry(member, name.unwrap_or(&member.id)));
        let regions: Vec<DataEntry> = cp
            .annotations
            .iter()
            .filter_map(|entry| {
                let AnnotationEntry::Annotation { number, annotation, .. } = entry else {
                    return None;
                };
                let ark = data_arks.annotation(&annotation.id)?;
                let remark = annotation.remark.as_deref().filter(|r| !r.trim().is_empty());
                Some(DataEntry {
                    role: data_region_role(*number, lang),
                    title: remark.unwrap_or_else(|| shape_word(annotation.shape).text(lang)).to_owned(),
                    title_lang: remark.and(annotation.remark_lang.clone()),
                    number: Some(*number),
                    class: None,
                    ark: ark.to_owned(),
                })
            })
            .collect();
        if !regions.is_empty() {
            annotations = Some(DataGroup {
                head: format!(
                    "{} · {}",
                    anno_register_heading(&register_page_label(cp, member, lang), lang),
                    regions.len()
                ),
                entries: regions,
            });
        }
    }

    (!entries.is_empty() || annotations.is_some()).then_some(DataPanel { entries, annotations, listing: None })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use cpe_core::ir::{
        Annotation, AnnotationLink, AnnotationShape, MemberHeader, Pagination, Property, PropertyValue,
        ResourceListPage, TimelinePayload,
    };

    use super::*;

    const ARK: &str = "https://ark.dasch.swiss/ark:/72163/1/0803/";

    fn lang(code: &str) -> Lang {
        cpe_core::lang::resolve_builtin(&[code.to_owned()])
    }

    fn labels() -> Vec<ClassLabel> {
        let label = |class: &str, de: &str, en: &str| ClassLabel {
            class: class.to_owned(),
            translations: vec![("de".to_owned(), de.to_owned()), ("en".to_owned(), en.to_owned())],
            plural: vec![],
        };
        vec![
            label("Book", "Buch", "Book"),
            label("Page", "Seite", "Page"),
            label("BorderStrip", "Randleiste", "Border strip"),
        ]
    }

    fn resource(id: &str, class: &str, name: &str) -> Resource {
        Resource {
            id: id.to_owned(),
            class: class.to_owned(),
            properties: vec![Property {
                name: "name".to_owned(),
                values: vec![PropertyValue::Text { text: name.to_owned(), lang: None }],
            }],
            representations: vec![],
        }
    }

    fn region(id: &str, number: u32, remark: Option<&str>) -> AnnotationEntry {
        AnnotationEntry::Annotation {
            number,
            annotation: Annotation {
                id: id.to_owned(),
                resource_id: "c1v".to_owned(),
                shape: AnnotationShape::Rectangle,
                x: 0.1,
                y: 0.1,
                w: 0.2,
                h: 0.2,
                points: vec![],
                color: None,
                remark: remark.map(str::to_owned),
                remark_lang: remark.map(|_| "la".to_owned()),
            },
            partners: vec![],
        }
    }

    fn page_link(id: &str) -> AnnotationEntry {
        AnnotationEntry::Link {
            link: AnnotationLink {
                id: id.to_owned(),
                name: "Holzschnitt".to_owned(),
                name_lang: None,
                comment: None,
                comment_lang: None,
            },
            partners: vec![],
        }
    }

    fn reader(member: Resource, annotations: Vec<AnnotationEntry>) -> CompoundPage {
        CompoundPage {
            members: vec![MemberHeader {
                id: member.id.clone(),
                class: member.class.clone(),
                name: member.id.clone(),
            }],
            member: Some(member),
            index: 1,
            total: 1,
            member_media: vec![],
            stale_page: None,
            annotations,
            annotated_members: BTreeSet::new(),
        }
    }

    fn arks(resources: &[&str], annotations: &[&str]) -> DataArks {
        let map = |ids: &[&str]| -> HashMap<String, String> {
            ids.iter().map(|id| ((*id).to_owned(), format!("{ARK}{id}"))).collect()
        };
        DataArks::from_stored(map(resources), map(annotations))
    }

    fn book() -> Resource {
        resource("narrenschiff", "Book", "[Das] Narrenschiff (dt.)")
    }

    fn summary(panel: &DataPanel) -> Vec<(String, String)> {
        panel
            .entries
            .iter()
            .chain(panel.annotations.iter().flat_map(|g| &g.entries))
            .map(|e| (e.role.clone(), e.title.clone()))
            .collect()
    }

    fn pair(role: &str, title: &str) -> (String, String) {
        (role.to_owned(), title.to_owned())
    }

    #[test]
    fn book_page_lists_book_page_and_both_regions_in_register_order() {
        let cp = reader(
            resource("c1v", "Page", "c1v"),
            vec![region("r1", 1, Some("Narr")), region("r2", 2, Some("Schiff"))],
        );
        let panel = collect(
            Some((&book(), Some("name"))),
            Some(&cp),
            &labels(),
            &lang("de"),
            &arks(&["narrenschiff", "c1v"], &["r1", "r2"]),
        )
        .expect("a panel");
        assert_eq!(
            summary(&panel),
            [
                pair("Buch", "[Das] Narrenschiff (dt.)"),
                pair("Seite", "c1v"),
                pair("Region · Annotation 1", "Narr"),
                pair("Region · Annotation 2", "Schiff"),
            ]
        );
        let group = panel.annotations.as_ref().expect("a group");
        assert_eq!(group.head, "Annotationen auf c1v · 2");
        assert_eq!(group.entries[0].ark, format!("{ARK}r1"));
        assert_eq!(group.entries[0].number, Some(1));
        assert_eq!(group.entries[0].title_lang.as_deref(), Some("la"));
        assert_eq!(panel.entries[0].number, None);
    }

    #[test]
    fn strip_detail_lists_its_one_resource() {
        let strip = resource("l-1a", "BorderStrip", "Randleiste l_1a");
        let panel = collect(
            Some((&strip, Some("name"))),
            None,
            &labels(),
            &lang("en"),
            &arks(&["l-1a"], &[]),
        )
        .expect("a panel");
        assert_eq!(summary(&panel), [pair("Border strip", "Randleiste l_1a")]);
        assert_eq!(panel.annotations, None);
    }

    #[test]
    fn member_without_an_ark_is_skipped() {
        let cp = reader(resource("c1v", "Page", "c1v"), vec![region("r1", 1, None)]);
        let panel = collect(
            Some((&book(), Some("name"))),
            Some(&cp),
            &labels(),
            &lang("en"),
            &arks(&["narrenschiff"], &["r1"]),
        )
        .expect("a panel");
        assert_eq!(
            summary(&panel),
            [
                pair("Book", "[Das] Narrenschiff (dt.)"),
                pair("Region · Annotation 1", "Rectangle"),
            ]
        );
    }

    #[test]
    fn member_with_only_page_level_links_has_no_group_head() {
        let cp = reader(resource("a3r", "Page", "a3r"), vec![page_link("l1")]);
        let panel = collect(
            Some((&book(), Some("name"))),
            Some(&cp),
            &labels(),
            &lang("de"),
            &arks(&["narrenschiff", "a3r"], &["l1"]),
        )
        .expect("a panel");
        assert_eq!(
            summary(&panel),
            [pair("Buch", "[Das] Narrenschiff (dt.)"), pair("Seite", "a3r")]
        );
        assert_eq!(panel.annotations, None);
    }

    #[test]
    fn annotation_without_an_ark_is_skipped_and_the_rest_keep_their_numbers() {
        let cp = reader(
            resource("c1v", "Page", "c1v"),
            vec![
                region("r1", 1, Some("eins")),
                region("r2", 2, Some("zwei")),
                region("r3", 3, Some("drei")),
                page_link("l1"),
            ],
        );
        let panel = collect(None, Some(&cp), &labels(), &lang("de"), &arks(&[], &["r1", "r3", "l1"])).expect("a panel");
        let group = panel.annotations.expect("a group");
        assert_eq!(group.head, "Annotationen auf c1v · 2");
        let numbers: Vec<_> = group.entries.iter().map(|e| e.number).collect();
        assert_eq!(numbers, [Some(1), Some(3)]);
        assert_eq!(group.entries[1].role, "Region · Annotation 3");
        assert!(panel.entries.is_empty());
    }

    #[test]
    fn an_empty_remark_falls_back_to_the_shape_word() {
        let cp = reader(resource("c1v", "Page", "c1v"), vec![region("r1", 1, Some("  "))]);
        let panel = collect(None, Some(&cp), &labels(), &lang("de"), &arks(&[], &["r1"])).expect("a panel");
        let entry = &panel.annotations.expect("a group").entries[0];
        assert_eq!(entry.title, "Rechteck");
        assert_eq!(entry.title_lang, None);
    }

    #[test]
    fn no_ark_at_all_gives_no_panel() {
        let cp = reader(resource("c1v", "Page", "c1v"), vec![region("r1", 1, None)]);
        let panel = collect(
            Some((&book(), Some("name"))),
            Some(&cp),
            &labels(),
            &lang("en"),
            &DataArks::default(),
        );
        assert_eq!(panel, None);
    }

    /// List rows and timeline markers carry ARKs here, and only the record is
    /// listed; incoming links never reach the collector at all.
    #[test]
    fn list_and_timeline_data_are_never_listed() {
        let rows = || ResourceListPage {
            items: vec![resource("other-book", "Book", "Postilla")],
            total: 1,
            page: 1,
            pagination: Pagination::Limit(1),
            unfiltered_total: None,
        };
        let page = Page {
            id: "book".to_owned(),
            route: "/books/:id".to_owned(),
            params: vec!["id".to_owned()],
            body: vec![
                Component::ResourceList {
                    id: Some("book-list".to_owned()),
                    class: "Book".to_owned(),
                    wheres: vec![],
                    sorts: vec![],
                    pagination: Pagination::Limit(1),
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
                Component::ResourceDetail {
                    id: Some("$id".to_owned()),
                    show_properties: None,
                    title_property: Some("name".to_owned()),
                    class: Some("Book".to_owned()),
                    media: None,
                    groups: vec![],
                    narrative_for: vec![],
                    incoming_links: None,
                    head: None,
                    navigator: false,
                },
            ],
            figure_slots: Vec::new(),
        };
        let components = [
            ComponentData::List(rows()),
            ComponentData::Detail(Some(book())),
            ComponentData::Timeline(TimelinePayload { resources: rows(), lanes: vec![] }),
        ];
        let panel = page_data_panel(
            &page,
            &components,
            &labels(),
            &lang("en"),
            &arks(&["narrenschiff", "other-book"], &[]),
            None,
        )
        .expect("a panel");
        assert_eq!(summary(&panel), [pair("Book", "[Das] Narrenschiff (dt.)")]);
    }

    // ── The listing form (figure blocks) ────────────────────────────

    use cpe_core::ir::{
        AnnotationCrop, CaseLayout, FigureSlot, MemberParent, PagePlate, ShowcaseCase, ShowcaseCrop, SiteConfig,
        WoodcutCasesPayload,
    };

    fn site() -> SiteConfig {
        SiteConfig {
            title: "Incunabula".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["de".to_owned(), "en".to_owned()],
            default_language: "de".to_owned(),
            nav: Vec::new(),
            chrome: Vec::new(),
            property_labels: Vec::new(),
            class_labels: {
                let mut labels = labels();
                labels[1].plural = vec![
                    ("de".to_owned(), "Seiten".to_owned()),
                    ("en".to_owned(), "pages".to_owned()),
                ];
                labels
            },
            theme: cpe_core::theme::ThemeKnob::default(),
            date_precision_word: false,
            footer_attribution: None,
            not_found_browse: Vec::new(),
            narrative_companion: Default::default(),
            narrative_side: Default::default(),
            text_align: Default::default(),
        }
    }

    fn crop_source(id: &str, page: &str) -> AnnotationCrop {
        let AnnotationEntry::Annotation { annotation, .. } = region(id, 1, None) else {
            unreachable!("region builds an annotation")
        };
        AnnotationCrop {
            annotation,
            resource_class: "Page".to_owned(),
            resource_label: page.to_owned(),
            parent: Some(MemberParent {
                id: "narrenschiff-dt".to_owned(),
                class: "Book".to_owned(),
                seq: 1,
            }),
            parent_title: Some("[Das] Narrenschiff (dt.)".to_owned()),
            image: None,
        }
    }

    fn crops_case(schramm: u32, ids: &[&str]) -> ShowcaseCase {
        ShowcaseCase::Crops {
            schramm: Some(schramm),
            title: None,
            remark: None,
            crops: ids
                .iter()
                .map(|id| ShowcaseCrop { annotation: (*id).to_owned(), alt: None })
                .collect(),
            detail: None,
        }
    }

    fn cases_block(cases: Vec<ShowcaseCase>) -> Component {
        Component::WoodcutCases {
            media: "image".to_owned(),
            layout: CaseLayout::Edition,
            cases,
        }
    }

    fn slot(section: &str) -> Option<FigureSlot> {
        Some(FigureSlot { section: section.to_owned(), count: None, more: None })
    }

    /// A showcase page: one narrative, then three blocks in sections `one`
    /// (two blocks) and `two` (a page case), with the data each renders.
    fn showcase_page(sections: &[&str]) -> (Page, Vec<ComponentData>) {
        let plate = |id: &str, page: &str| PagePlate {
            resource_id: id.to_owned(),
            resource_class: "Page".to_owned(),
            resource_label: page.to_owned(),
            parent: None,
            parent_title: Some("Reise ins Heilige Land".to_owned()),
            image: None,
        };
        let payload = |crops: Vec<AnnotationCrop>, pages: Vec<PagePlate>| {
            ComponentData::WoodcutCases(WoodcutCasesPayload { crops, links: Vec::new(), pages })
        };
        let mut body = vec![Component::Narrative { file: "woodcuts.md".to_owned(), companion: None, side: None }];
        let mut figure_slots = vec![None];
        let mut components = vec![ComponentData::Other];
        let blocks = [
            (
                cases_block(vec![crops_case(1124, &["a1", "a2"])]),
                payload(vec![crop_source("a1", "c1v"), crop_source("a2", "c8r")], vec![]),
            ),
            (
                cases_block(vec![crops_case(1125, &["b1"])]),
                payload(vec![crop_source("b1", "c3v")], vec![]),
            ),
            (
                cases_block(vec![ShowcaseCase::Pages {
                    link: "l1".to_owned(),
                    remark: None,
                    pages: vec!["reise-h8r".to_owned(), "reise-c6v".to_owned()],
                }]),
                payload(vec![], vec![plate("reise-c6v", "c6v"), plate("reise-h8r", "h8r")]),
            ),
        ];
        for ((block, data), section) in blocks.into_iter().zip(sections) {
            body.push(block);
            figure_slots.push(slot(section));
            components.push(data);
        }
        let page = Page {
            id: "woodcuts".to_owned(),
            route: "/woodcuts".to_owned(),
            params: vec![],
            body,
            figure_slots,
        };
        (page, components)
    }

    const WOODCUTS_DE: &str = "# Holzschnitte\n\nIntro.\n\n## Zwischen den *Ausgaben* {#one}\n\n\
                               Text.\n\n## Seitenebene {#two}\n\nText.\n";
    const WOODCUTS_EN: &str = "# Woodcuts\n\nIntro.\n\n## Between the editions {#one}\n\n\
                               Text.\n\n## Page level {#two}\n\nText.\n";

    fn listing_panel(sections: &[&str], code: &str, without: &[&str]) -> Option<DataPanel> {
        let (page, components) = showcase_page(sections);
        let ids = ["a1", "a2", "b1"];
        let resources = ["reise-h8r", "reise-c6v"];
        let keep = |ids: &[&'static str]| -> Vec<&'static str> {
            ids.iter().copied().filter(|id| !without.contains(id)).collect()
        };
        let site = site();
        let store = NarrativeStore::from_pairs(&[
            ("woodcuts.de.md".to_owned(), WOODCUTS_DE.to_owned()),
            ("woodcuts.en.md".to_owned(), WOODCUTS_EN.to_owned()),
        ]);
        let default_lang = lang("de");
        let src = ListingSource {
            project_id: "incunabula",
            site: &site,
            narrative: &store,
            default_lang: &default_lang,
        };
        page_data_panel(
            &page,
            &components,
            &site.class_labels,
            &lang(code),
            &arks(&keep(&resources), &keep(&ids)),
            Some(&src),
        )
    }

    fn sections_of(panel: &DataPanel) -> Vec<(String, Vec<String>)> {
        panel
            .listing
            .as_ref()
            .expect("a listing")
            .sections
            .iter()
            .map(|s| {
                let heads = s.groups.iter().map(|g| g.head.clone()).collect();
                (s.heading.clone(), heads)
            })
            .collect()
    }

    #[test]
    fn figure_blocks_are_listed_by_section_with_their_heading_and_counts() {
        let panel = listing_panel(&["one", "one", "two"], "de", &[]).expect("a panel");
        assert!(panel.entries.is_empty() && panel.annotations.is_none());
        assert_eq!(
            sections_of(&panel),
            [
                (
                    "Zwischen den Ausgaben".to_owned(),
                    vec!["Schramm 1124 · 2".to_owned(), "Schramm 1125 · 1".to_owned()]
                ),
                ("Seitenebene".to_owned(), vec!["Seitenebene · h8r / c6v · 2".to_owned()]),
            ]
        );
        let listing = panel.listing.expect("a listing");
        assert_eq!(listing.counts, ["3 Regionen", "2 Seiten"]);
        assert_eq!(listing.count_line, "3 Regionen und 2 Seiten · in 2 Abschnitten");
    }

    /// The folds follow the narrative's heading order, not the blocks'
    /// config order.
    #[test]
    fn sections_follow_the_narrative_not_the_block_order() {
        let panel = listing_panel(&["two", "one", "one"], "de", &[]).expect("a panel");
        assert_eq!(
            sections_of(&panel),
            [
                (
                    "Zwischen den Ausgaben".to_owned(),
                    vec!["Schramm 1125 · 1".to_owned(), "Seitenebene · h8r / c6v · 2".to_owned()]
                ),
                ("Seitenebene".to_owned(), vec!["Schramm 1124 · 2".to_owned()]),
            ]
        );
    }

    /// One section id in two narratives is two sections.
    #[test]
    fn a_section_is_keyed_by_its_narrative_too() {
        let (mut page, mut components) = showcase_page(&["one", "one", "one"]);
        page.body.insert(
            2,
            Component::Narrative { file: "woodcuts.md".to_owned(), companion: None, side: None },
        );
        page.figure_slots.insert(2, None);
        components.insert(2, ComponentData::Other);
        let site = site();
        let store = NarrativeStore::from_pairs(&[("woodcuts.de.md".to_owned(), WOODCUTS_DE.to_owned())]);
        let de = lang("de");
        let src = ListingSource {
            project_id: "incunabula",
            site: &site,
            narrative: &store,
            default_lang: &de,
        };
        let panel = page_data_panel(
            &page,
            &components,
            &site.class_labels,
            &de,
            &arks(&["reise-h8r", "reise-c6v"], &["a1", "a2", "b1"]),
            Some(&src),
        )
        .expect("a panel");
        let sections = sections_of(&panel);
        assert_eq!(sections.len(), 2, "{sections:?}");
        assert_eq!(sections[0].1, ["Schramm 1124 · 2"]);
        assert_eq!(sections[1].1.len(), 2);
    }

    #[test]
    fn the_section_heading_follows_the_request_language() {
        let panel = listing_panel(&["one", "one", "two"], "en", &[]).expect("a panel");
        assert_eq!(sections_of(&panel)[0].0, "Between the editions");
        assert_eq!(
            panel.listing.expect("a listing").count_line,
            "3 Regions and 2 pages · in 2 sections"
        );
    }

    #[test]
    fn one_section_drops_the_sections_part() {
        let panel = listing_panel(&["one", "one", "one"], "de", &[]).expect("a panel");
        let listing = panel.listing.expect("a listing");
        assert_eq!(listing.sections.len(), 1);
        assert_eq!(listing.count_line, "3 Regionen und 2 Seiten");
    }

    /// Counts are of listed entries; a section left with none is no section.
    #[test]
    fn an_unlisted_section_is_left_out_and_not_counted() {
        let panel = listing_panel(&["one", "one", "two"], "de", &["reise-h8r", "reise-c6v", "a2"]).expect("a panel");
        let listing = panel.listing.expect("a listing");
        assert_eq!(listing.sections.len(), 1);
        assert_eq!(listing.count_line, "2 Regionen");
        assert_eq!(listing.sections[0].groups[0].head, "Schramm 1124 · 1");
    }

    #[test]
    fn nothing_listed_and_no_record_gives_no_panel() {
        let all = ["a1", "a2", "b1", "reise-h8r", "reise-c6v"];
        assert_eq!(listing_panel(&["one", "one", "two"], "de", &all), None);
    }

    /// The record crumb line passes no listing source: today's panel.
    #[test]
    fn without_a_listing_source_figure_blocks_list_nothing() {
        let (page, components) = showcase_page(&["one", "one", "two"]);
        let panel = page_data_panel(&page, &components, &labels(), &lang("de"), &arks(&["reise-h8r"], &["a1"]), None);
        assert_eq!(panel, None);
    }

    fn rendered_listing(sections: &[&str], code: &str) -> String {
        let panel = listing_panel(sections, code, &[]).expect("a panel");
        render_data_panel(&panel, &lang(code)).into_string()
    }

    #[test]
    fn snapshot_data_panel_listing_folds_de() {
        insta::assert_snapshot!(rendered_listing(&["one", "one", "two"], "de"));
    }

    #[test]
    fn snapshot_data_panel_listing_folds_en() {
        insta::assert_snapshot!(rendered_listing(&["one", "one", "two"], "en"));
    }

    #[test]
    fn snapshot_data_panel_listing_flat_de() {
        insta::assert_snapshot!(rendered_listing(&["one", "one", "one"], "de"));
    }

    /// One fold per section, only the first open; each fold carries its own
    /// list; the entry ids run on across the folds; the summary stays the
    /// panel's first child and is named by the counts.
    #[test]
    fn folds_open_the_first_and_number_entries_across_them() {
        let out = rendered_listing(&["one", "one", "two"], "de");
        assert_eq!(out.matches("<details class=\"crumb-data__fold\"").count(), 2);
        assert!(
            out.contains(
                "<span class=\"crumb-data__fold-count\"><span aria-hidden=\"true\">3</span>\
                 <span class=\"visually-hidden\">, 3 Einträge</span></span>"
            ),
            "the fold count is named with its noun: {out}"
        );
        assert_eq!(out.matches("<details class=\"crumb-data__fold\" open>").count(), 1);
        let first = out.find("crumb-data__fold\" open>").expect("open fold");
        let second = out.rfind("<details class=\"crumb-data__fold\">").expect("closed fold");
        assert!(first < second, "{out}");
        assert_eq!(out.matches("<ol class=\"crumb-data__sublist\" role=\"list\">").count(), 2);
        for n in 1..=5 {
            assert!(out.contains(&format!("id=\"crumb-data-entry-{n}\"")), "{n}: {out}");
        }
        assert!(!out.contains("crumb-data-entry-6"), "{out}");
        assert!(second < out.find("crumb-data-entry-4").expect("entry 4"), "{out}");
        assert!(
            out.starts_with("<details class=\"crumb-data\" id=\"crumb-data\" data-signals=\"{_dataCopied: ''}\""),
            "{out}"
        );
        let summary = out.find("<summary").expect("summary");
        assert!(summary < out.find("crumb-data__fold").expect("fold"), "{out}");
        assert_eq!(
            attr_after(&out, "<summary", "aria-label"),
            "Im Datensatz zeigen — 3 Regionen, 2 Seiten"
        );
        assert!(
            out.contains("<p class=\"crumb-data__count\">3 Regionen und 2 Seiten · in 2 Abschnitten</p>"),
            "{out}"
        );
    }

    #[test]
    fn one_section_lists_its_groups_flat() {
        let out = rendered_listing(&["one", "one", "one"], "de");
        assert!(!out.contains("crumb-data__fold"), "{out}");
        assert!(!out.contains("crumb-data__sublist"), "{out}");
        assert_eq!(out.matches("<li class=\"crumb-data__group\">").count(), 3);
        assert_eq!(out.matches("crumb-data__entry--compact").count(), 5);
    }

    /// The compact entry keeps every hook the receipt and the close handlers
    /// read: its own `li` id, the copy button and the Open link inside it.
    #[test]
    fn a_compact_entry_keeps_the_receipt_hooks() {
        let out = rendered_listing(&["one", "one", "one"], "en");
        let at = out.find("id=\"crumb-data-entry-1\"").expect("entry");
        let end = at + out[at..].find("</li>").expect("entry end");
        let entry = &out[at..end];
        for needle in [
            "data-class=\"{'is-copied': $_dataCopied === el.id}\"",
            "<span class=\"crumb-data__role\">Region · c1v</span>",
            "<span class=\"visually-hidden\">ARK URL (PID)</span>",
            "<button class=\"crumb-data__copy\" type=\"button\" hidden",
            "<span class=\"crumb-data__live-text\" hidden>",
            "<a class=\"dasch-link\" href=\"https://ark.dasch.swiss/ark:/72163/1/0803/a1\"",
        ] {
            assert!(entry.contains(needle), "{needle}: {entry}");
        }
        let copy = entry.find("<button").expect("copy");
        let open = entry.find("dasch-link").expect("open");
        assert!(copy < open, "{entry}");
        assert!(!entry.contains("crumb-data__pid-label"), "{entry}");
    }

    // ── Rendering ────────────────────────────────────────────────────────────

    /// Book, Page and two Regions, rendered in `code`.
    fn rendered(code: &str) -> String {
        let cp = reader(
            resource("c1v", "Page", "c1v"),
            vec![region("r1", 1, Some("Narr")), region("r2", 2, None)],
        );
        let panel = collect(
            Some((&book(), Some("name"))),
            Some(&cp),
            &labels(),
            &lang(code),
            &arks(&["narrenschiff", "c1v"], &["r1", "r2"]),
        )
        .expect("a panel");
        render_data_panel(&panel, &lang(code)).into_string()
    }

    /// The value of the first `attr="…"` after `from` in `out`.
    fn attr_after<'a>(out: &'a str, from: &str, attr: &str) -> &'a str {
        let at = out.find(from).expect("anchor present");
        let open = format!("{attr}=\"");
        let start = at + out[at..].find(&open).expect("attribute present") + open.len();
        &out[start..start + out[start..].find('"').expect("attribute closed")]
    }

    /// WCAG 2.5.3: in every language the summary's, Copy PID's and Open's
    /// accessible names begin with their visible label.
    #[test]
    fn accessible_names_contain_the_visible_label_in_every_language() {
        for code in cpe_core::lang::BUILTIN_LANGUAGES {
            let l = lang(code);
            let out = rendered(code);
            for (anchor, attr, label) in [
                ("<summary", "aria-label", ChromeMsg::DataSummary.text(&l)),
                ("<button", "aria-label", ChromeMsg::DataCopyPid.text(&l)),
                ("<a class=\"dasch-link\"", "aria-label", ChromeMsg::DataOpen.text(&l)),
            ] {
                let name = attr_after(&out, anchor, attr);
                assert!(name.starts_with(label), "{code}: {name:?} vs {label:?}");
                assert!(out.contains(&format!(">{label}<")), "{code}: visible {label:?}");
            }
        }
    }

    #[test]
    fn accessible_names_name_the_entry_and_the_listed_resources() {
        let out = rendered("de");
        assert_eq!(
            attr_after(&out, "<summary", "aria-label"),
            "Im Datensatz zeigen — Buch [Das] Narrenschiff (dt.), Seite c1v, Region 1, Region 2"
        );
        let page = out.find("crumb-data-entry-2").expect("page entry");
        assert_eq!(
            attr_after(&out[page..], "<a class=\"dasch-link\"", "aria-label"),
            "In DSP öffnen: Seite „c1v“ (öffnet DSP, verlässt CPE)"
        );
        assert_eq!(attr_after(&out[page..], "<button", "aria-label"), "PID kopieren: Seite „c1v“");
        assert!(
            out[page..].contains("PID von Seite „c1v“ in die Zwischenablage kopiert"),
            "{out}"
        );
    }

    /// The panel renders closed, with no copy control until JS reveals it,
    /// and each entry's Copy PID comes before its Open link.
    #[test]
    fn panel_renders_closed_with_a_hidden_copy_button_before_open() {
        let out = rendered("en");
        assert!(out.starts_with(r#"<details class="crumb-data" id="crumb-data""#), "{out}");
        let details_tag = &out[..out.find('>').expect("tag end")];
        assert!(!details_tag.contains(" open"), "{details_tag}");
        assert_eq!(
            out.matches(r#"<button class="crumb-data__copy" type="button" hidden"#).count(),
            4
        );
        for n in 1..=4 {
            let at = out.find(&format!(r#"id="crumb-data-entry-{n}""#)).expect("entry");
            let copy = at + out[at..].find("<button").expect("copy");
            let open = at + out[at..].find("dasch-link").expect("open");
            assert!(copy < open, "entry {n}: {out}");
        }
        assert!(!out.contains("crumb-data-entry-5"), "{out}");
    }

    #[test]
    fn group_head_precedes_the_regions_and_pids_start_at_ark() {
        let out = rendered("en");
        let head = out
            .find(r#"<li class="crumb-data__group">Annotations on c1v · 2</li>"#)
            .expect("group head");
        assert!(out.find("crumb-data-entry-2").expect("page") < head, "{out}");
        assert!(head < out.find("crumb-data-entry-3").expect("region"), "{out}");
        assert!(
            out.contains(r#"<code class="crumb-data__code">ark:/72163/1/0803/r1</code>"#),
            "{out}"
        );
        assert!(out.contains(r#"href="https://ark.dasch.swiss/ark:/72163/1/0803/r1""#), "{out}");
        assert!(!out.contains("ext-link"), "{out}");
        assert!(
            out.contains(r#"<span class="crumb-data__title crumb-data__title--clamp" lang="la">Narr</span>"#),
            "{out}"
        );
    }

    /// DataStar splits a value expression on every `;`, so these must have
    /// none; and `$el` would read a signal named `el`, never the element.
    #[test]
    fn value_expressions_are_semicolon_free_and_use_el() {
        for expr in [DATA_SIGNALS, COPIED_CLASS_EXPR, LIVE_TEXT_EXPR, REVEAL_EXPR] {
            assert!(!expr.contains(';'), "{expr}");
        }
        for expr in [COPIED_CLASS_EXPR, LIVE_TEXT_EXPR, COPY_PID_EXPR] {
            assert!(!expr.contains("$el"), "{expr}");
        }
    }
}
