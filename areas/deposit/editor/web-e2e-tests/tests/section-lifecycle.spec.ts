import { expect, type Page, test } from "@playwright/test";

import {
  DEPOSITOR_STATE,
  LIFECYCLE_SHORTCODES,
  RDU_STATE,
} from "../playwright.config";
import { perPass } from "./support/per-pass";
import { claimReview, LOCKED, stateCell, submitAs } from "./support/review";
import { expectNoForbiddenVocabulary } from "./support/vocabulary";

// A retry would rerun the group against the state the failed attempt left.
test.describe.configure({ mode: "serial", retries: 0 });

test.use({ storageState: DEPOSITOR_STATE });

const shortcode = () => perPass({ LIFECYCLE_SHORTCODES });
const overview = () => `/projects/${shortcode()}/sections/overview`;
const nameInput = (page: Page) => page.locator('input[name="name"]');
const submittedName = () => `Lifecycle submitted ${shortcode()}`;

/** Read by the first test before any draft exists; the server's database starts empty. */
let published = "";

/** This pass's project's "Your changes" cell, read from a fresh load of `/projects`. */
async function state(page: Page): Promise<string> {
  await page.goto("/projects");
  return (await stateCell(page, shortcode()).innerText()).trim();
}

async function takeBack(page: Page): Promise<void> {
  await page.goto(overview());
  await page.getByRole("button", { name: "Take the submission back" }).click();
  await page.getByRole("button", { name: "Yes, take it back" }).click();
  await expect(page.locator("main")).toContainText("Submission withdrawn");
}

test("Discard draft then Keep the draft leaves the saved value in place", async ({
  page,
}) => {
  const edited = `Lifecycle draft ${shortcode()}`;
  await page.goto(overview());
  published = await nameInput(page).inputValue();
  await nameInput(page).fill(edited);
  await page.getByRole("button", { name: "Save draft" }).click();
  await expect(page.locator("main")).toContainText("Draft saved");

  await page.goto(overview());
  await page.getByRole("button", { name: "Discard draft" }).click();
  await page.getByRole("link", { name: "Keep the draft" }).click();

  // The control is back only once the link's load has landed.
  await expect(
    page.getByRole("button", { name: "Discard draft" }),
  ).toBeVisible();
  await expect(nameInput(page)).toHaveValue(edited);
});

test("Yes, discard the draft restores the published value and the state Online", async ({
  page,
}) => {
  await page.goto(overview());
  await page.getByRole("button", { name: "Discard draft" }).click();
  await page.getByRole("button", { name: "Yes, discard the draft" }).click();
  await expect(page.locator("main")).toContainText("Draft discarded");
  await expectNoForbiddenVocabulary(page, "the form after Discard draft");

  await page.goto(overview());
  await expect(nameInput(page)).toHaveValue(published);
  await expect.poll(() => state(page)).toBe("Online");
});

test("Take the submission back then Keep waiting leaves the form locked and the state Submitted", async ({
  browser,
  page,
}) => {
  await submitAs(browser, shortcode(), (form) =>
    nameInput(form).fill(submittedName()),
  );

  await page.goto(overview());
  await page.getByRole("button", { name: "Take the submission back" }).click();
  await page.getByRole("link", { name: "Keep waiting" }).click();

  // The control is back only once the link's load has landed.
  await expect(
    page.getByRole("button", { name: "Take the submission back" }),
  ).toBeVisible();
  await expect(page.locator("main")).toContainText(LOCKED);
  await expect.poll(() => state(page)).toBe("Submitted");
});

test("Yes, take it back makes the form editable with the submitted value and the state Draft", async ({
  page,
}) => {
  await takeBack(page);
  await expectNoForbiddenVocabulary(page, "the form after taking it back");

  await page.goto(overview());
  await expect(page.getByRole("button", { name: "Save draft" })).toBeVisible();
  await expect(nameInput(page)).toHaveValue(submittedName());
  await expect.poll(() => state(page)).toBe("Draft");
});

test("while In review the form stays locked and still offers Take the submission back", async ({
  browser,
  page,
}) => {
  await submitAs(browser, shortcode(), (form) =>
    nameInput(form).fill(`Lifecycle in review ${shortcode()}`),
  );
  const rdu = await browser.newContext({ storageState: RDU_STATE });
  try {
    await claimReview(await rdu.newPage(), shortcode());
  } finally {
    await rdu.close();
  }

  await page.goto(overview());
  // The claim landed: a Submitted project shows the same controls.
  await expect(page.locator("main")).toContainText(
    "RDU is reviewing this project",
  );
  await expect(
    page.getByRole("button", { name: "Take the submission back" }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Save draft" })).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Submit for review" }),
  ).toHaveCount(0);
});

test("taking it back while In review sets the state Draft and leaves RDU's pending submissions", async ({
  browser,
  page,
}) => {
  const rdu = await browser.newContext({ storageState: RDU_STATE });
  try {
    const queue = await rdu.newPage();
    const pendingRow = () =>
      queue
        .getByRole("table", { name: "Pending submissions" })
        .locator("tr", { hasText: shortcode() });
    await queue.goto("/review");
    await expect(pendingRow(), "the premise: it is pending").toBeVisible();

    await takeBack(page);
    await expect.poll(() => state(page)).toBe("Draft");

    await queue.goto("/review");
    // With nothing pending the table is not rendered, so the row's absence
    // alone would also fit a page that failed to load.
    await expect(
      queue.getByRole("heading", { name: "Review queue" }),
    ).toBeVisible();
    await expect(pendingRow()).toHaveCount(0);
  } finally {
    await rdu.close();
  }
});
