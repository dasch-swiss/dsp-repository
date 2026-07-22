import { expect, test } from "@playwright/test";

// Smoke test for the Mosaic playground showcase.
//
// Every showcase page must serve successfully and render its example blocks,
// located by their `data-example-key` wrapper (emitted by the showcase pages).
// Examples are discovered per page, so coverage tracks the showcase
// automatically rather than a hardcoded list.

const BASE_URL = "http://localhost:3000";

const ROUTES = [
  "theme",
  "badge",
  "breadcrumb",
  "button",
  "card",
  "copy-button",
  "icon",
  "link",
  "loading",
  "tabs",
];

test.describe("Mosaic playground showcase", () => {
  for (const route of ROUTES) {
    test(`${route} — serves and renders its examples`, async ({ page }) => {
      const response = await page.goto(`${BASE_URL}/${route}`);
      expect(response?.status()).toBe(200);

      const blocks = page.locator("[data-example-key]");
      await expect(blocks.first()).toBeVisible();
      expect(await blocks.count()).toBeGreaterThan(0);
    });
  }
});
