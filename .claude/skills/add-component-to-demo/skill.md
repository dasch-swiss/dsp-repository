---
name: add-mosaic-component-to-demo
description: Adds a newly created component to the demo crate
---
# Mosaic Demo - Component Examples Guide

This guide explains how to add a component demo page to the demo application.
The crate is found in modules/mosaic/demo.

## Overview

The Mosaic Demo application provides a showcase environment for testing and documenting components from the mosaic-tiles library.
Its audience is developers and project managers.

It features:

- **Leptos 0.8**: Web framework for server-side rendering and hydration
- **Automatic routing**: Routes generated from component TOML metadata
- **Syntax highlighting**: Prism.js for code examples
- **Live examples**: Interactive component demonstrations
- **Structured documentation**: Anatomy diagrams and multiple usage examples

## Demo Structure

Each component demo follows this structure:
The `component_name` is ${ARGUMENTS}.

```
src/components/[component_name]/
├── mod.rs                      # Module exports
├── anatomy.rs                  # Component anatomy/structure example
├── component.toml              # Component metadata and example definitions
└── examples/
    ├── mod.rs                  # Example exports
    ├── example_name_1.rs       # Individual example files
    ├── example_name_2.rs
    └── ...
```

You can find the code of the component in the /modules/mosaic/tiles/component/[component_name] folder.

## Step-by-Step: Adding a New Component Demo

### 1. Create Component Directory Structure

Create the component directory:

```bash
mkdir -p src/components/[component_name]/examples
```

### 2. Create Anatomy File

Create `src/components/[component_name]/anatomy.rs`:

```rust
use leptos::prelude::*;
use mosaic_tiles::component_name::*;

#[component]
pub fn ComponentNameAnatomy() -> impl IntoView {
    view! {
        <ComponentName>
            // Component structure example
        </ComponentName>
    }
}
```

This file shows the basic structure and required parts of the component.

### 3. Create Example Files

Create individual example files in `src/components/[component_name]/examples/`.

Example `src/components/[component_name]/examples/variants.rs`:

```rust
use leptos::prelude::*;
use mosaic_tiles::component_name::*;

#[component]
pub fn VariantsExample() -> impl IntoView {
    view! {
        <div class="flex gap-4 items-center">
            <ComponentName variant=Variant::Primary>
                "Primary Variant"
            </ComponentName>
            <ComponentName variant=Variant::Secondary>
                "Secondary Variant"
            </ComponentName>
        </div>
    }
}
```

#### Example Naming Convention

- Use descriptive names: `variants.rs`, `sizes.rs`, `interactive.rs`, `with_icons.rs`
- Component function should be `[ExampleName]Example` (PascalCase + "Example" suffix)
- File names should be snake_case

### 4. Create Examples Module

Create `src/components/[component_name]/examples/mod.rs`:

```rust
pub mod variants;
pub mod interactive;
pub mod with_icons;

pub use variants::*;
pub use interactive::*;
pub use with_icons::*;
```

List all example files and re-export them.

### 5. Create Component Module

Create `src/components/[component_name]/mod.rs`:

```rust
mod anatomy;
mod examples;

pub use anatomy::*;
pub use examples::*;
```

### 6. Create Component TOML Configuration

Create `src/components/[component_name]/component.toml`:

```toml
name = "ComponentName"
description = "Brief description of what this component does."

[[examples]]
name = "variants"
title = "Component Variants"
description = "Available visual styles and variants"

[[examples]]
name = "interactive"
title = "Interactive Example"
description = "Component with click handlers and reactive state"

[[references]]
name = "ComponentName"
description = "Main component description."
extra = "Additional implementation notes."

[[references.attrs]]
attr = "variant"
attr_type = "ComponentVariant"
default = "Primary"
description = "Visual style variant"

[[references.attrs]]
attr = "children"
attr_type = "Option<Children>"
default = "None"
description = "Component content"
```

#### TOML Structure

- **`name`**: Component display name
- **`description`**: Short component description
- **`[[examples]]`**: Array of example configurations
  - `name`: Must match the example file name (without .rs)
  - `title`: Display title for the example
  - `description`: What this example demonstrates
- **`[[references]]`**: Component API documentation
  - `name`: Component name
  - `description`: Component purpose
  - `extra`: Additional notes
  - `attrs`: Array of component attributes/props
    - `attr`: Attribute name
    - `attr_type`: Rust type
    - `default`: Default value
    - `description`: What this attribute does

### 7. Register in Components Module

Add to `src/components/mod.rs`:

```rust
pub mod component_name;
```

### 8. Add Navigation Link

Edit `src/app.rs` to add a navigation link in the sidebar.

```rust
<A href="/component-name" attr:class="text-gray-700 hover:text-gray-900">
    "ComponentName"
</A>
```

