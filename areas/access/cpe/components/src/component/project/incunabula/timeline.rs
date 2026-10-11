//! The Incunabula timeline — geometry (Slice 4 step 3), the lanes markup
//! (step 4) and the decades markup + the 860px switch (step 5).
//!
//! The geometry functions are pure: no Maud, no `RenderContext`, no DB. Both
//! variants ([`render_timeline`] emits both, unconditionally — the CSS media
//! query, not Rust, decides which one is visible) are built on top of them;
//! styles live in the `TIMELINE` block of `static/cpe/incunabula.css`, and inline `style=` carries
//! only the data-driven numbers.
//!
//! Every number reproduces the concept mockup (`projects/incunabula/design/
//! Incunabula.dc.html` L814–864): `pct(y) = (y − start) / span × 100`, a point
//! bar `2.2 %` wide, a ranged bar `max(pct(end + 1) − left, 2.4) %`, and the
//! mockup's greedy sub-row packer with its `1.6 %` gap.
//!
//! **Range-ness is never `end.is_some()`.** Every Incunabula date stores an
//! `end`, point dates included (`'1489','1489'`), so range-ness comes from
//! [`cpe_core::ir::date_is_range`] alone, via [`marker_years`].

use cpe_core::ir::{date_is_range, Component, DatePrecision, PropertyValue, Resource, SiteConfig, TimelinePayload};
use cpe_core::lang::Lang;
use cpe_core::view_data::ComponentData;
use maud::{html, Markup};

use crate::chrome::{timeline_decade_label, timeline_extent_text, ChromeMsg};
use crate::component::context::RenderContext;
use crate::component::links::LinkResolver;
use crate::component::values::{format_date_text, resolve_title_str};

/// A point bar's width, in percent of the track (mockup: `2.2`).
const POINT_WIDTH_PCT: f64 = 2.2;
/// The minimum width of a ranged bar, in percent of the track (mockup: `2.4`).
const MIN_SPAN_WIDTH_PCT: f64 = 2.4;
/// The horizontal gap the sub-row packer keeps between two bars on one row,
/// in percent of the track (mockup: `1.6`).
const ROW_GAP_PCT: f64 = 1.6;
/// The grid-line interval, in years ("Rasterlinien alle drei Jahre").
const GRID_YEARS: f64 = 3.0;
/// The tick and decade interval, in years.
const DECADE: i32 = 10;
/// A title longer than this many chars is shortened (mockup: `54`).
const SHORT_TITLE_MAX: usize = 54;
/// A shortened title is cut to this many chars before the ellipsis (`52`).
const SHORT_TITLE_CUT: usize = 52;
/// The separator between a marker label's segments.
const LABEL_SEPARATOR: &str = " \u{00B7} "; // U+00B7 MIDDLE DOT
/// The height of one lane sub-row, in rem (mockup: `row * 1.35 + "rem"`).
const ROW_HEIGHT_REM: f64 = 1.35;
/// The label of the final lane collecting markers with no (known) lane value.
const NO_LANE_LABEL: &str = "\u{2014}"; // U+2014 EM DASH
/// The property holding a marker's title — the `name`-property convention
/// `resource_list` shares (`projects/incunabula/DESIGN.md` §10).
const TITLE_PROPERTY: &str = "name";

/// One Resource placed on the timeline.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Marker<'a> {
    /// The Resource id (the bar's href target).
    pub id: &'a str,
    /// `shortTitle · date · N Seiten` — the bar's `title` and `aria-label`.
    pub label: String,
    /// The decades variant's link text.
    pub short_title: String,
    /// The start year.
    pub year: i32,
    /// The end year — `Some` only for a genuine multi-year range (see
    /// [`marker_years`]).
    pub year_end: Option<i32>,
    /// The raw ISO date-value `date_start` string (e.g. `"1497-03-01"` or the
    /// coarser `"1495"`), used ONLY as the chronological sort key (`(date_start,
    /// id)`, see [`lay_out_lane`] / [`decade_groups`]) — never for display or
    /// geometry. A zero-padded ISO string compares chronologically as a plain
    /// string, and a coarser precision (a shorter string that is a prefix of a
    /// finer one, e.g. `"1495"` vs. `"1495-03-03"`) sorts first, matching the
    /// mockup's stable JS sort by year.
    pub date_start: String,
    /// The lane's list-node id; `None` ⇒ the `—` lane.
    pub lane: Option<&'a str>,
}

/// A marker's bar in the lanes variant. `left`/`width` are percent of the
/// track; `row` is the sub-row inside the lane; `span` marks a ranged bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Bar {
    pub left: f64,
    pub width: f64,
    pub row: usize,
    pub span: bool,
}

/// One tick on the lanes variant's axis. `width` is percent of the track;
/// `None` ⇒ `auto` (the last tick).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Tick {
    pub year: i32,
    pub width: Option<f64>,
}

/// The position of `year` on the axis, in percent: `(y − start) / span × 100`.
///
/// Computed in `f64`, so no `i32` subtraction can overflow. Boot validation
/// guarantees `axis_end − axis_start ≥ 3`, so the division is never by zero.
pub(crate) fn pct(year: i32, axis_start: i32, axis_end: i32) -> f64 {
    (f64::from(year) - f64::from(axis_start)) / (f64::from(axis_end) - f64::from(axis_start)) * 100.0
}

/// The axis ticks: every tenth year from `axis_start` while `< axis_end`, each
/// `pct(min(y + 10, axis_end)) − pct(y)` wide, then `axis_end` itself with an
/// `auto` width. For 1470–1506: `[1470, 1480, 1490, 1500, 1506]`, the first
/// three 27.78 %, 1500 16.67 % — the widths sum to exactly 100 %, so the last
/// label starts on the track's right edge. (Deliberate deviation from the
/// mockup, which gives every decade tick 27.78 %: the row then totals ~111 %
/// and flex-shrink drags every label left of its grid line.)
pub(crate) fn ticks(axis_start: i32, axis_end: i32) -> Vec<Tick> {
    let mut out: Vec<Tick> = decade_starts(axis_start, axis_end)
        .into_iter()
        .map(|year| Tick {
            year,
            width: Some(
                pct(year.saturating_add(DECADE).min(axis_end), axis_start, axis_end) - pct(year, axis_start, axis_end),
            ),
        })
        .collect();
    out.push(Tick { year: axis_end, width: None });
    out
}

/// The grid-line interval as a `background-size` percentage: `3 / span × 100`
/// (8.333… for 1470–1506).
pub(crate) fn gridline_pct(axis_start: i32, axis_end: i32) -> f64 {
    GRID_YEARS / (f64::from(axis_end) - f64::from(axis_start)) * 100.0
}

