# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with the DSP Design System.

## Architecture
- **Return Type**: All components return `maud::Markup` for zero-copy composition
- **Parameters**: Use `impl Into<String>` for text, `Markup` for rich content
- **CSS Classes**: `dsp-` prefix with BEM methodology
- **Structure**: One component per file, export via `lib.rs`
- **Design Reference**: Verify against https://carbondesignsystem.com/components
- **Accessibility**: Use semantic HTML, proper ARIA attributes, keyboard navigation support

## Patterns

### Simple Components
```rust
use maud::{html, Markup};

pub fn button(text: impl Into<String>) -> Markup {
    html! { button .dsp-button { (text.into()) } }
}

pub fn button_with_variant(text: impl Into<String>, variant: ButtonVariant, disabled: bool) -> Markup {
    html! { button .dsp-button .(variant.css_class()) disabled[disabled] { (text.into()) } }
}
```

### Modular APIs
```rust
// Focused functions instead of complex props
pub fn accent_only(text: impl Into<String>) -> Markup { /* ... */ }
pub fn with_prefix(prefix: impl Into<String>, accent: impl Into<String>) -> Markup { /* ... */ }
```

### Composition
```rust
pub fn dashboard_tile() -> Markup {
    tile::base(html! {
        (banner::accent_only("Dashboard"))
        div { (button::button("Action")) }
    })
}
```

### Variants
```rust
#[derive(Debug, Clone)]
pub enum ButtonVariant { Primary, Secondary, Outline }

impl ButtonVariant {
    fn css_class(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "dsp-button--primary",
            ButtonVariant::Secondary => "dsp-button--secondary",
            ButtonVariant::Outline => "dsp-button--outline",
        }
    }
}
```

## Component Template
```rust
use maud::{html, Markup};

#[derive(Debug, Clone)]
pub enum ComponentVariant { Default }

impl ComponentVariant {
    fn css_class(&self) -> &'static str {
        match self { ComponentVariant::Default => "dsp-component" }
    }
}

pub fn component(content: impl Into<String>) -> Markup {
    html! { div class=(ComponentVariant::Default.css_class()) { (content.into()) } }
}
```

## Development Checklist
- [ ] Return `maud::Markup`
- [ ] Use `impl Into<String>` for text, `Markup` for content
- [ ] Use semantic HTML and proper ARIA attributes
- [ ] Export in `lib.rs`, add playground route
- [ ] **Verify design with dev against Carbon Design System**
- [ ] Test composition with other components

## Naming
- **Modules**: Singular (`button.rs`, `tile.rs`)
- **Functions**: Descriptive (`accent_only()`, `clickable()`)
- **Enums**: `ComponentVariant` pattern
- **CSS**: `dsp-component`, `dsp-component--variant`

## Current Components
- **Button**: ✅ Markup-based with Primary/Secondary/Outline variants (TODO: verify Carbon styling)
- **Banner**: ✅ Modular API with accent_only, with_prefix, with_suffix, full functions
- **Shell**: 🚧 Work in progress - application shell with navigation
- **Tile**: ✅ Base and Clickable variants (TODO: verify Carbon styling)

## Visual Design Guidance
When developing components, use these methods to understand design requirements:
- **Reference Images**: Screenshots/mockups can be provided and read for visual comparison
- **Reference URLs**: WebFetch existing design systems (IBM Carbon, etc.) for component patterns
- **Visual Specifications**: Detailed written descriptions of expected appearance, states, and variants
- **Design Tokens**: Follow documented color palette, spacing, and typography rules
- **Visual Testing**: Use WebFetch on playground at `http://localhost:3400/[component]` to compare against targets
