import { type Browser, expect, type Page, test } from "@playwright/test";

import {
  CREATED_SHORTCODES,
  DEPOSITOR_SHORTCODES,
  DEPOSITOR_STATE,
  RDU_STATE,
  REMOVED_SHORTCODES,
  SUBMITTED_CREATED_SHORTCODES,
} from "../playwright.config";
import { appendBox, appendOptions } from "./support/agents";
import { perPass } from "./support/per-pass";
import { LOCKED } from "./support/review";

test.use({ storageState: RDU_STATE });

/** Published, and assigned to no depositor, so a create that went through would touch no spec's project. */
const PUBLISHED_SHORTCODE = "0813";

async function postNewProject(
  page: Page,
  shortcode: string,
  name: string,
): Promise<void> {
  await page.goto("/projects/new");
  await page.getByLabel("Shortcode", { exact: true }).fill(shortcode);
  await page.getByLabel("Name", { exact: true }).fill(name);
  await page
    .locator("main")
    .getByRole("button", { name: "Create project" })
    .click();
}

async function createProject(page: Page, shortcode: string): Promise<void> {
  await postNewProject(
    page,
    shortcode,
    `Created by the E2E suite ${shortcode}`,
  );
  // A retry finds the project already created; that refusal is the only other outcome.
  if (!page.url().includes(`/projects/${shortcode}/sections/`)) {
    await expect(page.locator("main")).toContainText(
      `Project ${shortcode} has already been created.`,
    );
  }
}

/** `shortcode`'s row under New projects on RDU's `/projects`, which the caller has loaded. */
const createdRow = (page: Page, shortcode: string) =>
  page
    .getByRole("table", { name: "Projects created in the editor" })
    .locator("tr", {
      has: page.getByRole("link", { name: shortcode, exact: true }),
    });

const removeLink = (page: Page, shortcode: string) =>
  page.getByRole("link", { name: `Remove project ${shortcode}` });

async function expectNotCreated(page: Page, shortcode: string): Promise<void> {
  await page.goto("/projects");
  await expect(
    createdRow(page, shortcode),
    `${shortcode} is listed under New projects`,
  ).toHaveCount(0);
}

/**
 * As the shared depositor, answer every field a created project needs before it
 * can be submitted, and submit it.
 *
 * Without JavaScript in both passes: this is setup, and the plain form's round
 * trips save exactly when told to. A hidden branch (the funding note) shows
 * only after a round trip, which the field's Add provides, and the empty row it
 * adds is dropped on the next save.
 */
async function submitComplete(browser: Browser, shortcode: string) {
  const context = await browser.newContext({
    storageState: DEPOSITOR_STATE,
    javaScriptEnabled: false,
  });
  try {
    const page = await context.newPage();
    const main = page.locator("main");
    const open = (section: string) =>
      page.goto(`/projects/${shortcode}/sections/${section}`);
    const add = (field: string) =>
      page.locator(`button[formaction$="/fields/${field}/add"]`).click();
    const save = () => main.getByRole("button", { name: "Save draft" }).click();

    await open("overview");
    // A retry finds the project already submitted, and its form locked.
    if ((await main.getByText(LOCKED).count()) > 0) {
      return;
    }
    await page.fill('[name="shortDescription"]', "A created project.");
    await page.fill('[name="description.en"]', "A created project, submitted.");
    await page.check('[name="status"][value="Ongoing"]');
    await page.fill('[name="startDate"]', "2020-01-01");
    await save();

    await open("dataset");
    for (const field of [
      "keywords",
      "disciplines",
      "temporalCoverage",
      "spatialCoverage",
    ]) {
      await add(field);
    }
    await page.fill('[name="keywords.r0.text"]', "Archive");
    await page.selectOption('[name="keywords.r0.lang"]', "en");
    await page.check('[name="disciplines.r0.kind"][value="text"]');
    await page.fill('[name="disciplines.r0.text.en"]', "History");
    // A label the date table resolves: an unresolved period refuses the submit.
    await page.check('[name="temporalCoverage.r0.kind"][value="text"]');
    await page.fill('[name="temporalCoverage.r0.text.en"]', "16th Century");
    await page.selectOption('[name="spatialCoverage.r0.ref.type"]', "URL");
    await page.fill(
      '[name="spatialCoverage.r0.ref.url"]',
      "https://www.geonames.org/2657896",
    );
    await page.fill('[name="spatialCoverage.r0.ref.label"]', "Zurich");
    await save();

    await open("contributors");
    await appendBox(page, "attributions").fill("a");
    await page
      .locator("div.combobox")
      .filter({ has: appendBox(page, "attributions") })
      .locator('button[name="intent"][value="find-agent"]')
      .click();
    await appendOptions(page, "attributions").first().click();
    await page.check('[name="attributions.r0.role"][value="Project Member"]');
    await save();

    await open("access");
    await page.selectOption(
      '[name="accessRights.accessRights"]',
      "Full Open Access",
    );
    await page.check('[name="funding.kind"][value="text"]');
    await add("funding");
    await page.fill('[name="funding.text"]', "No funding");
    await save();

    await main.getByRole("button", { name: "Submit for review" }).click();
    await expect(
      main,
      "the setup's draft did not pass every submit gate",
    ).toContainText("Submitted for review.");
  } finally {
    await context.close();
  }
}

