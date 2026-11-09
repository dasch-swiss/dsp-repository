import { expect, type Locator, type Page, test } from "@playwright/test";

import { DEPOSITOR_STATE, PICKER_SHORTCODE } from "../playwright.config";
import {
  cards,
  isJs,
  optionName,
  saveAfter,
  trimCards,
} from "./support/agents";

test.use({ storageState: DEPOSITOR_STATE });

const DATASET = `/projects/${PICKER_SHORTCODE}/sections/dataset`;

/** The field's Add button; the shell's Sign out form sits above `main`. */
const addButton = (page: Page, field: string): Locator =>
  page.locator(`main [formaction$="/fields/${field}/add"]`).first();

/** One row's vocabulary search, found by the key in its box's name so added rows do not move it. */
function pickerOf(page: Page, field: string, key: string) {
  const box = `input[name="${field}.${key}.pick.q"]`;
  const row = cards(page, field).filter({ has: page.locator(box) });
  return {
    row,
    box: row.locator(box),
    results: page.locator(`[id="${field}.${key}.pick-results"]`),
    options: page.locator(
      `[id="${field}.${key}.pick-results"] button[role="option"][name="intent"][value^="pick:${field}.${key}:"]`,
    ),
  };
}

/** Adds a row to `field` and returns its key and how many rows there were before. */
async function addRow(
  page: Page,
  field: string,
): Promise<{ key: string; before: number }> {
  const before = await cards(page, field).count();
  await addButton(page, field).click();
  await expect
    .poll(() => cards(page, field).count(), {
      message: `Add on ${field} did not add a row (was ${before})`,
    })
    .toBe(before + 1);
  const name = await cards(page, field)
    .last()
    .locator('input[name$=".pick.q"]')
    .getAttribute("name");
  const key = name?.match(new RegExp(`^${field}\\.([^.]+)\\.pick\\.q$`))?.[1];
  expect(key, `the new ${field} row has no search box`).toBeTruthy();
  return { key: key as string, before };
}

/**
 * Types `query` into the row's search and waits for its options. With
 * JavaScript the options arrive as the box is typed in; without, they arrive
 * when the Search button beside the box is pressed.
 */
async function searchRow(
  page: Page,
  picker: ReturnType<typeof pickerOf>,
  query: string,
): Promise<void> {
  await picker.box.fill(query);
  if (!isJs()) {
    await picker.row
      .locator('div.combobox button[name="intent"][value="find-entry"]')
      .click();
    await page.waitForLoadState("domcontentloaded");
  }
  await expect(
    picker.options.first(),
    `the search for "${query}" offered no options`,
  ).toBeVisible();
}

/**
 * Switches the row to the branch labelled `label`, revealing its search: autosave with JavaScript,
 * the field's Add without (Save draft would drop the still-empty row).
 */
async function switchBranch(
  page: Page,
  field: string,
  picker: ReturnType<typeof pickerOf>,
  label: string,
): Promise<void> {
  const radio = picker.row.getByLabel(label, { exact: true });
  if (isJs()) {
    await saveAfter(page, () => radio.check());
  } else {
    await radio.check();
    await addButton(page, field).click();
    await page.waitForLoadState("domcontentloaded");
  }
  await expect(
    picker.box,
    `choosing "${label}" did not reveal the row's search`,
  ).toBeVisible();
}

test.describe("the vocabulary pickers", () => {
  // The row a test added, so the draft is left as found even when it fails.
  let added: { field: string; before: number } | undefined;

  test.afterEach(async ({ page }) => {
    if (!added) {
      return;
    }
    const { field, before } = added;
    added = undefined;
    // A fresh load shows what was stored: a blank row is only a rendering.
    await page.goto(DATASET);
    await trimCards(page, field, before);
  });

  test("a place picked by name fills the row and survives a reload", async ({
    page,
  }) => {
    const field = "spatialCoverage";
    await page.goto(DATASET);
    const { key, before } = await addRow(page, field);
    added = { field, before };
    const picker = pickerOf(page, field, key);

    await searchRow(page, picker, "switz");
    const switzerland = picker.options
      .filter({
        has: page.getByText("Switzerland", { exact: true }),
      })
      .first();
    await expect(switzerland, "no option is labelled Switzerland").toBeVisible();
    await switzerland.click();

    await expect(
      picker.row.locator(`input[name="${field}.${key}.ref.label"]`),
      "the pick did not fill Shown as",
    ).toHaveValue("Switzerland");
    await expect(
      picker.row.locator(`input[name="${field}.${key}.ref.url"]`),
      "the pick did not fill Link",
    ).toHaveValue(/geonames\.org\/2658434/);
    await expect(
      picker.box,
      "the search box was not emptied by the pick",
    ).toHaveValue("");

    await page.goto(DATASET);
    await expect(cards(page, field)).toHaveCount(before + 1);
    await expect(
      cards(page, field).last().locator('input[name$=".ref.label"]'),
      "the picked place did not survive a reload",
    ).toHaveValue("Switzerland");
  });

  test("a period picked from the recognised entries fills the row and survives a reload", async ({
    page,
  }) => {
    const field = "temporalCoverage";
    await page.goto(DATASET);
    const { key, before } = await addRow(page, field);
    added = { field, before };
    const picker = pickerOf(page, field, key);

    await switchBranch(page, field, picker, "A recognised entry");
    await searchRow(page, picker, "bronze age");
    const option = picker.options.first();
    const label = await optionName(option);
    await option.click();

    await expect(
      picker.row.locator(`input[name="${field}.${key}.ref.label"]`),
      "the pick did not fill Shown as with the option's label",
    ).toHaveValue(label);
    await expect(
      picker.row.locator(`select[name="${field}.${key}.ref.type"]`),
      "the pick did not set Source to a period source",
    ).toHaveValue(/^(Chronontology|Periodo)$/);

    await page.goto(DATASET);
    await expect(cards(page, field)).toHaveCount(before + 1);
    const last = cards(page, field).last();
    await expect(
      last.locator('input[name$=".ref.label"]'),
      `${label} did not survive a reload`,
    ).toHaveValue(label);
    await expect(
      last.getByLabel("A recognised entry", { exact: true }),
    ).toBeChecked();
  });

  test("a discipline picked from the SNSF list records its code and survives a reload", async ({
    page,
  }) => {
    const field = "disciplines";
    const name = "Visual arts and Art history";
    const chosen = `10404 ${name}`;
    await page.goto(DATASET);
    const { key, before } = await addRow(page, field);
    added = { field, before };
    const picker = pickerOf(page, field, key);

    await switchBranch(page, field, picker, "From the SNSF list");
    await searchRow(page, picker, "art history");
    await expect(
      picker.results.locator('[role="group"]').first(),
      "the options are not grouped under their research domain",
    ).toBeVisible();
    const option = picker.options
      .filter({
        has: page.getByText(name, { exact: true }),
      })
      .first();
    await expect(option, `no option is labelled ${name}`).toBeVisible();
    await option.click();

    await expect(
      picker.row.locator(`input[name="${field}.${key}.snsf"]`),
      "the pick did not record the SNSF code",
    ).toHaveValue("10404");
    await expect(
      picker.row.getByText(chosen),
      "the row does not show the chosen discipline",
    ).toBeVisible();

    await page.goto(DATASET);
    await expect(cards(page, field)).toHaveCount(before + 1);
    const last = cards(page, field).last();
    await expect(
      last.locator('input[name$=".snsf"]'),
      "the SNSF code did not survive a reload",
    ).toHaveValue("10404");
    await expect(last.getByText(chosen)).toBeVisible();
  });
});
