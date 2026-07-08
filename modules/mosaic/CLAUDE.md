# Mosaic - Claude Code Guidelines

Mosaic is the DaSCH design system. It has two crates:

- **`tiles`** (`mosaic-tiles`) — a reusable component library of **Maud** functions (`fn -> Markup`). Dependencies are just `maud` + `icondata`; no Leptos, no WASM.
- **`playground`** (`mosaic-playground`) — a plain **Axum + Maud** binary that showcases the tiles. No islands, no WASM, no `cargo-leptos`.

## Architecture

- Components are plain functions returning `maud::Markup` — server-rendered HTML, MPA-first.
- Each component lives in `tiles/src/components/` either as a directory (`mod.rs` + a co-located `.css`) or a single `.rs` file (e.g. `copy_button.rs`, `loading.rs`). Exported from `tiles/src/lib.rs`.
- Variant enums derive `Clone, Copy, Default` and expose a `css_class()` that returns **complete literal class strings** (so Tailwind's content scan sees them). Multi-option tiles are builders: `name(content) -> NameBuilder` with chained setters (e.g. `.variant(…)`, `.size(…)`) and a `.build()`; each builder implements `Render` so it can be spliced into `html!` directly. Content and labels are `impl Render`, not bare `Markup` (see `docs/src/mosaic/component-api-conventions.md`).
- The single sanctioned `PreEscaped` site is the `IconData` SVG (`icondata`).
- Interactivity, where needed, is CSS-only (e.g. `Tabs` uses hidden radio inputs) or a small inline handler (e.g. `copy_button`'s clipboard `onclick`).

## Adding a New Component

The `/add-mosaic-component` skill walks through this. In short:

1. Add `fn name(...) -> maud::Markup` in `tiles/src/components/<name>/mod.rs` (plus a co-located `<name>.css` if it needs component styles), and export it in `tiles/src/lib.rs`.
2. If you added a CSS file, `@import` it into each consuming Tailwind entry: `modules/dpe/style/main.css` and `modules/mosaic/playground/style/main.css`.
3. Add a hand-written showcase page `playground/src/showcase/<name>.rs` (component title + description + rendered examples), export it in `playground/src/showcase/mod.rs`, and register the route + sidebar nav entry in `playground/src/app.rs`.
4. Add unit tests rendering the component and asserting on its output.

## Build System (CSS)

- There is no `build.rs` CSS pipeline. Each component's CSS is self-contained (`@apply` on the design tokens, no DaisyUI) and lives next to its source.
- The consuming app's Tailwind entry `@import`s `tokens.css` + the component CSS files, then runs a single standalone Tailwind invocation.
- Playground stylesheet: `just css-mosaic` → `playground/public/assets/app.css` (gitignored). Dev loop: `just watch-mosaic-playground` (Tailwind `--watch` + `cargo watch`).

## Testing and Verification

```bash
just watch-mosaic-playground    # Run the playground with hot reload
cargo test -p mosaic-tiles      # Tile unit tests
just check                      # Clippy and formatting for the whole repo
just test                       # All workspace tests
```

## Design Tokens

- Brand colors and typography are defined via `@theme static` in `tiles/src/components/theme_provider/tokens.css` — the single token source.
- `tokens.css` is `@import`ed by both consuming Tailwind entries (DPE's `style/main.css` and the playground's `style/main.css`), so both pipelines share the same tokens.
- Tokens use OKLCH with 11-stop scales (50–950) per semantic color. Use semantic token classes (`primary-*`, `neutral-*`, `danger-*`, …) instead of hardcoded Tailwind colors (`blue-*`, `gray-*`, `red-*`).
- `info` tokens reference `secondary` via `var()` — intentionally identical. The neutral scale is experimental and subject to design review.
- Consuming apps load fonts (Lora/Lato) themselves; the tiles library is font-loading-agnostic.

## Key Conventions

- Prefer CSS-only interactions where possible.
- Icons use the `icondata` crate for compile-time tree-shaking.
- Keep each component's CSS next to its Rust source.
- Components: badge, breadcrumb, button, card, copy_button, icon, link, loading, tabs. The theme provider is just `tokens.css`, not a Rust component.

## Formatting

Run `just fmt` for all code in this module at the end of your work:

```bash
just fmt
```

`just fmt` runs `maudfmt` (which formats the contents of `maud::html!` macros — stock rustfmt does not) followed by `cargo +nightly fmt` for the surrounding Rust. `just check` verifies both.

Because tiles take content as `impl Render`, don't pass a non-trivial `html!` block directly as an argument (`card(html! { … })`) — `maudfmt` skips nested-as-argument blocks and `cargo fmt` then mangles them. Bind the inner markup to a Rust `let` (`let body = html! { … }; card(body)`) or extract a `fn … -> Markup` helper. Trivial one-liners like `html! { (label) }` are fine inline.
