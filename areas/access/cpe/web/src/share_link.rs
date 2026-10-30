//! Canonical view-URL composer for the "Copy link" control.
//!
//! This module is a pure, host-agnostic composer: it takes the current
//! request's raw query string plus a small amount of config-derived
//! context (the facet property names in rail order, and which surface is
//! being viewed) and produces the canonical query string / absolute URL a
//! visitor should be able to copy and share. It does **not** read request
//! state or environment variables, and touches no renderer.
//!
//! ## Canonicalization contract
//!
//! Two visitors who reach the same view via differently-encoded query
//! strings (different percent-encoding, `+` vs `%20`, shuffled param
//! order, repeated/duplicate keys) must get byte-identical composed
//! output. This is achieved by never passing raw query bytes through:
//! every value is decoded via [`cpe_core::util::decode_query_value`] first,
//! then re-encoded deterministically via
//! [`cpe_core::util::percent_encode_component`].
//!
//! ## Param order (fixed)
//!
//! `q` · facet params (rail order, repeated per value, values within a
//! group sorted lexicographically + deduped — see below) · `sort` ·
//! `view` · `page` (numeric, Browse-only) · `img` (1-based) · reader
//! `page=<member-id>` (resolved, caller-supplied) · `lang` (preserved
//! verbatim when the query carries it; appended last since it is never
//! part of a citable *refinement*).
//!
//! Defaults are omitted: empty `q`, an empty facet value (a hand-crafted
//! `?role=`), `sort=default` (per [`cpe_core::ir::Sort::parse_wire`]),
//! `view=list`, `page=1`, `img=1`, and the reader member on member 1 (the
//! caller passes `None`).
//!
//! ## Multi-valued facet canonicalization
//!
//! Within one facet group, selected values are sorted lexicographically
//! and deduped before being emitted — never left in raw query-string
//! appearance order. `facet_panel`'s `toggle_query_param` appends a
//! newly-toggled value to the end of the existing list, so two visitors
//! who arrive at the same set of selections via a different click order
//! would otherwise get two different (non-byte-identical) composed
//! links for what is the same logical view. Sorting is the
//! canonicalization step that closes that gap.
//!
//! Out-of-range `img=`/`member=`/`page=` values are echoed verbatim
//! (e.g. `img=999` on a 3-image record) rather than clamped — the
//! server that eventually serves the pasted link re-clamps at render
//! time, so this is a deliberately harmless pass-through, not a bug.
//!
//! ## Never included
//!
//! `accent`, `appearance`, `background`, `typeface`, `strip`, `reset`,
//! `datastar` — visitor display preferences, not properties of the view.
//! The composer achieves this by construction (allowlist: it only ever
//! looks up known keys), not by filtering a blocklist — see
//! [`NEVER_INCLUDED`] for the documented list this contract is tested
//! against.

use cpe_core::ir::Sort;
use cpe_core::util::{decode_query_value, percent_encode_component};

/// The "Never" list — query keys the composer
/// must never emit, regardless of their presence in the raw input. Not
/// consulted at runtime (the composer is an allowlist by construction —
/// it never looks these keys up), but kept as a single source of truth
/// for the unit tests that assert the contract holds.
pub const NEVER_INCLUDED: &[&str] = &[
    "accent",
    "appearance",
    "background",
    "typeface",
    "strip",
    "reset",
    "datastar",
];

/// Which URL-bearing surface the current page is. Determines which of `img` / the reader
/// `page=<member-id>` / the numeric pager `page` is meaningful — `img` is
/// Record-only, the reader member Record/Reader-only, the numeric `page`
/// Browse-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewSurface {
    /// A faceted or non-faceted resource-list browse page: `q` · facets ·
    /// `sort` · `view` · `page`.
    Browse,
    /// A resource-detail record with a media plate: adds `img` (1-based,
    /// omitted when 1), then the reader `page=<member-id>` when the record
    /// page also carries a compound-viewer.
    Record,
    /// A compound-viewer reader: adds `page=<member-id>` (the resolved
    /// member, omitted on member 1).
    Reader,
}

