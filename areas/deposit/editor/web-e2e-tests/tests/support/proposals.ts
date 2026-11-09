import { expect, type Locator, type Page } from "@playwright/test";

import { type AgentField, appendResults, searchAppendBox } from "./agents";

/** A search no published agent's name or affiliation contains, which is what offers a proposal. */
const NO_MATCH = "Zzqxj";

/**
 * On a section page, search `field`'s append box for an agent nobody published
 * and press Propose a new `kind` in the empty result. Returns the link the
 * "Proposal started" notice offers to the new proposal's form.
 */
export async function startProposal(
  page: Page,
  field: AgentField,
  kind: "person" | "organisation",
): Promise<Locator> {
  await searchAppendBox(page, field, NO_MATCH);
  await appendResults(page, field)
    .getByRole("button", { name: `Propose a new ${kind}`, exact: true })
    .click();
  return page.getByRole("link", { name: "Open it to fill in the details" });
}

/** Save draft on an entity form, waiting until the save has landed. */
export async function saveProposal(page: Page): Promise<void> {
  // Not any POST: with JavaScript the telemetry beacon posts too.
  const saved = page.waitForResponse(
    (r) => r.request().method() === "POST" && r.url().includes("/entities/"),
  );
  await page.getByRole("button", { name: "Save draft" }).click();
  await saved;
  await page.waitForLoadState("domcontentloaded");
}

/**
 * From `shortcode`'s contributors section, propose a new person through Contact
 * point and save it as Ada `family`, job title Archivist. Returns the
 * proposal's form URL.
 *
 * All three are what check_person asks for: with one missing, the proposal
 * refuses the submit and the approval its callers need.
 */
export async function proposePerson(
  page: Page,
  shortcode: string,
  family: string,
): Promise<string> {
  await page.goto(`/projects/${shortcode}/sections/contributors`);
  const open = await startProposal(page, "contactPoint", "person");
  await open.click();
  await expect(page).toHaveURL(/\/entities\/person-\d+\?/);
  const form = page.url();

  for (const [group, row, value] of [
    ["Given names", "Given name 1", "Ada"],
    ["Family names", "Family name 1", family],
    ["Job titles", "Job title 1", "Archivist"],
  ] as const) {
    await page
      .getByRole("group", { name: new RegExp(`^${group}`) })
      .getByRole("button", { name: "Add" })
      .click();
    await page.getByRole("textbox", { name: row, exact: true }).fill(value);
  }
  await saveProposal(page);
  return form;
}
