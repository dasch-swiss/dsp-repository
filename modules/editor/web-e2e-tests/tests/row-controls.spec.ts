import { expect, type Page, test } from "@playwright/test";

import {
  DEPOSITOR_SHORTCODE,
  DEPOSITOR_STATE,
  SWEEP_SHORTCODE,
} from "../playwright.config";

test.use({ storageState: DEPOSITOR_STATE });

const SECTIONS = [
  "overview",
  "dataset",
  "publications",
  "contributors",
  "access",
  "image",
];

const sectionUrl = (shortcode: string, section: string) =>
  `/projects/${shortcode}/sections/${section}`;

/**
 * How many rows a repeatable field currently renders.
 *
 * Counts `<li class="repeatable-row">` under the field's own `<ol>`, and the
 * child combinators are load-bearing twice over. An empty field still emits a
 * sentinel `<input name="{field}.row" value="">` outside any row, so counting
 * that marker reports one row for a field that has none — which looks exactly
 * like an Add that did nothing. And `funding` nests funder rows inside its own
 * rows, so an unscoped descendant count folds the inner list into the outer.
 *
 * The group is a `<fieldset id="{field}">`; its `role=group` is implicit and
 * therefore not selectable by attribute.
 */
const rowCount = (page: Page, field: string) =>
  page
    .locator(`fieldset#${field} > ol.repeatable-rows > li.repeatable-row`)
    .count();

/** Every field on this page that renders an Add control, by field name. */
async function addableFields(page: Page): Promise<string[]> {
  const actions = await page
    .locator('[formaction$="/add"]')
    .evaluateAll((els) => els.map((e) => e.getAttribute("formaction") ?? ""));
  const fields = actions
    .map((a) => a.match(/\/fields\/([^/]+)\/add$/)?.[1])
    .filter((f): f is string => Boolean(f));
  return [...new Set(fields)];
}

test.describe("row controls change the row count", () => {
  for (const section of SECTIONS) {
    test(`${section}: every Add and Remove does what it says`, async ({
      page,
    }) => {
      await page.goto(sectionUrl(DEPOSITOR_SHORTCODE, section));
      const fields = await addableFields(page);
      test.skip(fields.length === 0, `${section} renders no repeatable fields`);

      for (const field of fields) {
        const before = await rowCount(page, field);

        await page
          .locator(`[formaction$="/fields/${field}/add"]`)
          .first()
          .click();

        // Polled, not read once after `waitForLoadState`: with JavaScript on,
        // Datastar morphs the DOM in place and there is no navigation to wait
        // for, so a single read races and returns the pre-click count.
        //
        // The load-bearing assertion is the count, not the status. Datastar
        // calls preventDefault on the form's submit, and unless the handler
        // posts evt.submitter.formAction the submitter's URL is discarded and
        // every row control becomes a plain save — which answers 200 and
        // changes nothing.
        await expect
          .poll(() => rowCount(page, field), {
            message:
              `${field}: Add resolved but did not add a row (was ${before}). ` +
              "The control is inert — check that data-on:submit posts evt.submitter?.formAction.",
          })
          .toBe(before + 1);

        // And back down again, through the Remove control of the row just added.
        const removes = page.locator(
          `[formaction*="/fields/${field}/"][formaction$="/remove"]`,
        );
        expect(
          await removes.count(),
          `${field}: no Remove control for any of its ${before + 1} rows`,
        ).toBeGreaterThan(0);

        await removes.last().click();
        await expect
          .poll(() => rowCount(page, field), {
            message: `${field}: Remove resolved but did not remove a row. The control is inert.`,
          })
          .toBe(before);
      }
    });
  }
});

test.describe("row controls resolve", () => {
  for (const section of SECTIONS) {
    test(`${section}: no formaction answers 404 or 405`, async ({ page }) => {
      // Four row shapes were once missing from the route's shape allowlist, so
      // eleven of twenty-three controls answered 404 while the page rendered
      // perfectly. Status alone is not enough (see the suite above), but a
      // control that cannot resolve at all is worth naming separately.
      const failures: string[] = [];
      page.on("response", (response) => {
        if (response.status() === 404 || response.status() === 405) {
          failures.push(`${response.status()} ${response.url()}`);
        }
      });

      await page.goto(sectionUrl(SWEEP_SHORTCODE, section));
      const controls = page.locator("[formaction]");
      const total = await controls.count();

      for (let i = 0; i < total; i++) {
        // Re-locate each time: every click re-renders the page.
        const control = page.locator("[formaction]").nth(i);
        if (!(await control.isVisible().catch(() => false))) continue;
        await control.click();
        await page.waitForLoadState("domcontentloaded");
        await page.goto(sectionUrl(SWEEP_SHORTCODE, section));
      }

      expect(
        failures,
        `row controls that did not resolve:\n${failures.join("\n")}`,
      ).toEqual([]);
    });
  }
});
