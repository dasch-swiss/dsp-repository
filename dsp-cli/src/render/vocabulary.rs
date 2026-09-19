//! Shared vocabulary row-flattening + derivation helpers (plan 034 Step 3).
//!
//! Mirrors `src/render/value.rs`'s role: `build_vocabulary_list_rows` and
//! `build_vocabulary_rows` are the single implementation shared by the three
//! tabular renderers (lines/csv/tsv) for `vocabulary list` and `vocabulary
//! describe`, so no renderer inlines its own flattening. `localized_column`,
//! `join_labels_prose`, and `localized_path_segment` are also called directly
//! by `prose.rs` (D13's translation-join order and D11's path fallback are one
//! rule, used in both places). `json.rs` does NOT use `localized_column` (or
//! the other per-language collapsing helpers) — json stays lossless and
//! echoes the verbatim `labels`/`comments` arrays via its own
//! `localized_text_array`. json DOES use `nest_vocabulary_detail` (below) for
//! STRUCTURE and `number`, sharing the same `child_number` formula as the
//! tabular flattener rather than re-deriving it.
//!
//! NO sanitisation happens in this module — see `src/render/value.rs`'s
//! `render_value_content` SECURITY note for the same chokepoint discipline:
//! tabular cells are sanitised once, at `render_table`'s `QuoteMode::apply`
//! chokepoint; prose sanitises via `strip_control_chars`, applied by the
//! prose renderer itself.

use crate::model::{LocalizedText, Vocabulary, VocabularyDetail, VocabularyHeader, VocabularyNode};

/// D13's fixed language order everywhere: prose translation-join order AND
/// D11's path fallback order. `None` (untagged) is always last — callers
/// probe it separately via `localized_column(_, None)`.
const LANGUAGE_ORDER: [&str; 5] = ["en", "de", "fr", "it", "rm"];

/// Column value for one language slot (`lang: Some("en")`) or the untagged
/// slot (`lang: None`). Takes the **first** entry matching the slot — a
/// second entry sharing one tag was not observed on prod (the plan's census
/// measured distinct languages and labels-per-node, which does not strictly
/// prove absence) and cannot be represented in a fixed-column scheme; the
/// extra is dropped in tabular output, but present in json, which stays
/// lossless. `""` when no entry matches the slot.
pub(crate) fn localized_column(labels: &[LocalizedText], lang: Option<&str>) -> String {
    labels
        .iter()
        .find(|l| l.language.as_deref() == lang)
        .map(|l| l.value.clone())
        .unwrap_or_default()
}

/// Join every PRESENT translation in D13 order (`en → de → fr → it → rm`,
/// untagged last) with `" / "`, for prose. Order is fixed regardless of the
/// input array's wire order — DSP-API's label order is neither stable across
/// endpoints nor consistent within one (see the plan's Step 3 "Label join
/// order" note); rendering verbatim would make `vocabulary list` and
/// `vocabulary describe` disagree about the same vocabulary's label order.
/// `""` when `labels` is empty.
pub(crate) fn join_labels_prose(labels: &[LocalizedText]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for lang in LANGUAGE_ORDER {
        let v = localized_column(labels, Some(lang));
        if !v.is_empty() {
            parts.push(v);
        }
    }
    let untagged = localized_column(labels, None);
    if !untagged.is_empty() {
        parts.push(untagged);
    }
    parts.join(" / ")
}

/// D11: pick one label to represent a node in the `path` breadcrumb — first
/// available `en → de → fr → it → rm`, then untagged, then (only when the
/// header carries no label AT ALL) the IRI's local name.
///
/// Note the fallback-to-IRI branch triggers on `header.labels.is_empty()`
/// specifically, not on "every language probe came back empty" — those are
/// different conditions in principle (a header could carry a label whose
/// `value` is itself an empty string), and only the former is the "no label
/// at all" case the plan's D11 fallback describes.
pub(crate) fn localized_path_segment(header: &VocabularyHeader) -> String {
    if header.labels.is_empty() {
        return local_name(&header.iri);
    }
    for lang in LANGUAGE_ORDER {
        let v = localized_column(&header.labels, Some(lang));
        if !v.is_empty() {
            return v;
        }
    }
    localized_column(&header.labels, None)
}

