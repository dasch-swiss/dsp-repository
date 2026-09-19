//! Vocabulary domain shapes — shared across the client → action → render
//! boundary for `dsp vre vocabulary list` / `describe` (DSP-API "list" /
//! "list node").
//!
//! Types here are the dsp-cli vocabulary for DSP-API's list/list-node wire
//! shapes. Wire (de)serialization stays in `src/client/http.rs`; nothing
//! here derives `serde`. See ADR-0001 and ADR-0008.

/// One language-tagged string, as DSP-API returns it in a list's `labels` /
/// `comments`. ALL languages are kept (plan 034 D4) — no preferred-language
/// collapsing anywhere in this crate. `language` is `None` for DSP-API's
/// untagged `PlainStringLiteralV2` variant (D9b).
///
/// Same shape as [`ProjectDescription`]; deliberately not consolidated with
/// it — see the plan's BACKLOG note. `language` stays `Option<String>`
/// rather than a closed `Language` enum, even though D9 proves the server's
/// tag set is closed at exactly `{de, en, fr, it, rm}`: mirroring the
/// existing `ProjectDescription` precedent means an unexpected tag stays a
/// value the CLI can still display, rather than becoming a hard parse error
/// (`ServerError`). Reads should degrade, not refuse.
///
/// [`ProjectDescription`]: crate::model::ProjectDescription
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedText {
    /// The label/comment text.
    pub value: String,
    /// BCP-47-shaped language tag, if the server provides one (e.g. `"en"`).
    pub language: Option<String>,
}

/// Identity + labels of a vocabulary or one of its nodes. No children, no
/// counts — those live on [`Vocabulary`] / [`VocabularyNode`] / [`VocabularyTree`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyHeader {
    /// The vocabulary's or node's IRI (e.g. `http://rdfh.ch/lists/0838/…`).
    pub iri: String,
    /// Human-readable short name, if the server supplies one.
    pub name: Option<String>,
    /// Language-tagged labels (D4: all languages kept, no preference rule).
    pub labels: Vec<LocalizedText>,
    /// Language-tagged comments (D7: carried, out of the default column set).
    pub comments: Vec<LocalizedText>,
}

/// A vocabulary as shown by `dsp vre vocabulary list` — the lean list-index
/// projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vocabulary {
    pub header: VocabularyHeader,
    /// `Some` only when `--count` fetched this vocabulary's tree; `None`
    /// when a per-tree fetch failed (disclosed to the caller, not silently
    /// dropped). Counts nodes strictly BELOW the root — the root itself is
    /// never a node.
    pub node_count: Option<usize>,
    /// `Some` only under `--count`. Deepest level; the root's direct
    /// children sit at level 1.
    pub depth: Option<usize>,
}

/// One node in a vocabulary's tree, as returned by the DSP-API
/// (position-sorted by the server; the client re-sorts defensively).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyNode {
    pub header: VocabularyHeader,
    /// Raw, 0-based DSP-API `position` among siblings. NOT the 1-based
    /// outline `number` the renderer derives (D10) — kept unchanged here so
    /// `--columns position` still shows the untranslated wire value.
    pub position: i32,
    pub children: Vec<VocabularyNode>,
}

/// What the CLIENT returns from `describe_vocabulary`: a faithful,
/// position-ordered tree plus the facts the wire response carried. Holds NO
/// rendered-counts and no render decisions — structurally, since it has no
/// such fields, the client cannot express them even by accident. [`VocabularyDetail`]
/// is where those decisions live (Step 3's layer-ownership split).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyTree {
    pub root: VocabularyHeader,
    /// The WHOLE tree, always — never pruned. Absolute `number` (D10) and
    /// `path` (D11) are derived from each node's ancestor chain, which
    /// pruning would destroy.
    pub children: Vec<VocabularyNode>,
    /// From `listinfo.projectIri`, which a root response ALWAYS carries — so
    /// this is never absent and the cross-project guard never has to fail
    /// open.
    pub project_iri: String,
    /// Set when the address that was resolved turned out to be a NODE, not a
    /// root: the client is the layer that saw the `Node` response shape
    /// (D2), so it is the layer that records this.
    pub requested_node: Option<String>,
}

/// What the ACTION hands the renderer for `describe`: the tree plus the
/// render decisions only the action can make, because only it sees `--subtree`.
///
/// The split is deliberate and is what makes the layer ownership structural
/// rather than a convention: `describe_vocabulary` (the client) returns a
/// bare [`VocabularyTree`], so the client has no `node_count` field to fill
/// in even by accident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyDetail {
    pub tree: VocabularyTree,
    /// Set by the action from `--subtree` (D14): the node IRI to narrow
    /// RENDERING to. `None` renders the whole vocabulary.
    pub subtree_of: Option<String>,
    /// Nodes RENDERED (honouring `subtree_of`). Equals the tabular data-row
    /// count in both whole-vocabulary and `--subtree` mode (D15). Computed
    /// by the action via [`VocabularyTree::count_and_depth`].
    pub node_count: usize,
    /// Deepest level of what is RENDERED (a root's direct children sit at
    /// level 1). Under `--subtree` this is branch-relative — the addressed
    /// node itself is level 1 of its own branch, so a leaf renders `1 node ·
    /// 1 level`. The per-node `depth` COLUMN the renderer derives separately
    /// stays absolute, so it never disagrees with `number`'s segment count.
    pub depth: usize,
}

