---
title: "Google Fonts loaded externally for design token typography"
date: 2026-02-12
category: design-decisions
component: mosaic/demo
module: mosaic/demo
problem_type: architecture
severity: low
symptoms: "Security review flagged external Google Fonts CDN loading without SRI as a third-party dependency risk."
root_cause: "Design token typography (Lora for display, Lato for body) requires font loading. Google Fonts was chosen for simplicity over self-hosting."
tags: [mosaic, fonts, google-fonts, security, cdn, design-tokens, typography]
---

# Google Fonts loaded externally for design token typography

## Context

The Mosaic design token infrastructure defines `font-display` (Lora) and `font-body` (Lato) as typography tokens. The demo app loads these fonts from Google Fonts via `<link>` tags in `demo/src/app.rs`.

## Decision

Use Google Fonts CDN rather than self-hosting. Accepted risks:

- **No SRI (Subresource Integrity)**: Google Fonts dynamically generates CSS based on User-Agent, making SRI hashes impractical.
- **Third-party dependency**: If Google Fonts is unavailable, the font fallback chain (`"Lora", Georgia, serif` / `"Lato", -apple-system, ...`) activates.
- **Privacy**: Google sees visitor IP and User-Agent for font requests.

## Rationale

- The demo app is an internal documentation tool, not a public-facing product.
- Google Fonts is a widely trusted CDN with high availability.
- Self-hosting adds build complexity (font file management, WOFF2 generation, CSS `@font-face` rules) for minimal benefit in this context.
- The tiles library itself is font-loading-agnostic; consuming apps choose their own loading strategy.

## Revisit if

- The demo becomes publicly deployed or handles sensitive data.
- A Content-Security-Policy (CSP) is introduced that restricts external origins.
- Self-hosting fonts becomes trivial (e.g., via a build plugin).
