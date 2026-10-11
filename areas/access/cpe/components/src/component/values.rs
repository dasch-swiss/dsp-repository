//! Value text: the language pick, the resource title, keyword splitting, date
//! text and composed-part text that the record, the browse rows, the reader,
//! the dataset panel and the showcases read from a resource's properties.
//!
//! A leaf of the component modules: depends on `chrome`, `labels`, `ir`, `lang` and
//! `util`, and must not import another component, since the record, the list,
//! the reader and the panels all depend on it. Text only: a helper returning
//! `Markup` (`resource_detail::render_value`) stays with its renderer.

use cpe_core::ir::{Calendar, DatePrecision, Property, PropertyValue, Resource};
use cpe_core::labels::ResolvedGlossPart;
use cpe_core::lang::Lang;

use crate::chrome::ChromeMsg;

// ── Language-pick helpers ─────────────────────────────────────────────────────

/// Select which property values to display for the given language.
///
/// For `text`/`richtext`: follow the three-step fallback (exact lang →
/// universal null-lang → first-present). Other value types always render all
/// values.
///
/// EAV invariant assumed here: all values of a given property share one
/// value_type. A mixed-type property vec is a schema violation; we test for
/// text-ness on the first matching value type and apply the same branch
/// uniformly.
pub(crate) fn pick_values<'a>(property: &'a Property, lang: &Lang) -> Vec<&'a PropertyValue> {
    // Check if this is a text/richtext property (any value in the vec).
    let is_text_type = property
        .values
        .iter()
        .any(|v| matches!(v, PropertyValue::Text { .. } | PropertyValue::Richtext { .. }));

    if !is_text_type {
        // Non-text: render all values in seq order (data order).
        return property.values.iter().collect();
    }

    // Step 1: values with exact lang match.
    let exact: Vec<&PropertyValue> = property
        .values
        .iter()
        .filter(|v| match v {
            PropertyValue::Text { lang: l, .. } | PropertyValue::Richtext { lang: l, .. } => {
                l.as_deref() == Some(lang.as_str())
            }
            _ => false,
        })
        .collect();
    if !exact.is_empty() {
        return exact;
    }

    // Step 2: universal values (lang: None).
    let universal: Vec<&PropertyValue> = property
        .values
        .iter()
        .filter(|v| match v {
            PropertyValue::Text { lang: l, .. } | PropertyValue::Richtext { lang: l, .. } => l.is_none(),
            _ => false,
        })
        .collect();
    if !universal.is_empty() {
        return universal;
    }

    // Step 3: first-present fallback — all values.
    property.values.iter().collect()
}

// ── Title resolution ──────────────────────────────────────────────────────────

/// Resolve the display title for a resource to a borrowed `&str`: the first
/// non-empty language-picked text/richtext value of `title_property`.
///
/// Exposed so the breadcrumb renderer and the carousel fragment handler can
/// reuse the same title-resolution logic without constructing a
/// `ResourceDetailCfg`. Returns `resource.id` when no title property is
/// configured or produces no value.
pub fn resolve_title_str<'a>(resource: &'a Resource, title_property: Option<&str>, lang: &Lang) -> &'a str {
    let Some(prop_name) = title_property else {
        return &resource.id;
    };

    let Some(prop) = resource.properties.iter().find(|p| p.name == prop_name) else {
        return &resource.id;
    };

    let picked = pick_values(prop, lang);
    for v in picked {
        if let PropertyValue::Text { text, .. } | PropertyValue::Richtext { text, .. } = v {
            if !text.is_empty() {
                return text;
            }
        }
    }

    &resource.id
}

// ── Keyword split ───────────────────────────────────────────────────────────

/// Split a keyword label into (path_parts, leaf).
///
/// Splits on space-padded separators: U+2013 EN DASH `–`, U+2212 MINUS `−`,
/// U+002D HYPHEN `-`. The last segment becomes the leaf; all preceding segments
/// become path spans.
///
/// Examples:
/// - `"Fantasy – Surrealism"` → `(["Fantasy"], "Surrealism")`
/// - `"Theme – Fantasy – Surrealism"` → `(["Theme", "Fantasy"], "Surrealism")`
/// - `"Protagonist"` → `([], "Protagonist")`
pub(crate) fn split_keyword_label(label: &str) -> (Vec<&str>, &str) {
    // Match the separator pattern: one or more whitespace, separator char,
    // one or more whitespace.
    // We split manually using find on the separator patterns to avoid regex.
    // Separators (in order of check): " – " (U+2013), " − " (U+2212), " - ".
    // We collect all split positions and split at the rightmost one for
    // a rsplitn(2, …) style, but we want ALL splits — collect all segments.
    let parts = split_on_separators(label);
    let n = parts.len();
    if n <= 1 {
        // No separator found.
        (vec![], label.trim())
    } else {
        // All but the last are path; the last is the leaf.
        let leaf = parts[n - 1].trim();
        let path: Vec<&str> = parts[..n - 1].iter().map(|s| s.trim()).collect();
        (path, leaf)
    }
}