/// The substring after the last `/` or `#` in an IRI, whichever is later; the
/// whole IRI when neither separator is present.
fn local_name(iri: &str) -> String {
    let slash = iri.rfind('/');
    let hash = iri.rfind('#');
    match slash.max(hash) {
        Some(pos) => iri[pos + 1..].to_string(),
        None => iri.to_string(),
    }
}

/// One flattened row for `vocabulary list`, in `VOCABULARIES_COLUMNS` order
/// (`name, iri, label_en..rm, label, comment_en..rm, comment, nodes, depth`).
/// `name` renders as `""` when the header carries none. `nodes`/`depth`
/// render as `""` when `None` (mirrors `resource-type list`'s `count` column
/// convention).
pub(crate) fn build_vocabulary_list_rows(items: &[Vocabulary]) -> Vec<Vec<String>> {
    items
        .iter()
        .map(|item| {
            let h = &item.header;
            let mut row = vec![h.name.clone().unwrap_or_default(), h.iri.clone()];
            for lang in LANGUAGE_ORDER {
                row.push(localized_column(&h.labels, Some(lang)));
            }
            row.push(localized_column(&h.labels, None));
            for lang in LANGUAGE_ORDER {
                row.push(localized_column(&h.comments, Some(lang)));
            }
            row.push(localized_column(&h.comments, None));
            row.push(item.node_count.map(|n| n.to_string()).unwrap_or_default());
            row.push(item.depth.map(|d| d.to_string()).unwrap_or_default());
            row
        })
        .collect()
}

/// One flattened node, carrying its computed **absolute** `number`/`path`/
/// `depth`/`parent_iri` plus a borrow of its own header — before the
/// `subtree_of` filter and (for tabular output) column-projection pass.
///
/// `pub(crate)` (not private) so `prose.rs`'s `vocabulary_describe` can walk
/// the SAME flattened+filtered row set that `build_vocabulary_rows` projects
/// into tabular columns, rather than re-deriving number/path/depth by hand or
/// reconstructing a label join from stringified table cells.
pub(crate) struct FlatVocabularyNode<'a> {
    pub(crate) header: &'a VocabularyHeader,
    /// 1-based, dotted outline position (D10). Absolute by construction —
    /// unaffected by `--subtree`, which only narrows which rows are emitted.
    pub(crate) number: String,
    /// D11 breadcrumb, ` › `-joined, rooted at the vocabulary label.
    pub(crate) path: String,
    /// Absolute depth (root's direct children are depth 1). Distinct from
    /// `VocabularyDetail.depth`, the branch-relative summary value.
    pub(crate) depth: usize,
    /// Raw, 0-based DSP-API position among siblings (untranslated).
    pub(crate) position: i32,
    /// Immediate parent's IRI — the vocabulary root's IRI for a top-level
    /// node (there is no "no parent" case at this layer).
    pub(crate) parent_iri: String,
}

/// Phase 1 + 2 of the flattener: walk the **whole** tree (every node gets an
/// absolute `number`/`path`/`depth`/`parent_iri` from its complete ancestor
/// chain — see the `Renderer::vocabulary_describe` doc comment for why this
/// must not be pre-pruned), then filter down to `detail.subtree_of`'s branch.
///
/// `None` emits every node. `Some(iri)` emits the addressed node (D14b: it is
/// included, as the top of its own branch) and its descendants, found by a
/// leading-dot-segment prefix match on the already-computed absolute
/// `number` (numbers are unique per node in a DFS-ordered tree, so this is
/// both correct and simple). If `iri` does not address any node in the tree
/// — expected not to happen; the action layer validates `--subtree`'s
/// address before calling the renderer — returns an empty `Vec` rather than
/// panicking, since this function has no way to signal an error back through
/// `Renderer`'s infallible-once-called per-row shape.
pub(crate) fn flatten_vocabulary_detail(detail: &VocabularyDetail) -> Vec<FlatVocabularyNode<'_>> {
    let mut flat: Vec<FlatVocabularyNode> = Vec::new();
    let root_path_segment = localized_path_segment(&detail.tree.root);
    walk(
        &detail.tree.children,
        "",
        &root_path_segment,
        0,
        &detail.tree.root.iri,
        &mut flat,
    );

    match &detail.subtree_of {
        None => flat,
        Some(target_iri) => {
            let target_number = flat.iter().find(|n| &n.header.iri == target_iri).map(|n| n.number.clone());
            match target_number {
                None => Vec::new(),
                Some(target_number) => {
                    let prefix = format!("{target_number}.");
                    flat.into_iter()
                        .filter(|n| n.number == target_number || n.number.starts_with(&prefix))
                        .collect()
                }
            }
        }
    }
}

