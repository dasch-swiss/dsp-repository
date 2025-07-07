#[derive(Debug, Clone)]
pub struct ComponentInfo {
    pub name: String,
    pub route_name: String,
    pub variants: Vec<ComponentVariant>,
    pub supports_theme: bool,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct ComponentVariant {
    pub name: String,
    pub value: String,
    pub is_default: bool,
}

impl ComponentInfo {
    pub fn get_default_variant(&self) -> Option<&ComponentVariant> {
        self.variants.iter().find(|v| v.is_default)
    }
}

pub fn get_all_components() -> Vec<ComponentInfo> {
    vec![
        ComponentInfo {
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
                ComponentVariant {
                    name: "Outline".to_string(),
                    value: "outline".to_string(),
                    is_default: false,
                },
            ],
            supports_theme: true,
            description: "Interactive button component".to_string(),
        },
        ComponentInfo {
            name: "Banner".to_string(),
            route_name: "banner".to_string(),
            variants: vec![
                ComponentVariant {
                    name: "Accent Only".to_string(),
                    value: "accent_only".to_string(),
                    is_default: true,
                },
                ComponentVariant {
                    name: "With Prefix".to_string(),
                    value: "with_prefix".to_string(),
                    is_default: false,
                },
                ComponentVariant {
                    name: "With Suffix".to_string(),
                    value: "with_suffix".to_string(),
                    is_default: false,
                },
                ComponentVariant {
                    name: "Full".to_string(),
                    value: "full".to_string(),
                    is_default: false,
                },
            ],
            supports_theme: true,
            description: "Banner component with multiple display variants".to_string(),
        },
        ComponentInfo {
            name: "Link".to_string(),
            route_name: "link".to_string(),
            variants: vec![ComponentVariant {
                name: "Default".to_string(),
                value: "default".to_string(),
                is_default: true,
            }],
            supports_theme: true,
            description: "Link component".to_string(),
        },
        ComponentInfo {
            name: "Shell".to_string(),
            route_name: "shell".to_string(),
            variants: vec![ComponentVariant {
                name: "Default".to_string(),
                value: "default".to_string(),
                is_default: true,
            }],
            supports_theme: true,
            description: "Application shell component".to_string(),
        },
        ComponentInfo {
            name: "Tag".to_string(),
            route_name: "tag".to_string(),
            variants: vec![
                ComponentVariant {
                    name: "Gray".to_string(),
                    value: "gray".to_string(),
                    is_default: true,
                },
                ComponentVariant {
                    name: "Blue".to_string(),
                    value: "blue".to_string(),
                    is_default: false,
                },
                ComponentVariant {
                    name: "Green".to_string(),
                    value: "green".to_string(),
                    is_default: false,
                },
            ],
            supports_theme: true,
            description: "Tag component with different color variants".to_string(),
        },
        ComponentInfo {
            name: "Tile".to_string(),
            route_name: "tile".to_string(),
            variants: vec![
                ComponentVariant {
                    name: "Base".to_string(),
                    value: "base".to_string(),
                    is_default: true,
                },
                ComponentVariant {
                    name: "Clickable".to_string(),
                    value: "clickable".to_string(),
                    is_default: false,
                },
            ],
            supports_theme: true,
            description: "Tile component with base and clickable variants".to_string(),
        },
    ]
}

pub fn get_component_by_name(name: &str) -> Option<ComponentInfo> {
    get_all_components().into_iter().find(|c| c.route_name == name)
}
