---
name: create-mosaic-component
description: Create a new reusable leptos component in the modules/mosaic/tiles create
---

# Mosaic Tiles - Component Creation Guide

This guide explains how to create new Leptos components in the modules/mosaic/tiles crate.

## Overview

Mosaic Tiles is a Leptos component library that uses:

- **Leptos 0.8**: Web framework for Rust
- **Tailwind CSS v4**: Styling with utility classes
- **Feature flags**: Each component is opt-in via Cargo features
- **Build-time CSS bundling**: CSS is compiled at build time via build.rs

## Component Structure

Each component follows this structure:
The `component_name` is ${ARGUMENTS}.

```
src/components/[component_name]/
├── mod.rs              # Component implementation
└── [component_name].css # Tailwind CSS styles
```

## Step-by-Step: Creating a New Component

### 1. Create Component Directory and Files

Create the component directory:

```bash
mkdir -p src/components/[component_name]
```

Create `src/components/[component_name]/mod.rs`:

```rust
use leptos::prelude::*;

#[component]
pub fn ComponentName(
    #[prop(optional, into)] disabled: MaybeProp<bool>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="component-class">
            {
                if let Some(children) = children {
                    Either::Left(children())
                } else {
                    Either::Right(())
                }
            }
        </div>
    }
}
```

Next create the styling for the component in a separate file.
Whenever possible use tailwind css classes.
Do not add styles for a dark mode prefixed with `dark:`.
Create `src/components/[component_name]/[component_name].css`:

```css
@layer components {
  .component-class {
    @apply /* tailwind utility classes */;
  }
}
```

### 2. Register Component in Module System

Add to `src/components/mod.rs`:

```rust
#[cfg(feature = "component_name")]
pub mod component_name;
```

### 3. Add Feature Flag to Cargo.toml

Add the feature to the `[features]` section:

```toml
[features]
default = ["button", "theme_provider"]
button = []
component_name = []  # Add your new component here
theme_provider = []
```

Add to `default` always.

### 4. Register CSS in Build Script

Edit `build.rs` and add your component to the `features!` macro on line 108:

```rust
let features = features!("button", "component_name");
```

### 5. Export from Library Root (Optional)

The component should be available at the crate root.

Add to `src/lib.rs`:

```rust
pub use components::component_name::ComponentName;
```

This way users can import via:

```rust
use mosaic_tiles::ComponentName;
```

## Component Implementation Patterns

### Props

Use Leptos prop attributes:

- `#[prop(optional)]` - Optional prop
- `#[prop(optional, into)]` - Optional prop with Into conversion
- `#[prop(default = value)]` - Prop with default value

### Variants

Use enums for variant types:

```rust
#[derive(Debug, Clone, Default)]
pub enum ComponentVariant {
    #[default]
    Primary,
    Secondary,
}
```

### Children

Handle optional children with `Either`:

```rust
use leptos::either::Either;

#[component]
pub fn Component(
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <div>
            {
                if let Some(children) = children {
                    Either::Left(children())
                } else {
                    Either::Right(())
                }
            }
        </div>
    }
}
```

### Event Handlers

Use callbacks for event handlers:

```rust
use leptos::ev::MouseEvent;

#[component]
pub fn Component(
    #[prop(optional, into)] on_click: Option<Callback<MouseEvent>>,
) -> impl IntoView {
    let on_click = move |e| {
        let Some(on_click) = on_click.as_ref() else {
            return;
        };
        on_click.run(e);
    };

    view! {
        <button on:click=on_click>"Click"</button>
    }
}
```

### Reactive State

Use `Memo` for derived reactive values:

```rust
let is_disabled = Memo::new(move |_| disabled.get().unwrap_or(false));
```

## CSS Styling

### File Location

Each component has its own CSS file: `src/components/[component_name]/[component_name].css`

### Tailwind CSS Classes

Use `@apply` to compose utility classes:

```css
.btn {
  @apply inline-flex items-center gap-2 rounded-md px-3 py-2 text-sm font-semibold;
}

.btn-primary {
  @apply bg-indigo-600 text-white hover:bg-indigo-500;
}
```

### Dynamic Classes

Build class strings dynamically in Rust:

```rust
view! {
    <button class=move || {
        format!("btn {} {}",
            match variant {
                Variant::Primary => "btn-primary",
                Variant::Secondary => "btn-secondary"
            },
            if disabled.get() { "btn-disabled" } else { "" }
        )
    }>
        "Button"
    </button>
}
```

## Testing Components

### In the Demo Application

The demo app (`modules/mosaic/demo`) provides a full showcase environment for testing and documenting components.

#### Quick Testing

For quick testing during development:

1. Import your component in `demo/src/counter.rs` or the HomePage
2. Use the component in a view
3. Run the demo: `cargo leptos watch` (from the demo directory)

Example:

```rust
use mosaic_tiles::component_name::ComponentName;

#[component]
pub fn Demo() -> impl IntoView {
    view! {
        <ComponentName>
            "Test content"
        </ComponentName>
    }
}
```

## Build Process

### How CSS Bundling Works

The `build.rs` script:

1. Collects CSS files for enabled features
2. Bundles them into a single file
3. Runs Tailwind CSS to process utility classes
4. Minifies the output
5. Embeds the CSS via `include_str!` in `lib.rs`

## Checklist

When creating a new component:

- [ ] Create `src/components/[name]/mod.rs`
- [ ] Create `src/components/[name]/[name].css`
- [ ] Add feature flag to `Cargo.toml`
- [ ] Add to `src/components/mod.rs` with `#[cfg(feature = "...")]`
- [ ] Add to `build.rs` features macro
- [ ] Export from `src/lib.rs` if needed
- [ ] Verify CSS is bundled correctly
