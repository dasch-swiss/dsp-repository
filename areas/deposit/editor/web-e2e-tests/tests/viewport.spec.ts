import { expect, type Page, test } from "@playwright/test";

import {
  DEPOSITOR_STATE,
  NOTICE_SHORTCODE,
  RDU_STATE,
} from "../playwright.config";

test.use({ storageState: DEPOSITOR_STATE });

/** The save notice region — not a field's own error, which is a `<p>`. */
const noticeOf = (page: Page) =>
  page.locator('div[aria-live="polite"]').first();

async function saveFromBelowTheFold(page: Page): Promise<void> {
  await page.goto(`/projects/${NOTICE_SHORTCODE}/sections/overview`);
  await page.setViewportSize({ width: 1280, height: 720 });

  const save = page.getByRole("button", { name: "Save draft" });
  await expect(save).toBeAttached();

  // The premise. If the control is not actually below the fold this test
  // proves nothing about scrolling, and must say so rather than pass.
  const box = await save.boundingBox();
  if (!box) throw new Error("Save draft has no layout box");
  expect(
    box.y,
    "Save draft is not below the fold at 1280x720, so this no longer exercises the sticky notice",
  ).toBeGreaterThan(720);

  await save.click();
}

test("the notice from a below-the-fold save stays inside the viewport", async ({
  page,
}) => {
  test.skip(
    test.info().project.name !== "chromium-js",
    "there is no notice to measure without JS",
  );

  // The notice region is `sticky top-0`. A `sticky` element whose every offset
  // resolves to `auto` is inert and scrolls away with the page — which is what
  // an absent `top-0` in the built stylesheet produces.
  //
  // Read the computed position, never the class: the class can be present while
  // the built stylesheet has dropped the rule.
  await saveFromBelowTheFold(page);

  const notice = noticeOf(page);
  await expect(notice).toBeAttached();
  await expect
    .poll(async () => ((await notice.textContent()) ?? "").trim().length > 0, {
      message: "Save draft produced no notice",
    })
    .toBe(true);

  const box = await notice.boundingBox();
  if (!box)
    throw new Error("the notice has no layout box — it is not rendered");

  const viewport = page.viewportSize();
  if (!viewport) throw new Error("the page has no viewport size");
  expect(
    box.y,
    `the notice rendered at y=${box.y}, outside the viewport. \`position: sticky\` with ` +
      "every offset `auto` is inert — check that top-0 survived into the built app.css.",
  ).toBeGreaterThanOrEqual(0);
  expect(
    box.y,
    `the notice rendered at y=${box.y}, below the fold`,
  ).toBeLessThan(viewport.height);

  // The offset itself, not only the outcome: a notice that happens to be in
  // view because the page is short would pass the bounds check alone.
  const inset = await notice.evaluate((el) => {
    const style = getComputedStyle(el);
    return { position: style.position, top: style.top };
  });
  expect(
    inset,
    "the notice is sticky with no resolved top offset, so it is inert and will scroll away",
  ).not.toMatchObject({ position: "sticky", top: "auto" });
});

test("a save without JavaScript reports that it happened", async ({ page }) => {
  test.skip(
    test.info().project.name !== "chromium-nojs",
    "covered above with JavaScript on",
  );

  await saveFromBelowTheFold(page);

  const notice = noticeOf(page);
  await expect(notice).toBeAttached();
  await expect
    .poll(async () => ((await notice.textContent()) ?? "").trim().length > 0, {
      message: "the no-JavaScript save path renders no confirmation notice",
    })
    .toBe(true);

  // Also in the title: a live region filled at page load is not reliably announced.
  await expect(page).toHaveTitle(/^Draft saved — /);
});

// SC 1.4.10 Reflow: 320px is 400% zoom on a 1280px desktop. axe does not test it.
test.describe("at 320px wide", () => {
  test.use({ storageState: RDU_STATE, viewport: { width: 320, height: 640 } });

  test("a table scrolls inside its region, not the page", async ({ page }) => {
    await page.goto("/depositors");

    const region = page.getByRole("region", { name: "Accounts", exact: true });
    const [regionScroll, regionClient] = await region.evaluate((el) => [
      el.scrollWidth,
      el.clientWidth,
    ]);
    expect(
      regionScroll,
      "the accounts table fits at 320px, so this no longer exercises its scroll region",
    ).toBeGreaterThan(regionClient);

    const [pageScroll, pageClient] = await page.evaluate(() => [
      document.documentElement.scrollWidth,
      document.documentElement.clientWidth,
    ]);
    expect(
      pageScroll,
      "the page scrolls sideways: an sr-only header escapes the table's scroll region; " +
        "check `relative` on .data-table-scroll in modules/mosaic/tiles/src/components/table/table.css",
    ).toBeLessThanOrEqual(pageClient);
  });
});

// SC 2.4.11 Focus Not Obscured: a refused submit fills the status region, and a field the
// reader returns to must not end up underneath it.
test.describe("a refused submit", () => {
  test.use({ storageState: DEPOSITOR_STATE });

  for (const { width, sticky } of [
    { width: 320, sticky: false },
    { width: 1280, sticky: true },
  ]) {
    test(`does not cover a field focused afterwards at ${width}px wide`, async ({
      page,
    }) => {
      test.skip(
        test.info().project.name !== "chromium-js",
        "the status region is only patched in place with JS",
      );

      await page.setViewportSize({ width, height: 800 });

      // A malformed link in another section is what the refusal is about; a re-run
      // finds the row already there.
      await page.goto(`/projects/${NOTICE_SHORTCODE}/sections/dataset`);
      const link = page.locator("#additionalMaterial input:not([type=hidden])");
      if ((await link.count()) === 0) {
        await page.locator("#additionalMaterial button.repeatable-add").click();
      }
      await expect(link.first()).toBeVisible();
      await link.first().fill("www.dasch.swiss");
      await page.getByRole("button", { name: "Save draft" }).click();
      await expect(noticeOf(page)).not.toBeEmpty();

      // Submit, then scroll the first field to the top edge of the viewport, where the
      // sticky region sits: what an in-page link or a Shift+Tab from below does.
      await page.goto(`/projects/${NOTICE_SHORTCODE}/sections/overview`);
      await page.getByRole("button", { name: "Submit for review" }).click();
      const status = noticeOf(page);
      await expect(
        status,
        "the submit was not refused, so this exercises nothing",
      ).toContainText("Fields in other sections need changing");

      const field = page
        .locator("#section-form input:not([type=hidden])")
        .first();
      await field.evaluate((el) => {
        el.scrollIntoView({ block: "start" });
        el.focus({ preventScroll: true });
      });

      const position = await status.evaluate(
        (el) => getComputedStyle(el).position,
      );
      expect(position === "sticky", `status position is ${position}`).toBe(
        sticky,
      );

      const fieldBox = await field.boundingBox();
      const region = await status.boundingBox();
      if (!fieldBox || !region) throw new Error("no layout box");
      expect(
        fieldBox.y,
        `the focused field's top is ${fieldBox.y}, under the status region ending at ${region.y + region.height}`,
      ).toBeGreaterThanOrEqual(region.y + region.height);
    });
  }
});
