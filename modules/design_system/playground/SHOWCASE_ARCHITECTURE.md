# Playground Showcase Architecture

This document describes the architecture of the Design System Playground's showcase system.

## Overview

The playground showcases are now organized into a modular structure where each component has its own dedicated showcase file. This makes the codebase more maintainable and sets up the foundation for future features like code-view toggling.

## Directory Structure

```
playground/src/playground/
├── renderer.rs                 # Core traits, structs, and registry
├── showcases/                  # Component showcase modules
│   ├── mod.rs                  # Re-exports all showcases
│   ├── button_showcase.rs      # Button component examples
│   ├── footer_showcase.rs      # Footer component examples
│   ├── header_showcase.rs      # Header component examples
│   ├── hero_showcase.rs        # Hero component examples
│   ├── icon_showcase.rs        # Icon component examples
│   ├── link_showcase.rs        # Link component examples
│   ├── menu_item_showcase.rs   # Menu item component examples
│   ├── menu_showcase.rs        # Menu component examples
│   └── shell_showcase.rs       # Shell component examples
```

## Core Structures

### `ComponentRenderer` Trait

All showcase modules implement this trait:

```rust
pub trait ComponentRenderer {
    /// Render a component with the specified variant and parameters
    fn render_variant(&self, variant: &str, params: &PlaygroundParams)
        -> PlaygroundResult<Markup>;

    /// Get the default variant for this component
    fn default_variant(&self) -> &'static str;

    /// Get all supported variants for this component
    fn supported_variants(&self) -> Vec<&'static str>;
}
```

### `ComponentRendererRegistry`

Maps component names to their renderer implementations:

```rust
pub struct ComponentRendererRegistry;

impl ComponentRendererRegistry {
    pub fn get_renderer(component: &str) -> Option<Box<dyn ComponentRenderer>> {
        match component {
            "button" => Some(Box::new(ButtonRenderer)),
            "menu" => Some(Box::new(MenuRenderer)),
            // ... other components
            _ => None,
        }
    }
}
```

## Code-View Support (Future Feature)

The architecture includes structures for capturing both the rendered component and its source code, enabling a toggle between rendered view and code view.

### `ComponentExample`

Represents a single example with both markup and code:

```rust
#[derive(Debug, Clone)]
pub struct ComponentExample {
    pub id: &'static str,
    pub name: &'static str,
    pub description: Option<&'static str>,
    pub code: &'static str,      // Captured Rust code
    pub markup: Markup,           // Rendered component
}
```

### `ComponentSection`

Groups related examples into sections:

```rust
#[derive(Debug, Clone)]
pub struct ComponentSection {
    pub title: &'static str,
    pub description: Option<&'static str>,
    pub examples: Vec<ComponentExample>,
}
```

### `example!` Macro

Simplifies creating examples by automatically capturing the code:

```rust
use crate::example;

let ex = example!{
    id: "primary-button",
    name: "Primary Button",
    description: "Main call-to-action",
    code: {
        button::button("Click Me")
            .variant(ButtonVariant::Primary)
            .onclick("console.log('Clicked!')")
            .build()
    }
};
```

The macro uses `stringify!` to capture the code block as a string while also executing it to generate the markup.

## Creating a New Showcase

To add a new component showcase:

1. **Create the showcase file**: `showcases/my_component_showcase.rs`

```rust
use components::my_component;
use maud::{html, Markup};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// My component renderer
pub struct MyComponentRenderer;

impl ComponentRenderer for MyComponentRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams)
        -> PlaygroundResult<Markup> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "my-component".to_string(),
                variant: variant.to_string(),
            });
        }

        let markup = html! {
            div class="flex flex-col gap-6 p-8" {
                section {
                    h3 class="text-lg font-semibold mb-3" { "My Component Examples" }
                    (my_component::my_component("Example"))
                }
            }
        };

        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "default"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}
```

2. **Add to `showcases/mod.rs`**:

