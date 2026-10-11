//! The three list tables, from the lists a mapping declares.

use cpe_core::remodel::{RemodelList, RemodelListNode};

use crate::store::{ListNodeLabelRow, ListNodeRow, ListRow, Store};

/// The id of `node`, a node of `list`, in `list_nodes`: `<name>-<slug>`. The table has one key
/// for the nodes of every list.
pub(crate) fn node_id(list: &RemodelList, node: &RemodelListNode) -> String {
    format!("{}-{}", list.name, node.slug)
}

/// Fills `lists`, `list_nodes` and `list_node_labels` from `lists`.
///
/// A list's id is `list-<name>` and a node's is [`node_id`]. Rows are sorted by id while `seq`
/// is the declared position, so a node may stand before one with a smaller `seq`.
pub(crate) fn fill(lists: &[RemodelList], store: &mut Store) {
    for list in lists {
        let list_id = format!("list-{}", list.name);
        for (seq, node) in (0_i64..).zip(&list.nodes) {
            let id = node_id(list, node);
            for label in &node.labels {
                store.list_node_labels.push(ListNodeLabelRow {
                    node_id: id.clone(),
                    lang: label.language.clone(),
                    label: label.text.clone(),
                });
            }
            store
                .list_nodes
                .push(ListNodeRow { id, list_id: list_id.clone(), parent_id: None, seq });
        }
        store.lists.push(ListRow { id: list_id, name: list.name.clone() });
    }
    store.lists.sort_by(|a, b| a.id.cmp(&b.id));
    store.list_nodes.sort_by(|a, b| a.id.cmp(&b.id));
    store
        .list_node_labels
        .sort_by(|a, b| (&a.node_id, &a.lang).cmp(&(&b.node_id, &b.lang)));
}
