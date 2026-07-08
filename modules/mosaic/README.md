# Mosaic

A [Maud](https://maud.lambda.xyz/)-based UI component library for the DaSCH Service Platform.

## Crates

```txt
mosaic/
├── tiles/             # Reusable component library
└── playground/        # Showcase and documentation web application
```

- **tiles** contains the components themselves. Simple tiles are `fn -> maud::Markup`; multi-option tiles are builders (`name(content) -> NameBuilder`, chained setters, `.build()`) that implement `Render`. Variant enums expose a `css_class()`. Component CSS lives next to each component's Rust source and is collected through the consuming app's Tailwind entry.
- **playground** is a plain Axum + Maud web application that renders live examples and API references for each component. Routes are declared with the native Axum router in `playground/src/app.rs`, and the document shell (head, nav, sidebar) is hand-written Maud.

## Design Tokens

Brand colors, typography, and a neutral scale are defined as CSS custom properties via Tailwind v4's `@theme` directive in `tiles/src/components/theme_provider/tokens.css`. These tokens are available as both CSS variables (e.g., `var(--color-primary-500)`) and Tailwind utilities (e.g., `bg-primary-500`).

**Semantic colors:** primary, secondary, success, danger, warning, info, accent (each with 50–950 stops in OKLCH)
**Neutral scale:** Derived from DaSCH Slate (#3B4856), experimental
**Typography:** `font-display` (Lora) and `font-body` (Lato) with fallback chains

The playground includes a token showcase page under the "Foundation" sidebar section.

## Components

Badge, Breadcrumb, Button, Card, Copy Button, Icon, Link, Loading, Tabs

## Development

```bash
just watch-mosaic-playground    # Run playground with hot reload (watches tiles changes)
just fmt                        # Format all code: maudfmt for html! macros, then cargo +nightly fmt
```

See each crate's README for further details.
