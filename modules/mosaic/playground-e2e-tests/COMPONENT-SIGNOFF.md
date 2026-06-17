# Mosaic playground — migration parity sign-off (DEV-6642)

One-time reference for the Leptos→Maud migration. Delete once the migration has landed
and been signed off. This is **not** a permanent CI gate.

## Why the playground needs a different parity model than the DPE

The DPE keeps the same pages, so a page-level pixel A/B at tight tolerance proves parity.
The **playground pages are rebuilt** as hand-written Maud (Q4), so the page surface itself
changes — a page-level diff would be wholesale noise. Parity here is therefore at the
**component** level, by two complementary means:

1. **Automated component-isolation A/B** — `tests/component-isolation.spec.ts` screenshots
   each example block located by its macro-generated `data-example-key` wrapper. If the
   rebuilt showcase preserves that hook and keeps the example inputs/titles identical, the
   diff compares **component output**, which is a valid parity gate for the tiles port.
2. **Human visual sign-off** — for interactive states (hover/open/clicked) and any example
   whose inputs can't be kept identical across the rebuild. The DPE A/B already covers
   Card/Badge/Link/Breadcrumb/Icon/Button *as used*, so the playground is a secondary gate;
   be realistic that much of it will lean on human sign-off rather than the automated diff.

## Requirement on the rebuilt (Maud) showcase

For the automated isolation A/B to remain valid, the rebuilt playground must:

- Preserve a stable per-example hook (`data-example-key="{component}-{example}"`, and
  `{component}-anatomy`) on each example wrapper.
- Keep the **example inputs and titles identical** to the pre-migration showcase for the
  components below (so the rendered output is the only variable).

Where that isn't practical, record it here and sign off by eye on the Cloud Run preview.

## Components in scope

Surviving components (port to Maud, must reach parity). Dropped per Q6 — **no sign-off**:
`accordion`, `popover`, `button_group`.

| Component  | Route         | Examples                                            | Isolation A/B | Human sign-off |
|------------|---------------|-----------------------------------------------------|:-------------:|:--------------:|
| theme      | `/theme`      | colors, typography                                  | [ ]           | [ ]            |
| badge      | `/badge`      | variants, sizes, with_icons, usage                  | [ ]           | [ ]            |
| breadcrumb | `/breadcrumb` | basic, nested, with_icons                           | [ ]           | [ ]            |
| button     | `/button`     | variants, types, disabled, with_icons, interactive  | [ ]           | [ ]            |
| card       | `/card`       | variants, with_header_footer, with_images, with_icons, interactive | [ ] | [ ]   |
| icon       | `/icon`       | all_icons, sizes_and_colors, usage                  | [ ]           | [ ]            |
| link       | `/link`       | basic, external, as_button, disabled, target_attribute | [ ]        | [ ]            |
| tabs       | `/tabs`       | basic, with_icons, multiple_groups, interactive     | [ ]           | [ ]            |

Interactive examples (button `interactive`, card `interactive`, link, tabs) capture only
the initial server-rendered state in the automated A/B; their hover/click/open states are
human sign-off items.

## Procedure (Phase 6)

1. Build and serve the pre-migration playground (base commit, separate worktree) and the
   migrated playground in one environment.
2. Run `component-isolation.spec.ts` against both; diff the `[data-example-key]` blocks at
   tight tolerance. Unchanged blocks → parity proven.
3. For interactive states and any non-identical-input example, eyeball the Cloud Run
   preview and tick **Human sign-off** above.