/// Compose the canonical query string for the current view.
///
/// `raw_query` is the current request's raw query string (no leading
/// `?`), exactly as threaded through `RenderContext.current_query`.
/// `facet_properties` are the page's facet-panel property names **in
/// rail (config) order** — the caller must supply this order explicitly;
/// the composer does not (and cannot) infer it from the query string.
///
/// Returns the composed query string with **no** leading `?` (empty
/// string when the view has no non-default state at all — e.g. an
/// unrefined browse page). Pass the result to [`compose_absolute_url`]
/// to build the full copyable link.
///
/// `reader_member` is the **resolved** compound member id on a page that
/// carries a compound-viewer, `None` on member 1 or on a page without one.
/// The raw query cannot supply it: a typed
/// name/prefix or a legacy `?member=N` is canonicalised to the id only once
/// the server has resolved it. Ignored on [`ViewSurface::Browse`].
///
/// `declared_sort_keys` are the browse list's declared `order-by` keys:
/// a `?sort=` value is kept when it is a
/// built-in `Sort::parse_wire` token OR one of these keys, else dropped.
/// Consulted on [`ViewSurface::Browse`] only — the reader/record surfaces
/// pass `&[]` (a declared key means nothing there). A plain `&[&str]`, like
/// `facet_properties`, so this module stays IR-agnostic.
pub fn compose_canonical_query(
    raw_query: Option<&str>,
    facet_properties: &[&str],
    surface: ViewSurface,
    reader_member: Option<&str>,
    declared_sort_keys: &[&str],
) -> String {
    let decoded = decode_pairs(raw_query);
    let mut parts: Vec<String> = Vec::new();

    // `q` — omitted when empty.
    if let Some(q) = first_value(&decoded, "q") {
        if !q.is_empty() {
            parts.push(encode_pair("q", q));
        }
    }

    // Facet groups, rail order, one param per group, repeated per
    // selected value. Within each group the selected values are sorted
    // lexicographically and deduped — NOT left in raw appearance order.
    // `facet_panel`'s `toggle_query_param` appends a newly-toggled value
    // to the end of the existing list, so two visitors who select the
    // same facet values in a different click order would otherwise
    // produce different (non-byte-identical) query strings for what is
    // the same logical view — a canonicalization-contract violation
    // (empty values are also dropped here, mirroring the `q` empty
    // check above — a hand-crafted `?role=` must never survive into the
    // composed link).
    for property in facet_properties.iter().copied() {
        let mut values: Vec<&str> = all_values(&decoded, property).filter(|v| !v.is_empty()).collect();
        values.sort_unstable();
        values.dedup();
        for value in values {
            parts.push(encode_pair(property, value));
        }
    }

    // `sort` — omitted when default. Reuses `Sort::parse_wire` (the same
    // closed-vocabulary parser the resource-list sort control uses) so
    // "what counts as default" can never drift between the two; a browse
    // list's declared `order-by` key is kept too — any other token is
    // a typo that falls back to "As curated" and must not be shared.
    if let Some(sort) = first_value(&decoded, "sort") {
        let declared = surface == ViewSurface::Browse && declared_sort_keys.contains(&sort);
        if Sort::parse_wire(sort).is_some() || declared {
            parts.push(encode_pair("sort", sort));
        }
    }

    // `view` — browse-only; omitted when list (the default) or absent.
    if surface == ViewSurface::Browse {
        if let Some(view) = first_value(&decoded, "view") {
            if view == "grid" {
                parts.push(encode_pair("view", view));
            }
        }
    }

    // `page` (numeric pager page) — `Browse`-only; omitted when 1, absent,
    // or unparseable. On a reader/record surface `?page=` addresses a
    // compound member instead, so a numeric
    // raw value (`?page=3` matching a member *name*) must never be echoed
    // as a pager page there — the member slot below carries the resolved id.
    if surface == ViewSurface::Browse {
        if let Some(n) = parse_one_based(first_value(&decoded, "page")) {
            if n > 1 {
                parts.push(encode_pair("page", &n.to_string()));
            }
        }
    }

    // `img` / reader `page=<member-id>` — the img/member slot. `img` is
    // 1-based, omitted when 1, Record-only. The reader member is the
    // caller-supplied RESOLVED id (`None` on member 1 or on a page with no
    // compound-viewer): emitted on `Reader` and on `Record` (a record page
    // that also carries a compound-viewer, e.g. `/books/:id`), after `img`
    // when both are present. Never on `Browse`. The legacy `member=` key is
    // read by the server but never emitted — the resolved id replaces it.
    if surface == ViewSurface::Record {
        if let Some(n) = parse_one_based(first_value(&decoded, "img")) {
            if n > 1 {
                parts.push(encode_pair("img", &n.to_string()));
            }
        }
    }
    if matches!(surface, ViewSurface::Record | ViewSurface::Reader) {
        if let Some(id) = reader_member.filter(|id| !id.is_empty()) {
            parts.push(encode_pair("page", id));
        }
    }

    // `lang` — preserved verbatim when the query carries it.
    // In practice `current_query` rarely carries `lang=`
    // (the host 303-redirects a `?lang=` override to a cookie + clean URL
    // before rendering), but if present it is a real property of what's
    // on screen and must round-trip like everything else above.
    if let Some(lang) = first_value(&decoded, "lang") {
        if !lang.is_empty() {
            parts.push(encode_pair("lang", lang));
        }
    }

    parts.join("&")
}

