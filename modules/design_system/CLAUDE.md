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

## MCP Testing Workflow
Interactive component testing using Playwright MCP during development:

### Setup
1. Start playground server:
   - **Interactive**: `just run-watch-playground` (shows logs, http://localhost:3400)
   - **Background**: `just run-playground-background` (silent, for MCP testing)
2. Use MCP commands to test components interactively
3. Stop background server: `just stop-playground`

### Core MCP Commands
- `mcp__playwright__browser_navigate(url)` - Navigate to component pages
- `mcp__playwright__browser_take_screenshot()` - Visual verification
- `mcp__playwright__browser_click(element, ref)` - Test interactions
- `mcp__playwright__browser_snapshot()` - Accessibility structure

### Testing Pattern
```markdown
1. Navigate: http://localhost:3400/[component]
2. Screenshot: Verify visual rendering
3. Interact: Test clicks, hovers, navigation
4. Document: Note issues or improvements
```

### Development Integration
- **During Development**: Test components as you build them
- **Design Review**: Capture screenshots for comparison
- **Regression Testing**: Verify changes don't break existing components
- **Accessibility**: Use snapshots to verify semantic structure

## Automated Testing (TypeScript + Playwright)
Professional test suite with TypeScript, visual regression, and comprehensive tooling:

### Interactive Commands (for manual use)
```bash
just playground install         # Install dependencies and browsers
just playground test            # Run all tests (headless)
just playground test-functional # Run functional tests only (no visual regression)
just playground test-visual     # Run visual regression tests only
just playground test-headed     # Run with browser visible
just playground test-ui         # Interactive test runner
just playground test-debug      # Debug mode
```

### Claude Code Commands (non-blocking, optimized for automation)
```bash
just playground::claude::test-quick      # Quick test with first failure feedback
just playground::claude::test           # Run all tests (non-blocking)
just playground::claude::test-visual    # Visual regression tests only
just playground::claude::type-check     # TypeScript validation
just playground::claude::lint           # Code linting
just playground::claude::verify         # Silent test verification (exit code only)
just playground::claude::status         # Git status
```

### Development Commands
```bash
just playground type-check      # TypeScript validation
just playground lint            # Code linting
just playground format          # Code formatting
just playground lint-and-format # Both linting and formatting
```

### Visual Testing
```bash
# Local visual baseline updates (cross-platform consistent)
just playground docker-update-visuals  # Update visual baselines using Docker (Linux environment)
just playground update-visuals         # Update visual baselines (platform-specific, use docker version for CI consistency)

# Cleanup and reports
just playground clean-visuals           # Clean visual snapshots
just playground report                  # View test reports

# Docker commands for CI consistency
just playground docker-build           # Build Docker image for testing
just playground docker-test            # Run tests using Docker (Linux environment)
```

### Test Structure
- **Location**: `playground/tests/e2e/` (TypeScript files)
- **Config**: `playground/playwright.config.ts` (TypeScript, visual regression, multi-reporter)
- **Tooling**: ESLint v9 (flat config), Prettier, TypeScript with strict checking
- **Linting**: `eslint.config.js` with TypeScript, Playwright, and Prettier integration
- **Module**: Access via `just playground <command>` from project root
- **CI**: `.github/workflows/playwright.yml` (automatic on design system changes)
- **Screenshots**: Visual regression testing with 2% threshold tolerance (local development only)
- **CI Testing**: Functional, accessibility, and responsive tests only (visual tests skipped in CI)
- **Cross-platform**: Docker-based baseline generation for consistent local testing
- **Reports**: HTML + JSON output for CI/CD integration
