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
    pub is_default: bool,
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
    pub is_default: bool,
}

const COMPONENTS: &[ComponentSpec] = &[
    ComponentSpec {
        name: "Button",
        route_name: "button",
        variants: &[
            ComponentVariantSpec { name: "Primary", value: "primary", is_default: true },
            ComponentVariantSpec { name: "Secondary", value: "secondary", is_default: false },
            ComponentVariantSpec { name: "Outline", value: "outline", is_default: false },
        ],
        doc_file: "button.md",
    },
    ComponentSpec {
        name: "Banner",
        route_name: "banner",
        variants: &[
            ComponentVariantSpec { name: "Accent Only", value: "accent_only", is_default: true },
            ComponentVariantSpec { name: "With Prefix", value: "with_prefix", is_default: false },
            ComponentVariantSpec { name: "With Suffix", value: "with_suffix", is_default: false },
            ComponentVariantSpec { name: "Full", value: "full", is_default: false },
        ],
        doc_file: "banner.md",
    },
    ComponentSpec {
        name: "Link",
        route_name: "link",
        variants: &[ComponentVariantSpec { name: "Default", value: "default", is_default: true }],
        doc_file: "link.md",
    },
    ComponentSpec {
        name: "Shell",
        route_name: "shell",
        variants: &[
            ComponentVariantSpec { name: "Header Only", value: "header-only", is_default: true },
            ComponentVariantSpec {
                name: "With Side Nav",
                value: "with-side-nav",
                is_default: false,
            },
        ],
        doc_file: "shell.md",
    },
    ComponentSpec {
        name: "Tag",
        route_name: "tag",
        variants: &[
            ComponentVariantSpec { name: "Gray", value: "gray", is_default: true },
            ComponentVariantSpec { name: "Blue", value: "blue", is_default: false },
            ComponentVariantSpec { name: "Green", value: "green", is_default: false },
        ],
        doc_file: "tag.md",
    },
    ComponentSpec {
        name: "Tile",
        route_name: "tile",
        variants: &[
            ComponentVariantSpec { name: "Base", value: "base", is_default: true },
            ComponentVariantSpec { name: "Clickable", value: "clickable", is_default: false },
        ],
        doc_file: "tile.md",
    },
    ComponentSpec {
        name: "TailwindExperiment",
        route_name: "tailwind-experiment",
        variants: &[ComponentVariantSpec { name: "Default", value: "default", is_default: true }],
        doc_file: "tailwind_experiment.md",
    },
    ComponentSpec {
        name: "TailwindAskamaExperiment",
        route_name: "tailwind-askama-experiment",
        variants: &[ComponentVariantSpec { name: "Default", value: "default", is_default: true }],
        doc_file: "tailwind_askama_experiment.md",
    },
];

impl ComponentInfo {
    pub fn get_default_variant(&self) -> Option<&ComponentVariant> {
        self.variants.iter().find(|v| v.is_default)
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
    fn test_component_info_get_default_variant_with_default() {
        let component_info = ComponentInfo {
            name: "Button".to_string(),
            route_name: "button".to_string(),
            variants: vec![
                ComponentVariant {
                    name: "Primary".to_string(),
                    value: "primary".to_string(),
                    is_default: true,
                },
                ComponentVariant {
                    name: "Secondary".to_string(),
                    value: "secondary".to_string(),
                    is_default: false,
                },
            ],
        };
        let default_variant = component_info.get_default_variant();
        assert!(default_variant.is_some());
        let variant = default_variant.unwrap();
        assert_eq!(variant.name, "Primary");
        assert_eq!(variant.value, "primary");
        assert!(variant.is_default);
    }

    #[test]
    fn test_component_info_get_default_variant_no_default() {
        let component_info = ComponentInfo {
            name: "Test".to_string(),
            route_name: "test".to_string(),
            variants: vec![
                ComponentVariant {
                    name: "First".to_string(),
                    value: "first".to_string(),
                    is_default: false,
                },
                ComponentVariant {
                    name: "Second".to_string(),
                    value: "second".to_string(),
                    is_default: false,
                },
            ],
        };
        let default_variant = component_info.get_default_variant();
        assert!(default_variant.is_none());
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
    fn test_get_component_spec_by_route_name_valid_banner() {
        let spec = get_component_spec_by_route_name("banner");
        assert!(spec.is_some());
        let spec = spec.unwrap();
        assert_eq!(spec.name, "Banner");
        assert_eq!(spec.route_name, "banner");
        assert_eq!(spec.doc_file, "banner.md");
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
