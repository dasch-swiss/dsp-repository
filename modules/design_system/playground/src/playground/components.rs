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
        variants: &[ComponentVariantSpec { name: "Default", value: "default", is_default: true }],
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

pub fn get_component_by_name(name: &str) -> Option<ComponentInfo> {
    get_all_components().into_iter().find(|c| c.route_name == name)
}

pub fn get_component_spec_by_route_name(route_name: &str) -> Option<&ComponentSpec> {
    COMPONENTS.iter().find(|spec| spec.route_name == route_name.to_lowercase())
}
