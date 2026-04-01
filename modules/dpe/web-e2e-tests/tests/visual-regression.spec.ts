import { expect, test } from "@playwright/test";

const BASE_URL = "http://localhost:4000";
const PROJECT_SHORTCODE = "0803";
const PROJECT_URL = `${BASE_URL}/projects/${PROJECT_SHORTCODE}`;

test.describe("Visual regression — tab states", () => {
  test("overview tab matches baseline screenshot", async ({ page }) => {
    await page.goto(PROJECT_URL);

    // Wait for tab content to be fully rendered
    await expect(page.locator('[role="tabpanel"]')).toBeVisible();

    await expect(page.locator("#project-tabs")).toHaveScreenshot(
      "tabs-overview.png",
      { maxDiffPixelRatio: 0.01 },
    );
  });

  test("publications tab matches baseline screenshot", async ({ page }) => {
    await page.goto(`${PROJECT_URL}?tab=publications`);

    await expect(page.locator('[role="tabpanel"]')).toBeVisible();

    await expect(page.locator("#project-tabs")).toHaveScreenshot(
      "tabs-publications.png",
      { maxDiffPixelRatio: 0.01 },
    );
  });

  test("contributors tab matches baseline screenshot", async ({ page }) => {
    await page.goto(`${PROJECT_URL}?tab=contributors`);

    await expect(page.locator('[role="tabpanel"]')).toBeVisible();

    await expect(page.locator("#project-tabs")).toHaveScreenshot(
      "tabs-contributors.png",
      { maxDiffPixelRatio: 0.01 },
    );
  });

  test("tab bar active state visual matches baseline", async ({ page }) => {
    await page.goto(PROJECT_URL);

    // Screenshot just the tab bar (tablist area excluding the panel)
    const tablist = page.locator('[role="tablist"]');
    await expect(tablist).toBeVisible();

    await expect(tablist).toHaveScreenshot("tablist-overview-active.png", {
      maxDiffPixelRatio: 0.01,
    });
  });

  test("full project page matches baseline screenshot", async ({ page }) => {
    await page.goto(PROJECT_URL);

    // Wait for content to be fully rendered
    await expect(page.locator('[role="tabpanel"]')).toBeVisible();

    await expect(page).toHaveScreenshot("project-page-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });
});
