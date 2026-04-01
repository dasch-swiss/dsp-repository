import { expect, test } from "@playwright/test";

const BASE_URL = "http://localhost:4000";

// Use a project shortcode known to have publications (incunabula)
const PROJECT_SHORTCODE = "0803";
const PROJECT_URL = `${BASE_URL}/projects/${PROJECT_SHORTCODE}`;

test.describe("Tab switching — Datastar SSE interactions", () => {
  test("clicking a tab does NOT trigger full page navigation", async ({
    page,
  }) => {
    await page.goto(PROJECT_URL);

    let fullNavigation = false;
    page.on("load", () => {
      fullNavigation = true;
    });

    // Reset flag after initial load event settles
    fullNavigation = false;

    const contributorsTab = page.locator('[role="tab"]', {
      hasText: "Contributors",
    });
    await contributorsTab.click();

    // Wait for the tab panel content to update via SSE
    await expect(page.locator("#tab-panel")).not.toBeEmpty();
    // Short wait to ensure no navigation event fires
    await page.waitForTimeout(500);

    expect(fullNavigation).toBe(false);
  });

  test("clicking a tab updates tab panel content", async ({ page }) => {
    await page.goto(PROJECT_URL);

    const initialContent = await page.locator("#tab-panel").innerHTML();

    const contributorsTab = page.locator('[role="tab"]', {
      hasText: "Contributors",
    });
    await contributorsTab.click();

    // Wait for content to change
    await expect(page.locator("#tab-panel")).not.toHaveInnerHTML(
      initialContent,
    );
  });

  test("scroll position preserved after tab switch", async ({ page }) => {
    await page.goto(PROJECT_URL);

    // Scroll down
    await page.evaluate(() => window.scrollTo(0, 200));
    const scrollBefore = await page.evaluate(() => window.scrollY);
    expect(scrollBefore).toBeGreaterThan(0);

    const contributorsTab = page.locator('[role="tab"]', {
      hasText: "Contributors",
    });
    await contributorsTab.click();

    // Wait for tab panel to update
    await expect(page.locator('[role="tab"][aria-selected="true"]')).toHaveText(
      /Contributors/,
    );

    const scrollAfter = await page.evaluate(() => window.scrollY);
    expect(scrollAfter).toBe(scrollBefore);
  });

  test("browser URL updates to ?tab={tab} after tab switch", async ({
    page,
  }) => {
    await page.goto(PROJECT_URL);

    const contributorsTab = page.locator('[role="tab"]', {
      hasText: "Contributors",
    });
    await contributorsTab.click();

    // Wait for URL to be updated via history.replaceState
    await page.waitForFunction(
      () => window.location.search.includes("tab=contributors"),
      PROJECT_SHORTCODE,
    );

    expect(page.url()).toContain(`?tab=contributors`);
  });

  test("direct navigation to ?tab=publications renders correct tab", async ({
    page,
  }) => {
    await page.goto(`${PROJECT_URL}?tab=publications`);

    const activeTab = page.locator('[role="tab"][aria-selected="true"]');
    await expect(activeTab).toHaveText(/Publications/);
  });

  test("direct navigation to ?tab=contributors renders correct tab", async ({
    page,
  }) => {
    await page.goto(`${PROJECT_URL}?tab=contributors`);

    const activeTab = page.locator('[role="tab"][aria-selected="true"]');
    await expect(activeTab).toHaveText(/Contributors/);
  });

  test("refreshing after tab switch renders correct tab", async ({ page }) => {
    await page.goto(PROJECT_URL);

    const contributorsTab = page.locator('[role="tab"]', {
      hasText: "Contributors",
    });
    await contributorsTab.click();

    // Wait for URL update
    await page.waitForFunction(() =>
      window.location.search.includes("tab=contributors"),
    );

    // Reload the page
    await page.reload();

    const activeTab = page.locator('[role="tab"][aria-selected="true"]');
    await expect(activeTab).toHaveText(/Contributors/);
  });

  test("tabs work as plain links with JS disabled", async ({ browser }) => {
    const context = await browser.newContext({ javaScriptEnabled: false });
    const page = await context.newPage();

    await page.goto(PROJECT_URL);

    // Tabs should be <a> elements with href — verify they are navigable links
    const contributorsTab = page.locator('[role="tab"]', {
      hasText: "Contributors",
    });
    const href = await contributorsTab.getAttribute("href");
    expect(href).toContain(`/projects/${PROJECT_SHORTCODE}?tab=contributors`);

    // Click navigates via full page load (graceful degradation)
    await contributorsTab.click();
    await page.waitForLoadState("domcontentloaded");

    // The URL should contain the tab query parameter
    expect(page.url()).toContain("tab=contributors");

    // The correct tab should be active in the server-rendered output
    const activeTab = page.locator('[role="tab"][aria-selected="true"]');
    await expect(activeTab).toHaveText(/Contributors/);

    await context.close();
  });

  test("loading indicator appears during SSE request", async ({ page }) => {
    await page.goto(PROJECT_URL);

    // The data-indicator attribute on tab links triggers a loading state
    // Check that the tab link has the data-indicator attribute
    const contributorsTab = page.locator('[role="tab"]', {
      hasText: "Contributors",
    });
    const indicator = await contributorsTab.getAttribute(
      "data-indicator:_tab_loading",
    );
    // The attribute should exist (may be empty string for boolean attributes)
    expect(indicator).not.toBeNull();
  });

  test("rapid tab clicking does not cause stale content", async ({ page }) => {
    await page.goto(PROJECT_URL);

    // Click tabs rapidly in sequence
    const overviewTab = page.locator('[role="tab"]', { hasText: "Overview" });
    const contributorsTab = page.locator('[role="tab"]', {
      hasText: "Contributors",
    });

    await contributorsTab.click();
    await overviewTab.click();
    await contributorsTab.click();

    // Wait for the final state to settle
    await expect(page.locator('[role="tab"][aria-selected="true"]')).toHaveText(
      /Contributors/,
    );

    // Verify URL matches the last clicked tab
    await page.waitForFunction(() =>
      window.location.search.includes("tab=contributors"),
    );
    expect(page.url()).toContain("tab=contributors");
  });

  test("default tab is overview when no ?tab parameter", async ({ page }) => {
    await page.goto(PROJECT_URL);

    const activeTab = page.locator('[role="tab"][aria-selected="true"]');
    await expect(activeTab).toHaveText(/Overview/);
  });
});
