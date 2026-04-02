# mosaic-playground-macro

Procedural macros for generating Mosaic component documentation pages.

## Purpose

This crate scans the `playground/src/components` directory for component directories containing `component.toml` metadata files and generates Leptos page components at compile time.

## Macro

### `generate_component_pages!()`

Generates a Leptos page component for each component directory found. Each generated page includes:

- Component name and description
- Optional info block for special notices
- Examples with live demos and source code
- Anatomy section (if `anatomy.rs` exists)
- API reference tables

## Component Directory Structure

Each component in `playground/src/components/` should follow this structure:

```
playground/src/components/[component_name]/
├── component.toml           # Required: component metadata
├── mod.rs                   # Example module exports
├── anatomy.rs               # Optional: anatomy code snippet
└── examples/
    └── [example_name].rs    # Example implementations
```

## component.toml Format

```toml
name = "Button"
description = "A clickable button component."

# Optional info block
[info]
title = "Note"
description = "Special information about this component."

# Examples (rendered as live demos with source)
[[examples]]
name = "variants"
title = "Button Variants"
description = "Available button styles"

# API reference
[[references]]
name = "Button"
description = "Main button component."
extra = "Additional notes."

[[references.attrs]]
attr = "variant"
attr_type = "ButtonVariant"
default = "Primary"
description = "Visual style variant"
```

## Usage

In the playground crate's `lib.rs`:

```rust
mosaic_playground_macro::generate_component_pages!();
```

This generates components like `ButtonRoute`, `CardRoute`, etc., which can be used in Leptos router definitions.
