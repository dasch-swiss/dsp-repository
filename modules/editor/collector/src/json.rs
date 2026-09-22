//! Rendering a data file in the form the committed ones hold.
//!
//! The indent differs per file, the trailing newline does not. Both live here
//! so adding a row to one of them cannot reformat the rest.

use serde::Serialize;
use serde_json::ser::PrettyFormatter;

pub fn render<T: Serialize>(value: &T, indent: &[u8], what: &str) -> Result<String, String> {
    let mut buffer = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, PrettyFormatter::with_indent(indent));
    value
        .serialize(&mut serializer)
        .map_err(|error| format!("could not serialize {what}: {error}"))?;
    let mut rendered = String::from_utf8(buffer).map_err(|error| format!("{what} is not valid UTF-8: {error}"))?;
    rendered.push('\n');
    Ok(rendered)
}
