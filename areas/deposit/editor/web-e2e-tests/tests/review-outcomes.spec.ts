import { type Browser, expect, type Page, test } from "@playwright/test";

import {
  CHANGES_REQUESTED_SHORTCODES,
  DEPOSITOR_STATE,
  RDU_STATE,
  REJECTED_SHORTCODES,
} from "../playwright.config";
import { perPass } from "./support/per-pass";
import { claimReview, decision, stateCell, submitAs } from "./support/review";
import { expectNoForbiddenVocabulary } from "./support/vocabulary";

// A retry would rerun the group against the state the failed attempt left.
test.describe.configure({ mode: "serial", retries: 0 });

test.use({ storageState: RDU_STATE });

const returned = () => perPass({ CHANGES_REQUESTED_SHORTCODES });
const rejected = () => perPass({ REJECTED_SHORTCODES });
const overview = (code: string) => `/projects/${code}/sections/overview`;
const nameInput = (page: Page) => page.locator('input[name="name"]');

const submittedName = (code: string) => `Outcomes name ${code}`;
const changesNote = () => `Please add a longer description ${returned()}`;
const rejectNote = () => `Out of scope for this repository ${rejected()}`;

const NOTE_REQUIRED =
  "A note is required: it is the only thing the depositor will see.";
const ASKED_FOR_CHANGES = "RDU asked for changes";
const REJECTED_NOTICE = "RDU rejected this submission";

/**
 * On a claimed review page, open `verb`'s confirmation, type `note` and
 * confirm. Asserts no outcome: a refusal and a finished round both render here.
 */
async function finishWithNote(page: Page, verb: string, note: string) {
  await page
    .locator("main")
    .getByRole("button", { name: verb, exact: true })
    .click();
  await page.locator('textarea[name="note"]').fill(note);
  await page
    .getByRole("button", { name: `Yes, ${verb.toLowerCase()}` })
    .click();
}

async function expectPending(page: Page, code: string, pending: boolean) {
  await page.goto("/review");
  // With nothing pending the table is not rendered, so a row's absence alone
  // would also fit a page that failed to load.
  await expect(
    page.getByRole("heading", { name: "Review queue" }),
  ).toBeVisible();
  await expect(
    page
      .getByRole("table", { name: "Pending submissions" })
      .locator("tr", { hasText: code }),
  ).toHaveCount(pending ? 1 : 0);
}

async function asDepositor(
  browser: Browser,
  act: (page: Page) => Promise<void>,
) {
  const context = await browser.newContext({ storageState: DEPOSITOR_STATE });
  try {
    await act(await context.newPage());
  } finally {
    await context.close();
  }
}

test("Request changes with a whitespace-only note is refused, and the submission stays in the queue", async ({
  browser,
  page,
}) => {
  await submitAs(browser, returned(), async (form) => {
    await nameInput(form).fill(submittedName(returned()));
    await form
      .locator('[name="shortDescription"]')
      .fill(`Outcomes short description ${returned()}`);
  });
  await claimReview(page, returned());

  await finishWithNote(page, "Request changes", "   ");
  await expect(page.locator("main")).toContainText(NOTE_REQUIRED);

  await expectPending(page, returned(), true);
});

test("Request changes with a note, after accepting Name, returns a Draft with the note on the section page", async ({
  browser,
  page,
}) => {
  await page.goto(`/review/${returned()}`);
  await decision(page, "Name", "Accept").check();
  await finishWithNote(page, "Request changes", changesNote());
  await expect(
    page.getByRole("heading", { name: "Returned to the depositor", level: 1 }),
  ).toBeVisible();

  await asDepositor(browser, async (depositor) => {
    await depositor.goto("/projects");
    await expect(stateCell(depositor, returned())).toHaveText("Draft");
    await depositor.goto(overview(returned()));
    const main = depositor.locator("main");
    await expect(main).toContainText(ASKED_FOR_CHANGES);
    await expect(main).toContainText(changesNote());
  });
});

test("on the returned draft, the accepted Name is read-only text and Short description stays editable", async ({
  browser,
}) => {
  await asDepositor(browser, async (depositor) => {
    await depositor.goto(overview(returned()));
    // The label's parent is the field's own block, which holds its value.
    await expect(depositor.locator("#name-label").locator("..")).toContainText(
      submittedName(returned()),
    );
    await expect(nameInput(depositor)).toHaveCount(0);
    await expect(depositor.locator('[name="shortDescription"]')).toBeEditable();
  });
});

test("Reject with a whitespace-only note is refused with the note-required message", async ({
  browser,
  page,
}) => {
  await submitAs(browser, rejected(), (form) =>
    nameInput(form).fill(submittedName(rejected())),
  );
  await claimReview(page, rejected());

  await finishWithNote(page, "Reject", "   ");
  await expect(page.locator("main")).toContainText(NOTE_REQUIRED);
});

test("Reject with a note leaves the queue, keeps the depositor's draft, and shows the note", async ({
  browser,
  page,
}) => {
  await page.goto(`/review/${rejected()}`);
  await finishWithNote(page, "Reject", rejectNote());
  await expect(
    page.getByRole("heading", { name: "Rejected", level: 1 }),
  ).toBeVisible();
  await expectPending(page, rejected(), false);

  await asDepositor(browser, async (depositor) => {
    await depositor.goto(overview(rejected()));
    const main = depositor.locator("main");
    await expect(main).toContainText(REJECTED_NOTICE);
    await expect(main).toContainText(rejectNote());
    await expect(nameInput(depositor)).toHaveValue(submittedName(rejected()));
  });
});

test("both depositor notices pass the vocabulary check", async ({
  browser,
}) => {
  await asDepositor(browser, async (depositor) => {
    for (const [code, notice] of [
      [returned(), ASKED_FOR_CHANGES],
      [rejected(), REJECTED_NOTICE],
    ]) {
      await depositor.goto(overview(code));
      await expect(depositor.locator("main"), "the premise").toContainText(
        notice,
      );
      await expectNoForbiddenVocabulary(
        depositor,
        `${code}'s ${notice} notice`,
      );
    }
  });
});