/// One child's 1-based dotted `number` (D10) from its parent's already-joined
/// `number_prefix` and its own 0-based `position`. `number_prefix` is `""`
/// for a root's direct children, so their own number has no leading segment
/// (`1`, `2`, …); otherwise the child's segment is appended after a `.`.
///
/// The single implementation of D10's formula — shared by `walk` (the flat
/// tabular derivation) and `nest_children` (the nested json derivation) so
/// the two renderer families can never compute a node's `number` differently.
fn child_number(number_prefix: &str, position: i32) -> String {
    if number_prefix.is_empty() {
        (position + 1).to_string()
    } else {
        format!("{number_prefix}.{}", position + 1)
    }
}

/// Recursive DFS walker (not the untrusted-input parse path — this walks a
/// tree that already survived deserialisation in `src/client/http.rs`, which
/// is iterative; real data tops out at 9 levels).
///
/// `number_prefix`/`path_prefix` are the already-joined ancestor strings
/// (`number_prefix` is `""` for the root's direct children, so their own
/// number has no leading root segment — D10's numbers start at `1`, `2`, …;
/// `path_prefix` starts as the ROOT's own path segment, so D11's breadcrumb
/// DOES start at the vocabulary label). `depth` is the ancestor count (`0` for
/// the root's direct children, so their own depth is `1`). `parent_iri` is
/// the immediate parent's IRI.
fn walk<'a>(
    nodes: &'a [VocabularyNode],
    number_prefix: &str,
    path_prefix: &str,
    depth: usize,
    parent_iri: &str,
    out: &mut Vec<FlatVocabularyNode<'a>>,
) {
    for node in nodes {
        let number = child_number(number_prefix, node.position);
        let segment = localized_path_segment(&node.header);
        let path = if path_prefix.is_empty() {
            segment
        } else {
            format!("{path_prefix} \u{203a} {segment}")
        };
        let this_depth = depth + 1;
        out.push(FlatVocabularyNode {
            header: &node.header,
            number: number.clone(),
            path: path.clone(),
            depth: this_depth,
            position: node.position,
            parent_iri: parent_iri.to_string(),
        });
        walk(&node.children, &number, &path, this_depth, &node.header.iri, out);
    }
}

/// One flattened row per NODE for `vocabulary describe`, in
/// `VOCABULARY_DESCRIBE_COLUMNS` order (`node_iri, number, name, label_en..rm,
/// label, comment_en..rm, comment, path, position, depth, parent_iri`).
///
/// See `flatten_vocabulary_detail` for the two-phase (flatten whole tree,
/// then filter by `subtree_of`) shape this projects from.
pub(crate) fn build_vocabulary_rows(detail: &VocabularyDetail) -> Vec<Vec<String>> {
    flatten_vocabulary_detail(detail)
        .into_iter()
        .map(|n| {
            let h = n.header;
            let mut row = vec![h.iri.clone(), n.number, h.name.clone().unwrap_or_default()];
            for lang in LANGUAGE_ORDER {
                row.push(localized_column(&h.labels, Some(lang)));
            }
            row.push(localized_column(&h.labels, None));
            for lang in LANGUAGE_ORDER {
                row.push(localized_column(&h.comments, Some(lang)));
            }
            row.push(localized_column(&h.comments, None));
            row.push(n.path);
            row.push(n.position.to_string());
            row.push(n.depth.to_string());
            row.push(n.parent_iri);
            row
        })
        .collect()
}

