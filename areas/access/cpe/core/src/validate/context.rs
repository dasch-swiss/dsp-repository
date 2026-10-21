//! The caller-supplied context of one project's validation.
//!
//! A leaf of `validate`: the registry, `project` and `validate`'s own child
//! modules import [`ValidationContext`] from here, never through `validate`.

/// Caller-supplied context for a single project's validation.
///
/// This crate owns the validation checks; the caller (the host) supplies the
/// values it alone can enumerate (reserved ids, narrative file inventory).
pub struct ValidationContext<'a> {
    /// The project id (the kebab-case directory name under `projects/`).
    pub project_id: &'a str,
    /// Engine-reserved ids the host must not use as project ids. The host
    /// passes `&["_static"]`; this crate owns the check.
    pub reserved_ids: &'a [&'static str],
    /// The bare names of the project's narrative files (e.g. `home.de.md`),
    /// enumerated by the host before calling `validate`. Used by the narrative
    /// checks.
    pub narrative_files: &'a [String],
    /// Whether `narrative_files` is the project's whole narrative inventory.
    /// When it is, an empty list asserts "this project has no narrative
    /// files" and every named narrative is checked against it; when it is not,
    /// an empty list means "unknown" and existence is not checked.
    pub narrative_files_complete: bool,
    /// Relative names from the project's `assets/` directory, enumerated by
    /// the host before calling `validate`. Unlike `narrative_files` (which
    /// genuinely is bare), each entry carries the `assets/` segment as its
    /// own first path component (e.g. `"assets/cover.webp"`), matching how
    /// `cover-image` is authored in config. Used by the site index card's
    /// `cover-image` existence check.
    ///
    /// This is an **unconditional inventory**: callers MUST supply the
    /// project's real `assets/` listing. `&[]` is not "unknown" — it
    /// asserts "this project has no asset files", and will hard-fail any
    /// declared `cover-image` with
    /// [`DiagnosticKind::CoverImageNotFound`](crate::diagnostic::DiagnosticKind::CoverImageNotFound).
    pub asset_files: &'a [String],
}

impl<'a> ValidationContext<'a> {
    /// Convenience constructor for tests: empty `reserved_ids` + empty,
    /// incomplete `narrative_files` + empty `asset_files`.
    ///
    /// The empty `asset_files` means "this project has no asset files"
    /// (see the field doc) — fine for the configs that use this
    /// constructor, none of which declare a `cover-image`.
    #[doc(hidden)]
    pub fn for_tests(project_id: &'a str) -> Self {
        Self {
            project_id,
            reserved_ids: &[],
            narrative_files: &[],
            narrative_files_complete: false,
            asset_files: &[],
        }
    }
}
