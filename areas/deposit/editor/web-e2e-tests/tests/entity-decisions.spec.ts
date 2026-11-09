import { expect, test } from "@playwright/test";

import { ENTITY_DECISIONS_SHORTCODES, RDU_STATE } from "../playwright.config";
import { appendResults, cards, searchToAppend } from "./support/agents";
import { perPass } from "./support/per-pass";
import { proposePerson } from "./support/proposals";
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

const shortcode = () => perPass({ ENTITY_DECISIONS_SHORTCODES });
const review = () => `/review/${shortcode()}`;

const family = () => `Entityperson ${shortcode()}`;
/** The proposal's label on the review page, and the name of its decision group. */
const label = () => `Ada ${family()} (person)`;
/** Set by the first test from the entity form's URL; read by the last. */
let personId = "";

test("the depositor submits a project referencing a new person, and RDU's review offers Accept and Reject on it", async ({
  browser,
  page,
}) => {
  await submitAs(browser, shortcode(), async (form) => {
    const proposal = await proposePerson(form, shortcode(), family());
    personId = new URL(proposal).pathname.split("/").pop() ?? "";

    // The picker offers this project's own proposals beside the published agents.
    await form.goto(`/projects/${shortcode()}/sections/contributors`);
    await searchToAppend(form, "contactPoint", family());
    await appendResults(form, "contactPoint")
      .getByRole("option", { name: family() })
      .click();
    await expect(
      cards(form, "contactPoint").filter({ hasText: family() }),
    ).toHaveCount(1);
    await form.goto(`/projects/${shortcode()}/sections/overview`);
  });

  await claimReview(page, shortcode());
  await expect(decision(page, label(), "Accept")).toBeVisible();
  await expect(decision(page, label(), "Reject")).toBeVisible();
});

test("with every field decided and the proposal not reviewed yet, Approve is refused for the undecided proposal", async ({
  page,
}) => {
  await page.goto(review());
  await decision(page, "Contact point", "Accept").check();
  await saveDecisions(page);
  await expect(decision(page, label(), "Not reviewed yet")).toBeChecked();

  await approve(page);
  await expect(page.locator("main")).toContainText(
    "Every proposed person or organisation has to be decided before this can be approved.",
  );
});

test("Reject on the proposal while Contact point references it refuses Approve, naming the field", async ({
  page,
}) => {
  await page.goto(review());
  await decision(page, label(), "Reject").check();
  await saveDecisions(page);

  await approve(page);
  const main = page.locator("main");
  await expect(main).toContainText(
    "This project still refers to a proposed person or organisation that is being rejected.",
  );
  await expect(main).toContainText("Referenced by: Contact point.");
});

test("Accept on the proposal approves, and the approved record carries the person as a new entity", async ({
  page,
  request,
}) => {
  await page.goto(review());
  await decision(page, label(), "Accept").check();
  await saveDecisions(page);

  await approve(page);
  await expect(
    page.getByRole("heading", { name: "Approved", level: 1 }),
  ).toBeVisible();

  const record = await approvedRecord(request, shortcode());
  expect(record, `no approved record for ${shortcode()}`).toBeDefined();
  expect(record?.entities).toContainEqual(
    expect.objectContaining({ kind: "person", operation: "new", id: personId }),
  );
});
