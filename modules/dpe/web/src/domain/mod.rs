// dpe-web domain logic: the projects-list query/filter state (`ProjectQuery`),
// the project-list and single-project loaders, and the contributor resolver.
//
// Domain *types* (Project, Person, Record, …) live in dpe-core and are imported
// directly by consumers — there is no re-export shim here.

pub mod contributors;
pub mod project;
pub mod projects;

pub use contributors::get_contributors;
pub use project::ProjectQuery;
pub use projects::{get_project, list_data_languages, list_projects, list_type_of_data};
