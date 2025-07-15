use std::path::Path;

use pulldown_cmark::{html, Parser};

use crate::playground::components;

#[derive(Debug, Clone)]
pub struct ComponentDocumentationFrontmatter {
    pub name: String,
    pub route_name: String,
    pub variants: Vec<ComponentVariantFrontmatter>,
}

#[derive(Debug, Clone)]
pub struct ComponentVariantFrontmatter {
    pub name: String,
    pub value: String,
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct ComponentDocumentation {
    pub frontmatter: ComponentDocumentationFrontmatter,
    pub content_html: String,
}

impl ComponentDocumentation {
    pub fn to_component_info(&self) -> components::ComponentInfo {
        components::ComponentInfo {
            name: self.frontmatter.name.clone(),
            route_name: self.frontmatter.route_name.clone(),
            variants: self
                .frontmatter
                .variants
                .iter()
                .map(|v| components::ComponentVariant {
                    name: v.name.clone(),
                    value: v.value.clone(),
                    is_default: v.is_default,
                })
                .collect(),
        }
    }
}

pub fn load_component_documentation(
    spec: &components::ComponentSpec,
) -> Result<ComponentDocumentation, std::io::Error> {
    let docs_path = Path::new("docs/src/design_system/components").join(spec.doc_file);

    let content = std::fs::read_to_string(&docs_path)?;

    let frontmatter = ComponentDocumentationFrontmatter {
        name: spec.name.to_string(),
        route_name: spec.route_name.to_string(),
        variants: spec
            .variants
            .iter()
            .map(|v| ComponentVariantFrontmatter {
                name: v.name.to_string(),
                value: v.value.to_string(),
                is_default: v.is_default,
            })
            .collect(),
    };

    let parser = Parser::new(&content);
    let mut content_html = String::new();
    html::push_html(&mut content_html, parser);

    Ok(ComponentDocumentation { frontmatter, content_html })
}