```rust
pub mod my_component_showcase;
pub use my_component_showcase::MyComponentRenderer;
```

3. **Register in `renderer.rs`**:

```rust
impl ComponentRendererRegistry {
    pub fn get_renderer(component: &str) -> Option<Box<dyn ComponentRenderer>> {
        match component {
            // ... existing components
            "my-component" => Some(Box::new(MyComponentRenderer)),
            _ => None,
        }
    }
}
```

## Migrating to Code-View Structure

When implementing the code-view toggle feature, follow this migration pattern:

### Step 1: Update the Trait

Change `ComponentRenderer::render_variant()` to return sections:

```rust
fn render_variant(&self, variant: &str, params: &PlaygroundParams)
    -> PlaygroundResult<Vec<ComponentSection>>;
```

### Step 2: Convert Showcase to Use `example!` Macro

Replace manual HTML sections with structured examples:

```rust
// Before (current):
fn render_variant(...) -> PlaygroundResult<Markup> {
    let markup = html! {
        section {
            h3 { "Button Variants" }
            (button::button("Primary").build())
            (button::button("Secondary").variant(Secondary).build())
        }
    };
    Ok(markup)
}

// After (with code-view support):
fn render_variant(...) -> PlaygroundResult<Vec<ComponentSection>> {
    Ok(vec![
        ComponentSection {
            title: "Button Variants",
            description: None,
            examples: vec![
                example!{
                    id: "primary-button",
                    name: "Primary Button",
                    code: {
                        button::button("Primary")
                            .onclick("console.log('Primary clicked!')")
                            .build()
                    }
                },
                example!{
                    id: "secondary-button",
                    name: "Secondary Button",
                    code: {
                        button::button("Secondary")
                            .variant(ButtonVariant::Secondary)
                            .onclick("console.log('Secondary clicked!')")
                            .build()
                    }
                },
            ],
        }
    ])
}
```

### Step 3: Update the Renderer

Modify the playground template to support toggling between rendered and code views:

```rust
// Render with toggle buttons
for section in sections {
    html! {
        section {
            h3 { (section.title) }
            @for example in section.examples {
                div class="example-container" {
                    button onclick="toggleView()" { "Toggle Code" }
                    div class="rendered-view" { (example.markup) }
                    pre class="code-view hidden" { (example.code) }
                }
            }
        }
    }
}
```

## Benefits

### Current State
- **Maintainability**: Each component's showcase is in its own file (~100-300 lines each)
- **Clarity**: Clear separation of concerns
- **Scalability**: Easy to add new components without growing a single monolithic file

### Future State (with code-view)
- **Educational**: Users can see the exact code that generates each example
- **Copy-paste friendly**: Users can copy code directly from the playground
- **Interactive**: Toggle between rendered view and code view for each example
- **Documentation**: The code examples serve as live documentation

## File Size Comparison

**Before** (single file):
- `renderer.rs`: 789 lines

**After** (modular):
- `renderer.rs`: ~130 lines (trait + registry only)
- Individual showcases: ~80-200 lines each
- Total: Similar line count, but much better organized

## Guidelines

### Showcase Organization
- Group related examples into sections (e.g., "Button Variants", "Icon Buttons")
- Provide clear descriptions for each section
- Use consistent naming for example IDs (e.g., "primary-button", "icon-button-star")

### Code Examples
- Keep examples focused and minimal
- Include DataStar onclick handlers for interactive components
- Show practical use cases, not just API demonstrations
- Follow the component's documented best practices

### Documentation
- Add comments explaining non-obvious choices
- Include the example macro usage in comments for future migration
- Keep descriptions concise and actionable

## Related Files

- `/docs/src/design_system/components/` - Component documentation
- `modules/design_system/playground/SHOWCASE_GUIDELINES.md` - Visual showcase guidelines
- `modules/design_system/components/CLAUDE.md` - Component creation guidelines
