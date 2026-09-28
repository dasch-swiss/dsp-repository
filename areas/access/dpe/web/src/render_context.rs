/// View-rendering settings that vary by deployment or test, passed by reference
/// rather than read from a process-global. `dpe-server` builds one from
/// `AppState` and hands it to page functions and the components that need it.
pub struct RenderContext {
    /// Whether placeholder values ("MISSING", "CALCULATED") render, styled red
    /// for QA visibility, instead of being hidden.
    pub show_placeholder_values: bool,
}
