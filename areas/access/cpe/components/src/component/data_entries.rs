//! The entries of the crumb line's "Show in dataset" panel: one resource or
//! annotation (`DataEntry`) and a titled run of them (`DataGroup`).
//!
//! A leaf of the component modules, plain data with no crate imports: `data_links` builds and
//! renders the panel from them, and a registered component's listing hook
//! (`data_listing`) returns them, so they sit below both.

/// A titled run of entries under a non-interactive group head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataGroup {
    /// The group head's text, already localised, including the count of
    /// listed entries.
    pub(crate) head: String,
    /// The listed entries; never empty.
    pub(crate) entries: Vec<DataEntry>,
}

/// One resource or annotation in the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataEntry {
    /// The localised role line (a class label, or "Region · Annotation n").
    pub(crate) role: String,
    /// The display title (a resolved title, a member name or a remark).
    pub(crate) title: String,
    /// The title's language when it is not the page's (an annotation remark's).
    pub(crate) title_lang: Option<String>,
    /// An annotation's register number; `None` for a resource and for a
    /// listed Region.
    pub(crate) number: Option<u32>,
    /// The listed resource's class; `None` for an annotation. A listing's
    /// count line counts its entries by it.
    pub(crate) class: Option<String>,
    /// The full data ARK, a boot-checked value and so a safe `href` as it
    /// stands.
    pub(crate) ark: String,
}