/// Parse a date value's start and end years.
///
/// The years are the first four chars of the ISO strings. Range-ness is NOT
/// decided here: `year_end` is `Some(e)` only when
/// [`date_is_range`]`(start, end, precision)` holds **and** `e != year` — the
/// year-granularity guard, since the axis is in years and a within-year range
/// is a point bar. A degenerate `end == start` (how every point book is
/// stored) therefore yields `None`. An unparsable start yields `None` for the
/// whole marker; an unparsable end of a range degrades to a point.
pub(crate) fn marker_years(start: &str, end: Option<&str>, precision: DatePrecision) -> Option<(i32, Option<i32>)> {
    let year = parse_year(start)?;
    let year_end = if date_is_range(start, end, precision) {
        end.and_then(parse_year).filter(|e| *e != year)
    } else {
        None
    };
    Some((year, year_end))
}

/// A bar's `(left, width, span)`: `left = pct(year)`; a ranged bar is
/// `max(pct(year_end + 1) − left, 2.4)` wide, a point bar `2.2`.
///
/// Correct only because [`marker_years`] has already normalised a degenerate
/// end to `None` — never compute this from a raw `PropertyValue::Date.end`.
///
/// The axis end is inclusive (`year == axis_end` is a valid marker position,
/// per `docs/CONFIG_LANGUAGE.md`/`docs/COMPONENT_LIBRARY.md`), but a bar
/// painted at `left = 100%` would otherwise extend past the track: a point's
/// fixed width pushes its right edge to `100 + POINT_WIDTH_PCT`, and a range
/// whose end is `axis_end` computes its right edge at `pct(axis_end + 1)`,
/// also past 100. Clamp rather than drop the marker: a point is shifted
/// left so it ends exactly at 100%; a range's width is capped so its right
/// edge stops at 100% (its `left` is never moved).
pub(crate) fn bar_geometry(marker: &Marker<'_>, axis_start: i32, axis_end: i32) -> (f64, f64, bool) {
    let left = pct(marker.year, axis_start, axis_end);
    match marker.year_end {
        None => {
            let width = POINT_WIDTH_PCT;
            let left = if left + width > 100.0 {
                (100.0 - width).max(0.0)
            } else {
                left
            };
            (left, width, false)
        }
        Some(e) => {
            let raw_width = (pct_after(e, 1, axis_start, axis_end) - left).max(MIN_SPAN_WIDTH_PCT);
            let width = if left + raw_width > 100.0 {
                (100.0 - left).max(0.0)
            } else {
                raw_width
            };
            (left, width, true)
        }
    }
}

/// The mockup's greedy sub-row packer, verbatim (L834–836): each bar takes the
/// first row whose recorded right edge `r` satisfies `left >= r + 1.6`, and
/// records `left + width` as that row's new right edge.
///
/// `bars` are `(left, width)` pairs, **already sorted** by the caller (by
/// `(date_start, id)`, see [`lay_out_lane`]). Returns one row index per bar.
pub(crate) fn pack_rows(bars: &[(f64, f64)]) -> Vec<usize> {
    let mut right_edges: Vec<f64> = Vec::new();
    bars.iter()
        .map(|&(left, width)| {
            let row = right_edges
                .iter()
                .position(|&right| left >= right + ROW_GAP_PCT)
                .unwrap_or(right_edges.len());
            match right_edges.get_mut(row) {
                Some(edge) => *edge = left + width,
                None => right_edges.push(left + width),
            }
            row
        })
        .collect()
}

/// One lane's bars: the markers sorted by `(date_start, id)`, each paired with
/// its [`bar_geometry`] and its [`pack_rows`] sub-row.
pub(crate) fn lay_out_lane<'m, 'a>(
    markers: &[&'m Marker<'a>],
    axis_start: i32,
    axis_end: i32,
) -> Vec<(&'m Marker<'a>, Bar)> {
    let mut sorted: Vec<&'m Marker<'a>> = markers.to_vec();
    sorted.sort_by(|a, b| (a.date_start.as_str(), a.id).cmp(&(b.date_start.as_str(), b.id)));
    let geometry: Vec<(f64, f64, bool)> = sorted.iter().map(|m| bar_geometry(m, axis_start, axis_end)).collect();
    let edges: Vec<(f64, f64)> = geometry.iter().map(|&(l, w, _)| (l, w)).collect();
    let rows = pack_rows(&edges);
    sorted
        .into_iter()
        .zip(geometry)
        .zip(rows)
        .map(|((marker, (left, width, span)), row)| (marker, Bar { left, width, row, span }))
        .collect()
}

/// The mockup's `shortTitle`: a title over 54 chars is cut to 52, stripped of
/// a trailing run of ` `, `,` and `;` (`/[ ,;]+$/`), and ended with `…`.
/// Counts `char`s, never bytes (titles carry `ö`/`ü`).
pub(crate) fn short_title(title: &str) -> String {
    if title.chars().count() <= SHORT_TITLE_MAX {
        return title.to_owned();
    }
    let cut: String = title.chars().take(SHORT_TITLE_CUT).collect();
    let mut out = cut.trim_end_matches([' ', ',', ';']).to_owned();
    out.push('\u{2026}'); // U+2026 HORIZONTAL ELLIPSIS
    out
}

/// The decades variant's groups: one per decade from `axis_start` while
/// `< axis_end`, membership by start year (`d ≤ year < d + 10`), items sorted
/// by `(date_start, id)`, empty decades dropped.
///
/// The axis end is inclusive, so a marker at exactly `year == axis_end` must
/// still land somewhere: when the last decade's own range doesn't reach past
/// `axis_end` (e.g. axis 1470–1500 buckets only up to `[1490, 1500)`), that
/// last bucket's membership test also admits `year == axis_end` — otherwise
/// such a marker appears in the lanes view (see [`bar_geometry`]) but in no
/// decade at all.
pub(crate) fn decade_groups<'m, 'a>(
    markers: &'m [Marker<'a>],
    axis_start: i32,
    axis_end: i32,
) -> Vec<(i32, Vec<&'m Marker<'a>>)> {
    let starts = decade_starts(axis_start, axis_end);
    let last_decade = starts.last().copied();
    starts
        .into_iter()
        .filter_map(|decade| {
            let next = decade.saturating_add(DECADE);
            let is_last = Some(decade) == last_decade;
            let mut items: Vec<&'m Marker<'a>> = markers
                .iter()
                .filter(|m| m.year >= decade && (m.year < next || (is_last && m.year == axis_end)))
                .collect();
            items.sort_by(|a, b| (a.date_start.as_str(), a.id).cmp(&(b.date_start.as_str(), b.id)));
            (!items.is_empty()).then_some((decade, items))
        })
        .collect()
}

/// A decade item's year span: `1475–99` (U+2013, two-digit end) for a range,
/// else the bare year.
pub(crate) fn decade_item_year(marker: &Marker<'_>) -> String {
    match marker.year_end {
        Some(end) => format!("{}\u{2013}{:02}", marker.year, end.rem_euclid(100)),
        None => marker.year.to_string(),
    }
}