/// Split `label` on the keyword separator set (space-padded –, −, -).
///
/// Tolerates one or more whitespace characters on each side of the separator
/// character (U+2013 EN DASH, U+2212 MINUS SIGN, U+002D HYPHEN-MINUS).
///
/// Returns the segments in order. Never allocates for the common case of
/// no separator (returns a single-element vec).
fn split_on_separators(label: &str) -> Vec<&str> {
    // Separator characters: U+2013 EN DASH, U+2212 MINUS SIGN, U+002D HYPHEN-MINUS.
    // We match \s+<char>\s+ (one-or-more spaces on each side) to tolerate multi-space
    // padding while still requiring at least one space.
    const SEP_CHARS: &[char] = &['\u{2013}', '\u{2212}', '-'];

    let mut segments: Vec<&str> = Vec::new();
    let mut start = 0usize;

    'outer: loop {
        let rest = &label[start..];
        // Find the earliest separator: a sequence of spaces + sep_char + spaces.
        let mut earliest: Option<(usize, usize)> = None; // (byte offset in rest, total sep_len)

        for (byte_idx, ch) in rest.char_indices() {
            if SEP_CHARS.contains(&ch) {
                // Check that at least one space precedes this char in `rest`.
                let before = &rest[..byte_idx];
                if !before.ends_with([' ', '\t']) {
                    continue;
                }
                // Check that at least one space follows this char.
                let after_start = byte_idx + ch.len_utf8();
                let after = &rest[after_start..];
                if !after.starts_with([' ', '\t']) {
                    continue;
                }
                // Valid separator found. Measure the leading whitespace.
                let leading_ws = before.len() - before.trim_end_matches([' ', '\t']).len();
                let trailing_ws = after.len() - after.trim_start_matches([' ', '\t']).len();
                let sep_total = leading_ws + ch.len_utf8() + trailing_ws;
                let sep_start = byte_idx - leading_ws;

                match earliest {
                    None => earliest = Some((sep_start, sep_total)),
                    Some((ep, _)) if sep_start < ep => earliest = Some((sep_start, sep_total)),
                    _ => {}
                }
                // We found a candidate — but continue scanning to find the earliest one.
            }
        }

        match earliest {
            None => {
                // No more separators.
                segments.push(&label[start..]);
                break 'outer;
            }
            Some((offset, sep_len)) => {
                segments.push(&label[start..start + offset]);
                start += offset + sep_len;
            }
        }
    }

    segments
}

// ── Date formatting ───────────────────────────────────────────────────────────

/// Format a date value to its plain-text display string — the date only, no
/// precision word, no markup. The one text formatter every date surface
/// shares: the `span.v-date` value, the browse-row gloss, and the timeline
/// tooltip (Slice 4). Returns `""` for a non-`Date` value.
///
/// Decision order (Incunabula Slice 8 step 2):
/// 1. **Degenerate range** — a value that is not a range per [`cpe_core::ir::date_is_range`] (no
///    `end`, or `end` equal to `start` at the precision) renders as a single date, never `X–X`.
/// 2. **Display string** — a curated `display` whose `display_lang` is `None` or the request
///    language is emitted verbatim. A `display` written in ANOTHER language composes the long form
///    from the ISO bounds instead (`19 February 1491`, `February 1491`, `1489`, `1475–1499`), so a
///    visitor never reads another language's month name.
/// 3. **No display string** — ISO truncation at the precision (`YYYY`, `YYYY-MM`, `YYYY-MM-DD`),
///    bounds joined by an en dash.
pub(crate) fn format_date_text(value: &PropertyValue, lang: &Lang) -> String {
    let PropertyValue::Date { start, end, precision, display, display_lang, .. } = value else {
        return String::new();
    };
    let precision = *precision;
    let end = end
        .as_deref()
        .filter(|e| cpe_core::ir::date_is_range(start, Some(e), precision));

    let part: fn(&str, DatePrecision, &Lang) -> String = match display {
        Some(text) if display_lang.as_deref().is_none_or(|l| l == lang.as_str()) => {
            return text.clone();
        }
        Some(_) => format_date_part_long,
        None => |iso, precision, _| format_date_part(iso, precision),
    };
    let start_fmt = part(start, precision, lang);
    match end {
        None => start_fmt,
        Some(end) => format!("{start_fmt}\u{2013}{}", part(end, precision, lang)), // U+2013 EN DASH
    }
}

