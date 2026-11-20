//! The deterministic writer (§1): every quad rendered to its N-Quads line, the
//! lines kept sorted and unique; the caller writes them where it needs them.

use std::collections::BTreeSet;
use std::io::{self, Write};

use oxigraph::model::{GraphNameRef, NamedNode, NamedNodeRef, NamedOrBlankNodeRef, QuadRef, TermRef};

use super::Stats;

/// The snapshot under construction: every quad as its N-Quads line, so the set
/// is sorted bytewise and deduplicated by construction (§1). The line bytes are
/// oxrdf's `Display`, stable only under the pinned oxrdf version.
#[derive(Debug)]
pub struct Snapshot {
    graph: NamedNode,
    lines: BTreeSet<String>,
    pub(crate) stats: Stats,
}

impl Snapshot {
    pub(crate) fn new(graph: NamedNode) -> Snapshot {
        Snapshot { graph, lines: BTreeSet::new(), stats: Stats::default() }
    }

    pub(crate) fn put<'a>(&mut self, s: NamedNodeRef<'a>, p: NamedNodeRef<'a>, o: impl Into<TermRef<'a>>) {
        // A struct literal, not `QuadRef::new`: its `Into` bounds would pin every
        // part to `'a`, which the graph borrowed from `self` does not live for.
        let q = QuadRef {
            subject: NamedOrBlankNodeRef::NamedNode(s),
            predicate: p,
            object: o.into(),
            graph_name: GraphNameRef::NamedNode(self.graph.as_ref()),
        };
        self.lines.insert(format!("{q} ."));
    }

    pub(crate) fn len(&self) -> usize {
        self.lines.len()
    }

    /// Every line, sorted, each ended by `\n`.
    pub fn write_to(&self, w: &mut dyn Write) -> io::Result<()> {
        self.lines.iter().try_for_each(|l| writeln!(w, "{l}"))
    }

    /// What the run that built this snapshot counted.
    pub fn stats(&self) -> &Stats {
        &self.stats
    }
}
