import { expect, test } from "@playwright/test";

import { DEPOSITOR_STATE, PICKER_SHORTCODE } from "../playwright.config";
import {
  appendResults,
  cards,
  searchToAppend,
  trimCards,
} from "./support/agents";

test.use({ storageState: DEPOSITOR_STATE });

test.beforeEach(() => {
  test.skip(
    test.info().project.name !== "chromium-js",
    "the Canton select is hidden by Datastar alone; without JavaScript it is always shown",
  );
});

const CONTRIBUTORS = `/projects/${PICKER_SHORTCODE}/sections/contributors`;

test("Canton follows the Country select: hidden for Germany, shown for Switzerland", async ({
  page,
}) => {
  await page.goto(CONTRIBUTORS);
  const before = await cards(page, "attributions").count();

  await searchToAppend(page, "attributions", "organization-");
  const option = appendResults(page, "attributions").locator(
    `button[role="option"][value^="add-agent:attributions:"][value*=":organization-"]`,
  );
  await expect(option.first()).toBeVisible();
  await option.first().click();
  await expect
    .poll(() => cards(page, "attributions").count())
    .toBe(before + 1);

  await cards(page, "attributions")
    .last()
    .getByRole("button", { name: /^Propose changes to this/ })
    .click();
  await expect(page).toHaveURL(/\/entities\/organization-/);

  const country = page.locator('select[name="address.country"]');
  const canton = page.locator('select[name="address.canton"]');

  await country.selectOption("Germany");
  await expect(canton).toBeHidden();
  await country.selectOption("Switzerland");
  await expect(canton).toBeVisible();

  await page.getByRole("button", { name: "Discard this proposal" }).click();
  await page
    .getByRole("button", { name: "Yes, discard this proposal" })
    .click();
  await expect(page).toHaveURL(/\/sections\/contributors\?discarded_proposal=/);
  await trimCards(page, "attributions", before);
});
