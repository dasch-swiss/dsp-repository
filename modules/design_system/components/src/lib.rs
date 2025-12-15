pub mod builder_common;
pub mod button;
pub mod dropdown;
pub mod footer;
pub mod header;
pub mod hero;
pub mod icon;
pub mod link;
pub mod logo_cloud;
pub mod menu;
pub mod menu_item;
pub mod shell;

// Re-export trait so it's available when using builder pattern components
pub use builder_common::ComponentBuilder;
pub use button::ButtonVariant;
pub use icon::IconType;
pub use link::LinkTarget;
pub use logo_cloud::Logo;