test("the depositor's list holds its assigned projects and no other", async ({
  browser,
}) => {
  const context = await browser.newContext({ storageState: DEPOSITOR_STATE });
  try {
    const page = await context.newPage();
    await page.goto("/projects");
    const listed = (
      await page
        .getByRole("table", { name: "The projects assigned to your account" })
        .getByRole("link")
        .allInnerTexts()
    ).map((text) => text.trim());

    const assigned: readonly string[] = DEPOSITOR_SHORTCODES;
    // Listed only once RDU has created them, which this run may not have done yet.
    const created = new Set([
      ...Object.values(CREATED_SHORTCODES),
      ...Object.values(SUBMITTED_CREATED_SHORTCODES),
    ]);
    expect(
      listed.filter((code) => !assigned.includes(code)),
      "listed, but not assigned to the depositor",
    ).toEqual([]);
    expect(
      assigned.filter((code) => !created.has(code) && !listed.includes(code)),
      "published and assigned, but not listed",
    ).toEqual([]);
  } finally {
    await context.close();
  }
});

test("a malformed shortcode is refused on the form and creates nothing", async ({
  page,
}) => {
  const malformed = "08-1";
  await postNewProject(page, malformed, "Malformed shortcode");

  await expect(page).toHaveURL(/\/projects\/new$/);
  const shortcode = page.getByLabel("Shortcode", { exact: true });
  await expect(shortcode).toHaveAttribute("aria-invalid", "true");
  // "only": the field's hint also says "letters and digits".
  await expect(shortcode).toHaveAccessibleDescription(
    /only letters and digits/,
  );
  await expectNotCreated(page, malformed);
});

test("a published shortcode is refused and creates nothing", async ({
  page,
}) => {
  await postNewProject(page, PUBLISHED_SHORTCODE, "Published shortcode");

  await expect(page).toHaveURL(/\/projects\/new$/);
  await expect(page.locator("main")).toContainText(
    `Project ${PUBLISHED_SHORTCODE} is already published.`,
  );
  await expectNotCreated(page, PUBLISHED_SHORTCODE);
});

test.describe("removing a created project", () => {
  // The second test removes the project the first one creates.
  test.describe.configure({ mode: "serial" });

  test("Cancel on its remove page keeps it listed", async ({ page }) => {
    const shortcode = perPass({ REMOVED_SHORTCODES });
    await createProject(page, shortcode);

    await page.goto("/projects");
    await removeLink(page, shortcode).click();
    await page.locator("main").getByRole("link", { name: "Cancel" }).click();

    await expect(page).toHaveURL(/\/projects$/);
    await expect(createdRow(page, shortcode)).toHaveCount(1);
  });

  test("Remove project unlists it and its remove page answers 404", async ({
    page,
  }) => {
    const shortcode = perPass({ REMOVED_SHORTCODES });

    await page.goto("/projects");
    await removeLink(page, shortcode).click();
    await page
      .locator("main")
      .getByRole("button", { name: "Remove project" })
      .click();

    await expect(page).toHaveURL((url) => url.pathname === "/projects");
    await expect(page.locator("main")).toContainText(
      `Project ${shortcode} removed.`,
    );
    await expect(createdRow(page, shortcode)).toHaveCount(0);
    const remove = await page.goto(`/projects/${shortcode}/remove`);
    expect(remove?.status()).toBe(404);
  });
});

test("a created project holding a submission shows it is submitted and offers no removal", async ({
  page,
  browser,
}) => {
  const shortcode = perPass({ SUBMITTED_CREATED_SHORTCODES });
  await createProject(page, shortcode);
  await submitComplete(browser, shortcode);

  await page.goto("/projects");
  await expect(createdRow(page, shortcode)).toContainText(
    "Submitted or approved",
  );
  await expect(removeLink(page, shortcode)).toHaveCount(0);
});
