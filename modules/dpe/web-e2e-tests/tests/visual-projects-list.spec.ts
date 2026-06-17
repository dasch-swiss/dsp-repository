import { expect, test } from "@playwright/test";

// Pre-migration visual A/B (DEV-6642). Same specs drive the old (Leptos) and new (Maud)
// builds over HTTP. The projects list is fully URL-query-driven (search, filters,
// pagination, the mobile filters dialog), so every state is reachable by URL without
// SSE interaction. Project cards are `a[href^="/dpe/projects/0"]` (all shortcodes start
// with "0"); the result count and empty-state use visible text. Baselines are throwaway.

const LIST_URL = "http://localhost:4000/dpe/projects";

// All project shortcodes begin with "0", so this matches a project card link but not the
// pagination links (`?page=`) or the about link.
const PROJECT_CARD = 'a[href^="/dpe/projects/0"]';

test.describe("Visual parity — projects list (desktop)", () => {
  test("default list — full page", async ({ page }) => {
    await page.goto(LIST_URL);
    await expect(page.locator(PROJECT_CARD).first()).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("list-default-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });

  test("search results — full page", async ({ page }) => {
    // Server-side search matches name/description/shortcode/status; "080" matches the
    // many 080x shortcodes, giving a populated results page.
    await page.goto(`${LIST_URL}?search=080`);
    await expect(page.locator(PROJECT_CARD).first()).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("list-search-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });

  test("no results — full page", async ({ page }) => {
    await page.goto(`${LIST_URL}?search=zzqqxxnomatch`);
    await expect(page.getByText("No projects found")).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("list-no-results-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });

  test("status filter (finished) — full page", async ({ page }) => {
    await page.goto(`${LIST_URL}?finished=true`);
    await expect(page.locator(PROJECT_CARD).first()).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("list-filter-finished-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });

  test("pagination page 2 — full page", async ({ page }) => {
    await page.goto(`${LIST_URL}?page=2`);
    await expect(page.locator(PROJECT_CARD).first()).toBeVisible();
    await expect(page.locator('nav[aria-label="Pagination"]')).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("list-page-2-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });

  // Delta surface (expected to differ in Phase 6): the status/access-rights badges use
  // DaisyUI `tooltip`. Captured in isolation so the human-reviewed diff is focused.
  test("first project card — region", async ({ page }) => {
    await page.goto(LIST_URL);
    const card = page.locator(PROJECT_CARD).first();
    await expect(card).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(card).toHaveScreenshot("list-project-card.png", {
      maxDiffPixelRatio: 0.01,
    });
  });
});

test.describe("Visual parity — projects list (mobile)", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test("default list — full page (mobile filters button)", async ({ page }) => {
    await page.goto(LIST_URL);
    await expect(page.locator(PROJECT_CARD).first()).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("list-mobile-default-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });

  test("filters dialog open — full page", async ({ page }) => {
    await page.goto(`${LIST_URL}?dialog=true`);
    // The desktop sidebar is display:none at this viewport, so its checkboxes are out of
    // the a11y tree; getByRole('checkbox') targets the open dialog's filter controls.
    await expect(page.getByRole("checkbox").first()).toBeVisible();
    await page.evaluate(() => document.fonts.ready.then(() => {}));

    await expect(page).toHaveScreenshot("list-mobile-dialog-full.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: true,
    });
  });
});
