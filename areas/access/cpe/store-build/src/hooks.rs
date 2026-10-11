//! The builder's table of hooks: one row per project whose `remodel` block declares one.
//!
//! The block and the table are held to each other. A build refuses a block that declares a
//! hook when it is given none (`hook-not-registered`), and a hook for a block that declares
//! none (`hook-not-declared`).

use crate::hook::Hook;
use crate::projects;

/// Every project that has a hook. A project's row names its folder and the `fn` of its hook
/// module.
pub const HOOKS: &[Hook] = &[Hook {
    project: "incunabula",
    remodel: projects::incunabula::remodel,
}];

/// The hook of the project with the folder name `project`, if it has one.
pub fn resolve(project: &str) -> Option<&'static Hook> {
    HOOKS.iter().find(|hook| hook.project == project)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// `resolve` gives the first row of a project: a second one would never be called.
    #[test]
    fn test_hooks_every_row_names_another_project() {
        let projects: BTreeSet<&str> = HOOKS.iter().map(|hook| hook.project).collect();

        assert_eq!(projects.len(), HOOKS.len());
    }
}
