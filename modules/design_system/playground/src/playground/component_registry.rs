use crate::playground::docs_parser;

#[derive(Debug, Clone)]
pub struct ComponentInfo {
    pub name: String,
    pub route_name: String,
    pub variants: Vec<ComponentVariant>,
}

#[derive(Debug, Clone)]
pub struct ComponentVariant {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct ComponentSpec {
    pub name: &'static str,
    pub route_name: &'static str,
    pub variants: &'static [ComponentVariantSpec],
    pub doc_file: &'static str,
}

#[derive(Debug, Clone)]
pub struct ComponentVariantSpec {
    pub name: &'static str,
    pub value: &'static str,
}

const COMPONENTS: &[ComponentSpec] = &[
    ComponentSpec {
        name: "Button",
        route_name: "button",
        variants: &[
            ComponentVariantSpec { name: "Primary", value: "primary" },
            ComponentVariantSpec { name: "Secondary", value: "secondary" },
            ComponentVariantSpec { name: "Icon Hamburger", value: "icon-hamburger" },
            ComponentVariantSpec { name: "Icon Close", value: "icon-close" },
            ComponentVariantSpec { name: "Disabled Primary", value: "disabled-primary" },
        ],
        doc_file: "button.md",
    },
    ComponentSpec {
        name: "Dropdown",
        route_name: "dropdown",
        variants: &[
            ComponentVariantSpec { name: "Secondary Button", value: "secondary" },
            ComponentVariantSpec { name: "MoreVert Icon", value: "more-vert" },
            ComponentVariantSpec { name: "Hamburger Icon", value: "hamburger" },
        ],
        doc_file: "dropdown.md",
    },
    ComponentSpec {
        name: "Footer",
        route_name: "footer",
        variants: &[ComponentVariantSpec { name: "Default", value: "default" }],
        doc_file: "footer.md",
    },
    ComponentSpec {
        name: "Header",
        route_name: "header",
        variants: &[ComponentVariantSpec { name: "Default", value: "default" }],
        doc_file: "header.md",
    },
    ComponentSpec {
        name: "Hero",
        route_name: "hero",
        variants: &[ComponentVariantSpec { name: "Default", value: "default" }],
        doc_file: "hero.md",
    },
    ComponentSpec {
        name: "Icon",
        route_name: "icon",
        variants: &[ComponentVariantSpec { name: "Close", value: "close" }],
        doc_file: "icon.md",
    },
    ComponentSpec {
        name: "Link",
        route_name: "link",
        variants: &[
            ComponentVariantSpec { name: "Internal", value: "internal" },
            ComponentVariantSpec { name: "Blank (New Tab)", value: "blank" },
            ComponentVariantSpec { name: "Parent Frame", value: "parent" },
            ComponentVariantSpec { name: "Top Frame", value: "top" },
        ],
        doc_file: "link.md",
    },
    ComponentSpec {
        name: "Logo Cloud",
        route_name: "logo-cloud",
        variants: &[ComponentVariantSpec { name: "Default", value: "default" }],
        doc_file: "logo-cloud.md",
    },
    ComponentSpec {
        name: "Menu",
        route_name: "menu",
        variants: &[
            ComponentVariantSpec { name: "Text Trigger", value: "text-trigger" },
            ComponentVariantSpec { name: "Icon Trigger", value: "icon-trigger" },
        ],
        doc_file: "menu.md",
    },
    ComponentSpec {
        name: "Menu Item",
        route_name: "menu-item",
        variants: &[ComponentVariantSpec { name: "All Examples", value: "default" }],
        doc_file: "menu-item.md",
    },
    ComponentSpec {
        name: "Shell",
        route_name: "shell",
        variants: &[ComponentVariantSpec { name: "Header Only", value: "header-only" }],
        doc_file: "shell.md",
    },
];

impl ComponentInfo {
    /// Returns the default variant (first variant in the list)
    pub fn get_default_variant(&self) -> Option<&ComponentVariant> {
        self.variants.first()
    }
}

pub fn get_all_components() -> Vec<ComponentInfo> {
    COMPONENTS
        .iter()
        .filter_map(|spec| {
            docs_parser::load_component_documentation(spec)
                .ok()
                .map(|doc| doc.to_component_info())
        })
        .collect()
}

pub fn get_component_info_by_name(name: &str) -> Option<ComponentInfo> {
    get_all_components().into_iter().find(|c| c.route_name == name)
}

pub fn get_component_spec_by_route_name(route_name: &str) -> Option<&ComponentSpec> {
    COMPONENTS.iter().find(|spec| spec.route_name == route_name.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_info_get_default_variant_returns_first() {
        let component_info = ComponentInfo {
            name: "Button".to_string(),
            route_name: "button".to_string(),
            variants: vec![
                ComponentVariant { name: "Primary".to_string(), value: "primary".to_string() },
                ComponentVariant {
                    name: "Secondary".to_string(),
                    value: "secondary".to_string(),
                },
            ],
        };
        let default_variant = component_info.get_default_variant();
        assert!(default_variant.is_some());
        let variant = default_variant.unwrap();
        assert_eq!(variant.name, "Primary");
        assert_eq!(variant.value, "primary");
    }

    #[test]
    fn test_component_info_get_default_variant_returns_first_when_multiple() {
        let component_info = ComponentInfo {
            name: "Test".to_string(),
            route_name: "test".to_string(),
            variants: vec![
                ComponentVariant { name: "First".to_string(), value: "first".to_string() },
                ComponentVariant { name: "Second".to_string(), value: "second".to_string() },
            ],
        };
        let default_variant = component_info.get_default_variant();
        assert!(default_variant.is_some());
        let variant = default_variant.unwrap();
        assert_eq!(variant.name, "First");
        assert_eq!(variant.value, "first");
    }

    #[test]
    fn test_component_info_get_default_variant_empty_variants() {
        let component_info = ComponentInfo {
            name: "Test".to_string(),
            route_name: "test".to_string(),
            variants: vec![],
        };
        let default_variant = component_info.get_default_variant();
        assert!(default_variant.is_none());
    }

    // Tests for get_component_by_name()
    // no tests with valid input because they would depend on the documentation files existing
    #[test]
    fn test_get_component_by_name_invalid() {
        let component = get_component_info_by_name("nonexistent");
        assert!(component.is_none());
    }

    #[test]
    fn test_get_component_by_name_empty_string() {
        let component = get_component_info_by_name("");
        assert!(component.is_none());
    }

    // Tests for get_component_spec_by_route_name() - tests static data
    #[test]
    fn test_get_component_spec_by_route_name_valid_button() {
        let spec = get_component_spec_by_route_name("button");
        assert!(spec.is_some());
        let spec = spec.unwrap();
        assert_eq!(spec.name, "Button");
        assert_eq!(spec.route_name, "button");
        assert_eq!(spec.doc_file, "button.md");
        assert!(!spec.variants.is_empty());
    }

    #[test]
    fn test_get_component_spec_by_route_name_case_insensitive() {
        let spec = get_component_spec_by_route_name("BUTTON");
        assert!(spec.is_some());
        let spec = spec.unwrap();
        assert_eq!(spec.name, "Button");
        assert_eq!(spec.route_name, "button");
    }

    #[test]
    fn test_get_component_spec_by_route_name_invalid() {
        let spec = get_component_spec_by_route_name("nonexistent");
        assert!(spec.is_none());
    }

    #[test]
    fn test_get_component_spec_by_route_name_empty_string() {
        let spec = get_component_spec_by_route_name("");
        assert!(spec.is_none());
    }
}
