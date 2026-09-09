//! The project form: what the form knows about each field, and how the fields
//! are grouped.
//!
//! [`registry`] is data — labels, hints, obligations, sections. The controls
//! that render them are keyed by the same field ids, which is the split that
//! lets the grouping change without touching a renderer.

pub mod obligation;
pub mod registry;
pub mod submit;
pub mod widgets;

/// The name every control that says *what a POST is for* posts under.
///
/// One definition for both write surfaces: the project form and the review
/// diff each carry their own verbs, but the pair they ride on is one wire name,
/// and two copies of it could drift while both surfaces still compiled. The
/// verbs themselves stay with the page that offers them — a page's actions are
/// its own vocabulary.
pub const INTENT: &str = "intent";
