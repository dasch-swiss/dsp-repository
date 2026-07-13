//! Project page components.
//!
//! `organization_name`, `person`, `affiliation_name`, and the sidebar's
//! `entity_name` resolve persons/organizations synchronously from the
//! in-process `OnceLock<HashMap>` caches in `dpe-core` — no async layer.

pub mod breadcrumb;
pub mod contributor;
pub mod description;
pub mod info_card;
pub mod organization_name;
pub mod person;
pub mod project_details;
pub mod project_details_tabs;
pub mod project_header;
pub mod project_sidebar;
pub mod publications_section;
