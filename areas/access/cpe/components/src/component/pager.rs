//! The shared `.pg` one-at-a-time paging primitive (`← n / N →`).
//!
//! `.pg` was introduced by the media plate (`component::media`, embedded-media
//! plan step 6) and adopted by the compound-viewer's foot nav (embedded-media
//! plan step 7), replacing the old bespoke `.compound-nav` markup. This
//! module owns ONLY the shared `override_query_param` href-composition
//! helper now — it does NOT own query-param semantics, which stay
//! per-caller. The record viewer does not call it at all: `media.rs` hand-
//! rolls its own `nav.pg` / `.pg__step` / `.pg__count` markup, because its
//! ends stay real `<a>` elements toggled reactively from `$plate` rather
//! than switching to inert `<span>`s server-side.
//!
//! **`render_pager` (the markup-shape counterpart this module used to also
//! own) was removed here** (`reader-paging-fragment-swap` plan Steps 2/3
//! cleanup): once Step 4's `data-on:click__prevent` wiring gave the
//! compound-viewer's active steps a companion `@get(...)` expression this
//! shared signature had no parameter for, `compound_viewer.rs` forked its
//! own copy (`render_compound_pager`, alongside its own `.pg` markup —
//! including `aria-live="polite" aria-atomic="true"` on `.pg__count`, Step
//! 5a; see that module's doc comment for the full a11y rationale) rather
//! than widen this shared signature, leaving this fn with zero production
//! callers. `override_query_param` is unaffected — it stays shared with
//! `component::media` (`?img=`).
//!
//! Mirrors the retired carousel's / `.compound-nav`'s inert-ends idiom
//! exactly: at the first/last item the step is a
//! `span.pg__step[aria-disabled][aria-hidden]` (no wrap-around); `.pg__count`
//! + the `nav` `aria-label` carry the sequence extent for screen readers.

/// Build a query string with `key` set to `value`, preserving all other
/// params. If `key` already appears it is replaced; otherwise it is
/// appended. Empty pairs in `current_query` are skipped.
///
/// Shared between `component::media` (`?img=`) and `component::compound_viewer`
/// (`?member=`) — was duplicated verbatim in both modules before embedded-media
/// plan step 7. `component::resource_list`'s own `override_query_param` (a
/// third, pre-existing copy — `?sort=`/`?page=`) is intentionally out of
/// scope for this unification; see BACKLOG for a possible further merge.
pub(crate) fn override_query_param(current_query: Option<&str>, key: &str, value: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut found = false;
    if let Some(q) = current_query {
        for pair in q.split('&') {
            if pair.is_empty() {
                continue;
            }
            let k = match pair.find('=') {
                Some(idx) => &pair[..idx],
                None => pair,
            };
            if k == key {
                parts.push(format!("{key}={value}"));
                found = true;
            } else {
                parts.push(pair.to_owned());
            }
        }
    }
    if !found {
        parts.push(format!("{key}={value}"));
    }
    parts.join("&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_query_param_appends_when_absent() {
        let qs = override_query_param(Some("lang=de"), "member", "2");
        assert_eq!(qs, "lang=de&member=2");
    }

    #[test]
    fn override_query_param_replaces_existing() {
        let qs = override_query_param(Some("lang=de&member=1"), "member", "3");
        assert_eq!(qs, "lang=de&member=3");
    }

    #[test]
    fn override_query_param_none_query_produces_single_param() {
        let qs = override_query_param(None, "member", "2");
        assert_eq!(qs, "member=2");
    }

    /// `view=grid` survives a `page=N` override (grid-mode plan decision 12 —
    /// each query-param-preserving helper needs its own explicit assertion).
    #[test]
    fn override_query_param_preserves_view() {
        let qs = override_query_param(Some("view=grid&page=1"), "page", "2");
        assert_eq!(qs, "view=grid&page=2");
    }
}
