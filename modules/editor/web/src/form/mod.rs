//! The project form: what the form knows about each field, and how the fields
//! are grouped.
//!
//! [`registry`] is data — labels, hints, obligations, sections. The controls
//! that render them are keyed by the same field ids, which is the split that
//! lets the grouping change without touching a renderer.

pub mod registry;
