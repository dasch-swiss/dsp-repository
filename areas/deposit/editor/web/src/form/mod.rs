//! The project form's controls. What the form knows about each field (labels,
//! hints, obligations, sections) is `editor_core::form::registry`; [`widgets`]
//! renders the controls keyed by the same field ids, so the grouping can change
//! without touching a renderer.

pub mod widgets;

/// The name every control that says what a POST is for posts under. One
/// definition for both write surfaces, so the wire name cannot drift; the verbs
/// stay with the page that offers them.
pub const INTENT: &str = "intent";