impl VocabularyTree {
    /// Count nodes and measure depth at or below the node named by `from`
    /// (the whole tree when `from` is `None`).
    ///
    /// Shared derivation used by BOTH `list --count` and `describe`, so one
    /// walk has one implementation — the thing that makes the D15 invariant
    /// hold (`node_count` equals the rendered data-row count in both modes).
    ///
    /// - `from: None` — whole-vocabulary mode. The root itself is never a node, so it is not
    ///   counted; `depth` treats the root's direct children as level 1.
    /// - `from: Some(iri)` — branch mode (`--subtree`). The addressed node IS included, as the top
    ///   of its own branch (D14b): a leaf node yields `(1, 1)` — one node, one level, because the
    ///   node itself occupies level 1 of the branch, its children level 2, and so on. If `iri` does
    ///   not address any node in this tree, returns `(0, 0)` (the action layer validates that
    ///   `--subtree`'s address names a node present in the tree before calling this, so that case
    ///   should not arise from user input — this is a defensive default, not user-facing
    ///   behaviour).
    ///
    /// This is not on the untrusted-input parse path (that's
    /// `src/client/http.rs`, which must walk iteratively) — it walks a tree
    /// that already survived deserialisation, so plain recursion is fine;
    /// real data tops out at 9 levels.
    pub fn count_and_depth(&self, from: Option<&str>) -> (usize, usize) {
        match from {
            None => {
                let mut count = 0;
                let mut depth = 0;
                for child in &self.children {
                    let (child_count, child_height) = subtree_stats(child);
                    count += child_count;
                    depth = depth.max(child_height);
                }
                (count, depth)
            }
            Some(iri) => match find_node(&self.children, iri) {
                Some(node) => subtree_stats(node),
                None => (0, 0),
            },
        }
    }
}

/// Node count and height of the subtree rooted at `node`, INCLUSIVE of
/// `node` itself: a leaf returns `(1, 1)`.
fn subtree_stats(node: &VocabularyNode) -> (usize, usize) {
    let mut count = 1;
    let mut max_child_height = 0;
    for child in &node.children {
        let (child_count, child_height) = subtree_stats(child);
        count += child_count;
        max_child_height = max_child_height.max(child_height);
    }
    (count, 1 + max_child_height)
}

