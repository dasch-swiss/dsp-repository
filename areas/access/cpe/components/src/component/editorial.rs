//! Editorial home transformer: Markdown source → editorial `Markup`.
//!
//! Converts Markdown to the design's editorial article archetype
//! (`article.editorial.page`). Used exclusively by [`render_narrative`] for
//! the project home page; the richtext path (`render_richtext_fragment`) is
//! completely untouched.
//!
//! ## Pipeline
//!
//! 1. `Parser::new_ext` — parse CommonMark with tables and heading attributes (`## Title {#id}`)
//!    enabled.
//! 2. Filter `Event::Html` / `Event::InlineHtml` — strip all author raw HTML (same as the shared
//!    `markdown_to_html` pipeline).
//! 3. Collect into `Vec<Event>` — buffer the event stream so look-ahead and multi-event replacement
//!    can work without unbalancing `Start`/`End` pairs.
//! 4. Stateful rewrite over the Vec — recognise editorial conventions and replace groups of events
//!    with trusted `Event::Html` wrappers. Each convention has a **plain-prose fallback** when
//!    unmatched.
//! 5. `push_html` — emit the final HTML string, returned as `PreEscaped`.
//!
//! ## Conventions recognised
//!
//! - **Leading kicker (eyebrow)**: a level-3 heading that is the document's FIRST block and is
//!   immediately followed by the `# ` h1 → `<p class="eyebrow editorial__kicker">`, which
//!   `segment_html` then lands inside `header.editorial__head` above the `<h1>`. It is a *label*,
//!   not a section heading (emitting an `<h3>` there also broke heading order), so it is likewise
//!   excluded from page-title/breadcrumb derivation — see `util::page_title_heading`. Any other
//!   `###` (mid-document, or not followed by the h1) renders as a plain `<h3>`, unless it heads an
//!   invite group (below).
//! - **Subtitle**: a level-3 heading immediately after the `# ` h1 → `<p
//!   class="editorial__subtitle">`, which `segment_html` keeps inside `header.editorial__head`
//!   between the `</h1>` and the lede. Like the kicker it is a *label* (no `<h3>`, heading order
//!   stays valid) and touches neither the lede nor the drop-cap slot; it is not part of the h1
//!   text, so page-title derivation never sees it.
//! - **Lede**: an `Emphasis`-only paragraph → `<p class="editorial__lede">`.
//! - **Drop-cap open**: the first ordinary body paragraph → `<p class="editorial__open">`,
//!   suppressed only in the "h1-less document with a real section" shape (an `## h2` followed by an
//!   invite list, with no `# ` h1 — the secondary panel sharing a page with an h1 document, one
//!   drop cap per page); an h1-less document with no such section keeps its drop cap. The first
//!   letter is extracted from any leading inline tag (handles non-ASCII initials per render.js
//!   `dropcapReady`).
//! - **Epigraph**: a `blockquote` → `<blockquote class="epigraph">`, with a trailing ` —
//!   attribution` (space-padded em-dash U+2014) parsed into `<cite>`.
//! - **Invite section**: a `## heading` followed by an optional intro paragraph and a bullet list
//!   of `[Name](url) — gloss` items → `section.editorial__section > h2 + (p) + ul.invite-list`.
//!   Each item link is built with Maud (escapes name/gloss/href) with an href scheme allowlist;
//!   items that fail the scheme check fall back to plain text.
//! - **Invite groups**: a `## heading`, an optional intro paragraph, then one or more groups, each
//!   of them a `### heading`, an optional gloss paragraph and an invite list →
//!   `section.editorial__section > h2 + (p) + div.invite-groups`, which holds one
//!   `section.invite-group > h3 + (p.invite-group__gloss) + ul.invite-list` per group. One `###` in
//!   the run that does not head such a group leaves the whole section to the plain output.
//! - **Heading ids**: every top-level `## heading` renders an id, its authored `{#id}` or a
//!   generated `sec-<n>` (see `rewrite_events`). No other heading gets a generated id. A plain
//!   heading and a group's `###` keep an authored one; three shapes drop it, because they do not
//!   render the heading's own tag: the kicker and the subtitle (each a `<p>`), and a `##` nested in
//!   a list item that heads an invite section.

use cpe_core::markup_contract::RAIL_CAPTION_ID;
use cpe_core::narrative_outline::{
    editorial_events, events_to_html, find_end, look_ahead_invite_section, parse_invite_items, top_heading_at,
    top_level_flags, top_level_headings, InviteGroup, InviteItem, InviteSection,
};
use maud::{html, Markup, PreEscaped, Render};
use pulldown_cmark::{Event, Tag};

// ── href allowlist ─────────────────────────────────────────────────────────────

/// Return `true` iff the href is allowed to be emitted in an editorial link.
///
/// Allowed schemes: `https://`, `http://`, and relative paths starting with `/`.
/// Anything else (e.g. `javascript:`, `data:`, `mailto:`) is rejected.
fn href_is_allowed(href: &str) -> bool {
    href.starts_with("https://") || href.starts_with("http://") || href.starts_with('/')
}

// ── Drop-cap extraction ────────────────────────────────────────────────────────

/// Simulate render.js `dropcapReady`: pull the first letter out of a leading
/// inline tag (`<strong>`, `<em>`, `<b>`, `<i>`) so that `::first-letter`
/// can target it for the drop-cap effect.
///
/// `html_fragment` is the raw HTML of the paragraph's content (not including
/// the `<p>` wrapper). Returns the modified HTML fragment ready to be wrapped
/// in `<p class="editorial__open">`.
///
/// Handles non-ASCII initials in the range `[A-Za-zÀ-ÿ]` (German openings etc.).
fn dropcap_ready(html_fragment: &str) -> String {
    // Match: optional whitespace + "<tag>" + first letter
    let tag_re: &[&str] = &["strong", "em", "b", "i"];

    for tag in tag_re {
        let open = format!("<{tag}>");
        // Find the first non-whitespace occurrence of the tag at the start
        let trimmed = html_fragment.trim_start();
        if let Some(after_tag) = trimmed.strip_prefix(open.as_str()) {
            // Check if the first char of after_tag is a letter [A-Za-zÀ-ÿ]
            let mut chars = after_tag.chars();
            if let Some(first_char) = chars.next() {
                if is_dropcap_letter(first_char) {
                    let rest = &after_tag[first_char.len_utf8()..];
                    let close = format!("</{tag}>");
                    // The extracted letter was the WHOLE inline run — the
                    // common `**B**asel` hand-authored drop cap. Re-emitting
                    // the tag pair here would leave an empty
                    // `<strong></strong>` in the first body paragraph of every
                    // such page (no visual effect, but real DOM litter). Drop
                    // the pair instead of reconstructing it.
                    if let Some(after_close) = rest.strip_prefix(close.as_str()) {
                        return format!("{first_char}{after_close}");
                    }
                    // Reconstruct: letter + <tag> + rest
                    return format!("{first_char}<{tag}>{rest}");
                }
            }
        }
    }

    // No leading inline tag — check if the raw content itself starts with a letter
    // (in which case ::first-letter already works; no transformation needed)
    html_fragment.to_owned()
}

/// Return `true` for letters that qualify as drop-cap initials.
fn is_dropcap_letter(c: char) -> bool {
    c.is_ascii_alphabetic() || ('\u{00C0}'..='\u{00FF}').contains(&c)
}

// ── Lede detection ─────────────────────────────────────────────────────────────

