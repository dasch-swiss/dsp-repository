import * as fs from "node:fs";
import * as path from "node:path";
import { expect, test as setup } from "@playwright/test";

import {
  DEPOSITOR_EMAIL,
  DEPOSITOR_SHORTCODES,
  DEPOSITOR_STATE,
  RDU_EMAIL,
  RDU_STATE,
  RDU2_EMAIL,
  RDU2_STATE,
} from "../../playwright.config";
import { signIn } from "./sign-in";

setup(
  "sign in both reviewers and provision the depositor",
  async ({ page, context, browser }) => {
    for (const state of [RDU_STATE, RDU2_STATE, DEPOSITOR_STATE]) {
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

    // --- the second reviewer, also seeded by EDITOR_RDU_EMAILS, in its own context
    const rdu2Context = await browser.newContext();
    try {
      const rdu2Page = await rdu2Context.newPage();
      await signIn(rdu2Page, RDU2_EMAIL);
      // A depositor gets 403 here; a lost session would redirect to /login.
      const queue = await rdu2Page.goto("/review");
      expect(queue?.status(), `${RDU2_EMAIL} is not seeded as RDU`).toBe(200);
      await expect(rdu2Page).toHaveURL(/\/review$/);
      await rdu2Context.storageState({ path: RDU2_STATE });
    } finally {
      await rdu2Context.close();
    }
  },
);
