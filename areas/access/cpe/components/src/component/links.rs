//! Internal hrefs: the class → route resolver every component that links a
//! resource builds, and the one URL shape for a compound member in its
//! parent's reader.
//!
//! A leaf of the component modules: depends on `dom`, `pager`, `ir` and `util`, and
//! must not import `resource_detail`, `compound_viewer` or `annotation`, which
//! all depend on it (`tools/module-graph.py` fails the cycle). It builds hrefs
//! only; a link that needs a title or markup is composed by its caller.

use std::collections::HashMap;

use cpe_core::ir::AnnotationPartner;
use cpe_core::util::percent_encode_component;

use crate::component::dom::entry_id;
use crate::component::pager::override_query_param;

// ── LinkResolver ──────────────────────────────────────────────────────────────

/// Resolves a `target_id` + `target_class` pair into a project-relative href.
///
/// Built once per request from the project's `class_routes` map (class → route
/// template like `"/characters/:id"`). `href` substitutes the single `:param`
/// in the route template with the (URL-path-encoded) `target_id` and prefixes
/// `/{project_id}`.
///
/// Returns `None` when `target_class` is absent or has no binding in the map —
/// the caller falls back to the plain-text `span.v-text` label in that case.
///
/// This is an internal helper: it is constructed by `render_component` in
/// page assembly and is never part of the public engine surface.
#[doc(hidden)]
pub struct LinkResolver<'a> {
    project_id: &'a str,
    class_routes: &'a HashMap<String, String>,
}

impl<'a> LinkResolver<'a> {
    /// Create a new resolver. Both references must outlive the resolver (they
    /// borrow from the project which outlives the request).
    #[doc(hidden)]
    pub fn new(project_id: &'a str, class_routes: &'a HashMap<String, String>) -> Self {
        Self { project_id, class_routes }
    }

    /// Build the href for an internal link, or `None` when the class has no
    /// route binding.
    ///
    /// - `target_id` empty → `None` (avoids emitting a trailing-slash URL).
    /// - `target_class: None` → `None` (no class known, cannot route).
    /// - class absent from `class_routes` → `None` (graceful degradation).
    /// - class present → `/{project_id}<route_with_id_substituted>`.
    ///
    /// Boot invariants: `project_id` is validated `^[a-z0-9][a-z0-9-]*$` and
    /// the route template is boot-validated to contain exactly one `:param`
    /// segment — both are already URL-safe. Only `target_id` needs
    /// percent-encoding (`util::percent_encode_component`, `cpe-core`'s one
    /// encode set).
    pub(crate) fn href(&self, target_id: &str, target_class: Option<&str>) -> Option<String> {
        if target_id.is_empty() {
            return None;
        }
        let class = target_class?;
        let template = self.class_routes.get(class)?;
        let path = substitute_route_param(template, target_id);
        Some(format!("/{}{}", self.project_id, path))
    }

    /// Build the href for a compound MEMBER shown inside its parent's reader
    /// (Incunabula Slice 5 step 6, the incoming-links list's rung 2):
    /// `<parent href>?page=<member_id>`, no `#compound-viewer` anchor (owner
    /// decision 2026-09-29), or `None` when the parent's class has no detail
    /// binding.
    ///
    /// Built from a FRESH query (`member_href_for` with no base query) —
    /// never the linking page's own, whose `lang=`/`img=` belong to a
    /// different page. **Assumes** the parent class's bound detail page also
    /// hosts a `compound-viewer source="$id"`; nothing checks that pairing
    /// (on a parent page with no reader, `?page=` is ignored and the link
    /// lands on the plain record — wrong, not broken).
    pub(crate) fn member_href(&self, parent_id: &str, parent_class: &str, member_id: &str) -> Option<String> {
        if member_id.is_empty() {
            return None;
        }
        let parent_href = self.href(parent_id, Some(parent_class))?;
        Some(member_href_for(&parent_href, None, member_id))
    }

    /// Build the href of an annotation-link partner (`dsp-incubator
    /// cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md`): a compound
    /// member lands in its parent's reader on its register entry,
    /// `?page=<member>#annotation-<id>` — the partner annotation's id, or for
    /// a page partner the link's own id (its page-level entry there). Any
    /// other partner gets its own detail page, without a fragment. `None` ⇒
    /// no route at all (plain text).
    pub(crate) fn partner_href(&self, partner: &AnnotationPartner) -> Option<String> {
        let in_reader = partner.parent.as_ref().and_then(|parent| {
            let anchor = partner.annotation_id.as_deref().unwrap_or(&partner.link_id);
            self.reader_entry_href(&parent.id, &parent.class, &partner.resource_id, anchor)
        });
        in_reader.or_else(|| self.href(&partner.resource_id, Some(&partner.resource_class)))
    }

    /// The href of register entry `entry` (an annotation id, or a link id for
    /// a page-level entry) in compound member `member_id`'s reader:
    /// `?page=<member>#annotation-<entry>`, the entry percent-encoded. `None`
    /// when the parent's class has no detail binding.
    ///
    /// Every `#annotation-<id>` fragment is built here, never inside
    /// `member_href_for`: the reader's own hrefs carry no fragment.
    pub(crate) fn reader_entry_href(
        &self,
        parent_id: &str,
        parent_class: &str,
        member_id: &str,
        entry: &str,
    ) -> Option<String> {
        let href = self.member_href(parent_id, parent_class, member_id)?;
        Some(format!("{href}#{}", entry_id(&percent_encode_component(entry))))
    }
}

