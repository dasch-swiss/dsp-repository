//! The parse half of the editorial narrative: the buffered event stream and
//! the readings of its top-level `##` headings and invite conventions that
//! the renderer (`component::editorial`), the boot validator
//! (`UnknownFigureSection`, `FigureSectionHoldsInviteList`) and the dataset
//! panel (`component::data_links`) share.
//!
//! A domain leaf: pulldown-cmark and `util`'s parser options only, no Maud,
//! so the validator can read a narrative's sections without importing a
//! renderer. The renderer and the readers must read the same stream and the
//! same "top-level `##`" ([`top_heading_at`]), or boot would accept a section
//! the page never splits at.
//!
//! It lives in `cpe-core`, which cannot depend on the component, store or
//! page-assembly crates (the incubator's `just cpe crate-deps-check`). A new reading
//! of a narrative's structure goes here; only its rendering goes to
//! `component::editorial`.

use pulldown_cmark::{html as cmark_html, Event, Parser, Tag, TagEnd};

use crate::url::Base;

// Invite-list item parsing

/// Attempt to parse an invite-list item from a paragraph's inline events.
///
/// Expected shape: `[Name](url) — gloss` where `—` is U+2014 (em-dash).
/// The inline events are the content between `Event::Start(Tag::Paragraph)`
/// and `Event::End(TagEnd::Paragraph)` (or `Start(Tag::Item)` / `End(TagEnd::Item)`
/// in list context). This only handles the simplest case: a single link whose
/// display text is the name, optionally followed by ` — <gloss>`.
///
/// Returns `None` when the events don't match the invite-list pattern, meaning
/// the caller should fall back to plain rendering.
fn parse_invite_item(events: &[Event<'_>]) -> Option<InviteItem> {
    // Expected event sequence for `[Name](url) — gloss`:
    // Start(Link { dest, .. }), Text("Name"), End(Link), Text(" \u{2014} gloss")
    // We scan for the Link events and the trailing text.
    let mut idx = 0;

    // Skip any leading SoftBreak/HardBreak
    while idx < events.len() && matches!(events[idx], Event::SoftBreak | Event::HardBreak) {
        idx += 1;
    }

    // Must start with a Link
    let (dest, _title) = match events.get(idx) {
        Some(Event::Start(Tag::Link { dest_url, title, .. })) => (dest_url.clone(), title.clone()),
        _ => return None,
    };
    idx += 1;

    // Collect link display text (name): expect one or more Text events
    let mut name_parts: Vec<&str> = Vec::new();
    while idx < events.len() {
        match &events[idx] {
            Event::Text(t) => {
                name_parts.push(t.as_ref());
                idx += 1;
            }
            Event::End(TagEnd::Link) => {
                idx += 1;
                break;
            }
            _ => {
                idx += 1;
            }
        }
    }
    let name: String = name_parts.join("");
    if name.is_empty() {
        return None;
    }

    // Skip trailing whitespace text
    while idx < events.len() && matches!(&events[idx], Event::Text(t) if t.trim().is_empty()) {
        idx += 1;
    }

    // Optional gloss: remaining events should be a Text(" — gloss") or empty
    let gloss = if idx < events.len() {
        match &events[idx] {
            Event::Text(t) => {
                let text = t.as_ref();
                // Must start with " — " (space + em-dash + space)
                let em_dash = "\u{2014}";
                if let Some(after_dash) = text
                    .strip_prefix(&format!(" {em_dash} "))
                    .or_else(|| text.strip_prefix(&format!("{em_dash} ")))
                    .or_else(|| text.strip_prefix(&format!(" {em_dash}")))
                {
                    Some(after_dash.to_owned())
                } else {
                    return None; // malformed trailing text, don't render as invite item
                }
            }
            Event::SoftBreak | Event::HardBreak => None,
            _ => return None,
        }
    } else {
        None
    };

    Some(InviteItem { href: dest.to_string(), name, gloss })
}

pub struct InviteItem {
    pub href: String,
    pub name: String,
    pub gloss: Option<String>,
}

// Event buffer helpers

/// Find the matching `End` event index for a `Start` event at `start_idx`.
/// Counts nesting levels so nested same-tag pairs are handled correctly.
///
/// `start_idx` must point at the `Event::Start(tag)` event. The return value
/// is the index of the corresponding `End` event (inclusive).
///
/// Returns `None` when the buffer is malformed (no matching End found).
pub fn find_end(events: &[Event<'_>], start_idx: usize) -> Option<usize> {
    let target_end = match &events[start_idx] {
        Event::Start(tag) => tag_end_for(tag),
        _ => return None,
    };

    let mut depth = 0usize;
    for (i, ev) in events[start_idx..].iter().enumerate() {
        match ev {
            Event::Start(t) if tag_end_for(t) == target_end => depth += 1,
            Event::End(e) if *e == target_end => {
                depth -= 1;
                if depth == 0 {
                    return Some(start_idx + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Map a `Tag` to its corresponding `TagEnd` for depth tracking.
///
/// Uses the `Tag::to_end()` method provided by pulldown-cmark so that
/// all variant details (tuple payloads like `List(bool)`) are handled
/// correctly without an exhaustive manual match.
fn tag_end_for(tag: &Tag<'_>) -> TagEnd {
    tag.clone().to_end()
}

// Section/invite-list detection

/// Check whether a list (identified by its item-inner events) qualifies as an
/// invite-list. Every item must parse successfully as an `InviteItem`.
///
/// `items_events` is a slice of events starting right after `Start(List)` and
/// ending right before `End(List)` (exclusive of those two events).
/// Try to parse all list items as `InviteItem`s. Returns `None` if any item
/// fails to parse (i.e. the list is not a clean invite-list).
pub fn parse_invite_items(items_events: &[Event<'_>]) -> Option<Vec<InviteItem>> {
    let mut items: Vec<InviteItem> = Vec::new();
    let mut idx = 0;
    while idx < items_events.len() {
        match &items_events[idx] {
            Event::Start(Tag::Item) => {
                let item_end = find_end(items_events, idx)?;
                // Inner events of this item: skip the Start(Paragraph) / End(Paragraph)
                // wrapper if present — pulldown-cmark wraps tight-list items in Paragraph.
                let item_inner = &items_events[idx + 1..item_end];
                let content_events = if matches!(item_inner.first(), Some(Event::Start(Tag::Paragraph))) {
                    // Tight list: content wrapped in Paragraph
                    let para_end = find_end(item_inner, 0)?;
                    &item_inner[1..para_end]
                } else {
                    item_inner
                };
                let item = parse_invite_item(content_events)?;
                items.push(item);
                idx = item_end + 1;
            }
            Event::SoftBreak | Event::HardBreak => {
                idx += 1;
            }
            _ => {
                idx += 1;
            }
        }
    }
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

/// Render events from `start` to `end` (inclusive) to an HTML string using the
/// standard `push_html` path (no editorial rewriting for sub-content).
pub fn events_to_html(events: &[Event<'_>]) -> String {
    let mut out = String::new();
    cmark_html::push_html(&mut out, events.iter().cloned());
    out
}

/// The one reading of "a top-level `##` starts here", shared by the renderer
/// (`component::editorial`'s `rewrite_events`) and the readers
/// ([`top_level_headings`], [`section_headings`]): for a `##` starting at
/// `idx` outside every container (`top` from [`top_level_flags`]), its
/// authored id, closing index, and whether an invite list follows it (then
/// it is an invite section, never a figure-mode part).
pub fn top_heading_at<'e>(events: &'e [Event<'_>], idx: usize, top: &[bool]) -> Option<(Option<&'e str>, usize, bool)> {
    let Event::Start(Tag::Heading { level: pulldown_cmark::HeadingLevel::H2, id, .. }) = &events[idx] else {
        return None;
    };
    if !top[idx] {
        return None;
    }
    let end = find_end(events, idx)?;
    Some((id.as_deref(), end, invite_list_follows(events, end)))
}

/// What follows a `##` heading, as far as the invite conventions go.
pub enum InviteSection {
    /// Neither shape: the heading is a plain heading.
    None,
    /// An optional intro paragraph, then a bullet list starting at
    /// `list_idx`. Whether that list is an invite list is the caller's
    /// check (`parse_invite_items`).
    List { intro_html: Option<String>, list_idx: usize },
    /// An optional intro paragraph, then one or more groups; `end_idx` is
    /// the last group's closing event.
    Groups {
        intro_html: Option<String>,
        groups: Vec<InviteGroup>,
        end_idx: usize,
    },
}

/// One group of a groups section: a `###`, an optional gloss paragraph and
/// an invite list.
pub struct InviteGroup {
    /// The heading's authored `{#id}`. A group heading never gets a generated
    /// one: it is not a section and takes no number.
    pub id: Option<String>,
    pub heading_html: String,
    pub gloss_html: Option<String>,
    pub items: Vec<InviteItem>,
}

/// The first index at or after `idx` that is not a soft or hard break.
fn skip_breaks(events: &[Event<'_>], mut idx: usize) -> usize {
    while idx < events.len() && matches!(events[idx], Event::SoftBreak | Event::HardBreak) {
        idx += 1;
    }
    idx
}

/// Read the groups that start at `idx`, which must open a `###`: each a
/// `###`, an optional paragraph and an invite list, back to back. `None`
/// unless every `###` in the run heads such a group, so one malformed group
/// leaves the whole section to the plain output.
fn parse_invite_groups(events: &[Event<'_>], mut idx: usize) -> Option<(Vec<InviteGroup>, usize)> {
    let mut groups = Vec::new();
    let mut end_idx = idx;
    while let Some(Event::Start(Tag::Heading { level: pulldown_cmark::HeadingLevel::H3, id, .. })) = events.get(idx) {
        let heading_end = find_end(events, idx)?;
        let heading_html = events_to_html(&events[idx + 1..heading_end]);

        let mut next = skip_breaks(events, heading_end + 1);
        let mut gloss_html = None;
        if matches!(events.get(next), Some(Event::Start(Tag::Paragraph))) {
            let para_end = find_end(events, next)?;
            gloss_html = Some(events_to_html(&events[next + 1..para_end]));
            next = skip_breaks(events, para_end + 1);
        }

        if !matches!(events.get(next), Some(Event::Start(Tag::List(None)))) {
            return None;
        }
        let list_end = find_end(events, next)?;
        let items = parse_invite_items(&events[next + 1..list_end])?;

        groups.push(InviteGroup {
            id: id.as_deref().map(str::to_owned),
            heading_html,
            gloss_html,
            items,
        });
        end_idx = list_end;
        idx = skip_breaks(events, list_end + 1);
    }
    (!groups.is_empty()).then_some((groups, end_idx))
}

/// Look ahead from `after_heading` for an invite section: an optional single
/// intro paragraph, then either a bullet list or a run of groups.
pub fn look_ahead_invite_section(events: &[Event<'_>], after_heading: usize) -> InviteSection {
    let mut idx = skip_breaks(events, after_heading);

    let mut intro_html: Option<String> = None;
    if matches!(events.get(idx), Some(Event::Start(Tag::Paragraph))) {
        let Some(para_end) = find_end(events, idx) else {
            return InviteSection::None;
        };
        intro_html = Some(events_to_html(&events[idx..=para_end]));
        idx = skip_breaks(events, para_end + 1);
    }

    match events.get(idx) {
        Some(Event::Start(Tag::List(None))) => InviteSection::List { intro_html, list_idx: idx },
        Some(Event::Start(Tag::Heading { .. })) => match parse_invite_groups(events, idx) {
            Some((groups, end_idx)) => InviteSection::Groups { intro_html, groups, end_idx },
            None => InviteSection::None,
        },
        _ => InviteSection::None,
    }
}

/// Whether each event of `events` sits at the document's top level (outside
/// every container). `component::editorial`'s `rewrite_events` consumes
/// block quotes, paragraphs and headings whole and steps through everything
/// else, so a top-level event is exactly one its main loop reaches at depth 0.
pub fn top_level_flags(events: &[Event<'_>]) -> Vec<bool> {
    let mut depth = 0usize;
    events
        .iter()
        .map(|event| {
            let top = depth == 0;
            match event {
                Event::Start(_) => depth += 1,
                Event::End(_) => depth = depth.saturating_sub(1),
                _ => {}
            }
            top
        })
        .collect()
}

/// Whether an invite list follows the heading that ends at `heading_end`: the
/// shape `component::editorial`'s `rewrite_events` turns into a
/// `section.editorial__section`.
fn invite_list_follows(events: &[Event<'_>], heading_end: usize) -> bool {
    match look_ahead_invite_section(events, heading_end + 1) {
        InviteSection::None => false,
        InviteSection::List { list_idx, .. } => find_end(events, list_idx)
            .is_some_and(|list_end| parse_invite_items(&events[list_idx + 1..list_end]).is_some()),
        InviteSection::Groups { .. } => true,
    }
}

/// A top-level `##` heading with an explicit id: a place a figure block can
/// name with `section="<id>"`.
#[derive(Debug, PartialEq)]
pub struct SectionHeading {
    pub id: String,
    /// The heading's plain text (its text and code spans, markup dropped):
    /// the dataset panel names a figure block's section with it.
    pub text: String,
    /// An invite list follows it, so it renders as an invite section, not a
    /// figure-mode part: a figure block cannot attach there.
    pub holds_invite_list: bool,
}

/// Every top-level id'd `##` of an editorial narrative, in document order —
/// the headings figure mode can split at (boot's `UnknownFigureSection` and
/// `FigureSectionHoldsInviteList` read it). Authored ids only: a generated
/// `sec-<n>` never makes a section figure-attachable.
pub fn section_headings(src: &str) -> Vec<SectionHeading> {
    top_level_headings(src)
        .into_iter()
        .filter_map(|heading| {
            Some(SectionHeading {
                id: heading.id?,
                text: heading.text,
                holds_invite_list: heading.holds_invite_list,
            })
        })
        .collect()
}

/// A top-level `##` heading of an editorial narrative.
#[derive(Debug, PartialEq)]
pub struct TopHeading {
    /// Its authored `{#id}`, when it has one.
    pub id: Option<String>,
    /// Its plain text (text and code spans, markup dropped, a line break
    /// read as a space).
    pub text: String,
    /// An invite list follows it.
    pub holds_invite_list: bool,
}

/// Every top-level `##` of an editorial narrative, in document order: the
/// headings the renderer numbers and gives an id. `render_body` in `dsp-incubator
/// cpe/web` (`page`) counts them to give each narrative of a page its id
/// offset, so this and `component::editorial`'s `rewrite_events` must agree on
/// which headings count ([`top_heading_at`]).
pub fn top_level_headings(src: &str) -> Vec<TopHeading> {
    let events = editorial_events(src);
    let top = top_level_flags(&events);
    (0..events.len())
        .filter_map(|idx| {
            let (id, end, holds_invite_list) = top_heading_at(&events, idx, &top)?;
            let text = events[idx + 1..end]
                .iter()
                .filter_map(|event| match event {
                    Event::Text(t) | Event::Code(t) => Some(t.as_ref()),
                    Event::SoftBreak | Event::HardBreak => Some(" "),
                    _ => None,
                })
                .collect();
            Some(TopHeading { id: id.map(str::to_owned), text, holds_invite_list })
        })
        .collect()
}

/// The buffered event stream every editorial pass reads: parsed with the
/// editorial options, author raw HTML filtered out (same as the richtext
/// pipeline). The renderer and [`section_headings`] must read the same
/// stream, or boot would accept a section the page never splits at.
///
/// A heading keeps only its `{#id}`: the attribute block's classes and
/// `key=value` pairs are dropped, since `push_html` would write them onto the
/// tag (`{onclick=…}`, a DataStar `data-*`) past the raw-HTML filter.
pub fn editorial_events(src: &str) -> Vec<Event<'_>> {
    Parser::new_ext(src, crate::util::EDITORIAL_MARKDOWN_OPTIONS)
        .filter(|event| !matches!(event, Event::Html(_) | Event::InlineHtml(_)))
        .map(|event| match event {
            Event::Start(Tag::Heading { level, id, .. }) => {
                Event::Start(Tag::Heading { level, id, classes: Vec::new(), attrs: Vec::new() })
            }
            other => other,
        })
        .collect()
}

/// Put the base in front of every link and image destination of `events`
/// that names a path of CPE ([`Base::content_link`]). Applied to the stream
/// before anything is rendered or parsed from it, so an invite item's href
/// and a link in prose get the same answer.
pub fn rebase_links(events: &mut [Event<'_>], base: &Base) {
    for event in events {
        let (Event::Start(Tag::Link { dest_url, .. }) | Event::Start(Tag::Image { dest_url, .. })) = event else {
            continue;
        };
        let rebased = match base.content_link(dest_url) {
            std::borrow::Cow::Owned(rebased) => Some(rebased),
            std::borrow::Cow::Borrowed(_) => None,
        };
        if let Some(rebased) = rebased {
            *dest_url = rebased.into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_headings_are_the_top_level_idd_h2s() {
        let src = "# Title\n\nIntro.\n\n## Four {#four}\n\nText.\n\n## Plain\n\n\
                   ### Deep {#deep}\n\n> ## Quoted {#quoted}\n\n- ## Listed {#listed}\n\n\
                   ## Explore {#explore}\n\n- [Books](/books) — all books\n";
        assert_eq!(
            section_headings(src),
            vec![
                SectionHeading {
                    id: "four".to_owned(),
                    text: "Four".to_owned(),
                    holds_invite_list: false,
                },
                SectionHeading {
                    id: "explore".to_owned(),
                    text: "Explore".to_owned(),
                    holds_invite_list: true,
                },
            ]
        );
    }

    #[test]
    fn a_section_headings_text_drops_its_markup() {
        let src = "# T\n\n## Viermal *gedruckt* `x` {#four}\n\nText.\n";
        let texts: Vec<String> = section_headings(src).into_iter().map(|h| h.text).collect();
        assert_eq!(texts, ["Viermal gedruckt x"]);
    }
}
