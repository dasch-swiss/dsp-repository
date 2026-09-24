//! The project form: what the form knows about each field, and how the fields
//! are grouped. [`registry`] is data (labels, hints, obligations, sections);
//! the controls that render them are keyed by the same field ids, so the
//! grouping can change without touching a renderer.

pub mod obligation;
pub mod registry;
pub mod submit;
pub mod widgets;

/// The name every control that says what a POST is for posts under. One
/// definition for both write surfaces, so the wire name cannot drift; the verbs
/// stay with the page that offers them.
pub const INTENT: &str = "intent";
