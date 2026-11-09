import { expect, type Page, test } from "@playwright/test";

import { DECISIONS_SHORTCODES, RDU_STATE } from "../playwright.config";
import { perPass } from "./support/per-pass";
import {
  approve,
  approvedRecord,
  claimReview,
  decision,
  saveDecisions,
  submitAs,
} from "./support/review";

// A retry would rerun the group against the state the failed attempt left.
test.describe.configure({ mode: "serial", retries: 0 });

test.use({ storageState: RDU_STATE });

const shortcode = () => perPass({ DECISIONS_SHORTCODES });
const review = () => `/review/${shortcode()}`;

/** The in-place editor for Name, which posts under the field's own name. */
const nameEditor = (page: Page) =>
  page.locator('#review-form input[name="name"]');

/** Read by the first test, from the depositor's form before it is edited. */
let publishedShortDescription = "";
const substitute = () => `Decisions substitute ${shortcode()}`;

test("the depositor submits a published project with Name and Short description changed", async ({
  browser,
}) => {
  await submitAs(browser, shortcode(), async (form) => {
    const shortDescription = form.locator('[name="shortDescription"]');
    publishedShortDescription = await shortDescription.inputValue();
    await form
      .locator('input[name="name"]')
      .fill(`Decisions name ${shortcode()}`);
    await shortDescription.fill(`Decisions short description ${shortcode()}`);
  });
});

test("Accept on Name and Revert on Short description survive a fresh load", async ({
  page,
}) => {
  await claimReview(page, shortcode());
  await decision(page, "Name", "Accept").check();
  await decision(page, "Short description", "Revert").check();
  await saveDecisions(page);

  await page.goto(review());
  await expect(decision(page, "Name", "Accept")).toBeChecked();
  await expect(decision(page, "Short description", "Revert")).toBeChecked();
});

test("Approve is refused while a change is not reviewed yet, and the project stays in the queue", async ({
  page,
}) => {
  await page.goto(review());
  await decision(page, "Name", "Not reviewed yet").check();
  await saveDecisions(page);

  await approve(page);
  await expect(page.locator("main")).toContainText(
    "Every change has to be decided before this can be approved.",
  );

  await page.goto("/review");
  await expect(
    page
      .getByRole("table", { name: "Pending submissions" })
      .locator("tr", { hasText: shortcode() }),
  ).toBeVisible();
});

test("an in-place edit of the accepted Name is saved with the decisions", async ({
  page,
}) => {
  await page.goto(review());
  await decision(page, "Name", "Accept").check();
  await nameEditor(page).fill(substitute());
  await saveDecisions(page);

  await page.goto(review());
  await expect(decision(page, "Name", "Accept")).toBeChecked();
  await expect(nameEditor(page)).toHaveValue(substitute());
});

test("the approved record holds the substitute for Name and the published Short description", async ({
  page,
  request,
}) => {
  await page.goto(review());
  await approve(page);
  await expect(
    page.getByRole("heading", { name: "Approved", level: 1 }),
  ).toBeVisible();

  const record = await approvedRecord(request, shortcode());
  expect(record?.project.name, "an accepted field ships the substitute").toBe(
    substitute(),
  );
  expect(
    record?.project.shortDescription,
    "a reverted field ships the published value",
  ).toBe(publishedShortDescription);
});