/// Assemble an absolute, copyable URL from an origin, a path, and an
/// already-composed canonical query string (e.g. from
/// [`compose_canonical_query`]).
///
/// `origin` is used verbatim modulo a single trailing-slash trim (e.g.
/// `"https://cpe-pbjdzenira-oa.a.run.app"` or `"http://localhost:8080"`);
/// `path` must start with `/`. `query` is appended after a `?` only when
/// non-empty — an unrefined view composes to a bare `origin + path`.
pub fn compose_absolute_url(origin: &str, path: &str, query: &str) -> String {
    let origin = origin.trim_end_matches('/');
    if query.is_empty() {
        format!("{origin}{path}")
    } else {
        format!("{origin}{path}?{query}")
    }
}

// Internal helpers

/// Split `raw_query` into `(key, value)` pairs, decoding **both** halves
/// via [`decode_query_value`] (percent- and `+`-decoding) so every
/// downstream lookup and re-encode works from canonical decoded bytes,
/// never raw ones — the load-bearing step for byte-identical output
/// across differently-encoded inputs. Preserves the pairs' original
/// order (needed for `all_values`' "value order = appearance order").
fn decode_pairs(raw_query: Option<&str>) -> Vec<(String, String)> {
    raw_query
        .unwrap_or("")
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (k, v) = match pair.find('=') {
                Some(idx) => (&pair[..idx], &pair[idx + 1..]),
                None => (pair, ""),
            };
            (decode_query_value(k), decode_query_value(v))
        })
        .collect()
}

/// First decoded value for `key`, or `None` when absent.
fn first_value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

/// Every decoded value for `key`, in appearance order.
fn all_values<'a>(pairs: &'a [(String, String)], key: &'a str) -> impl Iterator<Item = &'a str> {
    pairs.iter().filter(move |(k, _)| k == key).map(|(_, v)| v.as_str())
}

/// Parse a 1-based index param: `None`/empty/non-numeric/`0` all read as
/// absent — mirrors the host's own `query_img`/`?member=` clamping
/// convention (garbage silently reads as the first item).
fn parse_one_based(raw: Option<&str>) -> Option<u32> {
    raw.and_then(|s| s.parse::<u32>().ok()).filter(|&n| n > 0)
}