/// The bar's `title`/`aria-label`: `shortTitle · date · N Seiten`, the extent
/// segment omitted when `extent` is `None` (and an empty `date_text` skipped).
///
/// `date_text` is [`format_date_text`] of the marker's date value, computed by
/// the caller — plain text, never the `span.prec` markup.
pub(crate) fn marker_label(short_title: &str, date_text: &str, extent: Option<i64>, lang: &Lang) -> String {
    let extent_text = extent.map(|n| timeline_extent_text(n, lang));
    [Some(short_title), Some(date_text), extent_text.as_deref()]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(LABEL_SEPARATOR)
}

/// Build a [`Marker`] from a Resource's parts. `None` (with a `tracing`
/// warning, never a panic) when `date` is not a `PropertyValue::Date` or its
/// start year is unparsable — the Resource is skipped, not drawn.
pub(crate) fn build_marker<'a>(
    id: &'a str,
    title: &str,
    date: &PropertyValue,
    extent: Option<i64>,
    lane: Option<&'a str>,
    lang: &Lang,
) -> Option<Marker<'a>> {
    let PropertyValue::Date { start, end, precision, .. } = date else {
        tracing::warn!(resource = id, "timeline: date value is not a date; marker skipped");
        return None;
    };
    let Some((year, year_end)) = marker_years(start, end.as_deref(), *precision) else {
        tracing::warn!(resource = id, start = %start, "timeline: unparsable start year; marker skipped");
        return None;
    };
    let short = short_title(title);
    let label = marker_label(&short, &format_date_text(date, lang), extent, lang);
    Some(Marker {
        id,
        label,
        short_title: short,
        year,
        year_end,
        date_start: start.to_owned(),
        lane,
    })
}

// ── Markup (steps 4–5) ─────────────────────────────────────────────────────────

/// The borrowed configuration of one `timeline` render: the property names
/// and axis bounds from `ir::Component::Timeline`, plus the href resolver
/// built from the request's `RenderContext` (`project_id` + `link_routes`).
pub(crate) struct TimelineCfg<'a> {
    /// The `date`-typed property positioning each marker.
    pub date_property: &'a str,
    /// The `list_value`-typed property naming each marker's lane.
    pub lane_property: &'a str,
    /// The optional `integer`-typed extent property (the label's last segment).
    pub extent_property: Option<&'a str>,
    /// Inclusive first year of the axis.
    pub axis_start: i32,
    /// Last year of the axis.
    pub axis_end: i32,
    /// Builds each bar's href from the Resource's class binding — never a
    /// hardcoded route.
    pub links: LinkResolver<'a>,
}

/// One lane ready to render: its label and its markers (unsorted — the
/// renderer lays them out via [`lay_out_lane`]).
struct Lane<'m, 'a> {
    label: &'m str,
    markers: Vec<&'m Marker<'a>>,
}

/// The timeline's registered render function (`component::project`,
/// row `("incunabula", "timeline")`): builds a [`TimelineCfg`] from the IR
/// variant and the request's `RenderContext`, then calls [`render_timeline`].
/// Renders nothing for a slot with no timeline data (the same defensive
/// fallback as the other data-carrying `render_component` arms) and for a
/// non-`Timeline` variant, which the registry never routes here.
pub(crate) fn render_registered(
    c: &Component,
    ctx: &RenderContext<'_>,
    data: &ComponentData,
    _site: &SiteConfig,
) -> Markup {
    let Component::Timeline {
        date_property,
        lane_property,
        extent_property,
        axis_start,
        axis_end,
        ..
    } = c
    else {
        return html! {};
    };
    match data {
        ComponentData::Timeline(payload) => {
            let cfg = TimelineCfg {
                date_property,
                lane_property,
                extent_property: extent_property.as_deref(),
                axis_start: *axis_start,
                axis_end: *axis_end,
                links: LinkResolver::new(ctx.project_id, ctx.link_routes),
            };
            render_timeline(payload, &cfg, &ctx.lang)
        }
        _ => html! {},
    }
}

