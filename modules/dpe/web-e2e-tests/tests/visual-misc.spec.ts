import { expect, test } from "@playwright/test";

// Pre-migration visual A/B (DEV-6642).
//
// Delta surface (EXPECTED to differ in Phase 6): the not-found page. Today it is the
// Leptos router fallback ("Page not found." inside the app shell). The migration
// replaces it with an Axum 404 fallback (REQ-1.7), so this capture exists to document
// the intended change for human review — not as a parity gate.

const NOT_FOUND_URL = "http://localhost:4000/dpe/this-route-does-not-exist";

test.describe("Visual parity — misc", () => {
  test("404 not-found — full page", async ({ page }) => {
    await page.goto(NOT_FOUND_URL);
    await expect(page.getByText("Page not found")).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("not-found-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });
});
