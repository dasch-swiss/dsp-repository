# CLAUDE.md - Design System Components

This file provides guidance for working with DSP Design System components.

## Component Usage

Components are imported from the `components` crate and return `maud::Markup` for composition:

```rust
use components::{button, banner, tile};

let page_content = html! {
    div {
        (banner::accent_only("Welcome"))
        (button::button("Get Started"))
        (tile::base(html! { p { "Content here" } }))
    }
};
```

See individual component documentation files for specific usage patterns and variants.

## Creating Components

### File Structure
- One component per file: `modules/design_system/components/src/component_name.rs`
- Export in `lib.rs`: `pub mod component_name;`
- Add playground route in `modules/design_system/playground/src/playground/components.rs`
- Create documentation: `docs/src/design_system/components/component_name.md`

### Builder Pattern with Shared Trait

Components that use the builder pattern should implement the `ComponentBuilder` trait to get common methods automatically:

```rust
use maud::{html, Markup};
use crate::builder_common::ComponentBuilder;

pub struct MyComponentBuilder {
    text: String,
    id: Option<String>,
    test_id: Option<String>,
    // ... other component-specific fields
}

// Implement the trait to get with_id(), with_test_id(), and build() automatically
impl ComponentBuilder for MyComponentBuilder {
    fn id_mut(&mut self) -> &mut Option<String> {
        &mut self.id
    }

    fn test_id_mut(&mut self) -> &mut Option<String> {
        &mut self.test_id
    }

    fn build(self) -> Markup {
        // Component-specific rendering logic
        html! {
            div
                id=[self.id]
                data-testid=[self.test_id.as_deref().unwrap_or("my-component")]
            {
                (self.text)
            }
        }
    }
}

// Component-specific methods go in a regular impl block
impl MyComponentBuilder {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            id: None,
            test_id: None,
        }
    }

    // Add component-specific builder methods here
    // with_id() and with_test_id() are provided by the trait
}

#[must_use = "call .build() to render the component"]
pub fn my_component(text: impl Into<String>) -> MyComponentBuilder {
    MyComponentBuilder::new(text)
}
```

**Key Points:**
- Implement `ComponentBuilder` trait to inherit `with_id()`, `with_test_id()`, and `build()` methods
- Add `id: Option<String>` and `test_id: Option<String>` fields to your builder struct
- Component-specific methods go in a separate `impl` block
- Don't forget `#[must_use]` attribute on the constructor function

### Simple Component Template (No Builder)

For components that don't need builder pattern:

```rust
use maud::{html, Markup};

pub fn component(content: impl Into<String>) -> Markup {
    html! {
        div class="my-component" {
            (content.into())
        }
    }
}
```

### Requirements
- Return `maud::Markup`
- Use `impl Into<String>` for text parameters, `Markup` for rich content
- Use semantic HTML with proper ARIA attributes
- Test in playground: `just run-watch-playground` (developer) or `just run-playground-background` (AI agent)

### Documentation Template
Create `docs/src/design_system/components/component_name.md`:

```markdown
# Component Name

Brief description of component purpose.

## Usage Guidelines

When and how to use this component.

## Variants

### Variant Name
Description of variant usage.

## Accessibility Notes

Accessibility considerations and features.

## Implementation Status

Current status: ✅ Complete / 🚧 In Progress / ⚠️ Needs Work

## Design Notes

Design system alignment and visual specifications.
```

## TailwindUI Migration (Temporary)

During the ongoing migration from Carbon to TailwindUI:

### Current Status
- Components being migrated to TailwindUI patterns
- DaSCH brand customization needed for all components
- Dark/light mode support being implemented

### Migration Checklist
- [ ] Update CSS classes to Tailwind utilities
- [ ] Verify responsive design across breakpoints
- [ ] Implement DaSCH brand colors and typography
- [ ] Test dark/light mode functionality
- [ ] Update component documentation

### Temporary Notes
- Some components may show placeholder styling during migration
- Refer to `.claude/tmp/tailwind-migration.md` for migration tasks
- Brand customization requirements noted in individual component docs with ⚠️ warnings
