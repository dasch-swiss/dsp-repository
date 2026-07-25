---
name: add-mosaic-component
description: Add a new reusable component to the Mosaic design system — a Maud `fn -> Markup` tile in modules/mosaic/tiles plus its showcase page in modules/mosaic/playground.
---

# Add a Mosaic Component

This skill creates a new component end-to-end: the reusable **tile** (a Maud
`fn -> Markup`) in `modules/mosaic/tiles`, and its **showcase page** in
`modules/mosaic/playground`. Both crates are plain Maud + Axum — no Leptos, no
WASM, no feature flags, no `build.rs` CSS pipeline.

The component name is `${ARGUMENTS}` (snake_case, e.g. `status_chip`).

## Overview

- A tile is a function returning `maud::Markup`. Variants are enums with a
  `css_class()` returning **complete literal class strings** (so Tailwind's
  content scan sees them); multi-option tiles take a `#[derive(Default)] *Props`
  struct. Content is passed as `Markup`.
- Component CSS lives next to the tile and is `@import`ed by each consuming
  Tailwind entry. There is no build-time bundling.
- The showcase is a hand-written Maud page per component; the playground is an
  MPA (one Axum route per page, active nav resolved server-side).

## Step 1 — Create the tile

Create `modules/mosaic/tiles/src/components/<name>/mod.rs`:

```rust
//! <Name> tile.

use maud::{html, Markup};

#[derive(Clone, Copy, Debug, Default)]
pub enum <Name>Variant {
    #[default]
    Primary,
    Secondary,
}

impl <Name>Variant {
    pub fn css_class(self) -> &'static str {
        match self {
            <Name>Variant::Primary => "<name>-primary",
            <Name>Variant::Secondary => "<name>-secondary",
        }
    }
}

/// Render the <name> wrapping the given content.
pub fn <name>(variant: <Name>Variant, content: Markup) -> Markup {
    html! {
        span class=(format!("<name> {}", variant.css_class())) {
            (content)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_class_mapping() {
        assert_eq!(<Name>Variant::Primary.css_class(), "<name>-primary");
    }

    #[test]
    fn renders_content() {
        let out = <name>(<Name>Variant::Primary, html! { "x" }).into_string();
        assert!(out.contains("class=\"<name> <name>-primary\""), "{out}");
    }
}
```

For a tile with several independent options, prefer a `*Props` struct (see
`badge`/`button`/`card` for the pattern) instead of many positional args.

If the tile needs component styles, create
`modules/mosaic/tiles/src/components/<name>/<name>.css`. Keep it self-contained
(`@apply` on the design tokens, no DaisyUI), no `dark:` variants:

```css
@layer components {
  .<name> {
    @apply inline-flex items-center gap-2 rounded-md px-3 py-2 text-sm font-medium;
  }
  .<name>-primary { @apply bg-primary-600 text-white; }
}
```

Tiles are exported wholesale from `tiles/src/lib.rs` (`pub use components::*;`),
so adding the module file is enough — but make sure the module is declared in
`tiles/src/components/mod.rs` if that file lists modules explicitly.

## Step 2 — Wire the CSS into both Tailwind entries

If you added a `.css` file, `@import` it into **both** consuming entries so the
classes resolve everywhere the tile is used:

- `modules/dpe/style/main.css`
- `modules/mosaic/playground/style/main.css`

## Step 3 — Create the showcase page

Create `modules/mosaic/playground/src/showcase/<name>.rs`:

```rust
//! <Name> showcase.

use maud::{html, Markup};
use mosaic_tiles::<name>::{<name>, <Name>Variant};

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header("<Name>", "Short description of the component.");
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        (example("<name>-variants", "Variants", "The available variants.", variants()))
    }
}

fn variants() -> Markup {
    html! {
        div class="flex flex-wrap gap-3 items-center" {
            (<name>(<Name>Variant::Primary, html! { "Primary" }))
            (<name>(<Name>Variant::Secondary, html! { "Secondary" }))
        }
    }
}
```

Keep each example wrapped via the shared `example("<name>-<example>", …)` helper
— the `data-example-key` it emits is the stable anchor the e2e smoke test (and
any visual tooling) uses to address each render in isolation (do not remove it).

Declare the module in `modules/mosaic/playground/src/showcase/mod.rs`
(`pub mod <name>;`) and add it to the `pages_render_with_example_keys` test list
there.

## Step 4 — Register the route and nav entry

In `modules/mosaic/playground/src/app.rs`:

1. Add a route in `router()`:
   ```rust
   .route("/<name>", get(|| async { render("/<name>", "<Name>", showcase::<name>::page()) }))
   ```
2. Add an entry to the `COMPONENT_NAV` list so it appears in the sidebar:
   ```rust
   ("/<name>", "<Name>"),
   ```

## Step 5 — Verify

```bash
cargo test -p mosaic-tiles          # tile unit tests
cargo test -p mosaic-playground     # showcase render tests
just css-mosaic                     # rebuild the playground stylesheet
just watch-mosaic-playground        # eyeball the new page at /<name>
just check                          # fmt (cargo +nightly fmt) + clippy
```

After CSS changes, grep the built `playground/public/assets/app.css` for your
new classes — a class that resolves to nothing is the common footgun.

## Checklist

- [ ] `tiles/src/components/<name>/mod.rs` with variant enum(s) + `fn -> Markup` + tests
- [ ] `tiles/src/components/<name>/<name>.css` (if it needs styles)
- [ ] Module declared in `tiles/src/components/mod.rs` (if listed explicitly)
- [ ] CSS `@import`ed into both `dpe/style/main.css` and `playground/style/main.css`
- [ ] `playground/src/showcase/<name>.rs` with `data-example-key`-wrapped examples
- [ ] Module declared + added to the test list in `playground/src/showcase/mod.rs`
- [ ] Route + `COMPONENT_NAV` entry in `playground/src/app.rs`
- [ ] `just check` and `just test` green
