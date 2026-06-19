import { expect, test } from "@playwright/test";

// Pre-migration visual A/B (DEV-6642). The about page is static content; the heading
// "Help & Documentation" is the stable wait anchor. Baselines are throwaway.

const ABOUT_URL = "http://localhost:4000/dpe/about";

test.describe("Visual parity — about", () => {
  test("about — full page", async ({ page }) => {
    await page.goto(ABOUT_URL);
    await expect(
      page.getByRole("heading", { name: "Help & Documentation" }),
    ).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("about-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });
});