/// The chrome word shown beside a date of `precision`, if any. Julian is
/// unmarked. Julian and Gregorian share one year count, so at year
/// precision the Gregorian word carries no information; a calendar with
/// its own year count (Islamic / AH) must show its word at every precision.
fn calendar_word(calendar: Calendar, precision: DatePrecision) -> Option<ChromeMsg> {
    match calendar {
        Calendar::Julian => None,
        Calendar::Gregorian if precision == DatePrecision::Year => None,
        Calendar::Gregorian => Some(ChromeMsg::CalendarGregorian),
    }
}

/// Append ` · <calendar word>` to `text` when [`calendar_word`] has one. The
/// single place a calendar word is rendered.
pub(crate) fn with_calendar_word(
    text: String,
    calendar: Option<Calendar>,
    precision: DatePrecision,
    lang: &Lang,
) -> String {
    match calendar.and_then(|c| calendar_word(c, precision)) {
        Some(msg) => format!("{text} · {}", msg.text(lang)),
        None => text,
    }
}

/// Format a single ISO date bound (`YYYY-MM-DD`) to the display precision by
/// truncation (`YYYY`, `YYYY-MM`, or whole).
fn format_date_part(date_iso: &str, precision: DatePrecision) -> String {
    precision.truncate(date_iso).to_owned()
}

/// Format a single ISO date bound in the localized long form
/// (`19 February 1491` / `19. Februar 1491`, `February 1491`, `1491`). A bound
/// whose month or day does not parse falls back to [`format_date_part`].
fn format_date_part_long(date_iso: &str, precision: DatePrecision, lang: &Lang) -> String {
    let year = DatePrecision::Year.truncate(date_iso);
    if precision == DatePrecision::Year {
        return year.to_owned();
    }
    let month = date_iso
        .get(5..7)
        .and_then(|m| m.parse::<u32>().ok())
        .and_then(|m| crate::chrome::month_name(m, lang));
    let day = match precision {
        DatePrecision::Day => date_iso.get(8..10).and_then(|d| d.parse::<u32>().ok()),
        _ => None,
    };
    match (month, precision, day) {
        (Some(month), DatePrecision::Month, _) => crate::chrome::date_long_text(year, month, None, lang),
        (Some(month), DatePrecision::Day, Some(day)) => crate::chrome::date_long_text(year, month, Some(day), lang),
        _ => format_date_part(date_iso, precision),
    }
}

// ── Member text ─────────────────────────────────────────────────────────────

/// The first language-picked, non-empty text of `property` on `member`.
pub(crate) fn member_text<'a>(member: &'a Resource, property: &str, lang: &Lang) -> Option<&'a str> {
    let prop = member.properties.iter().find(|p| p.name == property)?;
    pick_values(prop, lang).into_iter().find_map(|v| match v {
        PropertyValue::Text { text, .. } | PropertyValue::Richtext { text, .. } if !text.trim().is_empty() => {
            Some(text.as_str())
        }
        _ => None,
    })
}

// ── Composed parts ──────────────────────────────────────────────────────────

