---
title: "Proc macro and dual Tailwind pipeline desync with design tokens"
date: 2026-02-12
category: build-errors
component: leptos_component
module: mosaic/demo, mosaic/tiles
problem_type: integration
severity: high
symptoms: "Tailwind CSS class changes in demo example files do not appear in browser after recompile and hard refresh. Watch command recompiles but proc macro output and served CSS remain stale."
root_cause: "Proc macro reads source files at compile time; cargo-leptos watch does not invalidate proc macro output when those files change. Additionally, design tokens defined in only one of two independent Tailwind CSS pipelines cause utility classes to silently resolve to nothing in the other pipeline."
tags: [leptos, proc-macro, tailwind-css, design-tokens, build-system, cargo-leptos, css-pipeline, mosaic]
---

# Proc macro and dual Tailwind pipeline desync with design tokens

## Problem

After changing Rust source files in Mosaic demo example components (replacing hardcoded Tailwind classes like `gray-*` with semantic token classes like `neutral-*`), the changes did not appear in the browser despite:

1. `just watch-mosaic-demo` (cargo-leptos watch) detecting changes and recompiling successfully
2. Hard refresh (`Cmd+Shift+R`) in the browser

Two distinct root causes were identified:

### Root cause 1: Proc macro stale output

The `demo_macro` proc macro reads example `.rs` files at compile time via `fs::read_to_string`. Cargo's incremental compilation does not track files read inside proc macros -- it only invalidates when the proc macro crate itself or its Cargo-tracked dependencies change. The running server binary continued serving HTML with old file contents baked in from the previous compilation.

A hard refresh only re-fetches from the running server. It cannot force re-execution of a proc macro.

**Fix**: Full server restart (kill and re-run `just watch-mosaic-demo`).

### Root cause 2: Dual Tailwind pipelines with unshared tokens

The Mosaic project has two independent Tailwind CSS pipelines:

| Pipeline | Entry point | Runs via |
|----------|------------|----------|
| Tiles | `tiles/src/components/theme_provider/main.css` | `build.rs` + standalone Tailwind CLI |
| Demo | `demo/style/tailwind.css` | cargo-leptos + tailwindcss |

Design tokens (`@theme static` block) were only defined in the tiles pipeline. Tailwind v4 only generates utility classes for custom colors when they are declared via `@theme` in the input CSS. Without the tokens in the demo pipeline, classes like `bg-neutral-50` in the demo layout produced no CSS rules -- silently.

**Fix**: Extract tokens into a shared `tokens.css` imported by both pipelines. Copy `tokens.css` to `OUT_DIR` in `build.rs` so the `@import "./tokens.css"` resolves correctly.

## Key takeaways

- **Watch mode is not a clean build.** `cargo-leptos watch` uses incremental compilation. Proc macros that read files via `fs::read_to_string` are not re-invoked when those files change. A full restart is required.
- **Multiple Tailwind pipelines must share token definitions.** Define all `@theme` tokens in one canonical file and import it from every pipeline entry point. Token definitions in only one pipeline cause silent class resolution failures.
- **`@import` paths in `build.rs` resolve against `OUT_DIR`.** When `build.rs` copies CSS to `OUT_DIR` before running Tailwind CLI, any `@import` with a relative path resolves against `OUT_DIR`, not the source directory. Imported files must also be copied there.
- **Silent failures are the worst kind.** Both issues produced no error messages -- the build succeeded, the server responded, the HTML contained the right classes. Only visual inspection revealed that styles were not applying.

## Prevention patterns

- Use `cargo:rerun-if-changed` in `build.rs` for every file read at build time
- Keep all design tokens in a single `tokens.css` file imported by all Tailwind pipelines
- When debugging "styles not applying", check: (1) is the class in the served CSS? (2) is the CSS from the right pipeline? (3) was the server restarted after proc macro input changes?
