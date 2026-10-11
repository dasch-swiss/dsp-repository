//! The shared `.pg` one-at-a-time paging primitive (`← n / N →`).
//!
//! `.pg` is used by the media plate (`component::media`)
//! and by the compound-viewer's foot nav. This
//! module owns ONLY the shared `override_query_param` href-composition
//! helper — it does NOT own query-param semantics, which stay
//! per-caller. The record viewer uses no shared pager markup: `media.rs` hand-
//! rolls its own `nav.pg` / `.pg__step` / `.pg__count` markup, because its
//! ends stay real `<a>` elements toggled reactively from `$plate` rather
//! than switching to inert `<span>`s server-side.
//!
//! There is no shared `render_pager` markup function: the
//! compound-viewer's active steps carry a companion `@get(...)` expression a
//! shared signature would have no parameter for, so `compound_viewer.rs` has its
//! own (`render_compound_pager`, alongside its own `.pg` markup).
//! `override_query_param` is unaffected — it stays shared with
//! `component::media` (`?img=`).
//!
//! Inert ends: at the first/last item the step is a
//! `span.pg__step[aria-disabled][aria-hidden]` (no wrap-around); `.pg__count`
//! + the `nav` `aria-label` carry the sequence extent for screen readers.

/// Build a query string with `key` set to `value`, preserving all other
/// params. If `key` already appears it is replaced; otherwise it is
/// appended. Empty pairs in `current_query` are skipped.
///
/// Shared between `component::media` (`?img=`) and `component::compound_viewer`
/// (`?member=`). `component::resource_list`'s own `override_query_param` (a
/// third copy — `?sort=`/`?page=`) is separate.
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

    /// `view=grid` survives a `page=N` override (each
    /// query-param-preserving helper needs its own explicit assertion).
    #[test]
    fn override_query_param_preserves_view() {
        let qs = override_query_param(Some("view=grid&page=1"), "page", "2");
        assert_eq!(qs, "view=grid&page=2");
    }
}
