import { expect, test } from "@playwright/test";

const BASE_URL = "http://localhost:4000";

// The no-JavaScript path runs under the `chromium-no-js` project (`@no-js` tag); the
// rest runs under `chromium`.

test.describe("facet form without JavaScript @no-js", () => {
  test("checking a box and Apply filters the list and puts the parameter in the URL", async ({ page }) => {
    await page.goto(`${BASE_URL}/dpe/projects`);
    await page.locator("#filter-ongoing").check();
    await page.getByRole("button", { name: "Apply filters" }).click();
    await expect(page).toHaveURL(/\/dpe\/projects\?.*ongoing=true/);
    await expect(page.locator("#filter-ongoing")).toBeChecked();
  });

  test("a comma-joined URL opened directly renders with its boxes checked", async ({ page }) => {
    await page.goto(`${BASE_URL}/dpe/projects`);
    const values = await page
      .locator('input[name="type_of_data"]')
      .evaluateAll((inputs) => inputs.slice(0, 2).map((i) => (i as HTMLInputElement).value));
    expect(values.length).toBe(2);

    await page.goto(`${BASE_URL}/dpe/projects?type_of_data=${values.map(encodeURIComponent).join(",")}`);
    for (const value of values) {
      await expect(page.locator(`input[name="type_of_data"][value="${value}"]`)).toBeChecked();
    }
  });

  test("Apply with a search set keeps the search", async ({ page }) => {
    await page.goto(`${BASE_URL}/dpe/projects?search=a`);
    await page.locator("#filter-ongoing").check();
    await page.getByRole("button", { name: "Apply filters" }).click();
    await expect(page).toHaveURL(/search=a/);
    await expect(page).toHaveURL(/ongoing=true/);
  });

  test("following the page-2 link on a filtered page keeps the filter", async ({ page }) => {
    await page.goto(`${BASE_URL}/dpe/projects?ongoing=true`);
    const pageTwo = page.getByRole("navigation", { name: "Pagination" }).getByRole("link", { name: "2", exact: true });
    test.skip((await pageTwo.count()) === 0, "the ongoing projects fit on one page");
    await pageTwo.click();
    await expect(page).toHaveURL(/page=2/);
    await expect(page).toHaveURL(/ongoing=true/);
    await expect(page.locator("#filter-ongoing")).toBeChecked();
  });

  test.describe("mobile panel", () => {
    test.use({ viewport: { width: 375, height: 812 } });

    test("the Filters button opens the panel and Close closes it", async ({ page }) => {
      await page.goto(`${BASE_URL}/dpe/projects`);
      await expect(page.locator("#project-filters-panel")).toBeHidden();

      await page.getByRole("button", { name: "Filters", exact: true }).click();
      await expect(page.locator("#project-filters-panel")).toBeVisible();

      await page.getByRole("button", { name: "Close filters" }).click();
      await expect(page.locator("#project-filters-panel")).toBeHidden();
      await expect(page).not.toHaveURL(/dialog=/);
    });
  });
});