/// Render the timeline as `section.tl.page`, containing **both** variants —
/// `div.tl__lanes` and `div.tl__decades` — unconditionally. `incunabula.css`
/// shows one and `display: none`s the other at the 860px breakpoint (decision
/// 4); no DataStar, no `hidden` attribute, no client script.
///
/// Lanes: one per vocabulary node (in `payload.lanes` = `seq` order) that has
/// at least one marker, then — when any exist — a final `—` lane for markers
/// whose lane value is missing or names a node outside the vocabulary (never
/// silently dropped). A Resource with no parsable date is skipped (with a
/// `tracing` warning). With no markers at all the root is a bare, childless
/// `section.tl.page`: the mockup designs no empty state, and none is invented.
///
/// Unlike the mockup (one lane per office, even an empty one), a vocabulary
/// node with no marker gets no lane — a deliberate choice; all five Incunabula
/// offices are non-empty, so the output is the same there.
///
/// Decades: one `div.tl__decade` per non-empty decade (membership by start
/// year), items sorted by `(date_start, id)`, using the same `href`/no-route
/// fallback as a lane's bars. No key line, no axis, no office grouping in
/// this variant (per the mockup).
pub(crate) fn render_timeline(payload: &TimelinePayload, cfg: &TimelineCfg<'_>, lang: &Lang) -> Markup {
    let resources = &payload.resources.items;
    let built: Vec<(Marker<'_>, Option<String>)> = resources
        .iter()
        .filter_map(|r| resource_marker(r, cfg, lang).map(|m| (m, cfg.links.href(&r.id, Some(&r.class)))))
        // Both variants must see the SAME marker set: drop a marker
        // outside the inclusive `[axis_start, axis_end]` once, here,
        // with the same `tracing::warn!` as the unparsable-year skip.
        // In range, `bar_geometry` keeps a bar on the 0–100% track and
        // `decade_groups`' last bucket admits `axis_end`.
        .filter(|(m, _)| {
            let end = m.year_end.unwrap_or(m.year);
            let in_range = m.year >= cfg.axis_start && end <= cfg.axis_end;
            if !in_range {
                tracing::warn!(
                    resource = m.id,
                    year = m.year,
                    year_end = ?m.year_end,
                    axis_start = cfg.axis_start,
                    axis_end = cfg.axis_end,
                    "timeline: marker outside axis range; marker skipped"
                );
            }
            in_range
        })
        .collect();
    if built.is_empty() {
        return html! {
            section.tl.page {}
        };
    }
    let markers: Vec<&Marker<'_>> = built.iter().map(|(m, _)| m).collect();
    // `decade_groups` takes owned `Marker`s (see step 3); the decades block
    // below borrows from this alongside `markers`.
    let owned_markers: Vec<Marker<'_>> = built.iter().map(|(m, _)| m.clone()).collect();
    let href_of =
        |id: &str| -> Option<&str> { built.iter().find(|(m, _)| m.id == id).and_then(|(_, href)| href.as_deref()) };

    let mut lanes: Vec<Lane<'_, '_>> = payload
        .lanes
        .iter()
        .filter_map(|node| {
            let members: Vec<&Marker<'_>> = markers
                .iter()
                .copied()
                .filter(|m| m.lane == Some(node.node_id.as_str()))
                .collect();
            (!members.is_empty()).then(|| Lane {
                label: node.label.as_deref().unwrap_or(node.node_id.as_str()),
                markers: members,
            })
        })
        .collect();
    let orphans: Vec<&Marker<'_>> = markers
        .iter()
        .copied()
        .filter(|m| m.lane.is_none_or(|lane| !payload.lanes.iter().any(|n| n.node_id == lane)))
        .collect();
    if !orphans.is_empty() {
        lanes.push(Lane { label: NO_LANE_LABEL, markers: orphans });
    }

    let (start, end) = (cfg.axis_start, cfg.axis_end);
    let grid_style = format!("background-size: {:.4}% 100%", gridline_pct(start, end));
    html! {
        section.tl.page {
            div.tl__lanes {
                div.tl__ticks {
                    @for tick in ticks(start, end) {
                        @match tick.width {
                            Some(w) => span.tl__tick style=(format!("width: {w:.4}%")) { (tick.year) }
                            None => span.tl__tick { (tick.year) }
                        }
                    }
                }
                @for lane in &lanes {
                    @let laid = lay_out_lane(&lane.markers, start, end);
                    @let rows = laid.iter().map(|(_, bar)| bar.row + 1).max().unwrap_or(1);
                    div.tl__lane {
                        div.tl__lane-label {
                            span.tl__lane-name { (lane.label) }
                            span.tl__lane-count { (lane.markers.len()) }
                        }
                        div.tl__track style=(format!("min-height: {:.4}rem", rows_rem(rows))) {
                            div.tl__grid style=(grid_style) {}
                            @for (marker, bar) in &laid {
                                (render_bar(marker, bar, href_of(marker.id)))
                            }
                        }
                    }
                }
                p.tl__key { (ChromeMsg::TimelineKey.text(lang)) }
            }
            div.tl__decades {
                @for (decade, items) in &decade_groups(&owned_markers, start, end) {
                    div.tl__decade {
                        span.tl__decade-label { (timeline_decade_label(*decade, lang)) }
                        ul.tl__decade-list {
                            @for marker in items {
                                li {
                                    span.tl__decade-year { (decade_item_year(marker)) }
                                    (render_decade_item(marker, href_of(marker.id)))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One bar: an empty `<a>` whose `title` and `aria-label` carry the marker's
/// label. When the Resource's class has no detail binding there is nowhere to
/// link, so the bar degrades to a `span` (`role="img"` keeps its label).
fn render_bar(marker: &Marker<'_>, bar: &Bar, href: Option<&str>) -> Markup {
    let class = if bar.span { "tl__bar tl__bar--span" } else { "tl__bar" };
    let style = format!(
        "left: {:.4}%; width: {:.4}%; top: {:.4}rem",
        bar.left,
        bar.width,
        rows_rem(bar.row)
    );
    html! {
        @match href {
            Some(href) => a   class=(class)
                href=(href)
                title=(marker.label)
                aria-label=(marker.label)
                style=(style) {}
            None => span
                class=(class)
                role="img"
                title=(marker.label)
                aria-label=(marker.label)
                style=(style) {}
        }
    }
}

/// One decade-list item's link text (the marker's `short_title`), using the
/// **same** `href` source and no-route fallback as [`render_bar`]: a real
/// `<a>` when the class has a detail binding, else a plain `<span>`.
fn render_decade_item(marker: &Marker<'_>, href: Option<&str>) -> Markup {
    html! {
        @match href {
            Some(href) => a href=(href) { (marker.short_title) }
            None => span { (marker.short_title) }
        }
    }
}

/// A Resource's [`Marker`]: the first `Date` value of the date property, the
/// first `Integer` of the extent property, the first list node of the lane
/// property, and the `name` title. `None` (with a warning) when the Resource
/// carries no date, or [`build_marker`] rejects it.
fn resource_marker<'r>(resource: &'r Resource, cfg: &TimelineCfg<'_>, lang: &Lang) -> Option<Marker<'r>> {
    let Some(date) = values_of(resource, cfg.date_property).find(|v| matches!(v, PropertyValue::Date { .. })) else {
        tracing::warn!(
            resource = %resource.id,
            property = cfg.date_property,
            "timeline: resource has no date value; marker skipped"
        );
        return None;
    };
    let extent = cfg.extent_property.and_then(|p| {
        values_of(resource, p).find_map(|v| match v {
            PropertyValue::Integer(n) => Some(*n),
            _ => None,
        })
    });
    let lane = values_of(resource, cfg.lane_property).find_map(|v| match v {
        PropertyValue::ListValue { node_id, .. } => Some(node_id.as_str()),
        _ => None,
    });
    let title = resolve_title_str(resource, Some(TITLE_PROPERTY), lang);
    build_marker(&resource.id, title, date, extent, lane, lang)
}

/// Every value of `property` on `resource`, in data order.
fn values_of<'r, 'p>(
    resource: &'r Resource,
    property: &'p str,
) -> impl Iterator<Item = &'r PropertyValue> + use<'r, 'p> {
    resource
        .properties
        .iter()
        .filter(move |p| p.name == property)
        .flat_map(|p| p.values.iter())
}

/// `rows × 1.35`, in rem. Row counts are tiny; an impossible overflow of
/// `u32` saturates rather than casting with `as`.
fn rows_rem(rows: usize) -> f64 {
    f64::from(u32::try_from(rows).unwrap_or(u32::MAX)) * ROW_HEIGHT_REM
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// `pct(year + offset)`, with the addition done in `f64` so it cannot overflow.
fn pct_after(year: i32, offset: i32, axis_start: i32, axis_end: i32) -> f64 {
    (f64::from(year) + f64::from(offset) - f64::from(axis_start)) / (f64::from(axis_end) - f64::from(axis_start))
        * 100.0
}

/// Every tenth year from `axis_start` while `< axis_end`.
fn decade_starts(axis_start: i32, axis_end: i32) -> Vec<i32> {
    let mut out = Vec::new();
    let mut year = axis_start;
    while year < axis_end {
        out.push(year);
        match year.checked_add(DECADE) {
            Some(next) => year = next,
            None => break,
        }
    }
    out
}

/// The year in an ISO date's first four chars; `None` unless all four are
/// ASCII digits.
fn parse_year(iso: &str) -> Option<i32> {
    iso.get(..4)
        .filter(|y| y.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|y| y.parse().ok())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cpe_core::ir::{LaneNode, Pagination, Property, ResourceListPage};
    use cpe_core::lang::resolve_builtin;

    use super::*;

    const START: i32 = 1470;
    const END: i32 = 1506;

    fn lang(code: &str) -> Lang {
        resolve_builtin(&[code.to_owned()])
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn date(start: &str, end: &str, precision: DatePrecision) -> PropertyValue {
        PropertyValue::Date {
            start: start.to_owned(),
            end: Some(end.to_owned()),
            precision,
            display: None,
            display_lang: None,
            calendar: None,
        }
    }

    /// The 19 Books as stored in `projects/incunabula/data.sql` (re-verified
    /// at step 3): `(id, date_start, date_end, precision, office node)`.
    /// Point dates store `end == start`.
    const BOOKS: [(&str, &str, &str, DatePrecision, &str); 19] = [
        ("bereitung", "1489", "1489", DatePrecision::Year, "office-amerbach"),
        ("brandan", "1491-02-19", "1491-02-19", DatePrecision::Day, "office-other"),
        ("de-generatione", "1487", "1490", DatePrecision::Year, "office-ysenhut"),
        (
            "itinerarius-peregrinarius",
            "1489",
            "1489",
            DatePrecision::Year,
            "office-ysenhut",
        ),
        ("itinerarius-peregrinatio", "1475", "1499", DatePrecision::Year, "office-other"),
        ("lob-der-glieder", "1492", "1492", DatePrecision::Year, "office-amerbach"),
        ("melusine", "1471-03-21", "1471-03-21", DatePrecision::Day, "office-other"),
        ("methodij", "1504", "1504", DatePrecision::Year, "office-furter"),
        (
            "narrenschiff-dt",
            "1495-03-03",
            "1495-03-03",
            DatePrecision::Day,
            "office-bergmann",
        ),
        (
            "narrenschiff-lat-august",
            "1497-08-01",
            "1497-08-01",
            DatePrecision::Day,
            "office-bergmann",
        ),
        (
            "narrenschiff-lat-maerz",
            "1497-03-01",
            "1497-03-01",
            DatePrecision::Day,
            "office-bergmann",
        ),
        ("orationes", "1492", "1492", DatePrecision::Year, "office-amerbach"),
        (
            "passio-meynrhadi",
            "1496-09-20",
            "1496-09-20",
            DatePrecision::Day,
            "office-furter",
        ),
        ("postilla", "1491-07-28", "1491-07-28", DatePrecision::Day, "office-furter"),
        ("quadragesimale", "1495", "1495", DatePrecision::Year, "office-furter"),
        ("reise", "1481", "1481", DatePrecision::Year, "office-other"),
        ("walfart", "1489", "1489", DatePrecision::Year, "office-ysenhut"),
        ("zeitgloecklein-1490", "1490", "1490", DatePrecision::Year, "office-amerbach"),
        ("zeitgloecklein-1492", "1492", "1492", DatePrecision::Year, "office-amerbach"),
    ];

    fn books() -> Vec<Marker<'static>> {
        let de = lang("de");
        BOOKS
            .iter()
            .filter_map(|&(id, s, e, p, office)| build_marker(id, id, &date(s, e, p), None, Some(office), &de))
            .collect()
    }

    fn lane<'m>(markers: &'m [Marker<'static>], office: &str) -> Vec<&'m Marker<'static>> {
        markers.iter().filter(|m| m.lane == Some(office)).collect()
    }

    fn marker(id: &'static str, year: i32, year_end: Option<i32>) -> Marker<'static> {
        Marker {
            id,
            label: String::new(),
            short_title: String::new(),
            year,
            year_end,
            date_start: year.to_string(),
            lane: None,
        }
    }

    #[test]
    fn pct_maps_the_axis_bounds_to_0_and_100() {
        assert!(close(pct(START, START, END), 0.0));
        assert!(close(pct(END, START, END), 100.0));
        assert!(close(pct(1487, START, END), 17.0 / 36.0 * 100.0));
    }

    #[test]
    fn ticks_for_1470_1506_are_four_decades_and_the_end() {
        let t = ticks(START, END);
        let years: Vec<i32> = t.iter().map(|t| t.year).collect();
        assert_eq!(years, [1470, 1480, 1490, 1500, 1506]);
        for tick in &t[..3] {
            let w = tick.width.expect("the first three ticks have a width");
            assert!(close(w, 10.0 / 36.0 * 100.0));
            assert_eq!(format!("{w:.2}"), "27.78");
        }
        // 1500 only has 1500→1506 left: clamped to the axis end.
        let last = t[3].width.expect("the 1500 tick has a width");
        assert!(close(last, 6.0 / 36.0 * 100.0));
        assert_eq!(format!("{last:.2}"), "16.67");
        assert_eq!(t[4].width, None);
        // The widths fill the track exactly — no flex shrink is needed.
        let sum: f64 = t.iter().filter_map(|t| t.width).sum();
        assert!(close(sum, 100.0));
    }

    #[test]
    fn gridline_is_three_years_of_a_36_year_axis() {
        let g = gridline_pct(START, END);
        assert!(close(g, 100.0 / 12.0));
        assert_eq!(format!("{g:.3}"), "8.333");
    }

    #[test]
    fn point_dates_stored_with_end_equal_to_start_are_points() {
        // The range-detection trap: every non-ranged book stores end == start.
        assert_eq!(marker_years("1489", Some("1489"), DatePrecision::Year), Some((1489, None)));
        assert_eq!(
            marker_years("1491-02-19", Some("1491-02-19"), DatePrecision::Day),
            Some((1491, None))
        );
    }

    #[test]
    fn ranged_dates_carry_their_end_year() {
        assert_eq!(
            marker_years("1475", Some("1499"), DatePrecision::Year),
            Some((1475, Some(1499)))
        );
        assert_eq!(
            marker_years("1487", Some("1490"), DatePrecision::Year),
            Some((1487, Some(1490)))
        );
    }

    #[test]
    fn a_within_year_range_is_a_point() {
        // A range per `date_is_range` (the days differ), but not across years.
        assert_eq!(
            marker_years("1497-03-01", Some("1497-08-01"), DatePrecision::Day),
            Some((1497, None))
        );
    }

    #[test]
    fn an_unparsable_start_yields_no_marker() {
        assert_eq!(marker_years("14x9", None, DatePrecision::Year), None);
        assert_eq!(marker_years("", None, DatePrecision::Year), None);
        assert_eq!(marker_years("+148", None, DatePrecision::Year), None);
        let v = date("n.d.", "n.d.", DatePrecision::Year);
        assert_eq!(build_marker("x", "t", &v, None, None, &lang("de")), None);
        let not_a_date = PropertyValue::Integer(1489);
        assert_eq!(build_marker("x", "t", &not_a_date, None, None, &lang("de")), None);
    }

    #[test]
    fn the_19_books_have_exactly_two_spans() {
        let markers = books();
        assert_eq!(markers.len(), 19);
        let spans: Vec<&str> = markers.iter().filter(|m| bar_geometry(m, START, END).2).map(|m| m.id).collect();
        assert_eq!(spans, ["de-generatione", "itinerarius-peregrinatio"]);
        let counts: Vec<usize> = [
            "office-amerbach",
            "office-bergmann",
            "office-furter",
            "office-ysenhut",
            "office-other",
        ]
        .iter()
        .map(|o| lane(&markers, o).len())
        .collect();
        assert_eq!(counts, [5, 3, 4, 3, 4]);
    }

    #[test]
    fn bar_geometry_of_a_point_and_a_span() {
        let (left, width, span) = bar_geometry(&marker("p", 1489, None), START, END);
        assert!(close(left, 19.0 / 36.0 * 100.0));
        assert!(close(width, 2.2));
        assert!(!span);
        // de-generatione 1487–1490: left 47.22 %, right edge pct(1491) = 58.33 %.
        let (left, width, span) = bar_geometry(&marker("s", 1487, Some(1490)), START, END);
        assert!(close(left + width, 21.0 / 36.0 * 100.0));
        assert!(close(width, 4.0 / 36.0 * 100.0));
        assert!(span);
    }

    #[test]
    fn a_narrow_span_on_a_long_axis_is_floored_at_2_4() {
        // 1470–1506's narrowest real span is 11.1 %, so pin the floor on the
        // season chart's 1766–1905 axis: 2/139 = 1.44 % → 2.4.
        let (_, width, span) = bar_geometry(&marker("s", 1800, Some(1801)), 1766, 1905);
        assert!(close(width, 2.4));
        assert!(span);
    }

    #[test]
    fn a_point_marker_at_axis_end_stays_within_0_100() {
        // The axis end is inclusive: a boundary point must not paint past
        // the track's right edge.
        let (left, width, span) = bar_geometry(&marker("p", END, None), START, END);
        assert!(!span);
        assert!(left >= 0.0);
        assert!(left + width <= 100.0 + 1e-9);
        assert!(close(left + width, 100.0));
    }

    #[test]
    fn a_range_ending_at_axis_end_is_capped_at_100() {
        let (left, width, span) = bar_geometry(&marker("s", 1495, Some(END)), START, END);
        assert!(span);
        assert!(left + width <= 100.0 + 1e-9);
        assert!(close(left + width, 100.0));
    }

    #[test]
    fn a_1500_marker_on_a_1470_1500_axis_lands_in_the_last_decade() {
        // The last decade bucket [1490, 1500) must also admit year == 1500
        // (the inclusive axis end), else the marker is dropped from the
        // decades view entirely.
        let markers = [marker("boundary", 1500, None)];
        let groups = decade_groups(&markers, 1470, 1500);
        let shape: Vec<(i32, Vec<&str>)> = groups
            .iter()
            .map(|(d, items)| (*d, items.iter().map(|m| m.id).collect()))
            .collect();
        assert_eq!(shape, [(1490, vec!["boundary"])]);
    }

    #[test]
    fn pack_rows_opens_a_new_row_only_on_collision() {
        // Row 0 ends at 2.0, so the next bar needs left >= 2.0 + 1.6.
        assert_eq!(pack_rows(&[(0.0, 2.0), (4.0, 2.0)]), [0, 0]);
        assert_eq!(pack_rows(&[(0.0, 2.0), (3.5, 2.0)]), [0, 1]);
        // A later bar takes the FIRST free row, not the newest one.
        assert_eq!(pack_rows(&[(0.0, 2.0), (3.5, 10.0), (4.0, 2.0)]), [0, 1, 0]);
        assert_eq!(pack_rows(&[]), Vec::<usize>::new());
    }

    #[test]
    fn the_three_1492_amerbach_books_take_rows_0_1_2() {
        let markers = books();
        let laid = lay_out_lane(&lane(&markers, "office-amerbach"), START, END);
        let rows: Vec<(&str, usize)> = laid.iter().map(|(m, b)| (m.id, b.row)).collect();
        assert_eq!(
            rows,
            [
                ("bereitung", 0),
                ("zeitgloecklein-1490", 1),
                ("lob-der-glieder", 0),
                ("orationes", 1),
                ("zeitgloecklein-1492", 2),
            ]
        );
    }

    #[test]
    fn the_ysenhut_span_takes_row_0_and_the_1489_books_rows_1_and_2() {
        let markers = books();
        let laid = lay_out_lane(&lane(&markers, "office-ysenhut"), START, END);
        let rows: Vec<(&str, usize, bool)> = laid.iter().map(|(m, b)| (m.id, b.row, b.span)).collect();
        assert_eq!(
            rows,
            [
                ("de-generatione", 0, true),
                ("itinerarius-peregrinarius", 1, false),
                ("walfart", 2, false),
            ]
        );
        let (_, first) = laid[0];
        assert!(close(first.left, 17.0 / 36.0 * 100.0));
        assert!(close(first.width, 4.0 / 36.0 * 100.0));
    }

    #[test]
    fn the_bergmann_lane_orders_by_full_date_not_year_then_id() {
        // narrenschiff-lat-maerz (1497-03-01) and narrenschiff-lat-august
        // (1497-08-01) share a year — an `(year, id)` sort would put August
        // before März (`"narrenschiff-lat-august" < "narrenschiff-lat-maerz"`
        // lexically), backwards. Sorting by the full `date_start` string
        // orders them correctly and this is what the row packing below
        // depends on.
        let markers = books();
        let laid = lay_out_lane(&lane(&markers, "office-bergmann"), START, END);
        let order: Vec<&str> = laid.iter().map(|(m, _)| m.id).collect();
        assert_eq!(
            order,
            ["narrenschiff-dt", "narrenschiff-lat-maerz", "narrenschiff-lat-august",],
            "DOM order must be full-date order, got: {order:?}"
        );
        let rows: Vec<(&str, usize)> = laid.iter().map(|(m, b)| (m.id, b.row)).collect();
        assert_eq!(
            rows,
            [
                ("narrenschiff-dt", 0),
                ("narrenschiff-lat-maerz", 0),
                ("narrenschiff-lat-august", 1),
            ],
            "narrenschiff-dt (1495) and narrenschiff-lat-maerz (1497-03) pack into row 0 \
             (75.0 >= 69.44+2.2+1.6); narrenschiff-lat-august (1497-08, same left as \
             März) collides with row 0's new edge and opens row 1, got: {rows:?}"
        );
    }

    #[test]
    fn short_title_keeps_titles_up_to_54_chars() {
        let t54 = "a".repeat(54);
        assert_eq!(short_title(&t54), t54);
        assert_eq!(short_title("Orationes"), "Orationes");
        assert_eq!(short_title(&"a".repeat(55)), format!("{}…", "a".repeat(52)));
    }

    #[test]
    fn short_title_of_the_three_long_real_titles() {
        // `name` values in data.sql, re-verified at step 3 (97, 93, 56 chars).
        assert_eq!(
            short_title(
                "walfart oder bilgerschafft der aller seligisten Jungfrouwen Marie \
                 inhaltend alle staat jrs lebens"
            ),
            "walfart oder bilgerschafft der aller seligisten Jung…"
        );
        assert_eq!(
            short_title(
                "Eyn biechlin sancti Methodij mar//trers. vnd bischoffs zuo partinentz. \
                 jn kriechen land (...)"
            ),
            "Eyn biechlin sancti Methodij mar//trers. vnd bischof…"
        );
        assert_eq!(
            short_title("Itinerarius sive peregrinarius Beatissime Virginis Marie"),
            "Itinerarius sive peregrinarius Beatissime Virginis M…"
        );
    }

    #[test]
    fn short_title_strips_a_trailing_run_of_space_comma_semicolon() {
        let title = format!("{}, ;tail of the title", "a".repeat(49));
        assert_eq!(short_title(&title), format!("{}…", "a".repeat(49)));
    }

    #[test]
    fn short_title_counts_chars_not_bytes() {
        // `ö` occupies bytes 51..53, straddling byte 52: a byte cut would panic.
        let title = format!("{}ö{}", "a".repeat(51), "b".repeat(10));
        assert!(!title.is_char_boundary(52));
        assert_eq!(short_title(&title), format!("{}ö…", "a".repeat(51)));
    }

    #[test]
    fn decade_groups_of_the_19_books() {
        let markers = books();
        let groups = decade_groups(&markers, START, END);
        let shape: Vec<(i32, Vec<&str>)> = groups
            .iter()
            .map(|(d, items)| (*d, items.iter().map(|m| m.id).collect()))
            .collect();
        assert_eq!(
            shape,
            [
                (1470, vec!["melusine", "itinerarius-peregrinatio"]),
                (
                    1480,
                    vec![
                        "reise",
                        "de-generatione",
                        "bereitung",
                        "itinerarius-peregrinarius",
                        "walfart",
                    ]
                ),
                (
                    1490,
                    vec![
                        "zeitgloecklein-1490",
                        "brandan",
                        "postilla",
                        "lob-der-glieder",
                        "orationes",
                        "zeitgloecklein-1492",
                        "quadragesimale",
                        "narrenschiff-dt",
                        "passio-meynrhadi",
                        "narrenschiff-lat-maerz",
                        "narrenschiff-lat-august",
                    ]
                ),
                (1500, vec!["methodij"]),
            ]
        );
    }

    #[test]
    fn decade_groups_drop_empty_decades() {
        let markers = [marker("a", 1471, None), marker("b", 1501, None)];
        let decades: Vec<i32> = decade_groups(&markers, START, END).iter().map(|(d, _)| *d).collect();
        assert_eq!(decades, [1470, 1500]);
    }

    #[test]
    fn decade_item_year_is_two_digit_for_a_range() {
        assert_eq!(decade_item_year(&marker("a", 1475, Some(1499))), "1475–99");
        assert_eq!(decade_item_year(&marker("b", 1487, Some(1490))), "1487–90");
        assert_eq!(decade_item_year(&marker("c", 1495, Some(1500))), "1495–00");
        assert_eq!(decade_item_year(&marker("d", 1489, None)), "1489");
    }

    #[test]
    fn marker_label_joins_title_date_and_extent() {
        let de = lang("de");
        let en = lang("en");
        assert_eq!(marker_label("Bereitung", "1489", Some(88), &de), "Bereitung · 1489 · 88 Seiten");
        assert_eq!(marker_label("Bereitung", "1489", Some(1), &en), "Bereitung · 1489 · 1 page");
        assert_eq!(marker_label("Bereitung", "1489", None, &de), "Bereitung · 1489");
    }

    #[test]
    fn build_marker_composes_label_from_the_shared_date_text() {
        let brandan = PropertyValue::Date {
            start: "1491-02-19".to_owned(),
            end: Some("1491-02-19".to_owned()),
            precision: DatePrecision::Day,
            display: Some("19. Februar 1491".to_owned()),
            display_lang: Some("de".to_owned()),
            calendar: None,
        };
        let title = "Die wunderbare Meerfahrt des hl. Brandan";
        let de = build_marker("brandan", title, &brandan, Some(2), Some("office-other"), &lang("de"))
            .expect("a parsable date builds a marker");
        assert_eq!(
            de.label,
            "Die wunderbare Meerfahrt des hl. Brandan · 19. Februar 1491 · 2 Seiten"
        );
        assert_eq!(de.short_title, title);
        assert_eq!(
            (de.id, de.year, de.year_end, de.lane),
            ("brandan", 1491, None, Some("office-other"))
        );
        let en =
            build_marker("brandan", title, &brandan, None, None, &lang("en")).expect("a parsable date builds a marker");
        assert_eq!(en.label, "Die wunderbare Meerfahrt des hl. Brandan · 19 February 1491");
    }

    // ── render_timeline (step 4) — small synthetic resource bags ─────────────

    /// A synthetic Book: `name` (universal text), `pub-date` (year precision,
    /// `end` stored even for a point, as in Incunabula's data), an optional
    /// `extent`, an optional `office` list value.
    fn res(id: &str, name: &str, dates: (&str, &str), extent: Option<i64>, office: Option<&str>) -> Resource {
        let mut properties = vec![
            Property {
                name: "name".to_owned(),
                values: vec![PropertyValue::Text { text: name.to_owned(), lang: None }],
            },
            Property {
                name: "pub-date".to_owned(),
                values: vec![date(dates.0, dates.1, DatePrecision::Year)],
            },
        ];
        if let Some(n) = extent {
            properties.push(Property {
                name: "extent".to_owned(),
                values: vec![PropertyValue::Integer(n)],
            });
        }
        if let Some(node) = office {
            properties.push(Property {
                name: "office".to_owned(),
                values: vec![PropertyValue::ListValue { node_id: node.to_owned(), label: None }],
            });
        }
        Resource {
            id: id.to_owned(),
            class: "Book".to_owned(),
            properties,
            representations: vec![],
        }
    }

    fn lanes(nodes: &[(&str, &str)]) -> Vec<LaneNode> {
        nodes
            .iter()
            .map(|&(id, label)| LaneNode { node_id: id.to_owned(), label: Some(label.to_owned()) })
            .collect()
    }

    fn payload(items: Vec<Resource>, lanes: Vec<LaneNode>) -> TimelinePayload {
        let total = items.len();
        TimelinePayload {
            resources: ResourceListPage {
                items,
                total,
                page: 1,
                pagination: Pagination::Limit(1000),
                unfiltered_total: None,
            },
            lanes,
        }
    }

    fn book_routes() -> HashMap<String, String> {
        HashMap::from([("Book".to_owned(), "/books/:id".to_owned())])
    }

    fn render_with(p: &TimelinePayload, routes: &HashMap<String, String>, code: &str) -> String {
        let cfg = TimelineCfg {
            date_property: "pub-date",
            lane_property: "office",
            extent_property: Some("extent"),
            axis_start: START,
            axis_end: END,
            links: LinkResolver::new("demo", routes),
        };
        render_timeline(p, &cfg, &lang(code)).into_string()
    }

    fn render(p: &TimelinePayload, code: &str) -> String {
        render_with(p, &book_routes(), code)
    }

    /// Two populated lanes (vocabulary order, not data order), an empty
    /// vocabulary node (no lane), and a marker outside the vocabulary.
    fn populated() -> TimelinePayload {
        payload(
            vec![
                res("orationes", "Orationes", ("1492", "1492"), Some(40), Some("n-a")),
                res("reise", "Reise", ("1481", "1481"), Some(1), Some("n-b")),
                res("de-gen", "De generatione", ("1487", "1490"), None, Some("n-a")),
                res("stray", "Stray", ("1500", "1500"), Some(12), Some("n-unknown")),
            ],
            lanes(&[("n-b", "Beta"), ("n-a", "Alpha"), ("n-c", "Gamma")]),
        )
    }

    #[test]
    fn snapshot_timeline_populated_de() {
        insta::assert_snapshot!(render(&populated(), "de"));
    }

    #[test]
    fn snapshot_timeline_populated_en() {
        insta::assert_snapshot!(render(&populated(), "en"));
    }

    #[test]
    fn snapshot_timeline_single_marker_lane() {
        let p = payload(
            vec![res("reise", "Reise", ("1481", "1481"), Some(88), Some("n-a"))],
            lanes(&[("n-a", "Alpha")]),
        );
        insta::assert_snapshot!(render(&p, "de"));
    }

    #[test]
    fn snapshot_timeline_three_same_year_markers() {
        let p = payload(
            vec![
                res("c-book", "C", ("1492", "1492"), None, Some("n-a")),
                res("a-book", "A", ("1492", "1492"), None, Some("n-a")),
                res("b-book", "B", ("1492", "1492"), None, Some("n-a")),
            ],
            lanes(&[("n-a", "Alpha")]),
        );
        let out = render(&p, "de");
        // Sorted by (year, id) before packing: rows 0/1/2 = a/b/c.
        let a = out.find("/books/a-book").expect("a-book bar");
        let b = out.find("/books/b-book").expect("b-book bar");
        let c = out.find("/books/c-book").expect("c-book bar");
        assert!(a < b && b < c);
        assert!(out.contains("min-height: 4.0500rem"));
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_timeline_ranged_marker() {
        let p = payload(
            vec![res(
                "itinerarius",
                "Itinerarius",
                ("1475", "1499"),
                Some(250),
                Some("n-a"),
            )],
            lanes(&[("n-a", "Alpha")]),
        );
        let out = render(&p, "en");
        assert!(out.contains(r#"class="tl__bar tl__bar--span""#));
        insta::assert_snapshot!(out);
    }

    #[test]
    fn out_of_axis_markers_are_dropped_from_both_variants() {
        // `before` (1450) predates START (1470); `after` (1520) postdates END
        // (1506). Both must be absent from the lanes view (which used to
        // place them at an unclamped, off-track `pct(year)`) AND the decades
        // view (which already dropped them implicitly via its bucket range) —
        // the point of the fix is that both variants agree on the same set.
        let p = payload(
            vec![
                res("before", "Before", ("1450", "1450"), None, Some("n-a")),
                res("in-range", "In Range", ("1481", "1481"), None, Some("n-a")),
                res("after", "After", ("1520", "1520"), None, Some("n-a")),
            ],
            lanes(&[("n-a", "Alpha")]),
        );
        let out = render(&p, "en");
        assert!(!out.contains("/books/before"));
        assert!(!out.contains("/books/after"));
        assert!(out.contains("/books/in-range"));
    }

    #[test]
    fn a_ranged_marker_whose_end_year_exceeds_the_axis_is_dropped() {
        // year (1500) is in-range on its own, but year_end (1520) is not —
        // the whole marker must be dropped, not clamped to a truncated bar.
        let p = payload(
            vec![res("overhang", "Overhang", ("1500", "1520"), None, Some("n-a"))],
            lanes(&[("n-a", "Alpha")]),
        );
        let out = render(&p, "en");
        assert!(!out.contains("/books/overhang"));
    }

    #[test]
    fn snapshot_timeline_marker_without_lane() {
        let p = payload(
            vec![
                res("reise", "Reise", ("1481", "1481"), None, Some("n-a")),
                res("loose", "Loose", ("1495", "1495"), None, None),
            ],
            lanes(&[("n-a", "Alpha")]),
        );
        let out = render(&p, "de");
        assert!(out.contains(r#"<span class="tl__lane-name">—</span>"#));
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_timeline_empty() {
        let out = render(&payload(vec![], lanes(&[("n-a", "Alpha")])), "de");
        assert_eq!(out, r#"<section class="tl page"></section>"#);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn undated_resources_leave_the_empty_branch() {
        let mut undated = res("x", "X", ("1481", "1481"), None, Some("n-a"));
        undated.properties.retain(|p| p.name != "pub-date");
        let out = render(&payload(vec![undated], lanes(&[("n-a", "Alpha")])), "en");
        assert_eq!(out, r#"<section class="tl page"></section>"#);
    }

    #[test]
    fn lanes_follow_vocabulary_order_and_skip_empty_nodes() {
        let out = render(&populated(), "en");
        let beta = out.find(">Beta<").expect("Beta lane");
        let alpha = out.find(">Alpha<").expect("Alpha lane");
        let dash = out.find(">—<").expect("the — lane");
        assert!(beta < alpha && alpha < dash);
        assert!(!out.contains("Gamma"));
        // The out-of-vocabulary marker lands in the — lane, not dropped.
        assert!(out.contains("/demo/books/stray"));
    }

    #[test]
    fn hrefs_come_from_the_class_binding() {
        let routes = HashMap::from([("Book".to_owned(), "/werke/:id".to_owned())]);
        let out = render_with(&populated(), &routes, "de");
        assert!(out.contains(r#"href="/demo/werke/reise""#));
        assert!(!out.contains("/books/"));
    }

    #[test]
    fn an_unbound_class_degrades_bars_to_labelled_spans() {
        let out = render_with(&populated(), &HashMap::new(), "de");
        assert!(!out.contains("<a "));
        assert!(out.contains(r#"<span class="tl__bar" role="img" title="Reise · 1481 · 1 Seite""#));
    }

    #[test]
    fn snapshot_timeline_decades_ranged_item() {
        // A ranged marker's decade-list item reads `1475–99` (Verified facts).
        let p = payload(
            vec![res(
                "itinerarius-peregrinatio",
                "Itinerarius sive peregrinatio",
                ("1475", "1499"),
                Some(120),
                Some("n-a"),
            )],
            lanes(&[("n-a", "Alpha")]),
        );
        let out = render(&p, "de");
        assert!(out.contains("<span class=\"tl__decade-year\">1475–99</span>"));
        insta::assert_snapshot!(out);
    }

    #[test]
    fn decades_use_the_same_href_and_no_route_fallback_as_the_bars() {
        let out = render(&populated(), "de");
        // Bound class: a real `<a>` link, text is the short title.
        assert!(out.contains(r#"<a href="/demo/books/reise">Reise</a>"#));
        let unbound = render_with(&populated(), &HashMap::new(), "de");
        // Unbound class: a plain `<span>`, same as the bar's no-route fallback.
        assert!(unbound.contains("<span>Reise</span>"));
        assert!(!unbound.contains(r#"<a href="/demo/books/reise""#));
    }

    #[test]
    fn bar_title_and_aria_label_are_identical() {
        let out = render(&populated(), "de");
        assert!(out.contains(r#"title="Orationes · 1492 · 40 Seiten" aria-label="Orationes · 1492 · 40 Seiten""#));
    }
}
