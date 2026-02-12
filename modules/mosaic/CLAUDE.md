# Mosaic - Claude Code Guidelines

## Formatting

Use `leptosfmt` (not `rustfmt`) for all code in this module. The `rust-analyzer.toml` files in `tiles/` and `demo/` configure IDE formatting accordingly.

```bash
just fmt-mosaic
```

## Architecture

- Leptos 0.8 with the **islands** feature for selective client-side hydration
- MPA-first: server-side rendering by default, islands for interactivity
- CSS is bundled at build time per component via `tiles/build.rs`, processed by Tailwind, and injected by `ThemeProvider`
- Components are feature-gated in `tiles/Cargo.toml` — some have dependencies (e.g., `accordion` requires `icon`, `popover` requires `button`)

## Adding a New Component

1. Create the component in `tiles/src/components/` with a `mod.rs` and a CSS file
2. Register it as a feature in `tiles/Cargo.toml`
3. Export it in `tiles/src/lib.rs`
4. Add a demo in `demo/src/components/` with `component.toml`, examples, and optionally `anatomy.rs`
5. Register the route in `demo/src/app.rs`

The `/create-new-component` and `/add-component-to-demo` skills automate these steps.

## Build System

- `tiles/build.rs` collects CSS for enabled features, runs Tailwind v4.1.17 (downloaded or via `SINGLESTAGE_TAILWIND_PATH`), and minifies the output
- The demo uses `cargo-leptos` with config in `demo/Cargo.toml` under `[package.metadata.leptos]`
- `demo/` watches `tiles/src/components/` for changes via `watch-additional-files`

## Testing and Verification

```bash
just watch-mosaic-demo    # Run demo with hot reload
just fmt-mosaic           # Format check
just check                # Clippy and formatting for the whole repo
```

## Design Tokens

- Brand colors and typography are defined via `@theme static` in `tiles/src/components/theme_provider/tokens.css`
- `tokens.css` is imported by both `tiles/src/components/theme_provider/main.css` (tiles pipeline) and `demo/style/tailwind.css` (demo pipeline), so both pipelines share the same token definitions
- `build.rs` copies `tokens.css` to `OUT_DIR` so the `@import "./tokens.css"` in `main.css` resolves during the tiles build
- Tokens use OKLCH color format with 11-stop scales (50–950) for each semantic color
- Use semantic token classes (`primary-*`, `neutral-*`, `danger-*`, etc.) instead of hardcoded Tailwind colors (`blue-*`, `gray-*`, `red-*`, etc.)
- `info` tokens reference `secondary` via `var()` — they are intentionally identical
- The neutral scale is experimental and subject to design review
- Consuming apps load fonts (Lora/Lato) themselves; the tiles library is font-loading-agnostic

## Key Conventions

- Components use Leptos context for cross-component coordination (e.g., `Button` auto-binds to `Popover` via context)
- Prefer CSS-only interactions where possible (e.g., `Tabs` uses hidden radio inputs)
- Icons use the `icondata` crate for tree-shaking at compile time
- Each component's CSS lives next to its Rust source in `tiles/src/components/[name]/`
