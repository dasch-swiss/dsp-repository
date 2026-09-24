import { expect, test } from "@playwright/test";

import { DEPOSITOR_STATE, PICKER_SHORTCODE } from "../playwright.config";

test.use({ storageState: DEPOSITOR_STATE });

const CONTRIBUTORS = `/projects/${PICKER_SHORTCODE}/sections/contributors`;
const QUERY_INPUT = 'input[name="attributions.r0.contributor.q"]';

test("no menu before a search, a real select after one, and the choice survives a reload", async ({
  page,
}) => {
  await page.goto(CONTRIBUTORS);

  // Before the search there must be no menu at all. A picker that renders an
  // empty (or hidden) `<select>` up front is the shape that produces an
  // unlabelled control in the accessibility tree.
  const menus = page.locator('select[name^="attributions.r0.contributor"]');
  expect(await menus.count(), "a menu exists before any search was run").toBe(
    0,
  );

  await page.fill(QUERY_INPUT, "a");
  // The first find-agent button belongs to the first attribution row, which is
  // the row this test drives.
  await page
    .locator('button[name="intent"][value="find-agent"]')
    .first()
    .click();

  // A real `<select>`, not a div playing one: a listbox built from divs is not
  // reachable by keyboard and does not post a value without JavaScript.
  await expect
    .poll(() => menus.count(), { message: "the search produced no <select>" })
    .toBeGreaterThan(0);

  const menu = menus.first();
  const options = await menu.locator("option").count();
  expect(options, "the menu rendered with no options").toBeGreaterThan(0);

  // Pick the first option that carries a real value; the leading one is
  // usually a "choose…" placeholder.
  const value = await menu
    .locator("option")
    .evaluateAll(
      (els) =>
        els.map((e) => (e as HTMLOptionElement).value).find((v) => v !== "") ??
        "",
    );
  expect(
    value,
    "every option carries an empty value, so nothing can be chosen",
  ).not.toBe("");

  await menu.selectOption(value);
  await page.getByRole("button", { name: "Save draft" }).click();

  // The choice has to survive a fresh GET, not just the morph that follows the
  // save — a value held only in the DOM is lost the moment the page reloads.
  await page.goto(CONTRIBUTORS);
  const chosen = page.locator(
    `[name="attributions.r0.contributor"][value="${value}"]`,
  );
  await expect(
    chosen,
    `the chosen agent ${value} did not survive a reload`,
  ).toBeAttached();
});