/// Depth-first search for the node whose header IRI equals `iri`, anywhere
/// in `nodes` or their descendants.
fn find_node<'a>(nodes: &'a [VocabularyNode], iri: &str) -> Option<&'a VocabularyNode> {
    for node in nodes {
        if node.header.iri == iri {
            return Some(node);
        }
        if let Some(found) = find_node(&node.children, iri) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(iri: &str) -> VocabularyHeader {
        VocabularyHeader {
            iri: iri.to_string(),
            name: Some(iri.to_string()),
            labels: vec![LocalizedText { value: iri.to_string(), language: Some("en".into()) }],
            comments: vec![],
        }
    }

    fn leaf(iri: &str, position: i32) -> VocabularyNode {
        VocabularyNode { header: header(iri), position, children: vec![] }
    }

    /// Builds:
    /// ```text
    /// root
    ///   1   node1                       (leaf)
    ///   2   node2
    ///     2.1 node2a                    (leaf)
    ///     2.2 node2b
    ///       2.2.1 node2b1               (leaf)
    /// ```
    /// 5 nodes total; deepest level is 3 (node2 -> node2b -> node2b1).
    fn fixture_tree() -> VocabularyTree {
        let node2b1 = leaf("node2b1", 0);
        let node2b = VocabularyNode {
            header: header("node2b"),
            position: 1,
            children: vec![node2b1],
        };
        let node2a = leaf("node2a", 0);
        let node2 = VocabularyNode {
            header: header("node2"),
            position: 1,
            children: vec![node2a, node2b],
        };
        let node1 = leaf("node1", 0);
        VocabularyTree {
            root: header("root"),
            children: vec![node1, node2],
            project_iri: "http://rdfh.ch/projects/0001".into(),
            requested_node: None,
        }
    }

    #[test]
    fn count_and_depth_whole_tree() {
        let tree = fixture_tree();
        let (count, depth) = tree.count_and_depth(None);
        // 5 real nodes; root itself is not counted.
        assert_eq!(count, 5);
        // node1 is level 1; node2/node2a level 1/2; node2b1 level 3.
        assert_eq!(depth, 3);
    }

    #[test]
    fn count_and_depth_subtree_of_non_root_branch() {
        let tree = fixture_tree();
        // node2's own branch: node2, node2a, node2b, node2b1 = 4 nodes.
        let (count, depth) = tree.count_and_depth(Some("node2"));
        assert_eq!(count, 4);
        // node2 itself is level 1 of its branch, node2b1 is level 3.
        assert_eq!(depth, 3);
    }

    #[test]
    fn count_and_depth_subtree_of_leaf_yields_one_node_one_level() {
        let tree = fixture_tree();
        // D14b: the addressed node is included, as the top of its branch.
        let (count, depth) = tree.count_and_depth(Some("node2a"));
        assert_eq!(count, 1);
        assert_eq!(depth, 1);
    }

    #[test]
    fn count_and_depth_subtree_of_intermediate_branch() {
        let tree = fixture_tree();
        // node2b's branch: node2b, node2b1 = 2 nodes, 2 levels.
        let (count, depth) = tree.count_and_depth(Some("node2b"));
        assert_eq!(count, 2);
        assert_eq!(depth, 2);
    }

    #[test]
    fn count_and_depth_unknown_iri_returns_zero() {
        let tree = fixture_tree();
        let (count, depth) = tree.count_and_depth(Some("does-not-exist"));
        assert_eq!(count, 0);
        assert_eq!(depth, 0);
    }

    #[test]
    fn position_ordering_is_preserved_as_given() {
        // This model does no sorting itself (the client sorts defensively by
        // `position` before building the tree); this test just confirms the
        // field and `Vec` order carry through unchanged.
        let children = vec![leaf("a", 0), leaf("b", 1), leaf("c", 2)];
        let parent = VocabularyNode { header: header("parent"), position: 0, children };
        assert_eq!(parent.children[0].header.iri, "a");
        assert_eq!(parent.children[0].position, 0);
        assert_eq!(parent.children[1].header.iri, "b");
        assert_eq!(parent.children[1].position, 1);
        assert_eq!(parent.children[2].header.iri, "c");
        assert_eq!(parent.children[2].position, 2);
    }

    /// D15: `node_count` equals the rendered data-row count in BOTH
    /// whole-vocabulary and `--subtree` mode. Here we hand-count the fixture
    /// tree's nodes for each mode and assert `count_and_depth` agrees.
    #[test]
    fn d15_node_count_matches_manually_counted_rendered_rows() {
        let tree = fixture_tree();

        // Whole-vocabulary mode: every real node is a rendered row.
        let whole_vocabulary_rows = ["node1", "node2", "node2a", "node2b", "node2b1"];
        let (whole_count, _) = tree.count_and_depth(None);
        assert_eq!(whole_count, whole_vocabulary_rows.len());

        // `--subtree`-equivalent mode narrowed to node2: node2 and its
        // descendants are the rendered rows.
        let subtree_rows = ["node2", "node2a", "node2b", "node2b1"];
        let (subtree_count, _) = tree.count_and_depth(Some("node2"));
        assert_eq!(subtree_count, subtree_rows.len());

        // `--subtree`-equivalent mode narrowed to a leaf: exactly one
        // rendered row — the leaf itself.
        let leaf_rows = ["node2a"];
        let (leaf_count, _) = tree.count_and_depth(Some("node2a"));
        assert_eq!(leaf_count, leaf_rows.len());
    }

    #[test]
    fn localized_text_construction_and_equality() {
        let a = LocalizedText { value: "Period".into(), language: Some("en".into()) };
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(a.value, "Period");
        assert_eq!(a.language.as_deref(), Some("en"));
    }

    #[test]
    fn localized_text_untagged() {
        let a = LocalizedText { value: "untagged".into(), language: None };
        assert_eq!(a.language, None);
    }

    #[test]
    fn vocabulary_node_count_none_before_count_flag() {
        let vocab = Vocabulary { header: header("vocab"), node_count: None, depth: None };
        assert_eq!(vocab.node_count, None);
        assert_eq!(vocab.depth, None);
    }

    #[test]
    fn vocabulary_tree_requested_node_default_none() {
        let tree = fixture_tree();
        assert_eq!(tree.requested_node, None);
        assert_eq!(tree.project_iri, "http://rdfh.ch/projects/0001");
    }

    #[test]
    fn vocabulary_detail_construction_and_equality() {
        let tree = fixture_tree();
        let (node_count, depth) = tree.count_and_depth(None);
        let detail = VocabularyDetail { tree: tree.clone(), subtree_of: None, node_count, depth };
        let cloned = detail.clone();
        assert_eq!(detail, cloned);
        assert_eq!(detail.node_count, 5);
        assert_eq!(detail.depth, 3);
        assert_eq!(detail.subtree_of, None);
    }
}