/// The rendered text of each composed part (a row gloss's, or a record
/// header's imprint), in part order: the part's language-picked values joined
/// with ", ", a part with no rendered value skipped. The caller joins the
/// parts. `calendar_marker` appends ` · <Gregorian word>` to a
/// Gregorian-calendar date of day or month precision (never year,
/// `calendar_word`), as the record's date value does under
/// `site { date-precision-word #true }` (never the precision word).
pub(crate) fn composed_part_texts(
    resource: &Resource,
    parts: &[ResolvedGlossPart<'_>],
    calendar_marker: bool,
    lang: &Lang,
) -> Vec<String> {
    parts
        .iter()
        .filter_map(|part| {
            let property = resource.properties.iter().find(|p| p.name == part.property)?;
            let values: Vec<String> = pick_values(property, lang)
                .into_iter()
                .filter_map(|v| gloss_part_value(v, part.unit, calendar_marker, lang))
                .filter(|s| !s.is_empty())
                .collect();
            (!values.is_empty()).then(|| values.join(", "))
        })
        .collect()
}

/// One value's text within a composed part. A number is substituted
/// into the part's unit template (`{n}` only, as-is: no plural handling, and
/// no digit grouping — the values this renders are small page counts); with
/// no template it renders as bare digits. Boolean, link and list values
/// contribute nothing (boot validation rejects them as parts). See
/// [`composed_part_texts`] for `calendar_marker`.
fn gloss_part_value(value: &PropertyValue, unit: Option<&str>, calendar_marker: bool, lang: &Lang) -> Option<String> {
    let with_unit = |digits: String| match unit {
        Some(template) => cpe_core::util::substitute_count(template, &digits),
        None => digits,
    };
    match value {
        PropertyValue::Text { text, .. } | PropertyValue::Richtext { text, .. } => Some(text.clone()),
        PropertyValue::Date { calendar, precision, .. } => {
            let text = format_date_text(value, lang);
            Some(if calendar_marker && !text.is_empty() {
                with_calendar_word(text, *calendar, *precision, lang)
            } else {
                text
            })
        }
        PropertyValue::Integer(n) => Some(with_unit(n.to_string())),
        PropertyValue::Decimal(x) => Some(with_unit(x.to_string())),
        PropertyValue::Boolean(_) | PropertyValue::Link { .. } | PropertyValue::ListValue { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_word_at_precision() {
        use DatePrecision::{Day, Month, Year};

        use crate::chrome::ChromeMsg::CalendarGregorian;
        assert_eq!(calendar_word(Calendar::Gregorian, Year), None);
        assert_eq!(calendar_word(Calendar::Gregorian, Month), Some(CalendarGregorian));
        assert_eq!(calendar_word(Calendar::Gregorian, Day), Some(CalendarGregorian));
        for p in [Day, Month, Year] {
            assert_eq!(calendar_word(Calendar::Julian, p), None);
        }
    }

    #[test]
    fn split_keyword_single_segment_is_leaf_only() {
        let (path, leaf) = split_keyword_label("Protagonist");
        assert!(path.is_empty(), "no path for single segment, got: {path:?}");
        assert_eq!(leaf, "Protagonist");
    }

    #[test]
    fn split_keyword_en_dash_two_segments() {
        let (path, leaf) = split_keyword_label("Fantasy \u{2013} Surrealism");
        assert_eq!(path, vec!["Fantasy"], "path: {path:?}");
        assert_eq!(leaf, "Surrealism");
    }

    #[test]
    fn split_keyword_en_dash_three_segments() {
        let (path, leaf) = split_keyword_label("Theme \u{2013} Fantasy \u{2013} Surrealism");
        assert_eq!(path, vec!["Theme", "Fantasy"], "path: {path:?}");
        assert_eq!(leaf, "Surrealism");
    }

    #[test]
    fn split_keyword_minus_sign_u2212() {
        let (path, leaf) = split_keyword_label("Category \u{2212} Item");
        assert_eq!(path, vec!["Category"], "path: {path:?}");
        assert_eq!(leaf, "Item");
    }

    #[test]
    fn split_keyword_hyphen_separator() {
        let (path, leaf) = split_keyword_label("Category - Item");
        assert_eq!(path, vec!["Category"], "path: {path:?}");
        assert_eq!(leaf, "Item");
    }

    /// Multi-space padding around the separator must be tolerated.
    /// `"Fantasy  –  Surrealism"` (two spaces on each side) must split into
    /// path "Fantasy" + leaf "Surrealism".
    #[test]
    fn split_keyword_en_dash_multi_space() {
        let (path, leaf) = split_keyword_label("Fantasy  \u{2013}  Surrealism");
        assert_eq!(path, vec!["Fantasy"], "path for multi-space en-dash: {path:?}");
        assert_eq!(leaf, "Surrealism", "leaf for multi-space en-dash");
    }

    /// Multi-space padding with hyphen separator must also be tolerated.
    #[test]
    fn split_keyword_hyphen_multi_space() {
        let (path, leaf) = split_keyword_label("Category  -  Item");
        assert_eq!(path, vec!["Category"], "path for multi-space hyphen: {path:?}");
        assert_eq!(leaf, "Item", "leaf for multi-space hyphen");
    }
}
