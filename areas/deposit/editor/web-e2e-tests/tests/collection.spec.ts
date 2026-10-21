import { expect, test } from "@playwright/test";

import {
  COLLECTION_SHORTCODE,
  DEPOSITOR_STATE,
  RDU_STATE,
} from "../playwright.config";

// One arc across two roles, so the steps share one record's state: edit,
// submit, approve, then see it waiting for release and discard it.
test.describe.configure({ mode: "serial" });

const OVERVIEW = `/projects/${COLLECTION_SHORTCODE}/sections/overview`;
const EDITED_NAME = `Edited by the collection E2E suite ${COLLECTION_SHORTCODE}`;
// The editor stores the shortcode as written; this one is all digits, so the
// lowercase form the collector's branch name uses is the same string.
const COLLECT_BRANCH = `editor-collect%2F${COLLECTION_SHORTCODE.toLowerCase()}`;

test.describe("collection: waiting for release and discard", () => {
  test("a depositor edits and submits", async ({ browser }) => {
    const context = await browser.newContext({ storageState: DEPOSITOR_STATE });
    try {
      const page = await context.newPage();
      await page.goto(OVERVIEW);
      await page.fill('input[name="name"]', EDITED_NAME);
      await page.getByRole("button", { name: "Save draft" }).click();
      await page.getByRole("button", { name: "Submit for review" }).click();
    } finally {
      await context.close();
    }
  });

  test("RDU finds it in the queue and approves it", async ({ browser }) => {
    const context = await browser.newContext({ storageState: RDU_STATE });
    try {
      const page = await context.newPage();
      await page.goto("/review");
      await page
        .locator("tr", { hasText: COLLECTION_SHORTCODE })
        .getByRole("button", { name: /start review/i })
        .click();
      await expect(page).toHaveURL(
        new RegExp(`/review/${COLLECTION_SHORTCODE}`),
      );

      const acceptAll = page.getByRole("button", { name: /accept all/i });
      if ((await acceptAll.count()) > 0) {
        await acceptAll.first().click();
      }

      await page
        .getByRole("button", { name: "Approve", exact: true })
        .first()
        .click();
      // Approving is a two-step confirmation, deliberately.
      await page.getByRole("button", { name: "Yes, approve" }).click();

      await expect
        .poll(
          async () =>
            ((await page.locator("main").textContent()) ?? "").includes(
              "Approved",
            ),
          {
            message:
              "the record does not read as Approved after the confirmation",
          },
        )
        .toBe(true);
    } finally {
      await context.close();
    }
  });

  test("the surface shows it waiting for release with a link to its pull requests", async ({
    browser,
  }) => {
    const context = await browser.newContext({ storageState: RDU_STATE });
    try {
      const page = await context.newPage();
      await page.goto("/collection");
      const row = page.locator("tr", { hasText: COLLECTION_SHORTCODE });
      await expect(
        row,
        "a differing approved record must read as waiting for release",
      ).toContainText("Waiting for release");
      // Matched on `href`: the link's accessible name is its aria-label, which
      // names the shortcode rather than carrying the visible text.
      await expect(row.locator(`a[href*="${COLLECT_BRANCH}"]`)).toHaveAttribute(
        "href",
        /^https:\/\/github\.com\/dasch-swiss\/dsp-repository\/pulls\?/,
      );
    } finally {
      await context.close();
    }
  });

  test("discard removes the record and leaves its pull request alone", async ({
    browser,
  }) => {
    const context = await browser.newContext({ storageState: RDU_STATE });
    try {
      const page = await context.newPage();
      await page.goto("/collection");
      const row = page.locator("tr", { hasText: COLLECTION_SHORTCODE });

      // A link with a real `href`, not a JS-only control: it has to open with
      // JavaScript off too.
      const discard = row.getByRole("link", { name: "Discard" });
      await expect(discard).toHaveAttribute(
        "href",
        /^\/collection\/[^/]+\/discard$/,
      );
      await discard.click();

      const main = page.locator("main");
      await expect(main.locator(`a[href*="${COLLECT_BRANCH}"]`)).toHaveCount(1);
      await main.getByRole("button", { name: /discard/i }).click();

      await expect(page).toHaveURL(/\/collection$/);
      await expect(
        page.locator("tr", { hasText: COLLECTION_SHORTCODE }),
      ).toHaveCount(0);
    } finally {
      await context.close();
    }
  });
});
