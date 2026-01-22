use std::fs;
use std::path::{Path, PathBuf};

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;

/// Component page metadata from TOML
#[derive(Debug, Deserialize)]
struct ComponentPage {
    name: String,
    description: String,
    #[serde(default)]
    info: Option<ComponentSpecialInfo>,
    #[serde(default)]
    examples: Vec<ComponentExample>,
    #[serde(default)]
    references: Vec<ComponentReference>,
}

/// Optional info block for special notices
#[derive(Debug, Deserialize)]
struct ComponentSpecialInfo {
    title: String,
    description: String,
}

/// Example reference in TOML
#[derive(Debug, Deserialize)]
struct ComponentExample {
    name: String,
    title: Option<String>,
    description: Option<String>,
}

/// Component or sub-component API reference
#[derive(Debug, Deserialize)]
struct ComponentReference {
    name: String,
    description: String,
    #[serde(default)]
    extra: Option<String>,
    #[serde(default)]
    attrs: Vec<ReferenceAttribute>,
}

/// Attribute definition
#[derive(Debug, Deserialize)]
struct ReferenceAttribute {
    attr: String,
    attr_type: String,
    default: String,
    #[serde(default)]
    description: Option<String>,
}

/// Find all component directories with component.toml
fn find_components() -> Vec<(String, PathBuf)> {
    let components_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent dir")
        .join("demo/src/components");

    if !components_dir.exists() {
        return Vec::new();
    }

    let mut components = Vec::new();

    if let Ok(entries) = fs::read_dir(&components_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let component_toml = entry.path().join("component.toml");
                if component_toml.exists() {
                    if let Some(name) = entry.file_name().to_str() {
                        components.push((name.to_string(), entry.path()));
                    }
                }
            }
        }
    }

    components.sort_by(|a, b| a.0.cmp(&b.0));
    components
}

/// Parse component.toml file
fn parse_component_toml(path: &Path) -> ComponentPage {
    let toml_path = path.join("component.toml");
    let content = fs::read_to_string(&toml_path).unwrap_or_else(|_| panic!("Failed to read {:?}", toml_path));
    toml::from_str(&content).unwrap_or_else(|e| panic!("Failed to parse {:?}: {}", toml_path, e))
}

/// Read a file as plain text
fn read_file_as_string(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| panic!("Failed to read {:?}", path))
}