/// Return `true` iff the events between `Start(Paragraph)` and `End(Paragraph)`
/// (exclusive) consist of exactly one `Emphasis` span (i.e. `Start(Emphasis)`,
/// some `Text` events, `End(Emphasis)` — with nothing else at the top level).
///
/// `inner` is the slice of events between the paragraph start and end (exclusive).
fn is_lede_paragraph(inner: &[Event<'_>]) -> bool {
    // Must be non-empty
    if inner.is_empty() {
        return false;
    }
    // First event: Start(Emphasis)
    if !matches!(inner[0], Event::Start(Tag::Emphasis)) {
        return false;
    }
    // Find the matching End(Emphasis)
    let end_idx = match find_end(inner, 0) {
        Some(i) => i,
        None => return false,
    };
    // Nothing after the End(Emphasis)
    if end_idx != inner.len() - 1 {
        return false;
    }
    true
}

// ── Invite item Maud rendering ─────────────────────────────────────────────────

/// `ul.invite-list`: the one emission site of an invite list, for a
/// section's own list and for each group's.
fn render_invite_list(items: &[InviteItem]) -> Markup {
    html! {
        ul.invite-list {
            @for item in items { (render_invite_item(item)) }
        }
    }
}

/// `div.invite-groups`: a groups section's body. A group heading carries its
/// authored id only; the gloss paragraph is omitted when the group has none.
fn render_invite_groups(groups: &[InviteGroup]) -> Markup {
    html! {
        div.invite-groups {
            @for group in groups {
                section.invite-group {
                    h3 id=[group.id.as_deref()] { (PreEscaped(&group.heading_html)) }
                    @if let Some(gloss) = &group.gloss_html {
                        p.invite-group__gloss { (PreEscaped(gloss)) }
                    }
                    (render_invite_list(&group.items))
                }
            }
        }
    }
}

/// Render a single invite-list item as `<li><a …>…</a></li>`.
///
/// The `<a>` is built with Maud to escape name/gloss/href. If the href fails
/// the scheme allowlist, the item is rendered as plain text (no link).
fn render_invite_item(item: &InviteItem) -> Markup {
    if href_is_allowed(&item.href) {
        html! {
            li {
                a href=(item.href) {
                    span.invite__name { (item.name) }
                    @if let Some(gloss) = &item.gloss {
                        span.invite__gloss { (gloss) }
                    }
                    span.invite__arrow { "→" }
                }
            }
        }
    } else {
        // Fallback: plain text (no link)
        html! {
            li {
                (item.name)
                @if let Some(gloss) = &item.gloss { " — " (gloss) }
            }
        }
    }
}

// ── Leading-kicker detection ───────────────────────────────────────────────────

/// Recognise a **leading kicker** (the design's eyebrow) at `idx` and return
/// the index of its closing `End` event, or `None` when this level-3 heading
/// is an ordinary `<h3>`.
///
/// Two conditions, both required:
///
/// 1. **It is the document's first block.** Tested against `emitted` — the HTML produced so far —
///    rather than against `idx == 0`, because events that render to nothing may legitimately sit
///    ahead of it: the author raw-HTML filter in [`render_editorial`] runs BEFORE `rewrite_events`,
///    so a leading `<!-- … -->` comment has had its `Event::Html` payload removed but still leaves
///    its `Start`/`End(HtmlBlock)` markers in the stream, and `SoftBreak`s render as a bare
///    newline. Checking "nothing visible has been emitted yet" tolerates all of those without
///    enumerating them.
/// 2. **The h1 comes next.** Skipping only soft/hard breaks, the next event must open a level-1
///    heading. A `###` followed by prose, a list, or a lower-level heading is a real section
///    heading, not a label.
fn leading_kicker_end(events: &[Event<'_>], idx: usize, emitted: &str) -> Option<usize> {
    if !emitted.trim().is_empty() {
        return None;
    }
    let end_idx = find_end(events, idx)?;

    let mut next = end_idx + 1;
    while next < events.len() && matches!(events[next], Event::SoftBreak | Event::HardBreak) {
        next += 1;
    }
    matches!(
        events.get(next),
        Some(Event::Start(Tag::Heading { level: pulldown_cmark::HeadingLevel::H1, .. }))
    )
    .then_some(end_idx)
}

/// The subtitle's opening tag: emitted by `rewrite_events`, recognised by
/// `segment_html` to keep it inside the head.
const SUBTITLE_OPEN: &str = r#"<p class="editorial__subtitle">"#;

/// Whether a level-3 heading at this point is the **subtitle**: the
/// document's FIRST `</h1>` is the last thing emitted (trailing whitespace
/// tolerated). Only the first h1 counts, because `segment_html` only extends
/// the head past the first `</h1>`.
fn is_subtitle_position(emitted: &str) -> bool {
    emitted.trim_end().ends_with("</h1>") && emitted.matches("</h1>").count() == 1
}

// ── Main rewrite pass ──────────────────────────────────────────────────────────

/// Apply the editorial rewrite to a buffered event slice.
///
/// Recognises each editorial convention and replaces matched groups with
/// equivalent `Event::Html` events. Unmatched groups fall through to plain
/// `push_html` rendering.
///
/// `suppress_drop_cap` seeds the drop-cap slot as already consumed — the
/// caller ([`render_editorial`]) sets this on a second pass when the FIRST
/// pass (always run with `false`) proves, from its own output, that this is
/// an h1-less document with a real section: the drop-cap opens the
/// document's HEAD, so a document without an `# ` h1 that ALSO has a real
/// section (an `## h2` followed by an invite list — the shape `segment_html`
/// renders as div.prose.editorial__intro before div.editorial__sections,
/// e.g. the Incunabula `home-offices.*.md`) gets none: its slot starts
/// consumed, and a page never shows two drop caps (its own plus the h1
/// document it shares the page with). An h1-less document with NEITHER an
/// h1 nor a section (the `div.prose` fallback — e.g. a headless `## heading`
/// narrative with no following invite list, or a document with no heading
/// at all) is unaffected by that rule and keeps its drop cap. Deriving BOTH
/// halves of the gate — "no h1" AND "a real section" — from
/// `segment_html`'s OWN read of `rewrite_events`'s first-pass output (rather
/// than a second, independently-scanning `has_h1` predicate) makes
/// disagreement between the two impossible by construction — see
/// `render_editorial`.
///
/// Returns a string of HTML (no outer wrapper — that's applied in
/// [`render_editorial`]).
///
/// `figures` non-empty switches on **figure mode**: every top-level id'd
/// `##` that no invite list follows (exactly
/// [`cpe_core::narrative_outline::section_headings`]' reading) opens a
/// `section.editorial__part` inside one `div.editorial__parts`, and
/// each part closes with the blocks named for its id. Empty ⇒ the output is
/// the plain editorial rewrite, byte for byte.
///
/// **Heading ids.** Every top-level `##` renders an id: its authored
/// `{#id}`, else `sec-<n>`, where n is `id_offset` plus the heading's 1-based
/// position among this document's top-level `##` headings ([`top_heading_at`]
/// is the one reading of "top-level `##`", shared with
/// [`top_level_headings`], which the caller counts the offset with). An
/// authored id still takes its number out of use. A `##` nested in a list
/// item gets neither a number nor a generated id.
fn rewrite_events(events: &[Event<'_>], suppress_drop_cap: bool, figures: &[FigureBlock], id_offset: usize) -> String {
    // We accumulate output HTML by dispatching groups of events.
    let mut out = String::new();
    let mut idx = 0;

    // Track state for drop-cap (only the FIRST body paragraph gets it).
    let mut drop_cap_done = suppress_drop_cap;
    // Track whether we've emitted a lede yet (only the first candidate matters)
    let mut lede_done = false;

    // Figure mode: the id of the part currently open, if any.
    let figure_mode = !figures.is_empty();
    let top = top_level_flags(events);
    let mut open_part: Option<&str> = None;

    // The number of the top-level `##` last reached, page-wide.
    let mut heading_number = id_offset;

    while idx < events.len() {
        let heading = top_heading_at(events, idx, &top);
        let generated_id = heading.map(|_| {
            heading_number += 1;
            generated_heading_id(heading_number)
        });
        // The id this `##` renders: `None` for a nested one.
        let heading_id: Option<&str> = heading.and_then(|(authored, _, _)| authored.or(generated_id.as_deref()));

        // ── Heading (## h2, id'd, top level) — a figure-mode part ──────────
        let part = match heading {
            Some((Some(id), end_idx, false)) if figure_mode => Some((id, end_idx)),
            _ => None,
        };
        if let Some((id, end_idx)) = part {
            match open_part.take() {
                Some(open) => close_part(&mut out, open, figures),
                None => {
                    // The lede and the drop cap belong to the intro: a part
                    // never takes them, even when the intro has no prose.
                    lede_done = true;
                    drop_cap_done = true;
                    out.push_str(PARTS_MARKER);
                }
            }
            let labelledby = html! {
                (id)
            }
            .into_string();
            out.push_str(&format!(
                r#"<section class="editorial__part" aria-labelledby="{labelledby}"><div class="prose editorial__part-prose">"#
            ));
            out.push_str(&events_to_html(&events[idx..=end_idx]));
            open_part = Some(id);
            idx = end_idx + 1;
            continue;
        }

        match &events[idx] {
            // ── Heading (### h3) — leading kicker / eyebrow ────────────────
            Event::Start(Tag::Heading { level, .. }) if *level == pulldown_cmark::HeadingLevel::H3 => {
                match leading_kicker_end(events, idx, &out) {
                    Some(end_idx) => {
                        // A label, not a heading: emit `p.eyebrow.editorial__kicker`.
                        // Inner content goes through the same `events_to_html`
                        // path the h2 branch uses, so inline markup/escaping
                        // behaviour is unchanged. Deliberately touches NEITHER
                        // `lede_done` NOR `drop_cap_done` — the kicker is not a
                        // paragraph and must not consume either slot.
                        let inner_html = events_to_html(&events[idx + 1..end_idx]);
                        out.push_str(&format!(r#"<p class="eyebrow editorial__kicker">{inner_html}</p>"#));
                        idx = end_idx + 1;
                    }
                    None if is_subtitle_position(&out) => {
                        // Subtitle label directly after the h1. Like the
                        // kicker, touches neither `lede_done` nor
                        // `drop_cap_done`.
                        let Some(end_idx) = find_end(events, idx) else {
                            let raw = events_to_html(&events[idx..idx + 1]);
                            out.push_str(&raw);
                            idx += 1;
                            continue;
                        };
                        let inner_html = events_to_html(&events[idx + 1..end_idx]);
                        out.push_str(&format!("{SUBTITLE_OPEN}{inner_html}</p>"));
                        idx = end_idx + 1;
                    }
                    None => {
                        // Ordinary `<h3>` — byte-identical to the catch-all arm
                        // below (emit this single event, advance by one).
                        let raw = events_to_html(&events[idx..idx + 1]);
                        out.push_str(&raw);
                        idx += 1;
                    }
                }
            }

            // ── Heading (## h2) — invite-section header ────────────────────
            Event::Start(Tag::Heading { level, .. }) if *level == pulldown_cmark::HeadingLevel::H2 => {
                let end_idx = match find_end(events, idx) {
                    Some(i) => i,
                    None => {
                        // Malformed: emit as-is
                        let raw = events_to_html(&events[idx..idx + 1]);
                        out.push_str(&raw);
                        idx += 1;
                        continue;
                    }
                };

                let inner_events = &events[idx + 1..end_idx];
                let heading_html = events_to_html(inner_events);

                match heading_outcome(events, end_idx) {
                    HeadingOutcome::Section { intro_html, body, end } => {
                        let markup = html! {
                            section.editorial__section {
                                h2 id=[heading_id] { (PreEscaped(&heading_html)) }
                                @if let Some(intro) = &intro_html { (PreEscaped(intro)) }
                                (body)
                            }
                        };
                        out.push_str(&markup.into_string());
                        idx = end + 1;
                    }
                    HeadingOutcome::Plain { end } => {
                        let raw = heading_slice_html(&events[idx..=end], heading_id);
                        out.push_str(&raw);
                        idx = end + 1;
                    }
                }
            }

            // ── Paragraph ─────────────────────────────────────────────────
            Event::Start(Tag::Paragraph) => {
                let end_idx = match find_end(events, idx) {
                    Some(i) => i,
                    None => {
                        let raw = events_to_html(&events[idx..idx + 1]);
                        out.push_str(&raw);
                        idx += 1;
                        continue;
                    }
                };

                let inner_events = &events[idx + 1..end_idx];

                // Check for lede (emphasis-only paragraph, before any body paragraph)
                if !lede_done && is_lede_paragraph(inner_events) {
                    lede_done = true;
                    let inner_html = events_to_html(inner_events);
                    out.push_str(&format!(r#"<p class="editorial__lede">{inner_html}</p>"#));
                    idx = end_idx + 1;
                    continue;
                }

                // First ordinary body paragraph → drop-cap open
                if !drop_cap_done {
                    drop_cap_done = true;
                    let inner_html = events_to_html(inner_events);
                    let ready = dropcap_ready(&inner_html);
                    out.push_str(&format!(r#"<p class="editorial__open">{ready}</p>"#));
                    idx = end_idx + 1;
                    continue;
                }

                // Regular paragraph
                let raw = events_to_html(&events[idx..=end_idx]);
                out.push_str(&raw);
                idx = end_idx + 1;
            }

            // ── BlockQuote → epigraph ──────────────────────────────────────
            Event::Start(Tag::BlockQuote(_)) => {
                let end_idx = match find_end(events, idx) {
                    Some(i) => i,
                    None => {
                        let raw = events_to_html(&events[idx..idx + 1]);
                        out.push_str(&raw);
                        idx += 1;
                        continue;
                    }
                };

                let inner_events = &events[idx + 1..end_idx];
                // Extract the text content of the blockquote for attribution splitting
                let inner_html = events_to_html(inner_events);

                // Try to split attribution from the raw paragraph text
                // The blockquote inner will contain <p>…text…</p> — we need
                // to pull the text out of the last <p> to find the attribution.
                let (body_html, cite_html) = split_epigraph_html(&inner_html);

                out.push_str(r#"<blockquote class="epigraph">"#);
                // body_html comes from push_html (pulldown-cmark's output) which
                // already renders escaped HTML — safe to emit as PreEscaped.
                out.push_str(&body_html);
                if let Some(attr) = cite_html {
                    // `attr` was sliced from push_html output, so it is ALREADY
                    // HTML-escaped (& → &amp; etc.), and split_epigraph_html
                    // guarantees it contains no `<`. Emit it as PreEscaped inside
                    // a <cite> — escaping it again (plain Maud `(attr)`) would
                    // double-encode (& → &amp;amp;).
                    let cite_markup = html! {
                        cite { (maud::PreEscaped(attr)) }
                    };
                    out.push_str(&cite_markup.into_string());
                }
                out.push_str("</blockquote>");
                idx = end_idx + 1;
            }

            // All other events — pass through unchanged
            _ => {
                // Emit this single event as HTML
                let raw = events_to_html(&events[idx..idx + 1]);
                out.push_str(&raw);
                idx += 1;
            }
        }
    }

    if let Some(open) = open_part {
        close_part(&mut out, open, figures);
        out.push_str("</div>");
    }

    out
}

/// The opening tag of figure mode's parts container: emitted by
/// `rewrite_events` before the first part, recognised by `segment_html`.
const PARTS_MARKER: &str = r#"<div class="editorial__parts">"#;

/// Render a slice that opens with a heading, writing `id` onto that heading
/// when it carries none of its own. The id is set on the heading's event, so
/// pulldown writes and escapes the attribute as it does an authored one.
fn heading_slice_html(slice: &[Event<'_>], id: Option<&str>) -> String {
    let (Some(Event::Start(Tag::Heading { level, id: None, classes, attrs })), Some(id)) = (slice.first(), id) else {
        return events_to_html(slice);
    };
    let mut owned = slice.to_vec();
    owned[0] = Event::Start(Tag::Heading {
        level: *level,
        id: Some(id.to_owned().into()),
        classes: classes.clone(),
        attrs: attrs.clone(),
    });
    events_to_html(&owned)
}

/// Close the part `id`: its prose `div`, then every block named for `id`, in
/// the order given, then the `section`.
fn close_part(out: &mut String, id: &str, figures: &[FigureBlock]) {
    out.push_str("</div>");
    for figure in figures.iter().filter(|f| f.section == id) {
        out.push_str(&render_figure(figure).into_string());
    }
    out.push_str("</section>");
}

/// One figure block in its slot: the block at full width, then its optional
/// count line and "more" link.
fn render_figure(figure: &FigureBlock) -> Markup {
    html! {
        div.editorial__figure {
            (figure.block)
            @if let Some(count) = &figure.count {
                p.editorial__figure-count { (count) }
            }
            @if let Some(more) = &figure.more {
                p.editorial__figure-more {
                    a href=(more.href) {
                        (more.label)
                        " "
                        span.editorial__figure-arrow aria-hidden="true" { "→" }
                    }
                }
            }
        }
    }
}

/// How the `##` whose heading ends at `heading_end` renders; `end` is the
/// last event the rendering covers.
enum HeadingOutcome {
    /// An invite section: the heading, the intro and `body` (the invite list
    /// or the groups) inside one `section.editorial__section`.
    Section { intro_html: Option<String>, body: Markup, end: usize },
    /// A plain heading, rendered as one slice with what it covers: itself,
    /// or itself, an intro and a list that is not an invite list.
    Plain { end: usize },
}

fn heading_outcome(events: &[Event<'_>], heading_end: usize) -> HeadingOutcome {
    match look_ahead_invite_section(events, heading_end + 1) {
        InviteSection::Groups { intro_html, groups, end_idx } => HeadingOutcome::Section {
            intro_html,
            body: render_invite_groups(&groups),
            end: end_idx,
        },
        InviteSection::List { intro_html, list_idx } => {
            let Some(list_end) = find_end(events, list_idx) else {
                return HeadingOutcome::Plain { end: heading_end };
            };
            match parse_invite_items(&events[list_idx + 1..list_end]) {
                Some(items) => HeadingOutcome::Section { intro_html, body: render_invite_list(&items), end: list_end },
                None => HeadingOutcome::Plain { end: list_end },
            }
        }
        InviteSection::None => HeadingOutcome::Plain { end: heading_end },
    }
}

/// Extract the body HTML and optional citation from a blockquote's inner HTML.
///
/// The inner HTML will be something like `<p>"text" — Attribution</p>`. We
/// split on the last ` — ` (space + U+2014 + space) within the final `<p>` to
/// produce `(body_html, Some(attribution))` or `(body_html, None)`.
fn split_epigraph_html(inner_html: &str) -> (String, Option<String>) {
    let em_dash = "\u{2014}";
    let sep = format!(" {em_dash} ");

    // Find the last occurrence of the separator in the raw HTML
    if let Some(pos) = inner_html.rfind(&sep) {
        // Check there's no HTML tag between pos and the end (to avoid splitting
        // inside a tag attribute). Simple heuristic: no '<' in the suffix.
        let suffix = &inner_html[pos + sep.len()..];
        // Strip trailing </p> and whitespace from the suffix
        let attr_raw = suffix.trim_end().trim_end_matches("</p>").trim_end_matches("</P>").trim();

        if !attr_raw.is_empty() && !attr_raw.contains('<') {
            // Reconstruct body: everything before pos, close the paragraph tag
            let body_before_sep = &inner_html[..pos];
            let body_html = format!("{body_before_sep}</p>");
            return (body_html, Some(attr_raw.to_owned()));
        }
    }

    // No attribution found
    (inner_html.to_owned(), None)
}

// ── Figure sections ────────────────────────────────────────────────────────────

/// A rendered figure block for one narrative section: what `render_body`
/// hands the narrative for a component with a figure-block slot, with the
/// slot's chrome strings already resolved.
pub struct FigureBlock {
    /// The `##` heading id the block follows.
    pub section: String,
    pub block: Markup,
    /// The count line under the block.
    pub count: Option<String>,
    /// The "more" link under the block.
    pub more: Option<FigureLink>,
}

/// A figure block's resolved "more" link.
pub struct FigureLink {
    pub label: String,
    pub href: String,
}

/// The id of a top-level `##` whose author wrote none; `number` is its
/// 1-based position among the page's top-level `##` headings.
fn generated_heading_id(number: usize) -> String {
    format!("sec-{number}")
}

// ── The page rail ─────────────────────────────────────────────────────────────

/// One entry of the page rail: a top-level `##` and the id it renders.
#[derive(Debug, PartialEq)]
pub struct RailEntry {
    pub id: String,
    /// The heading's plain text: the link's name.
    pub text: String,
}

impl RailEntry {
    /// The entry has text a visitor can see or hear. A heading made only of
    /// an image without alt text, of white space or of zero-width characters
    /// has none, and its link would have no name.
    pub fn has_name(&self) -> bool {
        self.text
            .chars()
            .any(|c| !c.is_whitespace() && !matches!(c, '\u{200B}'..='\u{200D}' | '\u{2060}' | '\u{FEFF}'))
    }
}

/// One rail entry per top-level `##` of a narrative, in document order, each
/// with the id [`render_editorial`] gives that heading when it is called with
/// the same `id_offset`.
pub fn rail_entries(src: &str, id_offset: usize) -> Vec<RailEntry> {
    top_level_headings(src)
        .into_iter()
        .enumerate()
        .map(|(position, heading)| RailEntry {
            id: heading.id.unwrap_or_else(|| generated_heading_id(id_offset + position + 1)),
            text: heading.text,
        })
        .collect()
}

/// The page rail: a `nav` listing a narrative page's sections, named by its
/// own visible caption. One per page, in the page's first narrative.
pub struct Rail<'a> {
    pub entries: &'a [RailEntry],
    /// The caption (`ChromeMsg::NarrativeRailLabel`).
    pub label: &'a str,
}

impl Render for Rail<'_> {
    fn render(&self) -> Markup {
        html! {
            nav.narr__side aria-labelledby=(RAIL_CAPTION_ID) {
                p.eyebrow.toc__cap id=(RAIL_CAPTION_ID) { (self.label) }
                ul.toc {
                    @for entry in self.entries {
                        li {
                            a href={ "#" (entry.id) } { (entry.text) }
                        }
                    }
                }
            }
        }
    }
}

/// What [`render_editorial`] returns.
pub struct Editorial {
    pub markup: Markup,
    /// The rail it was given is in `markup`. `false` when it was given none,
    /// or when the document renders neither a head nor an intro for the rail
    /// to stand beside.
    pub rail_placed: bool,
}

// ── Segment detection for article wrapper ─────────────────────────────────────

/// Describes the high-level segments found in an editorial document.
///
/// Used by `render_editorial` to decide which panels
/// (`header.editorial__head`, `div.prose.editorial__intro`, and either
/// `div.editorial__sections` or figure mode's `div.editorial__parts`) to
/// emit, or to fall back to a single `div.prose` wrapper (`segment_html`
/// returns `None`). Each panel is `Some` only when the document has that
/// segment; see `segment_html` for the five document shapes.
#[derive(Debug, PartialEq)]
struct EditorialSegments {
    /// Everything up to and including the `</h1>` (kicker + h1), plus the
    /// subtitle `p` when one directly follows it. `None` for
    /// a document with sections but no h1 before them.
    head_html: Option<String>,
    /// Lede + drop-cap open + body paragraphs + epigraph — everything between
    /// the head and the first section. `None` when that block is empty in a
    /// no-h1 or h1-only document; always `Some` in an h1 + sections document
    /// (unchanged from the original both-or-fallback shape).
    intro_html: Option<String>,
    /// All `editorial__section` blocks. `None` for an h1-only document.
    sections_html: Option<String>,
    /// Figure mode's `div.editorial__parts`, wrapper included. `Some` only
    /// in the parts shape, where `sections_html` is always `None`.
    parts_html: Option<String>,
}

/// The first-section marker `rewrite_events` emits for an `## h2` followed
/// by an invite list.
const SECTION_MARKER: &str = r#"<section class="editorial__section">"#;

/// Attempt to segment the rewritten HTML into head / intro / sections.
///
/// Splits on the first `<section class="editorial__section">` and on the
/// first `</h1>` before it. Four shapes (decision 2, option i, Incunabula
/// Slice 4):
///
/// - **h1 + sections** ⇒ head + intro + sections (intro always emitted).
/// - **h1, no section** ⇒ head + intro (intro omitted when empty); no sections panel.
/// - **sections, no h1** ⇒ the leading block before the first section as the intro (omitted when
///   empty), then sections; no head.
/// - **neither** ⇒ `None` — the `div.prose` fallback.
///
/// A fifth shape is checked first, by its own marker: **parts** (figure
/// mode) ⇒ head (when there is an h1) + intro (omitted when empty) + the
/// parts container. An invite section before or inside the parts stays where
/// it is, so the four shapes above never see figure-mode output.
fn segment_html(full_html: &str) -> Option<EditorialSegments> {
    // End of the head: just past `</h1>`, extended over a directly following
    // `p.editorial__subtitle` so the subtitle never lands in the intro.
    let head_end = |before: &str, h1_end: usize| -> usize {
        let after_h1 = h1_end + "</h1>".len();
        let rest = &before[after_h1..];
        let trimmed = rest.trim_start();
        if trimmed.starts_with(SUBTITLE_OPEN) {
            if let Some(close) = trimmed.find("</p>") {
                return after_h1 + (rest.len() - trimmed.len()) + close + "</p>".len();
            }
        }
        after_h1
    };

    let non_empty = |s: &str| (!s.trim().is_empty()).then(|| s.to_owned());

    if let Some(start) = full_html.find(PARTS_MARKER) {
        let before = &full_html[..start];
        let head_split = before.find("</h1>").map(|h1_end| head_end(before, h1_end));
        let (head_html, intro) = match head_split {
            Some(end) => (Some(before[..end].to_owned()), &before[end..]),
            None => (None, before),
        };
        return Some(EditorialSegments {
            head_html,
            intro_html: non_empty(intro),
            sections_html: None,
            parts_html: Some(full_html[start..].to_owned()),
        });
    }

    let (before_sections, sections) = match full_html.find(SECTION_MARKER) {
        Some(start) => (&full_html[..start], Some(&full_html[start..])),
        None => (full_html, None),
    };

    let head_end = before_sections.find("</h1>").map(|h1_end| head_end(before_sections, h1_end));
    match (head_end, sections) {
        // Both: unchanged — intro is emitted even when empty.
        (Some(head_end), Some(sections)) => Some(EditorialSegments {
            head_html: Some(before_sections[..head_end].to_owned()),
            intro_html: Some(before_sections[head_end..].to_owned()),
            sections_html: Some(sections.to_owned()),
            parts_html: None,
        }),
        // h1 only: head + intro, no sections panel.
        (Some(head_end), None) => Some(EditorialSegments {
            head_html: Some(before_sections[..head_end].to_owned()),
            intro_html: non_empty(&before_sections[head_end..]),
            sections_html: None,
            parts_html: None,
        }),
        // Sections, no h1: the leading block is the intro, no head.
        (None, Some(sections)) => Some(EditorialSegments {
            head_html: None,
            intro_html: non_empty(before_sections),
            sections_html: Some(sections.to_owned()),
            parts_html: None,
        }),
        // Neither: fall back to div.prose.
        (None, None) => None,
    }
}

// ── Public entry point ─────────────────────────────────────────────────────────

/// Transform Markdown source into the editorial article `Markup`.
///
/// Pipeline: parse → filter author raw HTML → buffer → stateful rewrite →
/// segment into head/intro/sections (or parts) → wrap in
/// `article.editorial.page`, the head and intro in `div.narr`, the sections
/// or parts in `div.narr.narr-wide`.
///
/// Emits whichever panels `segment_html` found; falls back to
/// `article.editorial.page > div.narr > div.prose` when the document has
/// neither an h1 nor a section or parts marker.
///
/// The drop-cap gate cannot be decided ahead of the rewrite — whether a
/// nested construct (e.g. an `## h2` + invite list inside a blockquote,
/// which the `BlockQuote` arm renders opaquely and never turns into a real
/// section) yields a `SECTION_MARKER` is exactly what `rewrite_events`
/// itself decides. So this always renders once with the drop cap enabled;
/// only when `segment_html` reads that first pass's own output as the
/// "sections, no h1" shape (`head_html: None, sections_html: Some(_)`) does
/// it render a second time with the drop cap suppressed — the one case that
/// needs it, and the only case that pays for a second pass. Both halves of
/// the gate come from the SAME `segment_html` call that later decides which
/// panels to emit, so the gate and the segmentation can never disagree.
///
/// `figures` are the figure blocks of this narrative's sections; non-empty
/// renders figure mode (see `rewrite_events`), empty leaves the output
/// exactly as it is without figures.
///
/// `id_offset` is the number of top-level `##` headings in the page's
/// narratives before this one; generated heading ids count on from it (see
/// `rewrite_events`).
///
/// `rail` is the page rail, given to the page's first narrative only. It
/// renders after the intro, inside the first `div.narr`, and only there: a
/// document with neither head nor intro (sections or parts only, or the
/// fallback) leaves it out and says so in [`Editorial::rail_placed`].
pub(crate) fn render_editorial(
    src: &str,
    figures: &[FigureBlock],
    id_offset: usize,
    rail: Option<&Rail<'_>>,
) -> Editorial {
    let filtered = editorial_events(src);

    // First pass: drop cap enabled.
    let mut body_html = rewrite_events(&filtered, false, figures, id_offset);
    if matches!(
        segment_html(&body_html),
        Some(EditorialSegments { head_html: None, sections_html: Some(_), .. })
    ) {
        // Second pass: this pass's own segmentation proves the suppression
        // rule applies (no h1, a real section) — re-render with the
        // drop-cap slot pre-consumed. Suppressing the drop cap only changes
        // the FIRST paragraph's markup, never the `</h1>`/`SECTION_MARKER`
        // boundaries segment_html scans for, so the second pass yields the
        // same shape.
        body_html = rewrite_events(&filtered, true, figures, id_offset);
    }

    // Attempt to segment
    if let Some(segs) = segment_html(&body_html) {
        // Two boxes: `div.narr` holds the head and the intro, at the reading
        // measure; `div.narr.narr-wide` holds the sections or parts, at the
        // composition's width. Neither is emitted empty: the stylesheet
        // spaces a breakout with `.narr + .narr-wide`, which an empty first
        // box would trigger above a document that opens with a section.
        let lead = segs.head_html.is_some() || segs.intro_html.is_some();
        let placed_rail = rail.filter(|_| lead);
        let markup = html! {
            article.editorial.page {
                @if lead {
                    // editorial.css (THE COMPOSITION, the rail form) lays
                    // this box out as a two-row grid and reads its children:
                    // at most a head and an intro, in that order, then the
                    // rail. A further child needs those rules revisited.
                    div.narr {
                        @if let Some(head) = &segs.head_html {
                            header.editorial__head { (PreEscaped(head)) }
                        }
                        @if let Some(intro) = &segs.intro_html {
                            div.prose.editorial__intro { (PreEscaped(intro)) }
                        }
                        // After the intro in the DOM, so reading and focus
                        // order are head, intro, rail; the stylesheet places
                        // it beside them.
                        @if let Some(rail) = placed_rail { (rail) }
                    }
                }
                @if segs.sections_html.is_some() || segs.parts_html.is_some() {
                    div.narr.narr-wide {
                        @if let Some(sections) = &segs.sections_html {
                            div.editorial__sections { (PreEscaped(sections)) }
                        }
                        @if let Some(parts) = &segs.parts_html { (PreEscaped(parts)) }
                    }
                }
            }
        };
        Editorial { markup, rail_placed: placed_rail.is_some() }
    } else {
        // Fallback: single prose wrapper
        let markup = html! {
            article.editorial.page {
                div.narr {
                    div.prose { (PreEscaped(body_html)) }
                }
            }
        };
        Editorial { markup, rail_placed: false }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use cpe_core::narrative_outline::section_headings;
    use pulldown_cmark::TagEnd;

    use super::*;

    // ── Helper ───────────────────────────────────────────────────────────────

    fn render(src: &str) -> String {
        render_editorial(src, &[], 0, None).markup.into_string()
    }

    // ── Heading ids ──────────────────────────────────────────────────────────

    #[test]
    fn heading_attribute_becomes_the_heading_id() {
        let out = render("# Title\n\nBody.\n\n## Printed four times {#four}\n\nMore.");
        assert!(
            out.contains(r#"<h2 id="four">Printed four times</h2>"#),
            "expected the id on the h2 and no literal braces, got: {out}"
        );
    }

    /// The ids of every `<h2>` in `html`, in order; `None` for one without.
    fn h2_ids(html: &str) -> Vec<Option<String>> {
        html.split("<h2")
            .skip(1)
            .map(|rest| {
                rest.strip_prefix(" id=\"")
                    .and_then(|r| r.split_once('"'))
                    .map(|(id, _)| id.to_owned())
            })
            .collect()
    }

    fn ids(expected: &[&str]) -> Vec<Option<String>> {
        expected.iter().map(|id| Some((*id).to_owned())).collect()
    }

    /// An authored id is used as written and still takes its position's
    /// number out of use; the second narrative of the page counts on from
    /// the first one's headings.
    #[test]
    fn every_top_level_h2_gets_an_id_numbered_across_the_page() {
        let first = "# T\n\nIntro.\n\n## A {#a}\n\nText.\n\n## B\n\nText.\n";
        let second = "## C\n\nText.\n";
        let offset = top_level_headings(first).len();
        assert_eq!(offset, 2);
        let mut got = h2_ids(&render_editorial(first, &[], 0, None).markup.into_string());
        got.extend(h2_ids(&render_editorial(second, &[], offset, None).markup.into_string()));
        assert_eq!(got, ids(&["a", "sec-2", "sec-3"]));
    }

    #[test]
    fn generated_ids_count_on_from_the_offset() {
        let out = render_editorial("## One\n\nText.\n\n## Two\n\nText.\n", &[], 4, None)
            .markup
            .into_string();
        assert_eq!(h2_ids(&out), ids(&["sec-5", "sec-6"]));
    }

    /// A `##` inside a list item is not a section: it gets no generated id
    /// and does not take a number from the top-level headings around it.
    #[test]
    fn a_nested_h2_gets_no_id_and_takes_no_number() {
        let src = "## One\n\nText.\n\n- ## Nested\n\n## Two\n\nText.\n";
        let out = render(src);
        assert_eq!(
            h2_ids(&out),
            vec![Some("sec-1".to_owned()), None, Some("sec-2".to_owned())],
            "{out}"
        );
        assert_eq!(top_level_headings(src).len(), 2);
    }

    #[test]
    fn an_invite_sections_h2_carries_its_id() {
        let out =
            render("# T\n\n## Explore {#explore}\n\n- [Books](/books) — all\n\n## More\n\n- [Maps](/maps) — all\n");
        assert!(
            out.contains(r#"<section class="editorial__section"><h2 id="explore">Explore</h2>"#),
            "{out}"
        );
        assert!(
            out.contains(r#"<section class="editorial__section"><h2 id="sec-2">More</h2>"#),
            "{out}"
        );
    }

    #[test]
    fn an_h2_followed_by_a_plain_list_carries_its_id() {
        let out = render("# T\n\n## Notes\n\n- one\n- two\n");
        assert!(out.contains(r#"<h2 id="sec-1">Notes</h2>"#), "{out}");
        assert!(!out.contains("invite-list"), "{out}");
    }

    // ── The page rail ────────────────────────────────────────────────────────

    fn entry(id: &str, text: &str) -> RailEntry {
        RailEntry { id: id.to_owned(), text: text.to_owned() }
    }

    /// Render `src` with a two-entry rail, with `figures` for figure mode.
    fn render_with_rail(src: &str, figures: &[FigureBlock]) -> (String, bool) {
        let entries = [entry("a", "A & B"), entry("sec-2", "Two")];
        let rail = Rail { entries: &entries, label: "On this page" };
        let out = render_editorial(src, figures, 0, Some(&rail));
        (out.markup.into_string(), out.rail_placed)
    }

    /// The rail is the last child of the first `div.narr`: head, intro, rail.
    /// Its caption names the `nav`, and an entry's text is escaped.
    #[test]
    fn the_rail_follows_the_intro_and_is_named_by_its_caption() {
        let (out, placed) = render_with_rail("# T\n\nIntro.\n\n## A {#a}\n\n- [X](/x) \u{2014} g\n", &[]);
        assert!(placed);
        let expected = concat!(
            "</header><div class=\"prose editorial__intro\">\n",
            r#"<p class="editorial__open">Intro.</p></div>"#,
            r#"<nav class="narr__side" aria-labelledby="narr-rail-cap">"#,
            r#"<p class="eyebrow toc__cap" id="narr-rail-cap">On this page</p>"#,
            r##"<ul class="toc"><li><a href="#a">A &amp; B</a></li>"##,
            r##"<li><a href="#sec-2">Two</a></li></ul></nav></div>"##,
            r#"<div class="narr narr-wide">"#,
        );
        assert!(out.contains(expected), "{out}");
    }

    /// An intro without an h1 is enough for the rail to stand beside.
    #[test]
    fn the_rail_stands_beside_an_intro_without_a_head() {
        let (out, placed) = render_with_rail("Intro.\n\n## A {#a}\n\n- [X](/x) \u{2014} g\n", &[]);
        assert!(placed);
        assert!(!out.contains("editorial__head"), "{out}");
        assert!(out.contains(r#"</div><nav class="narr__side""#), "{out}");
    }

    /// A document that renders neither a head nor an intro has nothing for
    /// the rail to stand beside: it is left out, and the caller is told.
    #[test]
    fn no_rail_without_a_head_or_an_intro() {
        let sections_only = "## A {#a}\n\n- [X](/x) \u{2014} g\n\n## B\n\n- [Y](/y) \u{2014} g\n";
        let parts_only = "## Four {#four}\n\nProse.\n";
        let fallback = "Just prose.\n\nMore prose.\n";
        for (shape, src, figures) in [
            ("sections only", sections_only, vec![]),
            ("parts only", parts_only, vec![block("four", "A")]),
            ("fallback", fallback, vec![]),
        ] {
            let (out, placed) = render_with_rail(src, &figures);
            assert!(!placed, "{shape}: {out}");
            assert!(!out.contains("narr__side"), "{shape}: {out}");
        }
    }

    /// Without a rail the output is what it was: no `nav`, nothing placed.
    #[test]
    fn no_rail_is_rendered_unless_one_is_given() {
        let out = render_editorial("# T\n\nIntro.\n\n## A\n\nText.\n", &[], 0, None);
        assert!(!out.rail_placed);
        assert!(!out.markup.into_string().contains("narr__side"));
    }

    /// The rail's ids are the ids the headings render, authored or generated,
    /// counted from the same offset.
    #[test]
    fn rail_entries_carry_the_ids_the_headings_render() {
        let src = "# T\n\nIntro.\n\n## Plain\n\nText.\n\n## Named {#named}\n\n\
                   - [X](/x) \u{2014} g\n\n## *Last* `one`\n\n- [Y](/y) \u{2014} g\n";
        let entries = rail_entries(src, 3);
        assert_eq!(
            entries,
            [
                entry("sec-4", "Plain"),
                entry("named", "Named"),
                entry("sec-6", "Last one"),
            ]
        );
        let rendered = h2_ids(&render_editorial(src, &[], 3, None).markup.into_string());
        let listed: Vec<Option<String>> = entries.into_iter().map(|e| Some(e.id)).collect();
        assert_eq!(rendered, listed);
    }

    /// The same agreement over every section shape at once: a heading with no
    /// text, a `##` nested in a list item (rendered, never listed), an invite
    /// section, invite groups, an authored id between generated ones, and
    /// figure mode on top.
    #[test]
    fn rail_ids_equal_rendered_ids_across_every_section_shape() {
        let src = "# T\n\nIntro.\n\n##\n\nText.\n\n- ## Nested\n\n\
                   ## Invite\n\n- [X](/x) \u{2014} g\n\n\
                   ## Named {#named}\n\nProse.\n\n\
                   ## Groups\n\n### One\n\n- [Y](/y) \u{2014} g\n\n\
                   ## Last\n\n- one\n- two\n";
        let listed: Vec<String> = rail_entries(src, 7).into_iter().map(|e| e.id).collect();
        assert_eq!(listed, ["sec-8", "sec-9", "named", "sec-11", "sec-12"]);
        for figures in [vec![], vec![block("named", "A")]] {
            let out = render_editorial(src, &figures, 7, None).markup.into_string();
            let rendered: Vec<String> = h2_ids(&out).into_iter().flatten().collect();
            assert_eq!(rendered, listed, "{out}");
        }
    }

    /// A line break inside a heading is a space in the entry's text, and an
    /// entry without visible text has no name.
    #[test]
    fn a_rail_entrys_text_is_the_headings_readable_text() {
        let entries = rail_entries("Two\nlines\n---\n\n## \u{200B}\n\n## ![](x.png)\n", 0);
        let texts: Vec<&str> = entries.iter().map(|e| e.text.as_str()).collect();
        assert_eq!(texts, ["Two lines", "\u{200B}", ""]);
        let named: Vec<bool> = entries.iter().map(RailEntry::has_name).collect();
        assert_eq!(named, [true, false, false]);
    }

    // ── Invite groups ────────────────────────────────────────────────────────

    const GROUPS_DOC: &str = "# T\n\nIntro.\n\n## Offices {#offices}\n\nThe section's sentence.\n\n\
                              ### Amerbach {#amerbach}\n\nA *gloss*.\n\n- [Book A](/a) \u{2014} first\n\n\
                              ### Furter\n\n- [Book C](/c) \u{2014} third\n\n\
                              ## Ways {#ways}\n\n- [Books](/books) \u{2014} all\n";

    /// The markup of one invite item, as `render_invite_item` writes it.
    fn item(name: &str, href: &str, gloss: &str) -> String {
        let parsed = InviteItem {
            name: name.to_owned(),
            href: href.to_owned(),
            gloss: Some(gloss.to_owned()),
        };
        render_invite_item(&parsed).into_string()
    }

    #[test]
    fn a_groups_section_renders_its_groups_under_the_section_heading() {
        let out = render(GROUPS_DOC);
        let expected = [
            r#"<section class="editorial__section"><h2 id="offices">Offices</h2>"#,
            "<p>The section's sentence.</p>\n",
            r#"<div class="invite-groups">"#,
            // A group heading renders its authored id, as any `###` does.
            r#"<section class="invite-group"><h3 id="amerbach">Amerbach</h3>"#,
            r#"<p class="invite-group__gloss">A <em>gloss</em>.</p>"#,
            r#"<ul class="invite-list">"#,
            &item("Book A", "/a", "first"),
            "</ul></section>",
            // No gloss paragraph: no `p.invite-group__gloss`. No authored
            // id: none is generated, a group is not a numbered section.
            r#"<section class="invite-group"><h3>Furter</h3><ul class="invite-list">"#,
            &item("Book C", "/c", "third"),
            "</ul></section></div></section>",
            r#"<section class="editorial__section"><h2 id="ways">Ways</h2>"#,
        ]
        .concat();
        assert!(out.contains(&expected), "got: {out}");
        assert_eq!(out.matches("invite-group__gloss").count(), 1, "{out}");
        // The two `##` ids and the one authored `###` id, nothing generated.
        assert_eq!(out.matches(" id=\"").count(), 3, "{out}");
    }

    #[test]
    fn a_groups_section_needs_no_paragraph_before_its_first_group() {
        let out = render("# T\n\n## Offices\n\n### Amerbach\n\n- [Book A](/a) \u{2014} first\n");
        assert!(
            out.contains(concat!(
                r#"<section class="editorial__section"><h2 id="sec-1">Offices</h2>"#,
                r#"<div class="invite-groups"><section class="invite-group"><h3>Amerbach</h3>"#,
            )),
            "got: {out}"
        );
    }

    /// One group whose list is not an invite list (here a plain bullet)
    /// leaves the whole section to the plain output: no section, no groups.
    #[test]
    fn a_group_without_an_invite_list_falls_the_section_back_to_plain_output() {
        let src = "# T\n\n## Offices\n\n### Amerbach\n\n- [Book A](/a) \u{2014} first\n\n\
                   ### Furter\n\n- just a note\n";
        let out = render(src);
        assert!(!out.contains("invite-group"), "got: {out}");
        assert!(!out.contains("editorial__section"), "got: {out}");
        assert!(out.contains("<h2 id=\"sec-1\">Offices</h2>\n<h3>Amerbach</h3>"), "got: {out}");
        // The look-ahead consumed nothing: every block renders exactly once.
        for text in ["Offices", "Amerbach", "Book A", "Furter", "just a note"] {
            assert_eq!(out.matches(text).count(), 1, "{text}: {out}");
        }
        assert!(!top_level_headings(src)[0].holds_invite_list);
    }

    /// The renderer's numbering and `top_level_headings` count the same
    /// headings whatever sits between them: a groups section, an invite
    /// section, a block quote, a nested `##`, a figure-mode part.
    #[test]
    fn numbering_matches_the_reader_across_every_section_shape() {
        let src = "# T\n\n## Groups\n\n### G\n\n- [A](/a) \u{2014} a\n\n\
                   ## List\n\n- [B](/b) \u{2014} b\n\n> ## Quoted\n\n- ## Nested\n\n\
                   ## Part {#part}\n\nProse.\n\n## Plain\n\nEnd.\n";
        let count = top_level_headings(src).len();
        assert_eq!(count, 4);
        for figures in [vec![], vec![block("part", "A")]] {
            let out = render_editorial(src, &figures, 10, None).markup.into_string();
            let top: Vec<Option<String>> = h2_ids(&out).into_iter().flatten().map(Some).collect();
            assert_eq!(top, ids(&["sec-11", "sec-12", "part", "sec-14"]), "{out}");
        }
    }

    /// The renderer, `invite_list_follows` (through `top_level_headings`)
    /// and `section_headings` read a groups section the same way: an invite
    /// section, which figure mode never turns into a part.
    #[test]
    fn the_readers_and_the_renderer_agree_on_a_groups_section() {
        let events = editorial_events(GROUPS_DOC);
        let h2_end = events
            .iter()
            .position(|e| matches!(e, Event::End(TagEnd::Heading(pulldown_cmark::HeadingLevel::H2))))
            .expect("the document has an h2");
        assert!(matches!(
            look_ahead_invite_section(&events, h2_end + 1),
            InviteSection::Groups { ref groups, .. } if groups.len() == 2
        ));
        assert!(top_level_headings(GROUPS_DOC)[0].holds_invite_list);
        let offices = section_headings(GROUPS_DOC)
            .into_iter()
            .find(|h| h.id == "offices")
            .expect("'offices' is an authored id");
        assert!(offices.holds_invite_list);

        // With a figure block named for it the section still renders as an
        // invite section, and the block is placed nowhere.
        let out = render_editorial(GROUPS_DOC, &[block("offices", "A")], 0, None)
            .markup
            .into_string();
        assert!(out.contains(r#"<div class="invite-groups">"#), "got: {out}");
        assert!(!out.contains("editorial__part"), "got: {out}");
    }

    #[test]
    fn kicker_and_subtitle_h3s_are_not_groups() {
        let src = "### Kicker\n\n# Title\n\n### The subtitle\n\nBody.\n\n\
                   ## Offices\n\n### Amerbach\n\n- [Book A](/a) \u{2014} first\n";
        let out = render(src);
        assert!(
            out.contains(concat!(
                r#"<header class="editorial__head"><p class="eyebrow editorial__kicker">Kicker</p>"#,
                "<h1>Title</h1>\n",
                r#"<p class="editorial__subtitle">The subtitle</p></header>"#,
            )),
            "got: {out}"
        );
        assert_eq!(out.matches("<h3>").count(), 1, "got: {out}");
        assert_eq!(out.matches(r#"class="invite-group""#).count(), 1, "got: {out}");
    }

    /// A generated id never makes a section figure-attachable: only authored
    /// ids are reported, so `UnknownFigureSection` still fires for `sec-2`.
    #[test]
    fn section_headings_never_report_a_generated_id() {
        let reported: Vec<String> = section_headings(FIGURE_DOC).into_iter().map(|h| h.id).collect();
        assert_eq!(reported, vec!["four", "between"]);
        let all: Vec<Option<String>> = top_level_headings(FIGURE_DOC).into_iter().map(|h| h.id).collect();
        assert_eq!(all, vec![Some("four".to_owned()), None, Some("between".to_owned())]);
    }

    // ── Figure mode ──────────────────────────────────────────────────────────

    const FIGURE_DOC: &str = "### Showcase\n\n# Woodcuts\n\n*The lede.*\n\nIntro body.\n\n\
                              ## Four {#four}\n\nFour prose.\n\n## Plain\n\nStill four.\n\n\
                              ## Between {#between}\n\nBetween prose.\n";

    fn block(section: &str, text: &str) -> FigureBlock {
        FigureBlock {
            section: section.to_owned(),
            block: html! {
                div.block { (text) }
            },
            count: None,
            more: None,
        }
    }

    #[test]
    fn figure_mode_splits_at_each_idd_h2_and_places_blocks_in_order() {
        let figures = [block("four", "A"), block("between", "C"), block("four", "B")];
        let out = render_editorial(FIGURE_DOC, &figures, 0, None).markup.into_string();
        let expected_parts = concat!(
            r#"<div class="editorial__parts">"#,
            r#"<section class="editorial__part" aria-labelledby="four"><div class="prose editorial__part-prose">"#,
            r#"<h2 id="four">Four</h2>"#,
            // The id-less `## Plain` gets a generated id and stays inside
            // the part: only an authored id opens one.
            "\n<p>Four prose.</p>\n<h2 id=\"sec-2\">Plain</h2>\n<p>Still four.</p>\n</div>",
            r#"<div class="editorial__figure"><div class="block">A</div></div>"#,
            r#"<div class="editorial__figure"><div class="block">B</div></div>"#,
            "</section>",
            r#"<section class="editorial__part" aria-labelledby="between"><div class="prose editorial__part-prose">"#,
            r#"<h2 id="between">Between</h2>"#,
            "\n<p>Between prose.</p>\n</div>",
            r#"<div class="editorial__figure"><div class="block">C</div></div>"#,
            "</section></div></div></article>",
        );
        assert!(out.ends_with(expected_parts), "got: {out}");
        // The intro keeps its head, lede, drop cap and kicker.
        assert!(
            out.starts_with(concat!(
                r#"<article class="editorial page"><div class="narr"><header class="editorial__head">"#,
                r#"<p class="eyebrow editorial__kicker">Showcase</p><h1>Woodcuts</h1></header>"#,
                r#"<div class="prose editorial__intro">"#,
            )),
            "got: {out}"
        );
        assert!(out.contains(r#"<p class="editorial__lede"><em>The lede.</em></p>"#));
        assert!(out.contains(r#"<p class="editorial__open">Intro body.</p>"#));
        // Each section id exists exactly once on the page.
        for id in ["four", "between"] {
            assert_eq!(out.matches(&format!(r#"id="{id}""#)).count(), 1, "{id}: {out}");
        }
    }

    #[test]
    fn figure_mode_renders_the_count_line_and_the_more_link() {
        let figures = [FigureBlock {
            count: Some("4 von 14 Fällen".to_owned()),
            more: Some(FigureLink {
                label: "Alle Randleisten".to_owned(),
                href: "/incunabula/strips".to_owned(),
            }),
            ..block("four", "A")
        }];
        let out = render_editorial(FIGURE_DOC, &figures, 0, None).markup.into_string();
        assert!(
            out.contains(concat!(
                r#"<div class="editorial__figure"><div class="block">A</div>"#,
                r#"<p class="editorial__figure-count">4 von 14 Fällen</p>"#,
                r#"<p class="editorial__figure-more"><a href="/incunabula/strips">Alle Randleisten "#,
                r#"<span class="editorial__figure-arrow" aria-hidden="true">→</span></a></p></div>"#,
            )),
            "got: {out}"
        );
    }

    #[test]
    fn heading_attribute_block_keeps_only_the_id() {
        let out = render("# T {.big data-init=x}\n\n## Four {#four .c onclick=alert(1)}\n");
        assert!(out.contains("<h1>T</h1>"), "got: {out}");
        assert!(out.contains(r#"<h2 id="four">Four</h2>"#), "got: {out}");
        for leaked in ["onclick", "data-init", "class=\"c\"", "class=\"big\""] {
            assert!(!out.contains(leaked), "{leaked} leaked: {out}");
        }
    }

    #[test]
    fn figure_mode_parts_never_take_the_lede_or_the_drop_cap() {
        let src = "# T\n\n## Four {#four}\n\n*Note.*\n\nProse.\n";
        let out = render_editorial(src, &[block("four", "A")], 0, None).markup.into_string();
        assert!(!out.contains("editorial__lede"), "got: {out}");
        assert!(!out.contains("editorial__open"), "got: {out}");
        assert!(out.contains("<p><em>Note.</em></p>"), "got: {out}");
    }

    /// The renderer splits at exactly the non-invite headings the boot check
    /// (`section_headings`) reports: nested and invite-list `##`s stay inline.
    #[test]
    fn figure_mode_splits_exactly_where_section_headings_says() {
        let src = "# T\n\nIntro.\n\n## Four {#four}\n\nText.\n\n> ## Quoted {#quoted}\n\n\
                   - ## Listed {#listed}\n\n## Explore {#explore}\n\n- [Books](/books) — all books\n\n\
                   ## Last {#last}\n\nEnd.\n";
        let figures: Vec<FigureBlock> = ["four", "quoted", "listed", "explore", "last"]
            .iter()
            .map(|id| block(id, id))
            .collect();
        let out = render_editorial(src, &figures, 0, None).markup.into_string();
        let parts: Vec<String> = section_headings(src)
            .into_iter()
            .filter(|h| !h.holds_invite_list)
            .map(|h| h.id)
            .collect();
        assert_eq!(parts, vec!["four", "last"]);
        assert_eq!(out.matches(r#"class="editorial__part""#).count(), parts.len(), "{out}");
        for id in &parts {
            assert!(out.contains(&format!(r#"aria-labelledby="{id}""#)), "{id}: {out}");
        }
        for id in ["quoted", "listed", "explore"] {
            assert!(!out.contains(&format!(r#"<div class="block">{id}</div>"#)), "{id}: {out}");
        }
        assert!(out.contains(r#"<section class="editorial__section">"#), "{out}");
    }

    #[test]
    fn without_figures_heading_ids_do_not_split_the_narrative() {
        let out = render(FIGURE_DOC);
        assert!(!out.contains("editorial__part"), "got: {out}");
        assert!(out.contains(r#"<h2 id="four">Four</h2>"#), "got: {out}");
    }

    // ── Lede recognition ─────────────────────────────────────────────────────

    #[test]
    fn lede_emphasis_only_gets_lede_class() {
        // A paragraph with ONLY emphasis content → editorial__lede
        let out = render("*Curiouser and curiouser!*");
        assert!(
            out.contains(r#"class="editorial__lede""#),
            "emphasis-only paragraph must get editorial__lede class, got: {out}"
        );
        assert!(
            out.contains("Curiouser and curiouser!"),
            "lede text must be present, got: {out}"
        );
    }

    #[test]
    fn lede_mixed_text_and_emphasis_is_not_lede() {
        // Mixed text + emphasis → NOT a lede, just a regular paragraph
        let out = render("Some text with *emphasis* inside.");
        assert!(
            !out.contains(r#"class="editorial__lede""#),
            "mixed text+emphasis must NOT be a lede, got: {out}"
        );
    }

    #[test]
    fn lede_plain_paragraph_is_not_lede() {
        // Plain text paragraph → not a lede
        let out = render("Just a plain paragraph.");
        assert!(
            !out.contains(r#"class="editorial__lede""#),
            "plain paragraph must not be a lede, got: {out}"
        );
    }

    // ── Drop-cap open ─────────────────────────────────────────────────────────

    #[test]
    fn first_body_paragraph_gets_open_class() {
        let out = render("# Title\n\nFirst paragraph here.");
        assert!(
            out.contains(r#"class="editorial__open""#),
            "first body paragraph must get editorial__open class, got: {out}"
        );
    }

    #[test]
    fn dropcap_extracts_letter_from_bold_opening() {
        // **Alice** in Wonderland → "A<strong>lice</strong> in Wonderland"
        let result = dropcap_ready("<strong>Alice</strong> in Wonderland");
        assert!(
            result.starts_with('A'),
            "dropcap must start with the extracted letter 'A', got: {result}"
        );
        assert!(result.contains("<strong>"), "dropcap must keep the strong tag, got: {result}");
    }

    #[test]
    fn dropcap_extracts_letter_from_em_opening() {
        // *Alice* starts → "A<em>lice</em> starts"
        let result = dropcap_ready("<em>Alice</em> starts");
        assert!(result.starts_with('A'), "dropcap must start with 'A', got: {result}");
        assert!(result.contains("<em>"), "dropcap must keep the em tag, got: {result}");
    }

    #[test]
    fn dropcap_handles_non_ascii_initial() {
        // German opening: <strong>Über</strong> alles → "Ü<strong>ber</strong> alles"
        // (the comment used to name a different input and an output carrying
        // the empty `<strong></strong>` this pass no longer produces).
        let result = dropcap_ready("<strong>Über</strong> alles");
        assert_eq!(
            result, "Ü<strong>ber</strong> alles",
            "dropcap must handle non-ASCII initial 'Ü', got: {result}"
        );
    }

    #[test]
    fn dropcap_single_letter_run_leaves_no_empty_element() {
        // `**B**asel …` — the hand-authored drop-cap idiom every project's
        // home narrative uses. The extracted letter IS the whole bold run, so
        // re-emitting the tag pair would leave an empty `<strong></strong>`
        // (cosmetic, but real DOM litter that shipped in the Incunabula home
        // snapshot).
        let result = dropcap_ready("<strong>B</strong>asel war");
        assert_eq!(
            result, "Basel war",
            "a single-letter bold run must leave no empty element, got: {result}"
        );
        let em = dropcap_ready("<em>B</em>asel war");
        assert_eq!(em, "Basel war", "same for a single-letter em run, got: {em}");
        // Non-ASCII + whole run: `first_char.len_utf8()` slicing and the
        // closing-tag `strip_prefix` have to line up on a multi-byte initial.
        let non_ascii = dropcap_ready("<strong>Ü</strong>ber alles");
        assert_eq!(
            non_ascii, "Über alles",
            "a single-letter non-ASCII run must slice correctly, got: {non_ascii}"
        );
    }

    #[test]
    fn dropcap_no_leading_inline_tag_unchanged() {
        // No leading tag — pass through unchanged
        let result = dropcap_ready("Alice fell down");
        assert_eq!(result, "Alice fell down", "dropcap of plain text must be unchanged");
    }

    #[test]
    fn h1_less_document_with_sections_gets_no_open_class() {
        // A secondary, h1-less document with a real invite-list section
        // (e.g. `home-offices.*.md`) sharing a page with the h1 document
        // must not add a second drop cap.
        let out = render("Die Offizin steht hier.\n\n## Offizinen\n\n- [A](/a) \u{2014} gloss");
        assert!(
            !out.contains(r#"class="editorial__open""#),
            "an h1-less document with a section must not get editorial__open, got: {out}"
        );
    }

    #[test]
    fn h1_less_document_without_sections_keeps_open_class() {
        // "Neither" shape: no h1, no real section (the div.prose fallback).
        // A headless document — even one with an h2/h3 heading that never
        // resolves into an invite-list section — must keep its drop cap.
        let plain = render("Just prose.\n\nMore prose.");
        assert!(
            plain.contains(r#"class="editorial__open""#),
            "an h1-less, section-less document must keep editorial__open, got: {plain}"
        );
        let headless_h2 = render("## Our mission\n\nSome body text.");
        assert!(
            headless_h2.contains(r#"class="editorial__open""#),
            "a headless h2 with no following invite list must keep editorial__open, got: {headless_h2}"
        );
    }

    /// A `## h2` + invite list nested inside a blockquote is rendered
    /// opaquely by the `BlockQuote` arm (plain `push_html`, never recursed
    /// through `rewrite_events`'s own h2/invite-list matching), so it never
    /// produces a `SECTION_MARKER` — the drop-cap-suppression gate, derived
    /// from `rewrite_events`'s own output, must NOT fire here. Regression
    /// test for the disagreement between the old `has_section_without_h1`
    /// re-scan (which found the h2 + list textually, ignoring the
    /// blockquote) and `segment_html`'s real "neither" shape.
    #[test]
    fn h1_less_document_with_section_only_inside_blockquote_keeps_open_class() {
        let src = "Some intro without a heading.\n\n> ## Inside a quote\n>\n> - [A](/a) \u{2014} gloss";
        let out = render(src);
        assert!(
            !out.contains(SECTION_MARKER),
            "a blockquote-nested h2+list must not produce a real section, got: {out}"
        );
        assert!(
            out.contains(r#"class="editorial__open""#),
            "an h1-less document whose only section-shaped content is inside a blockquote must keep editorial__open, got: {out}"
        );
    }

    #[test]
    fn h1_document_with_sections_keeps_open_class() {
        // h1 + a REAL invite-list section (not just an `## h2` — an h2 alone
        // never yields a `SECTION_MARKER`, so this must use the actual
        // invite-list shape to exercise the "h1 + sections" gate case.
        let out = render("# Title\n\nIntro paragraph.\n\n## Section\n\n- [A](/a) \u{2014} gloss");
        assert!(
            out.contains(SECTION_MARKER),
            "premise: the invite list must form a real section, got: {out}"
        );
        assert_eq!(
            out.matches(r#"class="editorial__open""#).count(),
            1,
            "an h1 document keeps exactly one editorial__open, got: {out}"
        );
    }

    #[test]
    fn h1_after_first_section_still_suppresses_open_class() {
        // A document that segments as "sections, no h1" from segment_html's
        // point of view (the first `<section class="editorial__section">`
        // comes before any `</h1>` in the rewritten output) because its
        // ONLY h1 appears textually after the first section. The gate must
        // derive from segment_html's read of the FIRST pass's output, not a
        // separate whole-document `has_h1` scan — a stray h1 anywhere would
        // wrongly suppress the "no h1" classification here.
        let src = "Intro.\n\n## S\n\n- [A](/a) \u{2014} g\n\n# T";
        let out = render(src);
        assert!(
            !out.contains(r#"class="editorial__open""#),
            "an h1 appearing only after the first section must still suppress editorial__open, got: {out}"
        );
    }

    #[test]
    fn second_paragraph_is_not_open() {
        let out = render("# Title\n\nFirst para.\n\nSecond para.");
        // The second paragraph must not have editorial__open
        let open_count = out.matches(r#"class="editorial__open""#).count();
        assert_eq!(
            open_count, 1,
            "exactly one editorial__open must appear (first para only), got: {out}"
        );
    }

    // ── Lede before open: both present ───────────────────────────────────────

    #[test]
    fn lede_before_open_both_rendered() {
        // Lede (emphasis-only) then first body paragraph (open)
        let src = "# Title\n\n*Curiouser!*\n\n**Alice** started her adventure.";
        let out = render(src);
        assert!(out.contains(r#"class="editorial__lede""#), "lede must be present, got: {out}");
        assert!(out.contains(r#"class="editorial__open""#), "open must be present, got: {out}");
    }

    // ── Epigraph ──────────────────────────────────────────────────────────────

    #[test]
    fn blockquote_becomes_epigraph() {
        let out = render("> A wise saying.");
        assert!(
            out.contains(r#"class="epigraph""#),
            "blockquote must get epigraph class, got: {out}"
        );
    }

    #[test]
    fn epigraph_with_attribution_has_cite() {
        let out = render("> \"We're all mad here.\" — the Cheshire Cat");
        assert!(out.contains(r#"class="epigraph""#), "must be epigraph, got: {out}");
        assert!(out.contains("<cite>"), "epigraph with attribution must have <cite>, got: {out}");
        assert!(
            out.contains("the Cheshire Cat"),
            "cite must contain the attribution, got: {out}"
        );
    }

    #[test]
    fn epigraph_cite_attribution_ampersand_is_escaped() {
        // Attribution containing & must be HTML-escaped to &amp; in <cite>.
        let out = render("> A quote. — Carroll & Co.");
        assert!(out.contains("<cite>"), "epigraph with attribution must have <cite>, got: {out}");
        assert!(
            out.contains("&amp;"),
            "& in attribution must be escaped to &amp; in <cite>, got: {out}"
        );
        assert!(
            !out.contains("Carroll & Co."),
            "raw unescaped & must not appear in <cite>, got: {out}"
        );
        assert!(
            !out.contains("&amp;amp;"),
            "& must be escaped exactly once (no double-encode), got: {out}"
        );
        assert!(
            out.contains("Carroll &amp; Co."),
            "attribution must render with a single &amp;, got: {out}"
        );
    }

    #[test]
    fn epigraph_without_attribution_has_no_cite() {
        let out = render("> A quote without attribution.");
        assert!(out.contains(r#"class="epigraph""#), "must be epigraph, got: {out}");
        assert!(
            !out.contains("<cite>"),
            "epigraph without attribution must not have <cite>, got: {out}"
        );
    }

    // ── Invite list ───────────────────────────────────────────────────────────

    #[test]
    fn invite_list_items_get_invite_classes() {
        let src =
            "## Meet the characters\n\n- [Alice](/alice) — the protagonist\n- [White Rabbit](/rabbit) — always late";
        let out = render(src);
        assert!(
            out.contains(r#"class="invite-list""#),
            "must have invite-list class, got: {out}"
        );
        assert!(
            out.contains(r#"class="invite__name""#),
            "must have invite__name spans, got: {out}"
        );
        assert!(
            out.contains(r#"class="invite__gloss""#),
            "must have invite__gloss spans, got: {out}"
        );
        assert!(
            out.contains(r#"class="invite__arrow""#),
            "must have invite__arrow spans, got: {out}"
        );
        assert!(out.contains("→"), "must have arrow character, got: {out}");
    }

    #[test]
    fn invite_list_href_is_escaped() {
        // The href must be HTML-escaped by Maud: & → &amp; in attributes.
        let src = "## Section\n\n- [Name](/path?a=1&b=2) — a gloss";
        let out = render(src);
        // Maud escapes & as &amp; in href attribute values.
        assert!(
            out.contains("&amp;"),
            "href with & must be HTML-escaped to &amp; by Maud, got: {out}"
        );
        // The raw unescaped ampersand must NOT appear in an attribute context.
        // We check the specific query string fragment is not unescaped.
        assert!(
            !out.contains("a=1&b=2"),
            "raw unescaped 'a=1&b=2' must not appear in the output (& must be escaped), got: {out}"
        );
    }

    #[test]
    fn invite_list_javascript_href_falls_back_to_plain_text() {
        let src = "## Section\n\n- [Evil](javascript:alert(1)) — bad link";
        let out = render(src);
        // Must NOT contain an <a> to the javascript: url
        assert!(!out.contains("javascript:"), "javascript: href must not be emitted, got: {out}");
        // The name must still appear as text
        assert!(out.contains("Evil"), "name must appear as plain text fallback, got: {out}");
        // Must not produce an <a href=...> anchor for this item.
        // Use "<a " or "<a>" to avoid false matches on "<article".
        assert!(
            !out.contains("<a href"),
            "javascript: href must not produce an <a href> element, got: {out}"
        );
    }

    /// `data:` hrefs must be rejected by `href_is_allowed` and fall back to plain text,
    /// mirroring the `javascript:` test.
    #[test]
    fn invite_list_data_href_falls_back_to_plain_text() {
        let src = "## Section\n\n- [DataEvil](data:text/html,evil) — dangerous link";
        let out = render(src);
        // Must NOT contain a data: URL in any attribute
        assert!(!out.contains("data:text/html"), "data: href must not be emitted, got: {out}");
        // The name must still appear as plain text (fallback)
        assert!(
            out.contains("DataEvil"),
            "name must appear as plain text fallback for data: href, got: {out}"
        );
        // Must not produce an <a href=...> anchor for this item.
        // Use "<a href" to avoid false matches on "<article".
        assert!(
            !out.contains("<a href"),
            "data: href must not produce an <a href> element, got: {out}"
        );
    }

    #[test]
    fn non_link_list_becomes_plain_ul() {
        // A list whose items are plain text (not link-shaped) → plain <ul>
        let src = "## Section\n\n- just text\n- more text";
        let out = render(src);
        // Must NOT have invite-list class
        assert!(
            !out.contains(r#"class="invite-list""#),
            "non-link list must not get invite-list class, got: {out}"
        );
        // Must still render as a list
        assert!(
            out.contains("<ul>") || out.contains("<li>"),
            "must still render as list, got: {out}"
        );
    }

    #[test]
    fn invite_list_http_href_allowed() {
        let src = "## Section\n\n- [External](https://example.com) — a gloss";
        let out = render(src);
        assert!(out.contains("https://example.com"), "https href must be allowed, got: {out}");
        assert!(out.contains(r#"class="invite-list""#), "must be an invite-list, got: {out}");
    }

    // ── Plain paragraph fallback ──────────────────────────────────────────────

    #[test]
    fn plain_paragraph_after_first_is_just_p() {
        let src = "# Title\n\nFirst.\n\nSecond paragraph.";
        let out = render(src);
        // Second paragraph must be a plain <p>
        assert!(
            out.contains("<p>Second paragraph.</p>"),
            "second paragraph must be plain <p>, got: {out}"
        );
    }

    // ── Article structure ─────────────────────────────────────────────────────

    #[test]
    fn render_editorial_has_article_editorial_page() {
        let out = render("# Title\n\nContent.");
        assert!(
            out.contains(r#"class="editorial page""#),
            "output must have article.editorial.page, got: {out}"
        );
    }

    // ── segment_html: the four section shapes (decision 2, option i) ─────────

    const SEC: &str = r#"<section class="editorial__section"><h2>Ways in</h2></section>"#;

    #[test]
    fn segment_h1_and_sections_is_unchanged_three_panel() {
        let html = format!("<h1>T</h1><p>Intro.</p>{SEC}");
        assert_eq!(
            segment_html(&html),
            Some(EditorialSegments {
                head_html: Some("<h1>T</h1>".to_owned()),
                intro_html: Some("<p>Intro.</p>".to_owned()),
                sections_html: Some(SEC.to_owned()),
                parts_html: None,
            })
        );
    }

    #[test]
    fn segment_h1_and_sections_keeps_empty_intro() {
        // Unchanged shape: the intro panel is emitted even when empty.
        let html = format!("<h1>T</h1>{SEC}");
        let segs = segment_html(&html).expect("h1 + sections segments");
        assert_eq!(segs.intro_html, Some(String::new()));
    }

    #[test]
    fn segment_h1_only_yields_head_and_intro_without_sections() {
        let html = "<p class=\"eyebrow editorial__kicker\">K</p><h1>T</h1><p>Body.</p>";
        assert_eq!(
            segment_html(html),
            Some(EditorialSegments {
                head_html: Some(r#"<p class="eyebrow editorial__kicker">K</p><h1>T</h1>"#.to_owned()),
                intro_html: Some("<p>Body.</p>".to_owned()),
                sections_html: None,
                parts_html: None,
            })
        );
    }

    #[test]
    fn segment_sections_without_h1_puts_leading_block_in_intro() {
        // The `home-offices.md` shape: a plain `## …` block before the first
        // invite-list section.
        let html = format!("<h2>Die Offizinen</h2>\n<p>Lead.</p>\n{SEC}");
        assert_eq!(
            segment_html(&html),
            Some(EditorialSegments {
                head_html: None,
                intro_html: Some("<h2>Die Offizinen</h2>\n<p>Lead.</p>\n".to_owned()),
                sections_html: Some(SEC.to_owned()),
                parts_html: None,
            })
        );
    }

    #[test]
    fn segment_sections_without_h1_and_no_leading_block_emits_no_intro() {
        for html in [SEC.to_owned(), format!("\n  \n{SEC}")] {
            assert_eq!(
                segment_html(&html),
                Some(EditorialSegments {
                    head_html: None,
                    intro_html: None,
                    sections_html: Some(SEC.to_owned()),
                    parts_html: None,
                }),
                "input: {html:?}"
            );
        }
    }

    #[test]
    fn segment_neither_h1_nor_sections_falls_back() {
        assert_eq!(segment_html("<h2>A</h2><p>B.</p>"), None);
    }

    #[test]
    fn render_h1_only_document_has_head_and_intro_but_no_sections() {
        let out = render("# Title\n\nContent.");
        assert!(
            out.contains(r#"<header class="editorial__head"><h1>Title</h1></header>"#),
            "{out}"
        );
        assert!(out.contains(r#"<div class="prose editorial__intro">"#), "{out}");
        assert!(!out.contains("editorial__sections"), "{out}");
    }

    #[test]
    fn render_sections_without_h1_has_intro_and_sections_but_no_head() {
        let out = render("## Die Offizinen\n\nLead.\n\n## Ways in\n\n- [A](/a) \u{2014} gloss");
        assert!(!out.contains("editorial__head"), "{out}");
        let intro = out
            .find(r#"<div class="prose editorial__intro"><h2 id="sec-1">Die Offizinen</h2>"#)
            .expect("leading block must render as div.prose.editorial__intro");
        let sections = out
            .find(r#"<div class="editorial__sections"><section class="editorial__section">"#)
            .expect("sections panel must follow");
        assert!(intro < sections, "{out}");
    }

    /// A document that opens with a section renders the breakout box alone:
    /// an empty `div.narr` before it would give the breakout the top margin
    /// meant for one that follows a head or an intro.
    #[test]
    fn render_sections_without_h1_and_no_leading_block_has_no_intro() {
        let out = render("## Ways in\n\n- [A](/a) \u{2014} gloss");
        assert!(!out.contains("editorial__intro"), "{out}");
        assert!(!out.contains("editorial__head"), "{out}");
        assert!(
            out.starts_with(concat!(
                r#"<article class="editorial page"><div class="narr narr-wide">"#,
                r#"<div class="editorial__sections">"#,
            )),
            "{out}"
        );
        assert_eq!(out.matches(r#"class="narr"#).count(), 1, "{out}");
    }

    #[test]
    fn render_parts_without_head_or_intro_has_no_empty_wrapper() {
        let out = render_editorial("## Four {#four}\n\nProse.\n", &[block("four", "A")], 0, None)
            .markup
            .into_string();
        assert!(
            out.starts_with(concat!(
                r#"<article class="editorial page"><div class="narr narr-wide">"#,
                r#"<div class="editorial__parts">"#,
            )),
            "{out}"
        );
        assert_eq!(out.matches(r#"class="narr"#).count(), 1, "{out}");
    }

    #[test]
    fn render_head_intro_and_sections_sit_in_the_two_wrappers() {
        let out = render("# T\n\nIntro.\n\n## Ways in\n\n- [A](/a) \u{2014} gloss");
        assert!(
            out.starts_with(concat!(
                r#"<article class="editorial page"><div class="narr">"#,
                r#"<header class="editorial__head"><h1>T</h1></header>"#,
                r#"<div class="prose editorial__intro">"#,
            )),
            "{out}"
        );
        assert!(
            out.contains(concat!(
                r#"</div></div><div class="narr narr-wide"><div class="editorial__sections">"#,
                r#"<section class="editorial__section">"#,
            )),
            "{out}"
        );
        assert!(out.ends_with("</section></div></div></article>"), "{out}");
    }

    #[test]
    fn render_neither_h1_nor_sections_falls_back_to_plain_prose() {
        let out = render("## Heading\n\nContent.");
        assert!(
            out.starts_with(r#"<article class="editorial page"><div class="narr"><div class="prose">"#),
            "{out}"
        );
        assert!(out.ends_with("</div></div></article>"), "{out}");
    }

    // ── Leading kicker (eyebrow) ──────────────────────────────────────────────

    #[test]
    fn leading_h3_before_h1_becomes_eyebrow_kicker() {
        let out = render("### Showcase I\n\n# The Narrenschiff\n\nBody text.");
        assert!(
            out.contains(r#"<p class="eyebrow editorial__kicker">Showcase I</p>"#),
            "leading ### must become p.eyebrow.editorial__kicker, got: {out}"
        );
        assert!(!out.contains("<h3>"), "the kicker must NOT be emitted as an <h3>, got: {out}");
    }

    #[test]
    fn leading_kicker_sits_inside_editorial_head_above_the_h1() {
        // A `## …` + invite list is needed for `segment_html` to produce the
        // three-panel shape (header.editorial__head + intro + sections).
        let src =
            "### Showcase I\n\n# The Narrenschiff\n\n*A lede.*\n\nBody text.\n\n## Ways in\n\n- [A](/a) \u{2014} gloss";
        let out = render(src);
        let head_open = out
            .find(r#"<header class="editorial__head">"#)
            .expect("three-panel shape must produce a header.editorial__head");
        let kicker = out
            .find(r#"<p class="eyebrow editorial__kicker">"#)
            .expect("kicker must be present");
        let h1 = out.find("<h1>").expect("h1 must be present");
        let head_close = out.find("</header>").expect("header must close");
        assert!(
            head_open < kicker && kicker < h1 && h1 < head_close,
            "kicker <p> must sit inside header.editorial__head, above the <h1>, got: {out}"
        );
    }

    /// The `narrenschiff`/`woodcuts` shape: an author HTML comment on the
    /// first line, then the kicker, then the h1. `render_editorial`'s
    /// raw-HTML filter strips the comment's `Event::Html` payload but leaves
    /// its `HtmlBlock` markers in the stream, so leading-block detection must
    /// tolerate events that render to nothing.
    #[test]
    fn leading_kicker_after_html_comment_still_becomes_kicker() {
        let src = "<!-- an author note -->\n\n### Showcase I\n\n# The Narrenschiff\n\nBody text.";
        let out = render(src);
        assert!(
            out.contains(r#"<p class="eyebrow editorial__kicker">Showcase I</p>"#),
            "a comment-prefixed leading ### must still become the kicker, got: {out}"
        );
        assert!(!out.contains("<h3>"), "the kicker must NOT be emitted as an <h3>, got: {out}");
    }

    #[test]
    fn mid_document_h3_stays_a_plain_h3() {
        let src = "# Title\n\nBody text.\n\n### Deep section\n\nMore prose.";
        let out = render(src);
        assert!(
            out.contains("<h3>Deep section</h3>"),
            "a mid-document ### must stay a plain <h3>, got: {out}"
        );
        assert!(
            !out.contains("editorial__kicker"),
            "a mid-document ### must NOT become a kicker, got: {out}"
        );
    }

    #[test]
    fn leading_h3_not_followed_by_h1_stays_a_plain_h3() {
        let src = "### Deep\n\nSome prose.\n\n# Title";
        let out = render(src);
        assert!(
            out.contains("<h3>Deep</h3>"),
            "a leading ### not followed by the h1 must stay an <h3>, got: {out}"
        );
        assert!(
            !out.contains("editorial__kicker"),
            "a leading ### not followed by the h1 must NOT become a kicker, got: {out}"
        );
    }

    /// The kicker must not consume the lede or drop-cap slot: the emphasis
    /// paragraph after the h1 is still the lede, and the first body
    /// paragraph is still the drop-cap open.
    #[test]
    fn kicker_does_not_consume_lede_or_dropcap() {
        let src = "### Showcase I\n\n# The Narrenschiff\n\n*A lede.*\n\n**Brant**'s satire.";
        let out = render(src);
        assert!(
            out.contains(r#"class="editorial__lede""#),
            "lede must still be recognised after a kicker, got: {out}"
        );
        assert!(
            out.contains(r#"class="editorial__open""#),
            "drop-cap open must still be recognised after a kicker, got: {out}"
        );
    }

    // ── Subtitle (### directly after the h1) ──────────────────────────────────

    #[test]
    fn h3_directly_after_h1_becomes_subtitle_inside_head() {
        let src = "### Kicker\n\n# Title\n\n### The subtitle\n\n*A lede.*\n\n**Brant**'s satire.\n\n## Ways in\n\n- [A](/a) \u{2014} gloss";
        let out = render(src);
        let h1_end = out.find("</h1>").expect("h1");
        let sub = out
            .find(r#"<p class="editorial__subtitle">The subtitle</p>"#)
            .expect("subtitle p");
        let head_close = out.find("</header>").expect("header closes");
        let lede = out.find(r#"class="editorial__lede""#).expect("lede");
        assert!(
            h1_end < sub && sub < head_close && head_close < lede,
            "subtitle must follow the h1 inside the head, lede after it, got: {out}"
        );
        assert!(!out.contains("<h3>"), "no <h3> for the subtitle, got: {out}");
        assert!(
            out.contains(r#"class="editorial__open""#),
            "drop-cap open must be unaffected, got: {out}"
        );
    }

    #[test]
    fn subtitle_without_lede_leaves_body_paragraph_as_dropcap() {
        let out = render("# Title\n\n### x\n\nBody text.");
        assert!(out.contains(r#"<p class="editorial__subtitle">x</p>"#), "got: {out}");
        assert!(!out.contains("editorial__lede"), "got: {out}");
        assert!(out.contains(r#"<p class="editorial__open">"#), "got: {out}");
    }

    #[test]
    fn h3_not_directly_after_h1_stays_a_plain_h3() {
        let out = render("# Title\n\nBody text.\n\n### x\n\nMore.");
        assert!(out.contains("<h3>x</h3>"), "got: {out}");
        assert!(!out.contains("editorial__subtitle"), "got: {out}");
    }

    #[test]
    fn head_orders_kicker_h1_subtitle() {
        let out = render("### Kicker\n\n# Title\n\n### Sub\n\n*A lede.*\n\n## Ways in\n\n- [A](/a) \u{2014} gloss");
        let start = out.find(r#"<header class="editorial__head">"#).expect("head");
        let end = out.find("</header>").expect("head closes");
        let head = &out[start..end];
        let k = head.find("editorial__kicker").expect("kicker");
        let h = head.find("<h1").expect("h1");
        let s = head.find(r#"<p class="editorial__subtitle">Sub</p>"#).expect("subtitle");
        assert!(k < h && h < s, "got: {head}");
    }

    #[test]
    fn subtitle_as_last_block_does_not_panic() {
        let out = render("# T\n\n### S");
        let end = out.find("</header>").expect("head closes");
        let sub = out.find(r#"<p class="editorial__subtitle">S</p>"#).expect("subtitle");
        assert!(sub < end, "subtitle in head, got: {out}");
    }

    #[test]
    fn second_h3_after_subtitle_is_plain() {
        let out = render("# T\n\n### S\n\n### Not a subtitle");
        assert!(out.contains("<h3>Not a subtitle</h3>"), "got: {out}");
        assert_eq!(out.matches("editorial__subtitle").count(), 1, "got: {out}");
    }

    #[test]
    fn subtitle_then_plain_h2_without_invite_list() {
        let out = render("# T\n\n### S\n\n## Plain h2");
        assert!(out.contains(r#"<p class="editorial__subtitle">S</p>"#), "got: {out}");
        assert!(out.contains(r#"<h2 id="sec-1">Plain h2</h2>"#), "got: {out}");
    }

    #[test]
    fn h3_after_a_later_h1_is_a_plain_h3() {
        let out = render("# One\n\nBody.\n\n# Two\n\n### Not subtitle\n\nMore.");
        assert!(out.contains("<h3>Not subtitle</h3>"), "got: {out}");
        assert!(!out.contains("editorial__subtitle"), "got: {out}");
    }

    // ── Snapshot: Alice EN ────────────────────────────────────────────────────

    #[test]
    fn snapshot_editorial_alice_en() {
        // A copy of Alice's `narrative/home.en.md`, taken from dsp-incubator.
        let src = include_str!("testdata/alice-home.en.md");
        let out = render(src);
        insta::assert_snapshot!(out);
    }

    // ── Snapshot: Alice DE ────────────────────────────────────────────────────

    #[test]
    fn snapshot_editorial_alice_de() {
        // A copy of Alice's `narrative/home.de.md`, taken from dsp-incubator.
        let src = include_str!("testdata/alice-home.de.md");
        let out = render(src);
        insta::assert_snapshot!(out);
    }

    // ── Snapshot: Incunabula home DE (leading kicker) ─────────────────────────

    #[test]
    fn snapshot_editorial_incunabula_home_de() {
        // A copy of Incunabula's `narrative/home.de.md`, taken from
        // dsp-incubator: an instance of the leading
        // `###` kicker convention (Alice's narratives have none, so the two
        // Alice snapshots above are the "unchanged" control).
        let src = include_str!("testdata/incunabula-home.de.md");
        let out = render(src);
        insta::assert_snapshot!(out);
    }

    // ── split_epigraph_attribution unit test ─────────────────────────────────
    //
    // This is a pure-text helper tested here. The production path uses
    // `split_epigraph_html` (operates on rendered HTML), but the attribution
    // splitting logic is easiest to verify against plain text.

    /// Test-only helper that splits plain text on ` — ` (em-dash U+2014).
    fn split_epigraph_attribution(text: &str) -> (&str, Option<&str>) {
        let em_dash = "\u{2014}";
        let sep = format!(" {em_dash} ");
        if let Some(pos) = text.rfind(&sep) {
            let body = text[..pos].trim_end();
            let attr = text[pos + sep.len()..].trim();
            if attr.is_empty() {
                (text.trim(), None)
            } else {
                (body, Some(attr))
            }
        } else {
            (text.trim(), None)
        }
    }

    #[test]
    fn split_attribution_present() {
        let (body, attr) = split_epigraph_attribution("We're all mad here. — the Cheshire Cat");
        assert_eq!(body, "We're all mad here.");
        assert_eq!(attr, Some("the Cheshire Cat"));
    }

    #[test]
    fn split_attribution_absent() {
        let (body, attr) = split_epigraph_attribution("A plain quote without attribution.");
        assert_eq!(body, "A plain quote without attribution.");
        assert_eq!(attr, None);
    }
}
