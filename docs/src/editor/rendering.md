# Rendering, Datastar and Styling

The view layer's rules: server-rendered Maud, Datastar for enhancement, Tailwind built from explicit source globs.

## Rendering model

Same as DPE: server-rendered HTML with **Maud**, served by **Axum**, with **Datastar** for interactivity over SSE. No client-side WASM, no hydration, no islands. The server is the single source of truth for UI state.

## Three accessibility decisions the markup depends on

Each of these fails silently: the page renders correctly, and only a screen-reader user or a keyboard user notices.

- **A field's obligation lives inside its own `<label>` (or `<legend>`), not beside it.** No input carries `required` or `aria-required`: a draft may be missing anything and a browser refusing to save one is the opposite of what is asked for. That leaves the accessible name as the only channel the tier has, so a pill rendered as a sibling is visible and nothing else: a reader tabbing to the control hears "Name, edit text". Five control builders compose their own label, so one test asserts the obligation per field across every section rather than on an example; forgetting it in one builder renders identically to a sighted reader.
- **The status region is `aria-live="polite"` and holds no element with a live role of its own.** A refusal renders `AlertVariant::Warning` rather than `Danger` for that reason: `Danger` carries `role="alert"`, an implicit *assertive* region, and screen readers do not agree on which politeness wins when one is nested inside a polite region — some interrupt, which is the behaviour the polite region exists to avoid. The region announces; the alert only styles.
- **A rail link states its accessible name.** The section title and its progress are adjacent `<span>`s with no whitespace between them, because a flex column is what puts them on two lines — so the name computation concatenates them into "Overview5 of 5 required". The `aria-label` starts with the visible title, which is what WCAG 2.5.3 asks of an `aria-label` over visible text, and is omitted for a section with no requirements where the title is already the whole name.

The colour pairings are measured against the design tokens, with the method cross-checked on the four ratios `text_field.css` already documents: `warning-800` on `warning-50` 11.30:1, `info-800` on `info-50` 11.18:1, `neutral-700` on `neutral-100` 7.78:1, and `neutral-600` on the `gray-50` page 5.73:1 — all above WCAG 2.1 AA's 4.5:1 for the 12px bold pill text and the hint text.

## Datastar

The editor vendors Datastar from `areas/deposit/editor/public/vendor/`, whose README is the version of record; do not restate the version here. DPE vendors its own copy and is bumped independently.

One thing to get right, and it fails quietly: **keyed plugin attributes use `:`, not `-`** — `data-on:click`, `data-attr:disabled`, `data-class:open`, and `data-init` rather than `data-on-load`. This has been true since RC.6, so it matches DPE's markup too. The hyphen form produces a console error and an inert control: the page renders fine and a snapshot test asserting the attribute is present still passes.

## Styling

`areas/deposit/editor/style/main.css` is the single Tailwind entry, built by `just css-editor` (dev) or `just css-editor-release` (content-hashed). It imports the design tokens and the `mosaic-tiles` component barrel.

`@import 'tailwindcss' source(none)` means classes are collected **only** from the explicit `@source` globs, which must cover every crate that emits Tailwind classes. A missing glob produces no build error — just markup whose classes resolve to nothing. After a change that adds classes in a new location, grep the built stylesheet for them.

New Mosaic tiles are added **demand-driven**: a screen that needs a missing primitive adds it to `mosaic-tiles` with a playground showcase and a unit test at that point, rather than an up-front form kit. Their CSS goes in `mosaic-tiles/src/components/components.css`, the barrel every consumer imports.

**Check a tile against the surface you are putting it on.** Tiles are styled for light backgrounds — `link` is `text-primary-600`, which measures 2.35:1 on the footer's `bg-slate-800` and fails WCAG 2.1 AA. That is why the footer uses plain anchors inheriting `text-gray-300` (9.93:1), as DPE's does. A dark-surface variant of a tile is a design-system change, so it belongs in `mosaic-tiles` with its own showcase rather than being worked around locally.