/// One node in the NESTED tree json's `vocabulary describe` renders (plan 034
/// review fix — json is the lossless path and must preserve tree structure,
/// not flatten it). Carries only what a nested representation cannot derive
/// structurally: the absolute `number` (D10) and the raw `position`. `path`,
/// `depth`, and `parent_iri` are dropped here — they are derivable from
/// nesting/ancestry in a tree shape, unlike in the flat/tabular row shape
/// where they must be carried explicitly per row.
pub(crate) struct NestedVocabularyNode<'a> {
    pub(crate) header: &'a VocabularyHeader,
    /// 1-based, dotted outline position (D10), absolute by construction —
    /// identical formula and identical value to `FlatVocabularyNode::number`
    /// for the same node (see `child_number`).
    pub(crate) number: String,
    /// Raw, 0-based DSP-API position among siblings (untranslated).
    pub(crate) position: i32,
    pub(crate) children: Vec<NestedVocabularyNode<'a>>,
}

/// Nested counterpart to `flatten_vocabulary_detail`, for json's lossless
/// tree output. Same two-phase shape: build the WHOLE tree first (every node
/// gets its absolute `number` from `child_number`, unaffected by
/// `subtree_of`), then narrow.
///
/// `None` returns the whole forest (every top-level node, with descendants
/// nested beneath). `Some(iri)` narrows to a single-element `Vec` containing
/// the addressed node as the top of its own branch (D14b), with its
/// descendants nested beneath it and absolute `number`s retained — found by
/// searching the already-built forest for the node whose `header.iri`
/// matches. If `iri` addresses no node in the tree — expected not to happen;
/// the action layer validates `--subtree`'s address before calling the
/// renderer — returns an empty `Vec`, mirroring `flatten_vocabulary_detail`'s
/// behaviour in that case.
pub(crate) fn nest_vocabulary_detail(detail: &VocabularyDetail) -> Vec<NestedVocabularyNode<'_>> {
    let whole = nest_children(&detail.tree.children, "");
    match &detail.subtree_of {
        None => whole,
        Some(target_iri) => match take_node(whole, target_iri) {
            Some(node) => vec![node],
            None => Vec::new(),
        },
    }
}

/// Recursive builder for the whole (unfiltered) nested tree. `number_prefix`
/// is the already-joined ancestor `number` string (`""` for the root's direct
/// children — see `child_number`).
fn nest_children<'a>(nodes: &'a [VocabularyNode], number_prefix: &str) -> Vec<NestedVocabularyNode<'a>> {
    nodes
        .iter()
        .map(|node| {
            let number = child_number(number_prefix, node.position);
            let children = nest_children(&node.children, &number);
            NestedVocabularyNode {
                header: &node.header,
                number,
                position: node.position,
                children,
            }
        })
        .collect()
}

