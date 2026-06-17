# DPE visual parity — migration A/B reference (DEV-6642)

One-time reference for the Leptos→Maud migration. Delete once the migration has landed
and been signed off. This is **not** a permanent CI visual-regression gate.

## Model

The DPE keeps the same pages and URLs, so visual parity is proven by a **page-level pixel
A/B at tight tolerance** (`maxDiffPixelRatio: 0.01`). The specs are black-box over HTTP,
so the *same* specs (`tests/visual-*.spec.ts`) drive both:

- the **pre-migration** build (base commit, served from a separate worktree — the "HTML
  oracle", see `just oracle-*`), and
- the **migrated** build.

Locators use ARIA roles, contract ids (`#project-tabs`, `#tab-panel`), form/attribute
selectors and visible text — **never Tailwind classes** — so they resolve against both
builds even as the markup is rewritten. Screenshots are blind to HTML churn (`data-hk`,
whitespace, attribute order), so a tight-tolerance match on an unchanged surface *is* the
proof of parity.

Baselines are **throwaway** (gitignored) and captured fresh in one environment in Phase 6;
the committed artifact is the spec code.

## Procedure (Phase 6)

1. In one consistent environment, capture the pre-migration build (oracle) into an
   ephemeral baseline set, then capture the migrated build and diff at tight tolerance.
2. **Unchanged surfaces** → match = parity proven. Do not loosen tolerance globally.
3. **Delta surfaces** (below) → expected to differ; a human reviews the diff image and
   either fixes a regression or accepts the intended change.
4. The `#project-tabs` structural diff is also checked by the **HTML oracle** normalized
   semantic diff (`just oracle-diff`), which sees attributes/nesting that pixels can't.

## Surfaces expected to change pixels (delta surfaces)

Everything **not** listed here must stay pixel-identical. These are the only legitimate
diffs; each is a DaisyUI utility being removed (Phase 3) or a `.dpe-*`→Mosaic swap.

| Surface | DaisyUI / class | Source | Where it shows | Risk |
|---|---|---|---|---|
| Status & access-rights badges | `.tooltip` | `pages/projects/components/statusbadge.rs:25,32` | project cards (list) | **High** — tooltips are not pixel-identical |
| "How to cite" copy button | `.tooltip` + `.btn` | `pages/project/components/copy_button.rs:12` | detail → overview | High |
| Coverage info markers | `.tooltip` | `…/dataset_overview_section/coverage_section.rs:57,92` | detail → overview | Medium |
| Mobile filters button + close | `.btn btn-outline` / `.btn btn-circle btn-ghost` | `pages/projects/components/mobile_filters_button.rs:17,32` | list (mobile) | Medium |
| Tab loading spinner | `.loading .loading-spinner` | `components/loading.rs:7` | detail (SSE, transient) | Low (transient; rarely captured) |
| `.dpe-card` → Mosaic `Card` | `.dpe-card` | `…/project_details_tabs/mod.rs:29` | detail tabs container | Medium (only if not a 1:1 class match) |
| Not-found page | — (Leptos fallback → Axum 404, REQ-1.7) | router fallback | `/dpe/<unknown>` | Expected full rewrite |

Notes:
- The **filter info-tooltip** (`filter_checkbox_group.rs:29`) is a plain Tailwind
  `group-hover` tooltip, **not** DaisyUI — it is *not* a delta surface.
- **Tabs** styling is promoted to Mosaic reusing `tabs.css`; expected parity, not a delta.
- Other `.dpe-*` utilities (`dpe-title`, `dpe-subtitle`, `dpe-divider`, `dpe-small`,
  `dpe-max-layout-width`) are retained in the unified stylesheet → expected parity.

## Coverage (which spec captures what)

- `visual-project-detail.spec.ts` — project 0803 (all three tabs: `#project-tabs`,
  tablist, full page) + 0103 (long publications list, content variety).
- `visual-projects-list.spec.ts` — default, search results, no-results, status filter,
  pagination page 2, first-card region (delta); mobile: default (filters button) and
  open filters dialog.
- `visual-about.spec.ts` — about page.
- `visual-misc.spec.ts` — 404 not-found (delta).

A project with *neither* abstract nor publications does not exist in the data, so the
"publications tab absent" branch is unreachable — every project renders all three tabs.

## Locators / test hooks

Per the migration decision (plan Appendix C): the A/B uses **semantic locators** (roles,
contract ids, text). A minimal `data-testid` convention is introduced only **during the
Maud rewrite**, and only where semantic HTML is insufficient — not retrofitted into the
current Leptos markup.

## Running

Requires `node` on PATH and a release server binary at `target/release/dpe-server`:

```sh
cargo leptos build --release --project=dpe   # produces target/release/dpe-server + site
cd modules/dpe/web-e2e-tests
npm install                                   # first time (also: npx playwright install)
npx playwright test visual-                   # captures baselines on first run
```
