import { expect, type Page, test } from "@playwright/test";

import {
  DEPOSITOR_STATE,
  PROPOSALS_NEW_SHORTCODES,
} from "../playwright.config";
import { perPass } from "./support/per-pass";
import { saveProposal, startProposal } from "./support/proposals";

// A retry would rerun the group against the state the failed attempt left.
test.describe.configure({ mode: "serial", retries: 0 });

test.use({ storageState: DEPOSITOR_STATE });

const shortcode = () => perPass({ PROPOSALS_NEW_SHORTCODES });
const contributors = () => `/projects/${shortcode()}/sections/contributors`;

let personForm = "";

/**
 * Proposes a new `kind` from the contributors append box and follows the link
 * the notice offers.
 */
async function propose(
  page: Page,
  kind: "person" | "organisation",
): Promise<void> {
  await page.goto(contributors());
  const open = await startProposal(page, "attributions", kind);
  await expect(page.locator("main")).toContainText("Proposal started");
  await open.click();
}

/** The entity form `propose` lands on, for `id` (`person`, `organization`). */
const entityForm = (id: string) =>
  new RegExp(`/projects/${shortcode()}/entities/${id}-\\d+\\?`);

/** A string-rows field's group on the entity form, by its legend. */
const group = (page: Page, label: string) =>
  page.getByRole("group", { name: new RegExp(`^${label}`) });

/** The section's panel of this project's live proposals: the heading's parent. */
const proposalsPanel = (page: Page) =>
  page
    .getByRole("heading", { name: "Proposed persons and organisations" })
    .locator("xpath=..");

/** Back to the project from the entity form, then its proposals panel. */
async function backToProposals(page: Page) {
  await page.getByRole("link", { name: "Back to the project" }).click();
  await expect(page).toHaveURL(/\/sections\/contributors/);
  return proposalsPanel(page);
}

test.afterAll(async ({ browser }) => {
  const context = await browser.newContext({ storageState: DEPOSITOR_STATE });
  const page = await context.newPage();
  // Every live proposal, read from the panel rather than tracked, so one a
  // failed test started goes too: a later run against a reused server would
  // otherwise find them already listed.
  await page.goto(contributors());
  const forms = await proposalsPanel(page)
    .getByRole("link")
    .evaluateAll((links) => links.map((a) => a.getAttribute("href") ?? ""));
  for (const form of forms) {
    await page.goto(form);
    await page.getByRole("button", { name: "Discard this proposal" }).click();
    await page
      .getByRole("button", { name: "Yes, discard this proposal" })
      .click();
    await expect(page).toHaveURL(/discarded_proposal=/);
  }
  await context.close();
});

test("a search that matches nothing offers Propose a new person, whose notice links to its form", async ({
  page,
}) => {
  await propose(page, "person");
  await expect(page).toHaveURL(entityForm("person"));
  personForm = page.url();
});

test("Save draft keeps the person's name, and the project lists it under its proposals", async ({
  page,
}) => {
  const given = "Ada";
  const family = "Newperson";
  await page.goto(personForm);
  await group(page, "Given names").getByRole("button", { name: "Add" }).click();
  await page
    .getByRole("textbox", { name: "Given name 1", exact: true })
    .fill(given);
  await group(page, "Family names")
    .getByRole("button", { name: "Add" })
    .click();
  await page
    .getByRole("textbox", { name: "Family name 1", exact: true })
    .fill(family);
  await saveProposal(page);

  await page.goto(personForm);
  await expect(
    page.getByRole("textbox", { name: "Given name 1", exact: true }),
  ).toHaveValue(given);
  await expect(
    page.getByRole("textbox", { name: "Family name 1", exact: true }),
  ).toHaveValue(family);

  await expect(
    (await backToProposals(page)).getByRole("link", {
      name: `${given} ${family}`,
    }),
  ).toBeVisible();
});

test("Add and Remove on the person's Job titles change its row count", async ({
  page,
}) => {
  // Job titles: its Add sits in its own group and each Remove names its row.
  await page.goto(personForm);
  const jobTitles = group(page, "Job titles");
  const rows = jobTitles.getByRole("button", {
    name: /^Remove job title \d+$/,
  });
  const before = await rows.count();

  await jobTitles.getByRole("button", { name: "Add" }).click();
  await expect(rows).toHaveCount(before + 1);

  await jobTitles
    .getByRole("button", {
      name: `Remove job title ${before + 1}`,
      exact: true,
    })
    .click();
  await expect(rows).toHaveCount(before);
});

test("Propose a new organisation: Save draft keeps its name, and the project lists it", async ({
  page,
}) => {
  const name = "Neworganisation";
  await propose(page, "organisation");
  await expect(page).toHaveURL(entityForm("organization"));
  const form = page.url();
  await page.locator('#entity-form [name="name"]').fill(name);
  // An organisation without a URL does not parse, and an unparsed proposal is
  // listed by its id rather than its name.
  await page.locator('#entity-form [name="url"]').fill("https://example.org");
  await saveProposal(page);

  await page.goto(form);
  await expect(page.locator('#entity-form [name="name"]')).toHaveValue(name);

  await expect(
    (await backToProposals(page)).getByRole("link", { name }),
  ).toBeVisible();
});
