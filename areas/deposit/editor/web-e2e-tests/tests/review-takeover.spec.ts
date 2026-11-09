import { type Browser, expect, type Page, test } from "@playwright/test";

import {
  RDU_STATE,
  RDU2_STATE,
  TAKEOVER_SHORTCODES,
} from "../playwright.config";
import { isJs } from "./support/agents";
import { perPass } from "./support/per-pass";
import {
  claimReview,
  decision,
  saveDecisions,
  submitAs,
} from "./support/review";

// A retry would rerun the group against the state the failed attempt left.
test.describe.configure({ mode: "serial", retries: 0 });

test.use({ storageState: RDU_STATE });

const shortcode = () => perPass({ TAKEOVER_SHORTCODES });
const review = () => `/review/${shortcode()}`;

const HELD_ELSEWHERE = "Somebody else is reviewing this";
const heldBy = (name: string) => `${name} picked this submission up`;

async function asReviewerB(
  browser: Browser,
  act: (page: Page) => Promise<void>,
) {
  const context = await browser.newContext({ storageState: RDU2_STATE });
  try {
    await act(await context.newPage());
  } finally {
    await context.close();
  }
}

// On a fresh load, no "somebody else" banner; the save control is the premise,
// since an error page holds no banner either.
async function expectHeldByViewer(page: Page) {
  await page.goto(review());
  const main = page.locator("main");
  await expect(
    main.getByRole("button", { name: "Save review decisions" }),
  ).toBeVisible();
  await expect(main).not.toContainText(HELD_ELSEWHERE);
}

test("a second reviewer opening a claimed review is told rdu has it, and rdu is not", async ({
  browser,
  page,
}) => {
  await submitAs(browser, shortcode(), (form) =>
    form.locator('input[name="name"]').fill(`Takeover name ${shortcode()}`),
  );
  await claimReview(page, shortcode());

  await asReviewerB(browser, async (second) => {
    await second.goto(review());
    const main = second.locator("main");
    await expect(main).toContainText(HELD_ELSEWHERE);
    await expect(main).toContainText(heldBy("rdu"));
  });
  await expectHeldByViewer(page);
});

test("after Take over the review, reviewer-b holds it and rdu is told so", async ({
  browser,
  page,
}) => {
  await asReviewerB(browser, async (second) => {
    await second.goto(review());
    await second
      .locator("main")
      .getByRole("button", { name: "Take over the review" })
      .click();
    // A plain post and redirect in both passes, so no "You are now reviewing"
    // notice (DEV-7809): the fresh load is the outcome.
    await expectHeldByViewer(second);
  });

  await page.goto(review());
  await expect(page.locator("main")).toContainText(heldBy("reviewer-b"));
});

test("the decision reviewer-b saves last is the one rdu sees", async ({
  browser,
  page,
}) => {
  await page.goto(review());
  await decision(page, "Name", "Accept").check();
  await saveDecisions(page);

  await asReviewerB(browser, async (second) => {
    await second.goto(review());
    await decision(second, "Name", "Revert").check();
    await saveDecisions(second);
  });

  await page.goto(review());
  await expect(decision(page, "Name", "Revert")).toBeChecked();
});

// Last: rdu takes the review back, so no later test may expect reviewer-b to
// hold it.
test("Take over the review tells the new holder they are now reviewing it", async ({
  page,
}) => {
  test.skip(!isJs(), "without JavaScript the take-over is a plain redirect");
  test.fail(
    true,
    "DEV-7809: the take-over form has no Datastar handler, so the notice never renders",
  );
  await page.goto(review());
  const main = page.locator("main");
  await main.getByRole("button", { name: "Take over the review" }).click();
  await expect(main).toContainText("You are now reviewing this submission.");
});
