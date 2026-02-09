# AI Agent Guide for Leptos Learning Project

This document provides guidance for AI coding assistants (like Claude Code, GitHub Copilot, etc.) working with this Leptos project.

## Project Overview

This is a **Leptos 0.8.2** full-stack web application using the **Axum** backend framework. It's a workspace project with server-side rendering (SSR) and client-side hydration capabilities.

### Architecture

- **Framework**: Leptos (Rust full-stack web framework)
- **Backend**: Axum web server
- **Frontend**: WebAssembly (WASM) with Leptos reactive components
- **Styling**: Tailwind CSS 4.x + DaisyUI
- **Testing**: Playwright for end-to-end tests

### Workspace Structure

```
leptos-dpe/
├── app/              # Shared application code (components, pages, logic)
├── frontend/         # WASM frontend entry point
├── server/           # Axum server binary
├── public/           # Static assets
├── style/            # CSS/Tailwind configuration
└── end2end/          # Playwright E2E tests
```

## Key Technologies & Versions

- **Leptos**: 0.8.2
- **Axum**: 0.8.4
- **Tailwind CSS**: 4.1.18
- **DaisyUI**: 5.5.14
- **Rust Edition**: 2021
- **Target**: wasm32-unknown-unknown (for frontend)

## Development Commands

### Running the Development Server

```bash
cargo leptos watch
```

This starts the development server with hot-reload on `http://127.0.0.1:3000`

### Building for Production

```bash
cargo leptos build --release
```

### Running Tests

```bash
cargo leptos end-to-end        # Development mode
cargo leptos end-to-end --release  # Release mode
```

### Installing Prerequisites

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos --locked
```

## Code Organization Patterns

### Component Structure

Components are located in [app/src/components/](app/src/components/):

- [navbar.rs](app/src/components/navbar.rs) - Navigation bar
- [menu.rs](app/src/components/menu.rs) - Side menu
- [card.rs](app/src/components/card.rs) - Reusable card component
- [footer.rs](app/src/components/footer.rs) - Page footer
- [mod.rs](app/src/components/mod.rs) - Module exports

### Page Structure

Pages are located in [app/src/pages/](app/src/pages/):

- [home.rs](app/src/pages/home.rs) - Home page
- [about.rs](app/src/pages/about.rs) - About page
- [project.rs](app/src/pages/project.rs) - Project detail page
- [mod.rs](app/src/pages/mod.rs) - Module exports

### Main Application

- [app/src/lib.rs](app/src/lib.rs) - Main app component with routing setup
- Uses Leptos Router with static and dynamic routes
- SSR shell function for HTML generation

## Important Conventions

### Leptos Component Pattern

```rust
use leptos::prelude::*;

#[component]
pub fn ComponentName() -> impl IntoView {
    view! {
        <div class="tailwind-classes">
            // Component content
        </div>
    }
}
```

### Routing

- Uses `leptos_router` with new 0.8.x API
- Static routes: `Route path=StaticSegment("about")`
- Dynamic routes: `Route path=path!("projects/:id")`

### Styling

- Tailwind CSS 4.x syntax (note: v4 has breaking changes from v3)
- DaisyUI component classes available
- CSS in [style/main.css](style/main.css)

### Features

The project uses Cargo features for different compilation targets:

- `hydrate` - Client-side hydration (WASM)
- `ssr` - Server-side rendering (Axum)

## When Making Changes

### Adding a New Component

1. Create file in [app/src/components/](app/src/components/)
2. Define component with `#[component]` macro
3. Export in [app/src/components/mod.rs](app/src/components/mod.rs)
4. Import in [app/src/lib.rs](app/src/lib.rs) or relevant page

### Adding a New Page

1. Create file in [app/src/pages/](app/src/pages/)
2. Define page component
3. Export in [app/src/pages/mod.rs](app/src/pages/mod.rs)
4. Add route in [app/src/lib.rs](app/src/lib.rs) `<Routes>` section

### Modifying Styles

- Global styles: Edit [style/main.css](style/main.css)
- Component styles: Use Tailwind/DaisyUI classes in components
- Tailwind config is in Cargo.toml: `tailwind-input-file = "style/main.css"`

### Server-Side Code

- Backend logic goes in [server/src/main.rs](server/src/main.rs)
- Server actions can be defined in `app/` with `#[server]` macro

## Common Pitfalls & Solutions

### Islands Mode

If using islands, the frontend must import the app:

```rust
// In frontend/src/lib.rs
#[allow(clippy::single_component_path_imports)]
#[allow(unused_imports)]
use app;
```

### Leptos 0.8.x Breaking Changes

- New reactive primitives API: `use leptos::prelude::*;`
- Router API changes: Use `StaticSegment` and `path!` macro
- Signal APIs may differ from 0.7.x examples online

### WASM Build Issues

- Ensure `wasm32-unknown-unknown` target is installed
- Check that `wasm-bindgen` version matches (currently =0.2.105)
- Nightly Rust is required by default

### Styling Not Applying

- Stylesheet link is in [app/src/lib.rs](app/src/lib.rs): `<Stylesheet id="leptos" href="/pkg/leptos-dpe.css" />`
- Ensure Tailwind build is working (automatic with `cargo leptos watch`)
- Check DaisyUI theme configuration in [style/main.css](style/main.css)

## Best Practices for AI Agents

1. **Always read existing code first** before making changes to understand patterns
2. **Maintain consistency** with existing component and file structure
3. **Use Leptos 0.8.x syntax** - many online examples are for 0.7.x
4. **Test with `cargo leptos watch`** to verify changes compile for both server and WASM
5. **Follow Rust conventions**: snake_case for functions/variables, PascalCase for types/components
6. **Keep SSR/CSR compatibility in mind** - code runs in both environments
7. **Use DaisyUI components** where appropriate for consistent UI
8. **Check Tailwind v4 syntax** - some classes changed from v3

## Debugging Tips

### Hot Reload Not Working

- Check reload port (3001) isn't blocked
- Restart `cargo leptos watch`

### WASM Compilation Errors

- Ensure no `std` features incompatible with WASM
- Check for browser-only APIs not available in SSR
- Use `cfg_if!` macro for platform-specific code:

```rust
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        // Server-only code
    } else {
        // Client-only code
    }
}
```

### Hydration Mismatches

- Ensure server and client render the same initial HTML
- Use `suppressHydrationWarning` for dynamic content
- Check for browser-only code running during SSR

## Project-Specific Notes

- **Public assets** go in `public/` directory
- **Static site output** goes to `target/site/` (auto-generated)
- **Release builds** are optimized for size (`opt-level = 'z'`, LTO enabled)
- **Environment variables** for production are in README.md
- The project uses a **workspace** setup - be aware of feature flags when adding dependencies

## Resources

- [Leptos Documentation](https://leptos.dev)
- [Leptos 0.8 Migration Guide](https://leptos-rs.github.io/leptos/)
- [Cargo Leptos](https://github.com/leptos-rs/cargo-leptos)
- [DaisyUI Components](https://daisyui.com)
- [Tailwind CSS v4 Docs](https://tailwindcss.com)

## Questions to Ask When Unsure

- "Should this be a server action or client-side logic?"
- "Does this code need to be SSR-compatible?"
- "Should this use an existing DaisyUI component?"
- "Is this following the project's component structure pattern?"
- "Will this work in both dev (`cargo leptos watch`) and production builds?"

---

**Last Updated**: 2025-12-22
**Leptos Version**: 0.8.2
**For AI Agents**: This project is actively in development. Always verify current patterns by reading existing code before making changes.