/// Build one `key=value` pair with both halves percent-encoded.
fn encode_pair(key: &str, value: &str) -> String {
    format!("{}={}", percent_encode_component(key), percent_encode_component(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    // compose_canonical_query: fixed order

    #[test]
    fn fixed_order_q_facets_sort_view_page() {
        let raw = "page=2&sort=name&role=R_01&q=hat&view=grid&theme=fantasy";
        let out = compose_canonical_query(raw.into(), &["role", "theme"], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "q=hat&role=R_01&theme=fantasy&sort=name&view=grid&page=2");
    }

    #[test]
    fn facet_params_repeated_per_value_sorted_within_group() {
        let raw = "theme=gothic&role=R_02&role=R_01&theme=fantasy";
        let out = compose_canonical_query(raw.into(), &["role", "theme"], ViewSurface::Browse, None, &[]);
        // rail order is role, theme — within each group, values are
        // sorted lexicographically regardless of raw appearance order
        // (role's R_02 appeared before R_01 in the raw query; theme's
        // gothic appeared before fantasy).
        assert_eq!(out, "role=R_01&role=R_02&theme=fantasy&theme=gothic");
    }

    #[test]
    fn facet_values_different_click_order_produce_byte_identical_output() {
        // Two visitors select the same role+theme facets in a different
        // order — `facet_panel`'s toggle appends, so the raw query-string
        // order of multi-valued facet params reflects click order, not a
        // canonical order. The composed link must not leak that.
        let visitor_a = "role=R_01&role=R_02&theme=gothic&theme=fantasy";
        let visitor_b = "role=R_02&role=R_01&theme=fantasy&theme=gothic";
        let out_a = compose_canonical_query(visitor_a.into(), &["role", "theme"], ViewSurface::Browse, None, &[]);
        let out_b = compose_canonical_query(visitor_b.into(), &["role", "theme"], ViewSurface::Browse, None, &[]);
        assert_eq!(out_a, out_b);
        assert_eq!(out_a, "role=R_01&role=R_02&theme=fantasy&theme=gothic");
    }

    #[test]
    fn facet_values_are_deduped() {
        let raw = "role=R_01&role=R_01&role=R_02";
        let out = compose_canonical_query(raw.into(), &["role"], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "role=R_01&role=R_02");
    }

    #[test]
    fn empty_facet_value_is_dropped() {
        // A hand-crafted `?role=` must not survive into the composed link.
        let raw = "role=&role=R_01";
        let out = compose_canonical_query(raw.into(), &["role"], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "role=R_01");
    }

    #[test]
    fn facet_property_with_only_an_empty_value_produces_no_param() {
        let out = compose_canonical_query(Some("role="), &["role"], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "");
    }

    #[test]
    fn record_surface_includes_img_not_member() {
        let raw = "img=3&member=5";
        let out = compose_canonical_query(raw.into(), &[], ViewSurface::Record, None, &[]);
        assert_eq!(out, "img=3");
    }

    #[test]
    fn reader_surface_emits_resolved_member_id_not_img_or_member() {
        // The raw `member=5` is never echoed: the resolved id replaces it.
        let raw = "img=3&member=5";
        let out = compose_canonical_query(raw.into(), &[], ViewSurface::Reader, Some("SCH_005"), &[]);
        assert_eq!(out, "page=SCH_005");
    }

    #[test]
    fn reader_surface_omits_page_on_member_one() {
        // Member 1 → the caller passes `None`; a raw `?page=` naming member 1
        // (or `member=1`) must not survive either.
        let raw = "page=a1r%3B+Titelblatt&member=1";
        let out = compose_canonical_query(raw.into(), &[], ViewSurface::Reader, None, &[]);
        assert_eq!(out, "");
    }

    #[test]
    fn record_with_compound_emits_resolved_member_id() {
        let out = compose_canonical_query(Some("page=a1r"), &[], ViewSurface::Record, Some("postilla-a1r"), &[]);
        assert_eq!(out, "page=postilla-a1r");
    }

    #[test]
    fn record_with_compound_omits_page_on_member_one() {
        let out = compose_canonical_query(Some("page=zzz"), &[], ViewSurface::Record, None, &[]);
        assert_eq!(out, "");
    }

    #[test]
    fn numeric_page_never_leaks_onto_a_reader_or_record() {
        // `?page=3` resolved (e.g. by name) to member id `book-3`: only the
        // resolved id appears — never a second, numeric pager `page=3`.
        for surface in [ViewSurface::Reader, ViewSurface::Record] {
            let out = compose_canonical_query(Some("page=3"), &[], surface, Some("book-3"), &[]);
            assert_eq!(out, "page=book-3", "surface {surface:?}");
            let out = compose_canonical_query(Some("page=3"), &[], surface, None, &[]);
            assert_eq!(out, "", "surface {surface:?} on member 1");
        }
    }

    #[test]
    fn browse_ignores_reader_member_and_keeps_numeric_page() {
        let out = compose_canonical_query(Some("page=3"), &[], ViewSurface::Browse, Some("x-1"), &[]);
        assert_eq!(out, "page=3");
    }

    #[test]
    fn reader_member_sits_after_img_and_before_lang() {
        // Pinned canonical order for Record-with-compound: img · page=<id> · lang.
        let raw = "lang=de&page=a1r&img=2";
        let out = compose_canonical_query(Some(raw), &[], ViewSurface::Record, Some("postilla-a1r"), &[]);
        assert_eq!(out, "img=2&page=postilla-a1r&lang=de");
        let out = compose_canonical_query(Some(raw), &[], ViewSurface::Reader, Some("postilla-a1r"), &[]);
        assert_eq!(out, "page=postilla-a1r&lang=de");
    }

    #[test]
    fn browse_surface_omits_both_img_and_member() {
        let raw = "img=3&member=5";
        let out = compose_canonical_query(raw.into(), &[], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "");
    }

    #[test]
    fn lang_preserved_verbatim_after_everything_else() {
        let raw = "q=hat&lang=de";
        let out = compose_canonical_query(raw.into(), &[], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "q=hat&lang=de");
    }

    // omission rules

    #[test]
    fn empty_query_composes_to_empty_string() {
        assert_eq!(compose_canonical_query(None, &[], ViewSurface::Browse, None, &[]), "");
        assert_eq!(compose_canonical_query(Some(""), &[], ViewSurface::Browse, None, &[]), "");
    }

    #[test]
    fn empty_q_is_omitted() {
        let out = compose_canonical_query(Some("q="), &[], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "");
    }

    #[test]
    fn sort_default_literal_is_omitted() {
        let out = compose_canonical_query(Some("sort=default"), &[], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "");
    }

    #[test]
    fn sort_unrecognized_value_is_omitted() {
        let out = compose_canonical_query(Some("sort=bogus"), &[], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "");
        // An undeclared token is still dropped when the list declares
        // OTHER order-by keys.
        let out = compose_canonical_query(Some("sort=bogus"), &[], ViewSurface::Browse, None, &["date", "extent"]);
        assert_eq!(out, "");
    }

    #[test]
    fn sort_recognized_non_default_is_kept() {
        let out = compose_canonical_query(Some("sort=name-desc"), &[], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "sort=name-desc");
        // A built-in stays kept alongside declared keys.
        let out = compose_canonical_query(Some("sort=name-desc"), &[], ViewSurface::Browse, None, &["date"]);
        assert_eq!(out, "sort=name-desc");
    }

    /// A browse list's declared `order-by` key is
    /// kept in the canonical query (it is a real, shareable ordering), in
    /// the fixed `sort` slot.
    #[test]
    fn sort_declared_key_is_kept_on_browse() {
        let out = compose_canonical_query(
            Some("page=2&sort=date&role=R_01"),
            &["role"],
            ViewSurface::Browse,
            None,
            &["date", "extent"],
        );
        assert_eq!(out, "role=R_01&sort=date&page=2");
    }

    /// A key that is not declared drops out of the Copy link, while a
    /// declared one (`extent`) is kept.
    #[test]
    fn sort_undeclared_key_is_dropped_declared_key_kept() {
        let declared = ["extent"];
        assert_eq!(
            compose_canonical_query(Some("sort=date"), &[], ViewSurface::Browse, None, &declared),
            ""
        );
        assert_eq!(
            compose_canonical_query(Some("sort=extent"), &[], ViewSurface::Browse, None, &declared),
            "sort=extent"
        );
    }

    /// A declared key is only meaningful on the browse surface — a
    /// record/reader composition never consults it.
    #[test]
    fn sort_declared_key_is_ignored_off_browse() {
        for surface in [ViewSurface::Record, ViewSurface::Reader] {
            let out = compose_canonical_query(Some("sort=date"), &[], surface, None, &["date"]);
            assert_eq!(out, "", "{surface:?}");
        }
    }

    #[test]
    fn view_list_is_omitted_view_grid_is_kept() {
        assert_eq!(
            compose_canonical_query(Some("view=list"), &[], ViewSurface::Browse, None, &[]),
            ""
        );
        assert_eq!(
            compose_canonical_query(Some("view=grid"), &[], ViewSurface::Browse, None, &[]),
            "view=grid"
        );
    }

    #[test]
    fn view_is_never_emitted_on_record_or_reader() {
        assert_eq!(
            compose_canonical_query(Some("view=grid"), &[], ViewSurface::Record, None, &[]),
            ""
        );
        assert_eq!(
            compose_canonical_query(Some("view=grid"), &[], ViewSurface::Reader, None, &[]),
            ""
        );
    }

    #[test]
    fn page_1_is_omitted_page_other_is_kept() {
        assert_eq!(compose_canonical_query(Some("page=1"), &[], ViewSurface::Browse, None, &[]), "");
        assert_eq!(
            compose_canonical_query(Some("page=3"), &[], ViewSurface::Browse, None, &[]),
            "page=3"
        );
    }

    #[test]
    fn page_zero_or_garbage_is_omitted() {
        assert_eq!(compose_canonical_query(Some("page=0"), &[], ViewSurface::Browse, None, &[]), "");
        assert_eq!(
            compose_canonical_query(Some("page=nope"), &[], ViewSurface::Browse, None, &[]),
            ""
        );
    }

    #[test]
    fn img_and_member_1_are_omitted() {
        assert_eq!(compose_canonical_query(Some("img=1"), &[], ViewSurface::Record, None, &[]), "");
        assert_eq!(
            compose_canonical_query(Some("member=1"), &[], ViewSurface::Reader, None, &[]),
            ""
        );
    }

    #[test]
    fn absent_facet_property_produces_no_param() {
        // "theme" is not in the rail-order list, so its raw value is never
        // looked up, even though it is present in the raw query.
        let out = compose_canonical_query(Some("theme=fantasy"), &["role"], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "");
    }

    // excluded keys never pass through

    #[test]
    fn never_included_keys_are_all_dropped() {
        let raw = NEVER_INCLUDED.iter().map(|k| format!("{k}=x")).collect::<Vec<_>>().join("&");
        let out = compose_canonical_query(Some(&raw), &[], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "", "none of {NEVER_INCLUDED:?} may ever be emitted");
    }

    #[test]
    fn unknown_unrecognized_key_is_dropped() {
        // A key that is neither a facet property, nor q/sort/view/page/
        // img/member/lang, is silently dropped (allowlist by construction).
        let out = compose_canonical_query(Some("mystery=1&q=hat"), &[], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "q=hat");
    }

    // shuffled-input ⇒ byte-identical output; q round-trips

    #[test]
    fn shuffled_param_order_produces_byte_identical_output() {
        let a = "role=R_01&q=hat&sort=name&page=2";
        let b = "page=2&sort=name&role=R_01&q=hat";
        let out_a = compose_canonical_query(Some(a), &["role"], ViewSurface::Browse, None, &[]);
        let out_b = compose_canonical_query(Some(b), &["role"], ViewSurface::Browse, None, &[]);
        assert_eq!(out_a, out_b);
    }

    #[test]
    fn q_round_trips_byte_identically_regardless_of_input_encoding() {
        // Two encodings of the same decoded value "quantum cat": %20 vs +.
        let percent = "q=quantum%20cat";
        let plus = "q=quantum+cat";
        let out_percent = compose_canonical_query(Some(percent), &[], ViewSurface::Browse, None, &[]);
        let out_plus = compose_canonical_query(Some(plus), &[], ViewSurface::Browse, None, &[]);
        assert_eq!(out_percent, out_plus);
        assert_eq!(out_percent, "q=quantum%20cat");
    }

    #[test]
    fn q_with_curly_quotes_round_trips_deterministically() {
        // A non-ASCII decoded value must re-encode to the SAME percent
        // sequence regardless of how it arrived (already-decoded literal
        // here, since Rust source can hold the UTF-8 directly).
        let raw = "q=%E2%80%9Chat%E2%80%9D"; // “hat”
        let out = compose_canonical_query(Some(raw), &[], ViewSurface::Browse, None, &[]);
        assert_eq!(out, "q=%E2%80%9Chat%E2%80%9D");
    }

    // compose_absolute_url

    #[test]
    fn absolute_url_with_query() {
        let url = compose_absolute_url(
            "https://cpe-pbjdzenira-oa.a.run.app",
            "/alice-in-daschland/characters",
            "role=R_01",
        );
        assert_eq!(
            url,
            "https://cpe-pbjdzenira-oa.a.run.app/alice-in-daschland/characters?role=R_01"
        );
    }

    #[test]
    fn absolute_url_without_query_has_no_trailing_question_mark() {
        let url = compose_absolute_url("https://cpe-pbjdzenira-oa.a.run.app", "/alice-in-daschland/characters", "");
        assert_eq!(url, "https://cpe-pbjdzenira-oa.a.run.app/alice-in-daschland/characters");
    }

    #[test]
    fn absolute_url_trims_trailing_slash_on_origin() {
        let url = compose_absolute_url("https://example.com/", "/x", "");
        assert_eq!(url, "https://example.com/x");
    }

    #[test]
    fn absolute_url_end_to_end_from_composer() {
        let query = compose_canonical_query(
            Some("theme=fantasy&role=R_01&q=hat&sort=name&page=2"),
            &["role", "theme"],
            ViewSurface::Browse,
            None,
            &[],
        );
        let url = compose_absolute_url("http://localhost:8080", "/alice-in-daschland/characters", &query);
        assert_eq!(
            url,
            "http://localhost:8080/alice-in-daschland/characters?q=hat&role=R_01&theme=fantasy&sort=name&page=2"
        );
    }
}
