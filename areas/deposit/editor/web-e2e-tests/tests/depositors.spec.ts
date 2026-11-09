import { expect, type Page, test } from "@playwright/test";

import { DEPOSITOR_SHORTCODE, RDU_STATE } from "../playwright.config";
import {
  createDepositor,
  depositorRow,
  throwawayEmail,
} from "./support/depositors";
import { CODE_REJECTED, enterCode } from "./support/sign-in";

// Each test edits or removes a throwaway account of its own, never the shared
// depositor every other spec signs in as.
test.use({ storageState: RDU_STATE });

/** Well-formed, but neither in the published data set nor created by any spec. */
const UNKNOWN_SHORTCODE = "NOSUCH1";

/** Create a throwaway depositor for `purpose` and return its address. */
async function newDepositor(page: Page, purpose: string): Promise<string> {
  const email = throwawayEmail(purpose);
  await createDepositor(page, email, [DEPOSITOR_SHORTCODE]);
  return email;
}

test("Save changes lands on the list with the new name and shortcodes, and the edit form holds them", async ({
  page,
}) => {
  const email = await newDepositor(page, "edit-depositor");
  const name = `Edited ${email.split("@")[0]}`;
  const shortcodes = "0103, 0105";

  await depositorRow(page, email).getByRole("link", { name: "Edit" }).click();
  await page.fill('[name="name"]', name);
  await page.fill('[name="shortcodes"]', shortcodes);
  await page
    .locator("main")
    .getByRole("button", { name: "Save changes" })
    .click();

  await expect(page).toHaveURL(/\/depositors$/);
  const row = depositorRow(page, email);
  await expect(row).toContainText(name);
  await expect(row).toContainText(shortcodes);

  await row.getByRole("link", { name: "Edit" }).click();
  await expect(page.locator('[name="name"]')).toHaveValue(name);
  await expect(page.locator('[name="shortcodes"]')).toHaveValue(shortcodes);
});

test("an unknown shortcode is saved, and the list and the edit form warn about it", async ({
  page,
}) => {
  const email = await newDepositor(page, "unknown-shortcode");
  const warning = `Neither published nor created: ${UNKNOWN_SHORTCODE}.`;

  await depositorRow(page, email).getByRole("link", { name: "Edit" }).click();
  await page.fill('[name="shortcodes"]', UNKNOWN_SHORTCODE);
  await page
    .locator("main")
    .getByRole("button", { name: "Save changes" })
    .click();

  await expect(page).toHaveURL(/\/depositors$/);
  const row = depositorRow(page, email);
  await expect(row).toContainText(warning);

  await row.getByRole("link", { name: "Edit" }).click();
  await expect(page.locator("main")).toContainText(warning);
});

test("cancelling the remove confirmation keeps the account", async ({
  page,
}) => {
  const email = await newDepositor(page, "keep-depositor");

  await depositorRow(page, email).getByRole("link", { name: "Remove" }).click();
  await page.locator("main").getByRole("link", { name: "Cancel" }).click();

  await expect(page).toHaveURL(/\/depositors$/);
  await expect(depositorRow(page, email)).toHaveCount(1);
});

test("a removed account is unlisted and can no longer sign in", async ({
  page,
  browser,
}) => {
  const email = await newDepositor(page, "remove-depositor");

  await depositorRow(page, email).getByRole("link", { name: "Remove" }).click();
  await page
    .locator("main")
    .getByRole("button", { name: "Remove permanently" })
    .click();

  await expect(page).toHaveURL(/\/depositors$/);
  await expect(page.getByRole("heading", { name: "Accounts" })).toBeVisible();
  await expect(depositorRow(page, email)).toHaveCount(0);

  // Empty, or the context would carry this file's RDU session.
  const signedOut = await browser.newContext({
    storageState: { cookies: [], origins: [] },
  });
  try {
    const attempt = await signedOut.newPage();
    await attempt.goto("/login");
    await attempt.fill('[name="email"]', email);
    await attempt.getByRole("button", { name: "Send me a code" }).click();
    await attempt.waitForURL(/\/login\/code/);
    // A live account would be issued a code here, which this deployment reveals.
    await expect(attempt.getByText("No code to show")).toBeVisible();
    // No code was sent to a removed address, so any six digits stand in for one.
    await enterCode(attempt, "000000");

    await expect(attempt.getByText(CODE_REJECTED)).toBeVisible();
    await attempt.goto("/projects");
    await expect(attempt).toHaveURL(/\/login\?next=\/projects$/);
  } finally {
    await signedOut.close();
  }
});
