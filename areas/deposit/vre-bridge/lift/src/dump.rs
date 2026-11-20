//! Load a `dsp vre project dump` (bagit-zip, unzipped) into an in-memory
//! Oxigraph store and locate the project's data and ontology named graphs.
//!
//! The admin and permission graphs are deliberately *not* loaded: they carry
//! users/groups/permissions, which the archive sheds.

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use oxigraph::io::{RdfFormat, RdfParseError};
use oxigraph::model::{NamedNode, NamedOrBlankNode};
use oxigraph::store::{LoaderError, Store};

pub struct Dump {
    pub store: Store,
    /// `http://www.knora.org/data/<shortcode>/<name>`
    pub data_graph: NamedNode,
    /// All project ontology graphs (beol carries 4). Sorted by IRI.
    pub ontology_graphs: Vec<NamedNode>,
}

impl Dump {
    /// Load `<dump_dir>/data/rdf/data.nq` plus every `ontology-*.nq`.
    /// admin.nq / permission.nq are deliberately skipped (shed by the archive).
    /// Fails like [`Dump::from_store`] unless there is exactly one `/data/` graph.
    pub fn load(dump_dir: &Path) -> Result<Dump, Box<dyn Error>> {
        let store = Store::new()?;
        let rdf_dir = dump_dir.join("data").join("rdf");
        let mut loaded = 0usize;
        for entry in std::fs::read_dir(&rdf_dir)? {
            let path = entry?.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if name == "data.nq" || (name.starts_with("ontology-") && name.ends_with(".nq")) {
                let reader = BufReader::new(File::open(&path)?);
                store
                    .load_from_reader(RdfFormat::NQuads, reader)
                    .map_err(|e| format!("loading {}: {}", path.display(), load_error(&e)))?;
                loaded += 1;
            }
        }
        if loaded == 0 {
            return Err(format!("no data.nq/ontology-*.nq under {}", rdf_dir.display()).into());
        }
        Dump::from_store(store)
    }

    /// Locate the data and ontology graphs of an already-loaded store. Fails
    /// unless there is exactly one `/data/` graph: with several, which one a
    /// subcommand read would depend on the store's iteration order.
    pub fn from_store(store: Store) -> Result<Dump, Box<dyn Error>> {
        let mut data_graphs: Vec<NamedNode> = Vec::new();
        let mut ontology_graphs: Vec<NamedNode> = Vec::new();
        for g in store.named_graphs() {
            if let NamedOrBlankNode::NamedNode(graph) = g? {
                let s = graph.as_str();
                if s.contains("/data/") {
                    data_graphs.push(graph);
                } else if s.contains("/ontology/") && !s.contains("knora-base") {
                    ontology_graphs.push(graph);
                }
            }
        }
        data_graphs.sort();
        ontology_graphs.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        if data_graphs.len() > 1 {
            let names: Vec<&str> = data_graphs.iter().map(NamedNode::as_str).collect();
            return Err(format!("more than one /data/ named graph in dump: {}", names.join(", ")).into());
        }
        let data_graph = data_graphs.pop().ok_or("no /data/ named graph found in dump")?;
        if ontology_graphs.is_empty() {
            return Err("no project /ontology/ named graph found".into());
        }

        Ok(Dump { store, data_graph, ontology_graphs })
    }
}

/// A load error as its kind and, for a syntax error, its line: oxigraph's own text can quote
/// the dump's content, which may not reach a log.
fn load_error(e: &LoaderError) -> String {
    match e {
        LoaderError::Parsing(RdfParseError::Syntax(syntax)) => match syntax.location() {
            Some(at) => format!("syntax error at line {}", at.start.line + 1),
            None => "syntax error".to_owned(),
        },
        LoaderError::Parsing(RdfParseError::Io(io)) => format!("read error: {}", io.kind()),
        LoaderError::Storage(_) => "storage error".to_owned(),
        _ => "load error".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use oxigraph::model::{GraphName, Literal, Quad};

    use super::*;
    use crate::model::nn;
    use crate::test_support::TempDir;

    const DATA_NQ: &str = "<http://rdfh.ch/0803/A> <http://www.w3.org/2000/01/rdf-schema#label> \"a\" <http://www.knora.org/data/0803/incunabula> .\n";
    const ONTO_NQ: &str = "<http://www.knora.org/ontology/0803/incunabula#Page> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> <http://www.knora.org/ontology/0803/incunabula> .\n";
    /// A valid first line (so a lenient parse would load it) before a broken one.
    const SECRET_NQ: &str = "<http://rdfh.ch/users/SECRET> <http://www.w3.org/2000/01/rdf-schema#label> \"secret\" <http://www.knora.org/data/admin> .\nthis is not N-Quads\n";

    #[test]
    fn load_never_reads_admin_or_permission_graphs() {
        let dir = TempDir::new();
        let rdf = dir.path().join("data").join("rdf");
        std::fs::create_dir_all(&rdf).unwrap();
        std::fs::write(rdf.join("data.nq"), DATA_NQ).unwrap();
        std::fs::write(rdf.join("ontology-1.nq"), ONTO_NQ).unwrap();
        std::fs::write(rdf.join("admin.nq"), SECRET_NQ).unwrap();
        std::fs::write(rdf.join("permission.nq"), SECRET_NQ).unwrap();

        let dump = Dump::load(dir.path()).unwrap();

        assert_eq!(dump.store.len().unwrap(), 2);
        let secret = nn("http://rdfh.ch/users/SECRET");
        assert!(
            dump.store
                .quads_for_pattern(Some(secret.as_ref().into()), None, None, None)
                .next()
                .is_none()
        );
        assert_eq!(dump.data_graph.as_str(), "http://www.knora.org/data/0803/incunabula");
    }

    #[test]
    fn a_syntax_error_names_its_line_but_quotes_no_content() {
        let dir = TempDir::new();
        let rdf = dir.path().join("data").join("rdf");
        std::fs::create_dir_all(&rdf).unwrap();
        let broken = format!("{DATA_NQ}<http://rdfh.ch/0803/B> \"SECRET-LITERAL\" broken .\n");
        std::fs::write(rdf.join("data.nq"), broken).unwrap();

        let msg = Dump::load(dir.path()).err().expect("a broken line must fail").to_string();

        assert!(msg.contains("syntax error at line 2"), "{msg}");
        assert!(!msg.contains("SECRET"), "{msg}");
    }

    #[test]
    fn from_store_rejects_two_data_graphs_naming_both() {
        let store = Store::new().unwrap();
        // ZEBRA inserted before APPLE: the message must list them sorted.
        for g in [
            "http://www.knora.org/data/0803/ZEBRA",
            "http://www.knora.org/data/0803/APPLE",
            "http://www.knora.org/ontology/0803/incunabula",
        ] {
            let q = Quad::new(
                nn("http://rdfh.ch/0803/A"),
                nn(crate::model::RDFS_LABEL),
                Literal::new_simple_literal("a"),
                GraphName::NamedNode(nn(g)),
            );
            store.insert(q.as_ref()).unwrap();
        }

        let err = Dump::from_store(store).err().expect("two data graphs must fail");

        let msg = err.to_string();
        let apple = msg.find("0803/APPLE").expect("names APPLE");
        let zebra = msg.find("0803/ZEBRA").expect("names ZEBRA");
        assert!(apple < zebra, "graphs not sorted: {msg}");
    }
}