/// Substitute the single `:param` segment in a route template with a
/// URL-path-encoded `id`.
///
/// The map only ever contains single-`:param` routes (invariant enforced at
/// boot by `build_class_routes`). The substitution finds the `:…` segment and
/// replaces it with the percent-encoded id.
///
/// The id is encoded with `util::percent_encode_component`: every byte but
/// unreserved ASCII (`A-Z a-z 0-9 - _ . ~`) becomes `%XX`, so no id can add a
/// path segment, a query or a fragment.
fn substitute_route_param(template: &str, id: &str) -> String {
    let encoded = percent_encode_component(id);
    // Walk the segments, replacing the first (and only) `:param` segment.
    let mut out = String::with_capacity(template.len() + encoded.len());
    for segment in template.split('/') {
        out.push('/');
        if segment.starts_with(':') {
            out.push_str(&encoded);
        } else {
            out.push_str(segment);
        }
    }
    // strip the leading extra '/' added before the first (empty) segment
    out[1..].to_owned()
}

// ── Reader member links ─────────────────────────────────────────────────────

/// Compose the reader query for a link to the member with id `member_id`
/// (Incunabula Slice 2 step 7, decision 7): the inherited query with any
/// stale `member=` dropped and `page=<member_id>` set (replaced or
/// appended) via `override_query_param`. Every other param (lang,
/// appearance, …) is preserved in place.
///
/// `?member=N` is still READ (`ProjectReads::compound_parts`) so old links and
/// bookmarks keep landing on the same member; only the emitted form is
/// `page=<id>`. A leftover `member=` beside the new `page=` would be
/// harmless (page wins) but misleading, hence the drop.
///
/// **The id is percent-encoded** with `cpe-core`'s one query encode set
/// (`util::percent_encode_component`): a generic member id may carry
/// `&`/`#`/space, which written verbatim would split the pair, cut the URL at
/// the fragment, or break the value. URL-safe ids (Incunabula's `[a-z0-9-]`
/// slugs, Alice's `SCH_001`) pass through unchanged. The reader decodes the
/// value once (`query::reads::resolve_page_param` via `util::decode_query_value`)
/// before matching, so the round trip lands on the same member.
pub(crate) fn reader_query(current_query: Option<&str>, member_id: &str) -> String {
    let without_member: Vec<&str> = current_query
        .unwrap_or("")
        .split('&')
        .filter(|pair| {
            let key = pair.split_once('=').map_or(*pair, |(k, _)| k);
            !pair.is_empty() && key != "member"
        })
        .collect();
    let base = without_member.join("&");
    let encoded = percent_encode_component(member_id);
    override_query_param(Some(&base), "page", &encoded)
}

/// The ONE member-link URL shape (Incunabula Slice 5 step 6):
/// `{path}?{reader_query(base_query, member_id)}` — no `#compound-viewer`
/// fragment (owner decision 2026-09-29: no URL emitted by this codebase ever
/// carries that anchor; the element `id="compound-viewer"` itself stays, as
/// the target of the fragment-swap's programmatic focus-restore call, but no
/// href/action points at it any more).
///
/// Two callers, two base queries: the reader's own rail/prev-next links
/// (`compound_viewer::build_compound_href`) pass the CURRENT query, so
/// `lang=`/`appearance=` survive; an incoming-links row addressing a member
/// inside its parent's reader (`LinkResolver::member_href`) passes `None`, so
/// the linking page's own query (`lang=`, `img=`, …) never leaks into a link
/// to a different page.
pub(crate) fn member_href_for(path: &str, base_query: Option<&str>, member_id: &str) -> String {
    let qs = reader_query(base_query, member_id);
    format!("{path}?{qs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Layer-1: reader_query (step 7 — `page=<id>`, stale `member=` dropped) ─

    #[test]
    fn reader_query_none_query_is_page_only() {
        assert_eq!(reader_query(None, "SCH_002"), "page=SCH_002");
    }

    #[test]
    fn reader_query_clears_stale_member() {
        assert_eq!(reader_query(Some("member=5"), "SCH_002"), "page=SCH_002");
        assert!(
            !reader_query(Some("member=5&member=6"), "SCH_002").contains("member"),
            "every repeated member= must go"
        );
    }

    #[test]
    fn reader_query_preserves_lang_and_appearance_in_place() {
        assert_eq!(
            reader_query(Some("lang=fr&member=3&appearance=dark"), "SCH_004"),
            "lang=fr&appearance=dark&page=SCH_004"
        );
    }

    #[test]
    fn reader_query_replaces_existing_page_and_member() {
        assert_eq!(
            reader_query(Some("page=a1r&lang=de&member=2"), "postilla-a2v"),
            "page=postilla-a2v&lang=de"
        );
    }

    #[test]
    fn reader_query_keeps_member_prefixed_keys() {
        // Only the exact `member` key is dropped — not a key that merely
        // starts with it.
        assert_eq!(reader_query(Some("members=x&member=2"), "SCH_001"), "members=x&page=SCH_001");
    }

    #[test]
    fn reader_query_percent_encodes_reserved_chars_in_member_id() {
        let qs = reader_query(Some("lang=de&member=2"), "p&1#a b");
        assert_eq!(qs, "lang=de&page=p%261%23a%20b");
    }

    #[test]
    fn member_href_for_encodes_id_and_carries_no_fragment() {
        let href = member_href_for("/p/books/b1", None, "x&y#z");
        assert_eq!(href, "/p/books/b1?page=x%26y%23z");
        assert_eq!(
            href.matches('#').count(),
            0,
            "no `#compound-viewer` anchor (owner decision 2026-09-29)"
        );
    }

    #[test]
    fn member_href_for_none_base_query_carries_nothing_else() {
        assert_eq!(member_href_for("/p/books/b1", None, "b1-a1r"), "/p/books/b1?page=b1-a1r");
    }
}
