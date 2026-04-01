import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { formatViolations } from "./axe-helpers";

const BASE_URL = "http://localhost:4000";
const PROJECT_SHORTCODE = "0803";
const PROJECT_URL = `${BASE_URL}/projects/${PROJECT_SHORTCODE}`;

test.describe("Tab accessibility — ARIA roles and keyboard navigation", () => {
  test("ARIA roles are present on tab components", async ({ page }) => {
    await page.goto(PROJECT_URL);

    // Tab container has role="tablist"
    const tablist = page.locator('[role="tablist"]');
    await expect(tablist).toBeVisible();

    // Individual tabs have role="tab"
    const tabs = page.locator('[role="tab"]');
    const tabCount = await tabs.count();
    expect(tabCount).toBeGreaterThanOrEqual(2); // at minimum: overview + contributors

    // Tab panel has role="tabpanel"
    const tabpanel = page.locator('[role="tabpanel"]');
    await expect(tabpanel).toBeVisible();
  });

  test("tablist has accessible label", async ({ page }) => {
    await page.goto(PROJECT_URL);

    const tablist = page.locator('[role="tablist"]');
    await expect(tablist).toHaveAttribute("aria-label", "Project details");
  });

  test("aria-selected reflects active tab", async ({ page }) => {
    await page.goto(PROJECT_URL);

    // Default tab (overview) should be selected
    const overviewTab = page.locator('[role="tab"]', { hasText: "Overview" });
    await expect(overviewTab).toHaveAttribute("aria-selected", "true");

    // Other tabs should not be selected
    const contributorsTab = page.locator('[role="tab"]', {
      hasText: "Contributors",
    });
    await expect(contributorsTab).toHaveAttribute("aria-selected", "false");
  });

  test("aria-selected updates after tab switch", async ({ page }) => {
    await page.goto(PROJECT_URL);

    const contributorsTab = page.locator('[role="tab"]', {
      hasText: "Contributors",
    });
    await contributorsTab.click();

    // Wait for SSE update
    await expect(page.locator('[role="tab"][aria-selected="true"]')).toHaveText(
      /Contributors/,
    );

    // Overview should no longer be selected
    const overviewTab = page.locator('[role="tab"]', { hasText: "Overview" });
    await expect(overviewTab).toHaveAttribute("aria-selected", "false");
  });

  test("tab panel has aria-labelledby pointing to active tab", async ({
    page,
  }) => {
    await page.goto(PROJECT_URL);

    const tabpanel = page.locator('[role="tabpanel"]');
    await expect(tabpanel).toHaveAttribute("aria-labelledby", "tab-overview");
  });

  test("active tab has tabindex 0, inactive tabs have tabindex -1", async ({
    page,
  }) => {
    await page.goto(PROJECT_URL);

    const activeTab = page.locator('[role="tab"][aria-selected="true"]');
    await expect(activeTab).toHaveAttribute("tabindex", "0");

    const inactiveTabs = page.locator('[role="tab"][aria-selected="false"]');
    const count = await inactiveTabs.count();
    for (let i = 0; i < count; i++) {
      await expect(inactiveTabs.nth(i)).toHaveAttribute("tabindex", "-1");
    }
  });

  test("tabs have aria-controls pointing to tab panel", async ({ page }) => {
    await page.goto(PROJECT_URL);

    const tabs = page.locator('[role="tab"]');
    const count = await tabs.count();
    for (let i = 0; i < count; i++) {
      await expect(tabs.nth(i)).toHaveAttribute("aria-controls", "tab-panel");
    }
  });

  test("arrow key navigation between tabs", async ({ page }) => {
    await page.goto(PROJECT_URL);

    // Focus the active tab
    const overviewTab = page.locator('[role="tab"]', { hasText: "Overview" });
    await overviewTab.focus();

    // Press ArrowRight — focus should move to the next tab
    await page.keyboard.press("ArrowRight");

    // The next tab should now be focused
    const focusedTabText = await page.evaluate(() => {
      const el = document.activeElement;
      return el?.textContent?.trim() ?? "";
    });

    // The focused element should be a different tab (not Overview)
    expect(focusedTabText).not.toBe("");
    // It could be "Publications" or "Contributors" depending on whether
    // the project has publications
    expect(["Publications", "Contributors"]).toContain(focusedTabText);
  });

  test("no a11y violations after tab switch via SSE", async ({ page }) => {
    await page.goto(PROJECT_URL);

    // Click a non-default tab to trigger SSE patch
    await page.click('[role="tab"]:has-text("Contributors")');

    // Wait for the Datastar SSE patch to complete
    await expect(page.locator('[role="tab"][aria-selected="true"]')).toHaveText(
      /Contributors/,
    );

    // Run axe-core on the post-patch DOM
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
      .analyze();

    const count = results.violations.length;
    expect(
      count,
      `${count} accessibility violation(s) after tab switch:\n\n${formatViolations(results.violations)}`,
    ).toBe(0);
  });
});
