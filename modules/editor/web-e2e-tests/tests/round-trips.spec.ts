import { expect, type Page, test } from "@playwright/test";

import { DEPOSITOR_STATE, PICKER_SHORTCODE } from "../playwright.config";

test.use({ storageState: DEPOSITOR_STATE });

const ROUND_TRIPS = 3;

const rowsIn = (page: Page, field: string) =>
  page
    .locator(`fieldset#${field} > ol.repeatable-rows > li.repeatable-row`)
    .count();

async function saveDraft(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Save draft" }).click();
  await page.waitForLoadState("networkidle");
}

test("a role group grows no controls and carries no empty value across round trips", async ({
  page,
}) => {
  const url = `/projects/${PICKER_SHORTCODE}/sections/contributors`;
  await page.goto(url);

  const roleControls = page.locator('input[name$=".role"]');
  const before = await roleControls.count();
  const rowsBefore = await rowsIn(page, "attributions");

  for (let i = 0; i < ROUND_TRIPS; i++) {
    await saveDraft(page);
    await page.goto(url);
  }

  // Controls multiplying on every round trip is one of the defects the PR #384
  // review found: the re-render keeps the posted body and appends to it rather
  // than replacing it, so the group grows by one set per save.
  expect(
    await roleControls.count(),
    `role controls multiplied across ${ROUND_TRIPS} saves (${before} → ${await roleControls.count()})`,
  ).toBe(before);
  expect(
    await rowsIn(page, "attributions"),
    "attribution rows multiplied across saves",
  ).toBe(rowsBefore);

  // An empty-valued radio or checkbox renders as an unlabelled control: it is
  // in the tab order, it announces nothing, and it posts nothing.
  const empties = await roleControls.evaluateAll((els) =>
    els
      .filter((e) => {
        const input = e as HTMLInputElement;
        return (
          (input.type === "radio" || input.type === "checkbox") &&
          input.value.trim() === ""
        );
      })
      .map((e) => (e as HTMLInputElement).name),
  );
  expect(
    empties,
    `role controls with an empty value: ${empties.join(", ")}`,
  ).toEqual([]);
});

test("a funder group grows no controls across round trips", async ({
  page,
}) => {
  const url = `/projects/${PICKER_SHORTCODE}/sections/access`;
  await page.goto(url);

  const funderControls = page.locator('[name^="funding."][name*="funder"]');
  const before = await funderControls.count();
  const rowsBefore = await rowsIn(page, "funding");

  for (let i = 0; i < ROUND_TRIPS; i++) {
    await saveDraft(page);
    await page.goto(url);
  }

  expect(
    await funderControls.count(),
    `funder controls multiplied across ${ROUND_TRIPS} saves (was ${before})`,
  ).toBe(before);
  expect(
    await rowsIn(page, "funding"),
    "funding rows multiplied across saves",
  ).toBe(rowsBefore);
});
