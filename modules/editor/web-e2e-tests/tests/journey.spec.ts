import { expect, test } from "@playwright/test";

import {
  DEPOSITOR_STATE,
  JOURNEY_SHORTCODE,
  RDU_STATE,
} from "../playwright.config";

// One ordered journey across two roles, so the steps share the record's state.
test.describe.configure({ mode: "serial" });

const OVERVIEW = `/projects/${JOURNEY_SHORTCODE}/sections/overview`;
const EDITED_NAME = `Edited by the E2E suite ${JOURNEY_SHORTCODE}`;

test.describe("edit, submit, review, approve", () => {
  test("a depositor edits and submits", async ({ browser }) => {
    const context = await browser.newContext({ storageState: DEPOSITOR_STATE });
    try {
      const page = await context.newPage();
      await page.goto(OVERVIEW);

      await page.fill('input[name="name"]', EDITED_NAME);
      await page.getByRole("button", { name: "Save draft" }).click();

      // The edit has to survive a fresh GET, not just the re-render.
      await page.goto(OVERVIEW);
      await expect(page.locator('input[name="name"]')).toHaveValue(EDITED_NAME);

      await page.getByRole("button", { name: "Submit for review" }).click();

      // Submitting locks the form: the name is no longer an editable input.
      await page.goto(OVERVIEW);
      await expect
        .poll(() => page.locator('input[name="name"]').count(), {
          message: "the form is still editable after Submit for review",
        })
        .toBe(0);
    } finally {
      await context.close();
    }
  });

  test("RDU finds it in the queue and approves it", async ({ browser }) => {
    const context = await browser.newContext({ storageState: RDU_STATE });
    try {
      const page = await context.newPage();

      await page.goto("/review");
      const queue = page.locator("main");
      await expect(
        queue,
        "the submission is not in the review queue",
      ).toContainText(JOURNEY_SHORTCODE);

      // The queue's action is a control, not a link — a review is claimed by
      // posting, because starting one changes state. Scoped to this project's
      // row rather than `.first()`: `status.spec.ts` also leaves a submission
      // pending, and whichever sorts first is not necessarily this one.
      await page
        .locator("tr", { hasText: JOURNEY_SHORTCODE })
        .getByRole("button", { name: /start review/i })
        .click();
      await expect(page).toHaveURL(new RegExp(`/review/${JOURNEY_SHORTCODE}`));

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
});
