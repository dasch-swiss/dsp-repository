# Component API Conventions

How Mosaic tile functions are shaped. Mosaic is a Maud component library: every
tile ultimately produces `maud::Markup`. These conventions exist so tiles read
naturally in Maud, compose without ceremony, and stay consistent as the library
grows and gains a second consumer.

## Background: not a Leptos port

Mosaic was ported from Leptos, whose components take a `Props` struct and
`children`. Translating that shape literally into Rust — a `*Props` struct on
every tile plus a `Markup` content argument — reads against the grain of Maud.
Maud has no `view!`-style macro that lets a user function participate in the
brace-nesting grammar, so the Leptos mental model does not carry over. These
conventions describe the Maud-native shape instead.

## Leaf tiles vs container tiles

Distinguish two kinds of tile, because they differ in how many options they carry:

- **Leaf tiles** render self-contained markup from a little data: `badge`, `icon`,
  `copy_button`, `breadcrumb_item`, `tab`.
- **Container tiles** wrap arbitrary caller-supplied content: `card` and its
  body/header/footer partials, `tabs`.

Both take content the same way (see below). The *form* (plain function vs
builder) is driven by how many options the tile carries — not by this
leaf/container distinction.

## Content is `impl Render`

Every content or label parameter is `impl Render`, never a bare `Markup` and
never `&str`/`impl Into<String>`.

