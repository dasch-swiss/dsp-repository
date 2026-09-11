import * as fs from "node:fs";
import * as path from "node:path";
import { expect, test as setup } from "@playwright/test";

import {
  DEPOSITOR_EMAIL,
  DEPOSITOR_SHORTCODES,
  DEPOSITOR_STATE,
  RDU_EMAIL,
  RDU_STATE,
} from "../../playwright.config";
import { logOffset, waitForLoginCode } from "./server-log";

/**
 * Drive the real two-step sign-in: post the address, read the code the console
 * mailer wrote to the log, post the code.
 *
 * Deliberately the whole flow rather than a forged session cookie. The session
 * cookie carries the `__Host-` prefix, which requires `Secure`, and whether a
 * browser accepts that over plain `http://127.0.0.1` rests on localhost being a
 * trustworthy origin — forging the jar would skip the one check that answers it.
 *
 * NEVER select a submit control as `button[type="submit"]` on a signed-in page:
 * the shell header renders `<form method="post" action="/logout">` above
 * `<main>`, so the first such button is Sign out. Name the button instead.
 */
export async function signIn(
  page: import("@playwright/test").Page,
  email: string,
): Promise<void> {
  const offset = logOffset();

  await page.goto("/login");
  await page.fill('[name="email"]', email);
  await page.getByRole("button", { name: "Send me a code" }).click();
  await page.waitForURL(/\/login\/code/);

  const code = await waitForLoginCode(offset);
  await page.fill('[autocomplete="one-time-code"]', code);
  // Signed out at this point, so the header carries no Sign out button.
  await page.locator("main").getByRole("button").first().click();

  // The destination differs by role, so assert on having left the login flow
  // rather than on a particular landing page.
  await expect(page).not.toHaveURL(/\/login/);
}

setup(
  "sign in as RDU and provision the depositor",
  async ({ page, context, browser }) => {
    for (const state of [RDU_STATE, DEPOSITOR_STATE]) {
      fs.mkdirSync(path.dirname(state), { recursive: true });
    }

    // --- RDU: seeded by EDITOR_RDU_EMAILS, so it exists at startup ------------
    await signIn(page, RDU_EMAIL);
    await page.goto("/depositors");
    await expect(page.locator("body")).toContainText(/depositor/i);
    await context.storageState({ path: RDU_STATE });

    // --- the depositor: nothing seeds it, RDU creates it through the UI -------
    const alreadyExists = await page
      .getByText(DEPOSITOR_EMAIL, { exact: false })
      .count();
    if (alreadyExists === 0) {
      await page.goto("/depositors/new");
      await page.fill('[name="name"]', "E2E Depositor");
      await page.fill('[name="email"]', DEPOSITOR_EMAIL);
      await page.fill('[name="shortcodes"]', DEPOSITOR_SHORTCODES.join(", "));
      await page.getByRole("button", { name: "Create depositor" }).click();
      await expect(
        page.getByText(DEPOSITOR_EMAIL, { exact: false }).first(),
      ).toBeVisible();
    }

    // --- the depositor's own jar, in a context that shares no cookies with RDU -
    const depositorContext = await browser.newContext();
    try {
      const depositorPage = await depositorContext.newPage();
      await signIn(depositorPage, DEPOSITOR_EMAIL);
      await depositorContext.storageState({ path: DEPOSITOR_STATE });
    } finally {
      await depositorContext.close();
    }
  },
);
