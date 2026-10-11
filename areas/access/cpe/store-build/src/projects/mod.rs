//! The hook modules: one per project whose `remodel` block declares a hook, each a row of the
//! builder's table of hooks.
//!
//! A hook module reaches the build through the seam alone: the draft a hook is given, the sink
//! for its refusals, the rule names, and the builder's helpers that read nothing but their
//! arguments. It names no IRI of the archive's ontology: the block maps those.

pub(crate) mod incunabula;
