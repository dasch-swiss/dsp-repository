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

### Component Template
```rust
use maud::{html, Markup};

#[derive(Debug, Clone)]
pub enum ComponentVariant {
    Default,
    // Add variants as needed
}

impl ComponentVariant {
    fn css_class(&self) -> &'static str {
        match self {
            ComponentVariant::Default => "dsp-component",
        }
    }
}

pub fn component(content: impl Into<String>) -> Markup {
    html! {
        div class=(ComponentVariant::Default.css_class()) {
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
