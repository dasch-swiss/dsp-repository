import { expect, test } from "@playwright/test";

// Pre-migration component-isolation A/B for the Mosaic playground (DEV-6642).
//
// The playground PAGES are rebuilt as hand-written Maud in Phase 4, so a page-level
// pixel diff is invalid (the page chrome itself changes). Instead we screenshot each
// example block in ISOLATION, located by the macro-generated `data-example-key` wrapper
// — the showcase's stable, intrinsic hook. If the rebuilt showcase preserves
// `data-example-key` and keeps the example inputs/titles identical, this compares
// COMPONENT OUTPUT (a valid parity gate for the tiles port), not the page chrome.
//
// Where keeping inputs identical isn't practical, fall back to human visual sign-off
// (see COMPONENT-SIGNOFF.md). Baselines are throwaway, captured fresh in Phase 6.
//
// Surviving components only (Q6): accordion, popover, button_group are dropped, so they
// are intentionally absent here.

const BASE_URL = "http://localhost:3000";

const ROUTES = [
  "theme",
  "badge",
  "breadcrumb",
  "button",
  "card",
  "icon",
  "link",
  "tabs",
];

test.describe("Visual parity — Mosaic playground (component isolation)", () => {
  for (const route of ROUTES) {
    test(`${route} — each example in isolation`, async ({ page }) => {
      await page.goto(`${BASE_URL}/${route}`);

      const blocks = page.locator("[data-example-key]");
      await expect(blocks.first()).toBeVisible();
      await page.evaluate(() => document.fonts.ready.then(() => {}));

      // Discover every example (and the anatomy block) on the page, so coverage tracks
      // the showcase automatically rather than a hardcoded list.
      const keys = await blocks.evaluateAll((els) =>
        els.map((el) => el.getAttribute("data-example-key") ?? ""),
      );
      expect(keys.length).toBeGreaterThan(0);

      for (const key of keys) {
        await expect(
          page.locator(`[data-example-key="${key}"]`),
        ).toHaveScreenshot(`example-${key}.png`, { maxDiffPixelRatio: 0.01 });
      }
    });
  }
});