/// Depth-first search-and-take: consumes `nodes`, returning the owned subtree
/// rooted at the first node whose `header.iri == target_iri` (or `None` if
/// absent). Takes ownership (rather than borrowing) so the matched node's own
/// already-built `children` can be returned as-is, with no re-derivation.
fn take_node<'a>(nodes: Vec<NestedVocabularyNode<'a>>, target_iri: &str) -> Option<NestedVocabularyNode<'a>> {
    for node in nodes {
        if node.header.iri == target_iri {
            return Some(node);
        }
        if let Some(found) = take_node(node.children, target_iri) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str, language: Option<&str>) -> LocalizedText {
        LocalizedText {
            value: value.to_string(),
            language: language.map(|s| s.to_string()),
        }
    }

    fn header(iri: &str, name: Option<&str>, labels: Vec<LocalizedText>) -> VocabularyHeader {
        VocabularyHeader {
            iri: iri.to_string(),
            name: name.map(|s| s.to_string()),
            labels,
            comments: vec![],
        }
    }

    // ── localized_column ────────────────────────────────────────────────

    #[test]
    fn localized_column_returns_matching_language() {
        let labels = vec![text("Period", Some("en")), text("A3 Period", Some("de"))];
        assert_eq!(localized_column(&labels, Some("en")), "Period");
        assert_eq!(localized_column(&labels, Some("de")), "A3 Period");
    }

    #[test]
    fn localized_column_empty_when_absent() {
        let labels = vec![text("Period", Some("en"))];
        assert_eq!(localized_column(&labels, Some("fr")), "");
    }

    #[test]
    fn localized_column_untagged_slot_uses_none() {
        let labels = vec![text("untagged value", None), text("Period", Some("en"))];
        assert_eq!(localized_column(&labels, None), "untagged value");
    }

    #[test]
    fn localized_column_duplicate_tag_first_wins() {
        // Two entries sharing one tag — pin that the FIRST wins, per the plan's
        // explicit instruction to test this rather than leave it incidental.
        let labels = vec![text("first", Some("en")), text("second", Some("en"))];
        assert_eq!(localized_column(&labels, Some("en")), "first");
    }

    // ── join_labels_prose ───────────────────────────────────────────────

    #[test]
    fn join_labels_prose_fixed_order_regardless_of_input_order() {
        // Real endpoints return label order inconsistently — feed the SAME
        // set of labels in two different wire orders and assert identical
        // output (the regression guard for that hash-order instability).
        let order_a = vec![text("Period", Some("en")), text("A3 Period", Some("de"))];
        let order_b = vec![text("A3 Period", Some("de")), text("Period", Some("en"))];
        assert_eq!(join_labels_prose(&order_a), "Period / A3 Period");
        assert_eq!(join_labels_prose(&order_b), "Period / A3 Period");
    }

    #[test]
    fn join_labels_prose_full_five_languages_plus_untagged() {
        let labels = vec![
            text("rm-val", Some("rm")),
            text("it-val", Some("it")),
            text("untagged-val", None),
            text("fr-val", Some("fr")),
            text("de-val", Some("de")),
            text("en-val", Some("en")),
        ];
        assert_eq!(
            join_labels_prose(&labels),
            "en-val / de-val / fr-val / it-val / rm-val / untagged-val"
        );
    }

    #[test]
    fn join_labels_prose_empty_when_no_labels() {
        assert_eq!(join_labels_prose(&[]), "");
    }

    // ── localized_path_segment ──────────────────────────────────────────

    #[test]
    fn localized_path_segment_prefers_english() {
        let h = header(
            "http://rdfh.ch/lists/0838/x",
            None,
            vec![text("A3 Period", Some("de")), text("Period", Some("en"))],
        );
        assert_eq!(localized_path_segment(&h), "Period");
    }

    #[test]
    fn localized_path_segment_falls_back_through_chain_to_french_only() {
        let h = header(
            "http://rdfh.ch/lists/0838/x",
            None,
            vec![text("Phase de recherche", Some("fr"))],
        );
        assert_eq!(localized_path_segment(&h), "Phase de recherche");
    }

    #[test]
    fn localized_path_segment_untagged_when_no_tagged_language_present() {
        let h = header("http://rdfh.ch/lists/0838/x", None, vec![text("untagged value", None)]);
        assert_eq!(localized_path_segment(&h), "untagged value");
    }

    #[test]
    fn localized_path_segment_falls_back_to_iri_local_name_with_slash() {
        let h = header("http://rdfh.ch/lists/0838/JbNT7lvfS9yaB5bgbkoa2w", None, vec![]);
        assert_eq!(localized_path_segment(&h), "JbNT7lvfS9yaB5bgbkoa2w");
    }

    #[test]
    fn localized_path_segment_falls_back_to_iri_local_name_with_hash() {
        let h = header("http://example.com/ontology#Node", None, vec![]);
        assert_eq!(localized_path_segment(&h), "Node");
    }

    // ── build_vocabulary_list_rows ──────────────────────────────────────

    #[test]
    fn build_vocabulary_list_rows_one_row_per_item_with_correct_columns() {
        let items = vec![
            Vocabulary {
                header: header(
                    "http://rdfh.ch/lists/0838/epoch",
                    Some("epoch"),
                    vec![text("Period", Some("en")), text("A3 Period", Some("de"))],
                ),
                node_count: Some(33),
                depth: Some(3),
            },
            Vocabulary {
                header: header("http://rdfh.ch/lists/0838/other", Some("other"), vec![]),
                node_count: None,
                depth: None,
            },
        ];
        let rows = build_vocabulary_list_rows(&items);
        assert_eq!(rows.len(), 2);
        // 16 columns: name, iri, label_en..rm, label, comment_en..rm, comment, nodes, depth.
        assert_eq!(rows[0].len(), 16);
        assert_eq!(rows[0][0], "epoch");
        assert_eq!(rows[0][1], "http://rdfh.ch/lists/0838/epoch");
        assert_eq!(rows[0][2], "Period"); // label_en
        assert_eq!(rows[0][3], "A3 Period"); // label_de
        assert_eq!(rows[0][14], "33"); // nodes
        assert_eq!(rows[0][15], "3"); // depth
        // Second item: no counts -> "" for nodes/depth.
        assert_eq!(rows[1][14], "");
        assert_eq!(rows[1][15], "");
    }

    // ── build_vocabulary_rows ───────────────────────────────────────────

    /// Builds a hand-made 3-level tree:
    /// ```text
    /// root (fr-only label: "Phase de recherche")
    ///   1   node1 (en+de)                      (leaf)
    ///   2   node2 (en+de)
    ///     2.1 node2a (en-only)                 (leaf)
    ///     2.2 node2b (en+de)
    ///       2.2.1 node2b1 (en+de)              (leaf)
    /// ```
    fn fixture_detail(subtree_of: Option<&str>) -> VocabularyDetail {
        use crate::model::VocabularyTree;

        let node2b1 = VocabularyNode {
            header: header(
                "http://rdfh.ch/lists/0001/node2b1",
                Some("node2b1"),
                vec![text("Node 2b1 (en)", Some("en")), text("Knoten 2b1", Some("de"))],
            ),
            position: 0,
            children: vec![],
        };
        let node2b = VocabularyNode {
            header: header(
                "http://rdfh.ch/lists/0001/node2b",
                Some("node2b"),
                vec![text("Node 2b (en)", Some("en")), text("Knoten 2b", Some("de"))],
            ),
            position: 1,
            children: vec![node2b1],
        };
        let node2a = VocabularyNode {
            header: header(
                "http://rdfh.ch/lists/0001/node2a",
                Some("node2a"),
                vec![text("Node 2a (en)", Some("en"))],
            ),
            position: 0,
            children: vec![],
        };
        let node2 = VocabularyNode {
            header: header(
                "http://rdfh.ch/lists/0001/node2",
                Some("node2"),
                vec![text("Node 2 (en)", Some("en")), text("Knoten 2", Some("de"))],
            ),
            position: 1,
            children: vec![node2a, node2b],
        };
        let node1 = VocabularyNode {
            header: header(
                "http://rdfh.ch/lists/0001/node1",
                Some("node1"),
                vec![text("Node 1 (en)", Some("en")), text("Knoten 1", Some("de"))],
            ),
            position: 0,
            children: vec![],
        };
        let tree = VocabularyTree {
            root: header(
                "http://rdfh.ch/lists/0001/root",
                Some("root"),
                vec![text("Phase de recherche", Some("fr"))],
            ),
            children: vec![node1, node2],
            project_iri: "http://rdfh.ch/projects/0001".into(),
            requested_node: None,
        };
        let (node_count, depth) = tree.count_and_depth(subtree_of);
        VocabularyDetail {
            tree,
            subtree_of: subtree_of.map(|s| s.to_string()),
            node_count,
            depth,
        }
    }

    #[test]
    fn build_vocabulary_rows_one_row_per_node_in_dfs_order() {
        let detail = fixture_detail(None);
        let rows = build_vocabulary_rows(&detail);
        assert_eq!(rows.len(), 5);
        let ids: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "http://rdfh.ch/lists/0001/node1",
                "http://rdfh.ch/lists/0001/node2",
                "http://rdfh.ch/lists/0001/node2a",
                "http://rdfh.ch/lists/0001/node2b",
                "http://rdfh.ch/lists/0001/node2b1",
            ]
        );
    }

    #[test]
    fn build_vocabulary_rows_number_is_one_based_and_segment_count_equals_depth() {
        let detail = fixture_detail(None);
        let rows = build_vocabulary_rows(&detail);
        // node_iri, number, name, label_en..rm(5), label, comment_en..rm(5), comment,
        // path, position, depth, parent_iri  -> depth at index 17.
        let by_iri = |iri: &str| rows.iter().find(|r| r[0] == iri).unwrap();

        let node1 = by_iri("http://rdfh.ch/lists/0001/node1");
        assert_eq!(node1[1], "1"); // number
        assert_eq!(node1[17], "1"); // depth == segment count
        assert_eq!(node1[16], "0"); // position (0-based, unchanged)

        let node2 = by_iri("http://rdfh.ch/lists/0001/node2");
        assert_eq!(node2[1], "2");

        let node2a = by_iri("http://rdfh.ch/lists/0001/node2a");
        assert_eq!(node2a[1], "2.1");
        assert_eq!(node2a[17], "2");

        let node2b1 = by_iri("http://rdfh.ch/lists/0001/node2b1");
        // Deep node numbers correctly: 2.2.1 (parent node2 is "2", node2b is "2.2").
        assert_eq!(node2b1[1], "2.2.1");
        assert_eq!(node2b1[17], "3");
        // Last segment equals position + 1 (node2b1's own position is 0).
        assert_eq!(node2b1[16], "0");
    }

    #[test]
    fn build_vocabulary_rows_path_breadcrumb_mixed_language_fallback() {
        let detail = fixture_detail(None);
        let rows = build_vocabulary_rows(&detail);
        let by_iri = |iri: &str| rows.iter().find(|r| r[0] == iri).unwrap();

        // path column is index 15. Root is fr-only ("Phase de recherche"),
        // node2 is en-preferred ("Node 2 (en)"), node2a is en-only.
        let node2a = by_iri("http://rdfh.ch/lists/0001/node2a");
        assert_eq!(node2a[15], "Phase de recherche \u{203a} Node 2 (en) \u{203a} Node 2a (en)");
    }

    #[test]
    fn build_vocabulary_rows_parent_iri_correctness() {
        let detail = fixture_detail(None);
        let rows = build_vocabulary_rows(&detail);
        let by_iri = |iri: &str| rows.iter().find(|r| r[0] == iri).unwrap();
        // parent_iri is the last column (index 18).
        let node1 = by_iri("http://rdfh.ch/lists/0001/node1");
        assert_eq!(node1[18], "http://rdfh.ch/lists/0001/root");
        let node2b1 = by_iri("http://rdfh.ch/lists/0001/node2b1");
        assert_eq!(node2b1[18], "http://rdfh.ch/lists/0001/node2b");
    }

    #[test]
    fn build_vocabulary_rows_subtree_narrows_and_keeps_absolute_numbers() {
        let whole = fixture_detail(None);
        let whole_rows = build_vocabulary_rows(&whole);
        let whole_node2_number =
            whole_rows.iter().find(|r| r[0] == "http://rdfh.ch/lists/0001/node2").unwrap()[1].clone();

        let subtree = fixture_detail(Some("http://rdfh.ch/lists/0001/node2"));
        let subtree_rows = build_vocabulary_rows(&subtree);
        // node2 + its 3 descendants = 4 rows.
        assert_eq!(subtree_rows.len(), 4);
        let ids: Vec<&str> = subtree_rows.iter().map(|r| r[0].as_str()).collect();
        assert!(ids.contains(&"http://rdfh.ch/lists/0001/node2"));
        assert!(ids.contains(&"http://rdfh.ch/lists/0001/node2a"));
        assert!(ids.contains(&"http://rdfh.ch/lists/0001/node2b"));
        assert!(ids.contains(&"http://rdfh.ch/lists/0001/node2b1"));
        assert!(!ids.contains(&"http://rdfh.ch/lists/0001/node1"));

        // Absolute numbers unchanged from whole-tree numbering.
        let subtree_node2_number =
            subtree_rows.iter().find(|r| r[0] == "http://rdfh.ch/lists/0001/node2").unwrap()[1].clone();
        assert_eq!(subtree_node2_number, whole_node2_number);

        // D15: row count equals detail.node_count in subtree mode.
        assert_eq!(subtree_rows.len(), subtree.node_count);
    }

    #[test]
    fn build_vocabulary_rows_subtree_leaf_yields_exactly_one_row() {
        let detail = fixture_detail(Some("http://rdfh.ch/lists/0001/node2a"));
        let rows = build_vocabulary_rows(&detail);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], "http://rdfh.ch/lists/0001/node2a");
    }

    #[test]
    fn build_vocabulary_rows_row_count_equals_node_count_whole_vocabulary() {
        let detail = fixture_detail(None);
        let rows = build_vocabulary_rows(&detail);
        // D15: node_count equals the number of tabular DATA rows.
        assert_eq!(rows.len(), detail.node_count);
        assert_eq!(detail.node_count, 5);
    }

    // ── nest_vocabulary_detail ──────────────────────────────────────────

    #[test]
    fn nest_vocabulary_detail_whole_tree_has_expected_shape() {
        let detail = fixture_detail(None);
        let nested = nest_vocabulary_detail(&detail);
        // 2 top-level nodes (node1, node2).
        assert_eq!(nested.len(), 2);
        let node2 = nested
            .iter()
            .find(|n| n.header.iri == "http://rdfh.ch/lists/0001/node2")
            .unwrap();
        // node2 has 2 children (node2a, node2b).
        assert_eq!(node2.children.len(), 2);
        let node2b = node2
            .children
            .iter()
            .find(|n| n.header.iri == "http://rdfh.ch/lists/0001/node2b")
            .unwrap();
        // node2b has 1 child (node2b1).
        assert_eq!(node2b.children.len(), 1);
    }

    #[test]
    fn nest_vocabulary_detail_absolute_numbers_match_flat_builder() {
        let detail = fixture_detail(None);
        let flat_rows = build_vocabulary_rows(&detail);
        let flat_node2b1_number =
            flat_rows.iter().find(|r| r[0] == "http://rdfh.ch/lists/0001/node2b1").unwrap()[1].clone();
        assert_eq!(flat_node2b1_number, "2.2.1");

        let nested = nest_vocabulary_detail(&detail);
        let node2 = nested
            .iter()
            .find(|n| n.header.iri == "http://rdfh.ch/lists/0001/node2")
            .unwrap();
        let node2b = node2
            .children
            .iter()
            .find(|n| n.header.iri == "http://rdfh.ch/lists/0001/node2b")
            .unwrap();
        let node2b1 = node2b
            .children
            .iter()
            .find(|n| n.header.iri == "http://rdfh.ch/lists/0001/node2b1")
            .unwrap();
        assert_eq!(node2b1.number, flat_node2b1_number);
    }

    #[test]
    fn nest_vocabulary_detail_subtree_returns_single_branch() {
        let detail = fixture_detail(Some("http://rdfh.ch/lists/0001/node2"));
        let nested = nest_vocabulary_detail(&detail);
        // Single top-level node: node2 itself.
        assert_eq!(nested.len(), 1);
        let node2 = &nested[0];
        assert_eq!(node2.header.iri, "http://rdfh.ch/lists/0001/node2");

        // Descendants nested beneath: node2a and node2b (with node2b1 beneath node2b).
        let ids: Vec<&str> = node2.children.iter().map(|n| n.header.iri.as_str()).collect();
        assert!(ids.contains(&"http://rdfh.ch/lists/0001/node2a"));
        assert!(ids.contains(&"http://rdfh.ch/lists/0001/node2b"));
        let node2b = node2
            .children
            .iter()
            .find(|n| n.header.iri == "http://rdfh.ch/lists/0001/node2b")
            .unwrap();
        assert_eq!(node2b.children.len(), 1);
        assert_eq!(node2b.children[0].header.iri, "http://rdfh.ch/lists/0001/node2b1");

        // node1 (a sibling branch) must not appear anywhere in the subtree.
        assert_ne!(node2.header.iri, "http://rdfh.ch/lists/0001/node1");
    }

    #[test]
    fn nest_vocabulary_detail_leaf_subtree_returns_one_node_no_children() {
        let detail = fixture_detail(Some("http://rdfh.ch/lists/0001/node2a"));
        let nested = nest_vocabulary_detail(&detail);
        assert_eq!(nested.len(), 1);
        assert_eq!(nested[0].header.iri, "http://rdfh.ch/lists/0001/node2a");
        assert!(nested[0].children.is_empty());
    }
}
