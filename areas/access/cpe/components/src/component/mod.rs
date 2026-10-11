//! The component renderers, one module per component.
//!
//! Page assembly is not here: it calls these renderers and builds their config
//! carriers. `page_picker` and `pager` are crate-internal; every other child
//! is `pub`.

pub mod annotation;
pub mod breadcrumb;
pub mod compound_viewer;
pub mod context;
pub mod data_entries;
pub mod data_links;
pub mod dom;
pub mod editorial;
pub mod facet_panel;
pub mod links;
pub mod markdown;
pub mod media;
pub mod narrative;
pub(crate) mod page_picker;
pub(crate) mod pager;
pub mod project;
pub mod resource_detail;
pub mod resource_list;
pub mod search_bar;
pub mod values;
