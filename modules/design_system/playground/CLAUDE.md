# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with the DSP Design System Playground.

## Purpose

Live development environment for testing and developing design system components in isolation.

## Server Details

- **Port**: 3400 (http://localhost:3400)
- **Live Reload**: Automatic browser refresh via WebSocket when files change
- **Static Assets**: Served from `/assets` directory

## Development Workflow

- **Server Management**:
  - **Always assume the playground is already running** at http://localhost:3400
  - Only start the server if it's not responding or you get connection errors
  - If you must start the server, in the project root, use `just run-playground-background` (non-blocking background task)
  - **Never stop a server you didn't start yourself** - use `just stop-playground` only if you started it
- **Visual Testing**: Use WebFetch tool to view rendered components at `http://localhost:3400/[component]` while developing
- **Page Structure**: Each component gets its own dedicated route/page for testing
- **Component Routes**:
  - `/` - Home page with component list
  - `/button` - Button component examples
  - `/shell` - Shell component examples

## Visual Testing Notes

Visual regression tests are platform-specific and generate different baselines on different operating systems:

- **Platform Dependency**: Visual baselines should be generated on the same OS where tests will run
- **Local Testing**: Use `just update-visuals` to generate baselines for your platform
- **CI Considerations**: Visual tests may need to be disabled or run conditionally in CI environments

Visual tests have been separated from functional tests to provide flexibility in testing workflows.

## File Structure

- `main.rs` - Server setup and routing
- `pages.rs` - Page handlers for each component
- `skeleton.rs` - Page template wrapper
- `livereload.rs` - WebSocket live reload functionality
- `src/playground/`
  - `renderer.rs` - Core showcase traits and registry
  - `showcases/` - Component showcase implementations
    - `button_showcase.rs`
    - `icon_showcase.rs`
    - `menu_showcase.rs`
    - ... (one file per component)

## Showcase Architecture

Component showcases are organized in a modular structure where each component has its own dedicated showcase file. This makes the codebase maintainable and prepares for future features like code-view toggling.

**See `SHOWCASE_ARCHITECTURE.md` for detailed documentation.**

### Creating a New Showcase

1. Create `src/playground/showcases/my_component_showcase.rs`
2. Implement the `ComponentRenderer` trait
3. Add to `showcases/mod.rs`
4. Register in `renderer.rs` registry
5. Add route in `main.rs`

### Future: Code-View Feature

The architecture includes the `example!` macro for capturing both rendered markup and source code:

```rust
example!{
    id: "primary-button",
    name: "Primary Button",
    description: "Main call-to-action",
    code: {
        button::button("Click Me")
            .variant(ButtonVariant::Primary)
            .build()
    }
}
```

This enables a toggle between rendered view and code view for each example.

## Component Development Pattern

1. Create/modify component in `../components/src/`
2. Create showcase in `src/playground/showcases/my_component_showcase.rs`
3. Add page route in `main.rs`
4. Create page handler in `pages.rs`
5. Use WebFetch to view rendered component during development
6. Test component variations and edge cases
