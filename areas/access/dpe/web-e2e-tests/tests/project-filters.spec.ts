import { expect, test } from "@playwright/test";

const BASE_URL = "http://localhost:4000";

// The no-JavaScript path runs under the `chromium-no-js` project (`@no-js` tag); the
// rest runs under `chromium`.

test.describe("facet form without JavaScript @no-js", () => {
  test("checking a box and Apply filters the list and puts the parameter in the URL", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/dpe/projects`);
    await page.locator("#filter-ongoing").check();
    await page.getByRole("button", { name: "Apply filters" }).click();
    await expect(page).toHaveURL(/\/dpe\/projects\?.*ongoing=true/);
    await expect(page.locator("#filter-ongoing")).toBeChecked();
  });

  test("a comma-joined URL opened directly renders with its boxes checked", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/dpe/projects`);
    const values = await page
      .locator('input[name="type_of_data"]')
      .evaluateAll((inputs) =>
        inputs.slice(0, 2).map((i) => (i as HTMLInputElement).value),
      );
    expect(values.length).toBe(2);

    await page.goto(
      `${BASE_URL}/dpe/projects?type_of_data=${values.map(encodeURIComponent).join(",")}`,
    );
    for (const value of values) {
      await expect(
        page.locator(`input[name="type_of_data"][value="${value}"]`),
      ).toBeChecked();
    }
  });

  test("Apply with a search set keeps the search", async ({ page }) => {
    await page.goto(`${BASE_URL}/dpe/projects?search=a`);
    await page.locator("#filter-ongoing").check();
    await page.getByRole("button", { name: "Apply filters" }).click();
    await expect(page).toHaveURL(/search=a/);
    await expect(page).toHaveURL(/ongoing=true/);
  });

  test("following the page-2 link on a filtered page keeps the filter", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/dpe/projects?ongoing=true`);
    const pageTwo = page
      .getByRole("navigation", { name: "Pagination" })
      .getByRole("link", { name: "2", exact: true });
    test.skip(
      (await pageTwo.count()) === 0,
      "the ongoing projects fit on one page",
    );
    await pageTwo.click();
    await expect(page).toHaveURL(/page=2/);
    await expect(page).toHaveURL(/ongoing=true/);
    await expect(page.locator("#filter-ongoing")).toBeChecked();
  });

  test.describe("mobile panel", () => {
    test.use({ viewport: { width: 375, height: 812 } });

    test("the Filters button opens the panel and Close closes it", async ({
      page,
    }) => {
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

test.describe("facet form with JavaScript", () => {
  test("toggling a box updates the list in place and writes the canonical URL", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/dpe/projects`);
    const count = page.locator("#project-results-count");
    await count.evaluate((el) => {
      (el as HTMLElement & { __marker?: boolean }).__marker = true;
    });
    const before = await count.textContent();
    await page.evaluate(() => {
      (window as Window & { __sameDocument?: boolean }).__sameDocument = true;
    });

    const box = page.locator("#filter-ongoing");
    await box.focus();
    await page.keyboard.press("Space");

    await expect(page).toHaveURL(`${BASE_URL}/dpe/projects?ongoing=true`);
    await expect(count).not.toHaveText(before ?? "");
    expect(
      await page.evaluate(
        () => (window as Window & { __sameDocument?: boolean }).__sameDocument,
      ),
    ).toBe(true);
    await expect(box).toBeFocused();
    expect(
      await count.evaluate(
        (el) => (el as HTMLElement & { __marker?: boolean }).__marker,
      ),
    ).toBe(true);
    await expect(page.locator("#project-filters-clear")).toContainText(
      "Clear all",
    );

    await page.reload();
    await expect(page.locator("#filter-ongoing")).toBeChecked();
  });

  test("the Apply button is hidden", async ({ page }) => {
    await page.goto(`${BASE_URL}/dpe/projects`);
    await expect(
      page.getByRole("button", { name: "Apply filters" }),
    ).toBeHidden();
  });

  test("after filtering, opening a project and Back, the boxes match the URL", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/dpe/projects`);
    await page.locator("#filter-ongoing").check();
    await expect(page).toHaveURL(/ongoing=true/);
    await page
      .locator("#project-results a[href^='/dpe/projects/']")
      .first()
      .click();
    await expect(page).toHaveURL(/\/dpe\/projects\/[^?]+$/);
    await page.goBack();
    await expect(page).toHaveURL(/ongoing=true/);
    await expect(page.locator("#filter-ongoing")).toBeChecked();
  });

  test("a failed results request falls back to a full page load with the filter applied", async ({
    page,
  }) => {
    await page.route("**/dpe/projects/results**", (route) =>
      route.fulfill({ status: 500, body: "" }),
    );
    await page.goto(`${BASE_URL}/dpe/projects`);
    await page.locator("#filter-ongoing").check();
    await expect(page).toHaveURL(/\/dpe\/projects\?ongoing=true/);
    await expect(page.locator("#filter-ongoing")).toBeChecked();
  });

  test.describe("mobile panel", () => {
    test.use({ viewport: { width: 375, height: 812 } });

    test("opens without a request and closes with Escape, Close and the backdrop", async ({
      page,
    }) => {
      await page.goto(`${BASE_URL}/dpe/projects`);
      const trigger = page.getByRole("button", {
        name: "Filters",
        exact: true,
      });
      const panel = page.locator("#project-filters-panel");
      const requests: string[] = [];
      page.on("request", (r) => {
        if (r.url().startsWith(BASE_URL)) requests.push(r.url());
      });

      // Escape with the panel closed changes nothing.
      await page.keyboard.press("Escape");
      await expect(panel).toBeHidden();

      await trigger.click();
      await expect(panel).toBeVisible();
      await expect(trigger).toHaveAttribute("aria-expanded", "true");
      await expect(panel).toBeFocused();
      await page.keyboard.press("Escape");
      await expect(panel).toBeHidden();
      await expect(trigger).toHaveAttribute("aria-expanded", "false");
      await expect(trigger).toBeFocused();

      await trigger.click();
      await page.getByRole("button", { name: "Close filters" }).click();
      await expect(panel).toBeHidden();
      await expect(trigger).toBeFocused();

      await trigger.click();
      // Below `md` the panel covers the whole width, so the backdrop has no free pixel.
      await page.locator("#project-filters-backdrop").dispatchEvent("click");
      await expect(panel).toBeHidden();
      await expect(trigger).toBeFocused();

      expect(requests).toEqual([]);
      await expect(page).toHaveURL(`${BASE_URL}/dpe/projects`);
    });
  });

  test.describe("reflow", () => {
    test.use({ viewport: { width: 320, height: 640 } });

    test("the open panel has no horizontal scroll and 24 px label rows", async ({
      page,
    }) => {
      await page.goto(`${BASE_URL}/dpe/projects`);
      await page.getByRole("button", { name: "Filters", exact: true }).click();
      await expect(page.locator("#project-filters-panel")).toBeVisible();
      const noHorizontalScroll = await page.evaluate(
        () =>
          document.documentElement.scrollWidth <=
          document.documentElement.clientWidth,
      );
      expect(noHorizontalScroll).toBe(true);
      const heights = await page
        .locator("#project-filters label")
        .evaluateAll((labels) =>
          labels.map((l) => l.getBoundingClientRect().height),
        );
      expect(heights.length).toBeGreaterThan(0);
      for (const h of heights) expect(h).toBeGreaterThanOrEqual(24);
    });
  });
});
