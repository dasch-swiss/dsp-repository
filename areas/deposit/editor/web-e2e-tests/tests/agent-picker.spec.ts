import { expect, test } from "@playwright/test";

import { DEPOSITOR_STATE, PICKER_SHORTCODE } from "../playwright.config";

test.use({ storageState: DEPOSITOR_STATE });

const CONTRIBUTORS = `/projects/${PICKER_SHORTCODE}/sections/contributors`;
const QUERY_INPUT = 'input[name="attributions.r0.contributor.q"]';
const VALUE_INPUT = 'input[name="attributions.r0.contributor"]';
const CHOOSE =
  'button[name="intent"][value^="choose-agent:attributions.r0.contributor:"]';

test("choosing a match closes the list and survives a reload", async ({
  page,
}) => {
  await page.goto(CONTRIBUTORS);

  const choices = page.locator(CHOOSE);
  expect(await choices.count(), "choose buttons exist before any search").toBe(
    0,
  );

  await page.fill(QUERY_INPUT, "a");
  // The first find-agent button belongs to the first attribution row.
  await page
    .locator('button[name="intent"][value="find-agent"]')
    .first()
    .click();
  await expect
    .poll(() => choices.count(), { message: "the search produced no matches" })
    .toBeGreaterThan(0);

  // A project run earlier may already hold the first match; pick another so the
  // reload assertion cannot pass on a stale value.
  const current = await page.locator(VALUE_INPUT).inputValue();
  const values = await choices.evaluateAll((els) =>
    els.map((e) => (e as HTMLButtonElement).value),
  );
  const index = values.findIndex(
    (v) => v.slice(v.lastIndexOf(":") + 1) !== current,
  );
  expect(
    index,
    "no match differs from the current choice",
  ).toBeGreaterThanOrEqual(0);
  const chosen = values[index].slice(values[index].lastIndexOf(":") + 1);

  await choices.nth(index).click();
  await expect(choices, "the list stayed open after a choice").toHaveCount(0);

  // The plain path saves by redirect, which lands at the top of the page.
  if (test.info().project.name === "chromium-js") {
    await expect(
      page.locator(QUERY_INPUT),
      "focus was not returned to the row's search box",
    ).toBeFocused();
  }

  await page.goto(CONTRIBUTORS);
  await expect(
    page.locator(VALUE_INPUT),
    `the chosen agent ${chosen} did not survive a reload`,
  ).toHaveValue(chosen);
});
