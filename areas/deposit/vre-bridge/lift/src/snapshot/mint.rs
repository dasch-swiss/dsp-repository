//! Checked constructors for every IRI and language tag the snapshot builds from
//! dump data. `model::nn` is unchecked, so it is kept for constant IRIs; here an
//! invalid input becomes an error or a violation, never a panic.

use oxigraph::model::{Literal, NamedNode};

use super::violations::{Rule, Violations};
use super::vocab;

const DATA_GRAPH_PREFIX: &str = "http://www.knora.org/data/";

/// §1: `urn:dsp:project:<shortcode>`, the shortcode read from the data graph IRI
/// `http://www.knora.org/data/<shortcode>/<name>`.
pub(crate) fn project_graph(data_graph: &NamedNode) -> Result<NamedNode, String> {
    let shortcode = data_graph
        .as_str()
        .strip_prefix(DATA_GRAPH_PREFIX)
        .and_then(|rest| rest.split('/').next())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("data graph {data_graph} names no shortcode"))?;
    NamedNode::new(format!("{}{shortcode}", vocab::PROJECT_GRAPH_PREFIX))
        .map_err(|e| format!("shortcode {shortcode:?} of data graph {data_graph} forms no IRI: {e}"))
}

/// `<prefix><uuid>` for a `kb:valueHasUUID`, which must be non-empty base64url
/// (§3): `NamedNode::new` alone accepts `urn:dsp:value:` and `urn:dsp:value:a#b`.
pub(crate) fn minted_node(prefix: &str, uuid: &str, subject: &str, violations: &mut Violations) -> Option<NamedNode> {
    let base64url = !uuid.is_empty() && uuid.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    let node = if base64url {
        NamedNode::new(format!("{prefix}{uuid}")).ok()
    } else {
        None
    };
    if node.is_none() {
        violations.record(Rule::InvalidUuid, subject, uuid);
    }
    node
}

/// A language-tagged literal with the tag validated and lowercased (§1).
pub(crate) fn lang_literal(value: &str, tag: &str, subject: &str, violations: &mut Violations) -> Option<Literal> {
    let literal = Literal::new_language_tagged_literal(value, tag).ok();
    if literal.is_none() {
        violations.record(Rule::InvalidLanguageTag, subject, tag);
    }
    literal
}
