// Placeholder components for basic_website
// These should eventually be moved to the main design system

pub mod article_category;
pub mod callout;
pub mod info_card;
pub mod news_card;
pub mod pagination;
pub mod project_card;
pub mod section_header;
pub mod service_card;
pub mod status_badge;

// Layout components
pub mod container;
pub mod cta_link;
pub mod flex_center;
pub mod grid;
pub mod info_box;
pub mod section;

pub use article_category::{article_category, Article};
pub use callout::{callout, callout_with_heading, CalloutVariant};
pub use container::{container, ContainerWidth};
pub use cta_link::cta_link_centered;
pub use flex_center::flex_between;
pub use grid::{card_grid, grid, grid_constrained, project_grid, stats_grid, GridColumns};
pub use info_card::info_card;
pub use news_card::{news_card, NewsCard};
pub use pagination::pagination;
pub use project_card::{project_card, ProjectCard};
pub use section::{
    content_section, content_section_compact_labeled, content_section_gray_labeled, content_section_labeled,
};
pub use section_header::section_header_simple;
pub use service_card::{service_card, ServiceCard};
pub use status_badge::status_badge_active;
