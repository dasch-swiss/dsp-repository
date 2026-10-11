//! What a project's mapping makes of a resource: its id, and the values of its properties.
//!
//! The ids come first. A resource that gets none is mapped no further, so that one mistake is
//! one refusal.

pub(crate) mod curated;
pub(crate) mod ids;
pub(crate) mod label;
pub(crate) mod values;
