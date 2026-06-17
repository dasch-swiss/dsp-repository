import { expect, test } from "@playwright/test";

// Pre-migration visual A/B (DEV-6642). These specs are black-box over HTTP, so the
// SAME specs drive both the pre-migration (Leptos) build and the migrated (Maud) build
// at verification time. Locators use ARIA roles, contract ids (#project-tabs,
// #tab-panel) and visible text — never Tailwind classes — so they resolve against both.
// Baselines are throwaway (gitignored) and captured fresh in one environment in Phase 6.

const BASE_URL = "http://localhost:4000";

// 0803 (incunabula) has both an abstract and publications, so all three tabs render:
// Overview, Publications, Contributors. This is the canonical detail-page surface.
const PROJECT_URL = `${BASE_URL}/dpe/projects/0803`;

// 0103 has 50 publications and non-Latin punctuation in its title — content variety
// (a long publications list), same page structure.
const VARIETY_URL = `${BASE_URL}/dpe/projects/0103`;

test.describe("Visual parity — project detail", () => {
  test("0803 overview — #project-tabs", async ({ page }) => {
    await page.goto(PROJECT_URL);
    await expect(page.locator('[role="tabpanel"]')).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page.locator("#project-tabs")).toHaveScreenshot(
      "detail-0803-overview-tabs.png",
      { maxDiffPixelRatio: 0.01 },
    );
  });

  test("0803 overview — tablist active state", async ({ page }) => {
    await page.goto(PROJECT_URL);
    await expect(page.locator('[role="tablist"]')).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page.locator('[role="tablist"]')).toHaveScreenshot(
      "detail-0803-tablist-overview-active.png",
      { maxDiffPixelRatio: 0.01 },
    );
  });

  test("0803 overview — full page", async ({ page }) => {
    await page.goto(PROJECT_URL);
    await expect(page.locator('[role="tabpanel"]')).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("detail-0803-overview-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });

  test("0803 publications — #project-tabs", async ({ page }) => {
    await page.goto(`${PROJECT_URL}?tab=publications`);
    await expect(page.locator('[role="tabpanel"]')).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page.locator("#project-tabs")).toHaveScreenshot(
      "detail-0803-publications-tabs.png",
      { maxDiffPixelRatio: 0.01 },
    );
  });

  test("0803 publications — full page", async ({ page }) => {
    await page.goto(`${PROJECT_URL}?tab=publications`);
    await expect(page.locator('[role="tabpanel"]')).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("detail-0803-publications-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });

  test("0803 contributors — #project-tabs", async ({ page }) => {
    await page.goto(`${PROJECT_URL}?tab=contributors`);
    await expect(page.locator('[role="tabpanel"]')).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page.locator("#project-tabs")).toHaveScreenshot(
      "detail-0803-contributors-tabs.png",
      { maxDiffPixelRatio: 0.01 },
    );
  });

  test("0803 contributors — full page", async ({ page }) => {
    await page.goto(`${PROJECT_URL}?tab=contributors`);
    await expect(page.locator('[role="tabpanel"]')).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("detail-0803-contributors-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });

  test("0103 publications — #project-tabs (long list)", async ({ page }) => {
    await page.goto(`${VARIETY_URL}?tab=publications`);
    await expect(page.locator('[role="tabpanel"]')).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page.locator("#project-tabs")).toHaveScreenshot(
      "detail-0103-publications-tabs.png",
      { maxDiffPixelRatio: 0.01 },
    );
  });

  test("0103 overview — full page", async ({ page }) => {
    await page.goto(VARIETY_URL);
    await expect(page.locator('[role="tabpanel"]')).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("detail-0103-overview-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });
});