/// Generate Leptos routes for all components
#[proc_macro]
pub fn generate_component_routes(_input: TokenStream) -> TokenStream {
    let components = find_components();

    let routes = components.iter().map(|(name, _)| {
        let route_name = format!("/{}", name);
        let component_ident = format_ident!("{}Route", to_pascal_case(name));

        quote! {
            <Route path=StaticSegment(#route_name) view=#component_ident />
        }
    });

    let expanded = quote! {
        {
            use leptos::prelude::*;
            use leptos_router::components::*;
            use leptos_router::*;

            #(#routes)*
        }
    };

    TokenStream::from(expanded)
}

/// Generate page components for all components
#[proc_macro]
pub fn generate_component_pages(_input: TokenStream) -> TokenStream {
    let components = find_components();

    let pages = components.iter().map(|(name, path)| {
        let page = parse_component_toml(path);
        let component_ident = format_ident!("{}Route", to_pascal_case(name));
        let component_name = &page.name;
        let component_description = &page.description;

        // Import module name for component examples
        let module_name = format_ident!("{}", name);

        // Generate info block if present
        let info_block = if let Some(ref info) = page.info {
            let info_title = &info.title;
            let info_description = &info.description;
            quote! {
                <div class="mb-6 p-4 bg-blue-50 border border-blue-200 rounded">
                    <h3 class="font-semibold text-blue-900 mb-2">{#info_title}</h3>
                    <p class="text-blue-800">{#info_description}</p>
                </div>
            }
        } else {
            quote! {}
        };

        // Generate examples
        let examples = page.examples.iter().map(|example| {
            let example_name = &example.name;
            let example_title = example.title.as_deref().unwrap_or(example_name);
            let example_description = example.description.as_deref();

            let example_file = path.join(format!("examples/{}.rs", example_name));
            let example_code = read_file_as_string(&example_file);

            let example_component = format_ident!("{}Example", to_pascal_case(example_name));

            let description_view = if let Some(desc) = example_description {
                quote! { <p class="text-gray-600 mb-3">{#desc}</p> }
            } else {
                quote! {}
            };

            quote! {
                <div class="mb-8">
                    <h3 class="text-xl font-semibold mb-2">{#example_title}</h3>
                    #description_view
                    <div class="mb-4 p-6 border rounded bg-white">
                        {#example_component().into_any()}
                    </div>
                    <details class="mt-2">
                        <summary class="cursor-pointer text-sm text-blue-600 hover:text-blue-800">
                            "View Code"
                        </summary>
                        <div class="mt-2 p-4 bg-gray-50 rounded overflow-x-auto">
                            <pre><code class="language-rust">{#example_code}</code></pre>
                        </div>
                    </details>
                </div>
            }
        });

        // Generate anatomy section
        let anatomy_file = path.join("anatomy.rs");
        let anatomy_code = if anatomy_file.exists() {
            let code = read_file_as_string(&anatomy_file);
            quote! {
                <div class="mb-8">
                    <h2 class="text-2xl font-bold mb-4">"Anatomy"</h2>
                    <div class="p-4 bg-gray-50 rounded overflow-x-auto">
                        <pre><code class="language-rust">{#code}</code></pre>
                    </div>
                </div>
            }
        } else {
            quote! {}
        };

        // Generate API references
        let references = page.references.iter().map(|reference| {
            let ref_name = &reference.name;
            let ref_description = &reference.description;
            let ref_extra = reference.extra.as_deref();

            let extra_view = if let Some(extra) = ref_extra {
                quote! { <p class="text-sm text-gray-600 mt-1">{#extra}</p> }
            } else {
                quote! {}
            };

            let attrs_table = if !reference.attrs.is_empty() {
                let rows = reference.attrs.iter().map(|attr| {
                    let attr_name = &attr.attr;
                    let attr_type = &attr.attr_type;
                    let attr_default = &attr.default;
                    let attr_description = attr.description.as_deref().unwrap_or("");

                    quote! {
                        <tr class="border-t">
                            <td class="py-2 px-4 font-mono text-sm">{#attr_name}</td>
                            <td class="py-2 px-4 font-mono text-sm text-blue-600">{#attr_type}</td>
                            <td class="py-2 px-4 font-mono text-sm">{#attr_default}</td>
                            <td class="py-2 px-4 text-sm">{#attr_description}</td>
                        </tr>
                    }
                });

                quote! {
                    <table class="w-full mt-3 border rounded">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="py-2 px-4 text-left text-sm font-semibold">"Attribute"</th>
                                <th class="py-2 px-4 text-left text-sm font-semibold">"Type"</th>
                                <th class="py-2 px-4 text-left text-sm font-semibold">"Default"</th>
                                <th class="py-2 px-4 text-left text-sm font-semibold">"Description"</th>
                            </tr>
                        </thead>
                        <tbody>
                            #(#rows)*
                        </tbody>
                    </table>
                }
            } else {
                quote! {}
            };

            quote! {
                <div class="mb-6">
                    <h3 class="text-lg font-semibold">{#ref_name}</h3>
                    <p class="text-gray-700 mt-1">{#ref_description}</p>
                    #extra_view
                    #attrs_table
                </div>
            }
        });

        let api_section = if !page.references.is_empty() {
            quote! {
                <div class="mb-8">
                    <h2 class="text-2xl font-bold mb-4">"API Reference"</h2>
                    #(#references)*
                </div>
            }
        } else {
            quote! {}
        };

        // Generate the page component
        quote! {
            #[::leptos::component]
            pub fn #component_ident() -> impl ::leptos::IntoView {
                use ::leptos::prelude::*;
                use crate::components::#module_name::*;

                view! {
                    <div class="max-w-5xl mx-auto p-6">
                        <h1 class="text-4xl font-bold mb-3">{#component_name}</h1>
                        <p class="text-xl text-gray-600 mb-6">{#component_description}</p>

                        #info_block

                        <div class="mb-8">
                            <h2 class="text-2xl font-bold mb-4">"Examples"</h2>
                            #(#examples)*
                        </div>

                        #anatomy_code

                        #api_section
                    </div>
                }
            }
        }
    });

    let expanded = quote! {
        #(#pages)*
    };

    TokenStream::from(expanded)
}

/// Convert kebab-case or snake_case to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split(['-', '_'])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}
