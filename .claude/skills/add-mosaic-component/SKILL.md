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

- A tile is a function returning `maud::Markup`, or — with a variant or several optional axes — a
  **builder** implementing `Render` and `ComponentBuilder`. Variants are enums with a `css_class()`
  returning **complete literal class strings** (so Tailwind's content scan sees them). Content and
  label parameters are `impl Render`, never a bare `Markup` and never `&str`. No tile takes a
  `*Props` struct; `docs/src/mosaic/component-api-conventions.md` is the authority and this skill
  only covers the mechanics.
- Component CSS lives next to the tile and is `@import`ed by the
  `tiles/src/components/components.css` barrel, which every consuming Tailwind
  entry imports. There is no build-time bundling.
- The showcase is a hand-written Maud page per component; the playground is an
  MPA (one Axum route per page, active nav resolved server-side).

## Step 1 — Create the tile

Create `modules/mosaic/tiles/src/components/<name>/mod.rs`. Read
`docs/src/mosaic/component-api-conventions.md` first; the shape below is a
builder, which is what anything with a variant or more than one optional axis
wants. A tile with no variant and one or two required arguments is a plain
`fn -> Markup` instead (see `icon`, `copy_button`, `loading`).

Copy the nearest existing tile rather than this sketch — `badge` is the smallest
complete builder, `card` the smallest container, `table` a builder with partials.

```rust
//! <Name> tile.

use maud::{html, Markup, Render};

use crate::builder::ComponentBuilder;

#[derive(Clone, Copy, Debug, Default)]
pub enum <Name>Variant {
    #[default]
    Primary,
    Secondary,
}

impl <Name>Variant {
    /// Complete, literal class string, so Tailwind's source scan sees it.
    pub fn css_class(self) -> &'static str {
        match self {
            <Name>Variant::Primary => "<name>-primary",
            <Name>Variant::Secondary => "<name>-secondary",
        }
    }
}

#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct <Name>Builder {
    content: Markup,
    variant: <Name>Variant,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start a <name> wrapping the given content.
pub fn <name>(content: impl Render) -> <Name>Builder {
    <Name>Builder {
        content: content.render(),
        variant: <Name>Variant::default(),
        id: None,
        test_id: None,
    }
}

impl <Name>Builder {
    pub fn variant(mut self, variant: <Name>Variant) -> Self {
        self.variant = variant;
        self
    }

    // `Render::render` and `build` both route through this one private method,
    // so a spliced builder and a built one cannot diverge.
    fn markup(&self) -> Markup {
        html! {
            span
                class=(format!("<name> {}", self.variant.css_class()))
                id=[self.id.as_deref()]
                data-testid=[self.test_id.as_deref()]
            { (self.content) }
        }
    }
}

impl ComponentBuilder for <Name>Builder {
    fn id_mut(&mut self) -> &mut Option<String> { &mut self.id }
    fn test_id_mut(&mut self) -> &mut Option<String> { &mut self.test_id }
    fn build(self) -> Markup { self.markup() }
}

impl Render for <Name>Builder {
    fn render(&self) -> Markup { self.markup() }
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
        let out = <name>("x").build().into_string();
        assert!(out.contains(r#"class="<name> <name>-primary""#), "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = <name>("x").build().into_string();
        let spliced = html! { (<name>("x")) }.into_string();
        assert_eq!(built, spliced);
    }
}
```

Accessibility is the tile's job, not a caller-set knob: derive `role` and the
`aria-*` state from a semantic method or the variant, and bundle attributes that
only work together behind one intent method (`link().external()`,
`text_field().one_time_code(6)`).

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
so adding the module file is enough — but the module has to be declared, and
`tiles/src/components/mod.rs` lists them explicitly. **A form input goes under
`components/form/` and is declared in `components/form/mod.rs` instead**; that
directory is re-exported flat, so the import path is `mosaic_tiles::<name>::<name>`
either way and the CSS barrel imports it as `./form/<name>/<name>.css`. See
`docs/src/mosaic/component-api-conventions.md`.

## Step 2 — Wire the CSS into the component barrel

If you added a `.css` file, `@import` it into the barrel:

- `modules/mosaic/tiles/src/components/components.css`

That is the only edit. Every consuming Tailwind entry (`modules/dpe/style/main.css`,
`modules/mosaic/playground/style/main.css`, `modules/editor/style/main.css`) imports
the barrel, so the new classes resolve everywhere the tile is used. Do **not** add
per-file imports to the entries — a tile missing from one hand-maintained list
silently drops its classes with no build error.

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
            (<name>("Primary").variant(<Name>Variant::Primary))
            (<name>("Secondary").variant(<Name>Variant::Secondary))
        }
    }
}
```

A builder splices into `html!` directly — never call `.build()` inside a
template. A multi-element `html!` block must not be passed inline as a call
argument either: bind it to a `let` first, or `maudfmt` skips it and
`cargo fmt` mangles it.

Keep each example wrapped via the shared `example("<name>-<example>", …)` helper
— the `data-example-key` it emits is the stable anchor the e2e smoke test (and
any visual tooling) uses to address each render in isolation (do not remove it).

Declare the module in `modules/mosaic/playground/src/showcase/mod.rs`
(`pub mod <name>;`) and add it to the `pages_render_with_example_keys` test list
there.

Check the tile against the surface it lands on rather than against the class
being present: build the stylesheet and read the computed values off the rendered
page. Contrast is the usual trap — a token pair that passes on white can fail on
a tinted or dark surface, and an input's border has to clear 3:1 (WCAG 2.1
SC 1.4.11) because it is the only thing marking where the control is.

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
3. Add the route to the `ROUTES` array in
   `modules/mosaic/playground-e2e-tests/tests/showcase-smoke.spec.ts`. It is a
   hardcoded list, so a page left out of it is simply never smoke-tested — and
   nothing fails to tell you.

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

- [ ] `tiles/src/components/<name>/mod.rs`: a plain `fn -> Markup`, or a builder implementing `Render` + `ComponentBuilder`, with `impl Render` content params and tests
- [ ] `tiles/src/components/<name>/<name>.css` (if it needs styles)
- [ ] Module declared in `tiles/src/components/mod.rs` — or in `components/form/mod.rs` for a form input
- [ ] CSS `@import`ed into `tiles/src/components/components.css` (the barrel — not the app entries)
- [ ] `playground/src/showcase/<name>.rs` with `data-example-key`-wrapped examples
- [ ] Module declared + added to the test list in `playground/src/showcase/mod.rs`
- [ ] Route + `COMPONENT_NAV` entry in `playground/src/app.rs`
- [ ] Route added to `ROUTES` in `playground-e2e-tests/tests/showcase-smoke.spec.ts`
- [ ] Contrast and focus measured on the rendered page, not inferred from the class list
- [ ] `just check` and `just test` green