And add a route (around line 108-112):

```rust
<Route path=StaticSegment("component-name") view=ComponentNameRoute />
```

Note: The route is auto-generated by the `demo_macro::generate_component_pages!()` macro based on the component name.

### 9. Build and Test

Run the demo application:

```bash
cargo leptos watch
```

The demo will be available at `http://localhost:3000`.

Navigate to `/component-name` to see your component page.

## Example Categories

Common example patterns:

- **`variants.rs`**: Different visual styles (primary, secondary, etc.)
- **`sizes.rs`**: Size variations (small, medium, large)
- **`types.rs`**: Different component types
- **`disabled.rs`**: Disabled/inactive states
- **`interactive.rs`**: Components with event handlers and reactive state
- **`with_icons.rs`**: Components combined with icon components
- **`with_images.rs`**: Components containing images
- **`usage.rs`**: Common usage patterns
- **`basic.rs`**: Simple, minimal example
- **`default_open.rs`**: Components in default/initial states
- **`rich_content.rs`**: Complex content examples

## Auto-Generated Routes

The `demo_macro::generate_component_pages!()` macro in `src/lib.rs`:

1. Scans all `component.toml` files in `src/components/*/`
2. Generates route components named `[ComponentName]Route`
3. Creates pages with:
   - Component title and description
   - Anatomy section
   - Multiple example sections with live demos
   - API reference table
   - Syntax-highlighted source code for each example

The generated routes are automatically imported and available in `src/app.rs`.

## Page Layout

Each generated component page includes:

1. **Header**: Component name and description
2. **Anatomy**: Basic component structure
3. **Examples**: Interactive demonstrations for each example
4. **API Reference**: Component properties table
5. **Source Code**: Collapsible Rust code for each example

## Styling Examples

Use Tailwind CSS classes for layout within examples:

```rust
view! {
    <div class="flex gap-4 items-center">
        // Examples arranged horizontally
    </div>

    <div class="grid grid-cols-2 gap-4">
        // Examples in a grid
    </div>

    <div class="space-y-4">
        // Examples stacked vertically
    </div>
}
```

## Testing Examples

### Manual Testing

1. Run `cargo leptos watch` from the demo directory
2. Navigate to the component page
3. Interact with examples
4. Verify responsive behavior
5. Test dark mode if applicable
6. Check syntax highlighting displays correctly

## Checklist

When adding a new component demo:

- [ ] Create `src/components/[name]/` directory
- [ ] Create `anatomy.rs` with component structure example
- [ ] Create `examples/` directory with individual example files
- [ ] Create `examples/mod.rs` exporting all examples
- [ ] Create `mod.rs` exporting anatomy and examples
- [ ] Create `component.toml` with metadata and example definitions
- [ ] Add component to `src/components/mod.rs`
- [ ] Add navigation link in `src/app.rs`
- [ ] Add route in `src/app.rs`
- [ ] Verify all examples compile and display correctly
- [ ] Check source code syntax highlighting
- [ ] Ensure example names in TOML match file names exactly

## Common Patterns

### Interactive Examples with State

```rust
use leptos::prelude::*;
use mosaic_tiles::button::*;

#[component]
pub fn InteractiveExample() -> impl IntoView {
    let (count, set_count) = signal(0);

    view! {
        <div class="flex flex-col gap-4">
            <p>"Count: " {count}</p>
            <Button on_click=move |_| set_count.update(|n| *n += 1)>
                "Increment"
            </Button>
        </div>
    }
}
```

### Examples with Icons

```rust
use leptos::prelude::*;
use mosaic_tiles::button::*;
use mosaic_tiles::icon::*;

#[component]
pub fn WithIconsExample() -> impl IntoView {
    view! {
        <Button>
            <Icon icon=IconKind::ChevronRight />
            "Button with Icon"
        </Button>
    }
}
```

### Grid Layout Examples

```rust
#[component]
pub fn VariantsExample() -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            // Multiple variations
        </div>
    }
}
```

## Troubleshooting

### Route Not Found

- Verify the component name in TOML matches the expected pattern
- Check that `demo_macro::generate_component_pages!()` is called in `src/lib.rs`
- Rebuild the project completely: `cargo clean && cargo leptos watch`

### Example Not Showing

- Ensure example name in TOML exactly matches the file name (without .rs)
- Verify the example is exported in `examples/mod.rs`
- Check that the component function name follows the `[Name]Example` pattern

### Syntax Highlighting Not Working

- Verify Prism.js scripts are loaded in `src/app.rs`
- Check that code blocks use the correct language class
- Wait for the page to fully load (Prism runs after hydration)

### Navigation Link Not Appearing

- Check that the link is added in `src/app.rs` within the nav element
- Verify the href matches the route path
-