Maud's `Render` trait is implemented by string-like types (rendered as escaped
text), by `Markup`/`PreEscaped`, and — by our own tiles (see [Builders](#everything-else-builders)).
A single `impl Render` parameter therefore accepts all of them:

```rust
pub fn badge(label: impl Render) -> BadgeBuilder { /* … */ }

badge("archaeology")                              // a string
badge(html! { (icon(Info, "w-4 h-4")) "info" })   // markup
```

Prefer `impl Render` over `&str`/`impl Into<String>` (too narrow — cannot hold an
icon) and over a bare `Markup` parameter (too heavy — forces `html! { … }` at
every call site, and cannot accept a builder directly).

This applies to **container slots too**: a card body is `impl Render`, not
`Markup`. That lets a builder, a bare `html!` block, or a string all flow into a
container with no conversion:

```rust
card(button("Save").variant(ButtonVariant::Primary)).variant(CardVariant::Bordered)  // a builder as content
card(html! { h2 { (title) } p { (summary) } }).variant(CardVariant::Bordered)   // markup as content
```

The trade-off is that `impl Render` slots are permissive — they accept anything
renderable. That is acceptable here: the alternative (`Markup`) buys no real
safety and costs ergonomics.

## Options: plain functions vs builders

Choose the form by how many options the tile carries. Do not impose one pattern
globally — a `Props` struct on every tile would reintroduce the Leptos shape
for no benefit.

In practice this is a **two-way** split. A named-constructor middle tier
(`badge::secondary(...)`) does not survive contact with the real call sites:
every tile that has a variant also carries at least one more axis (an extra
class, a size, an id/test-id), so it wants a builder rather than a fixed-shape
function.

### Trivial tiles: plain functions

No variant and one or two required arguments → a plain function returning
`Markup`. These are the genuinely simple primitives:

```rust
icon(IconType::Search, "w-4 h-4")
copy_button(text)
loading()
```

### Everything else: builders

A variant, or several independent optional axes (a card: variant + extra class +
id/test-id; a link: href + as_button + target + rel + aria_label + disabled) → a
**builder**. A builder reads better than a `Props` struct — no `Some(...)`
wrapping, no `..Default::default()`, you pay only for what you set:

```rust
card(body).variant(CardVariant::Bordered).class("overflow-visible")
link("Docs", "/x").target("_blank").rel("noopener").build()
```

Builders follow four rules:

1. **They implement Maud's `Render` trait**, so they splice into `html!` directly
   — no `.build()` inside a template:

   ```rust
   html! {
       (button("Save").variant(Secondary))
   }
   ```

2. **The struct is `#[must_use]`**, so a builder that is constructed but never
   spliced or built warns at compile time.

3. **`.build()` materialises a `Markup`** for the rare standalone path (a function
   whose body is just a builder, or a test). Inside `html!` you never call it.
   Route `Render::render(&self)` and `build(self)` through one private
   `markup(&self)`. Do **not** add `impl From<Builder> for Markup` / `Into` —
   `.build()` is self-documenting and needs no type annotation, whereas `.into()`
   is opaque and inference-hungry, and `Render` already covers the splice case.

4. **Universal options come from a shared `ComponentBuilder` trait** — `with_id`,
   `with_test_id`, and `build` as defaults — so every builder gets them without
   per-component code.

```rust
pub trait ComponentBuilder: Sized {
    fn id_mut(&mut self) -> &mut Option<String>;
    fn test_id_mut(&mut self) -> &mut Option<String>;
    fn build(self) -> Markup;

    #[must_use] fn with_id(mut self, id: impl Into<String>) -> Self { /* … */ self }
    #[must_use] fn with_test_id(mut self, id: impl Into<String>) -> Self { /* … */ self }
}
```

## Inlining content at call sites

Because a container takes its content as an **argument** (Maud has no child-block
syntax for functions), a multi-element `html!` block passed *inline* into the call
is skipped by `maudfmt` and then mangled by `cargo fmt`. Bind it to a Rust `let`
first, or extract a `fn -> Markup` helper:

```rust
// Good — the body is named, and maudfmt formats it.
let body = html! {
    h2 { (project.name) }
    p  { (project.short_description) }
};
html! { (card(body).variant(CardVariant::Bordered)) }
```

`maudfmt` only formats `html!` at Rust statement/`let` position; a block nested as
a call argument (or via Maud's in-macro `@let x = html! { … }`) is not. Note this
is only about *inline `html!` blocks* — a builder or a pre-bound variable passed
into an `impl Render` slot needs no such care. See also the formatting note in
`CONVENTIONS.md`.

## CSS classes

- Variant enums expose `css_class(self) -> &'static str` returning **complete
  literal class strings**, so Tailwind's content scan sees them. Never assemble
  class names from fragments.
- No BEM — it adds nothing under Tailwind. Use utility classes directly in the
  markup, and **wrap a reused Tailwind class combination behind a single semantic
  class** (via `@apply` in the component's co-located CSS) where the combination
  repeats. A tile function and any hand-written markup then share that one class
  instead of duplicating a long utility string.

## Accessibility

Accessibility is the tile's responsibility, not the caller's. Model the semantic
state and emit the correct `aria-*` / `role` internally, rather than exposing raw
aria attributes as knobs:

- **State and structure** (`aria-selected`, `aria-expanded`, `aria-current`,
  `aria-disabled`, `role`, `aria-controls`) follow from a semantic method. The
  link's `.disabled()` already emits `aria-disabled="true"` (and, in button mode,
  `tabindex="-1"`); a tab's `.checked()` would ideally also emit `aria-selected="true"`
  plus `role="tab"` (not yet implemented — the tabs tile is currently a CSS-only
  radio group). If one of these keeps appearing as a caller-set value, that is the
  signal a semantic method is missing — add the method, not a raw knob.
- **Author-supplied labels** (`aria-label` for icon-only or image-only controls)
  are genuinely the caller's to provide, so they are a builder method
  (`.aria_label(...)`) on the interactive tiles that need one — not forced onto
  every tile via `ComponentBuilder`, since e.g. a card has no use for one.
- **Bundle safe defaults into intent methods.** `link(...).external()` sets both
  `target="_blank"` and `rel="noopener noreferrer"`, so the `noopener` guard can't
  be forgotten. Prefer such a method over the raw `target`/`rel` knobs.
- **Avoid a generic `.attr(name, value)` escape hatch.** Raw attribute passthrough
  invites incorrect a11y and quietly defeats the guarantees the design system
  exists to provide.

## Test IDs

Builder-backed tiles that implement `ComponentBuilder` (`button`, `card`, `link`,
`badge`) carry `.with_test_id(...)` for free, giving the Playwright suites stable,
per-instance selectors. `tab` is a builder but — being a compound of three sibling
elements with no single id target — does not implement `ComponentBuilder`.

Leaf free-function tiles do **not** emit a test id. A constant default would be
non-unique across repeated instances (every badge in a list sharing one id) and
adds output noise for no selector value. When a specific leaf needs a stable
selector, add `data-testid` in the surrounding markup at the call site; if a tile
genuinely needs several configurable attributes, that is the signal to promote it
to a builder.

## Status

Convergence is complete. The builders — `button`, `card`, `link`, `badge`, `tab`
— follow these conventions; `button`, `card`, `link`, and `badge` implement the
`ComponentBuilder` trait in `tiles/src/builder.rs`, while `tab` is a builder
without it (a 3-element compound with no single id target).
`breadcrumb`/`breadcrumb_current` and the container `tabs`/`breadcrumb` take
`impl Render`. The trivial tiles (`icon`, `copy_button`, `loading`) intentionally
remain plain functions — they have no variant or optional axes. No tile carries a
`*Props` struct anymore. New tiles should follow this shape from the start.
