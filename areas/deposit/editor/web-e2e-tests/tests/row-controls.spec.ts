import { expect, type Page, test } from "@playwright/test";

import {
  DEPOSITOR_SHORTCODE,
  DEPOSITOR_STATE,
  SWEEP_SHORTCODE,
  TYPED_SHORTCODE,
  VARIANT_SHORTCODE,
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
      // A control missing from the route's shape allowlist answers 404 while the
      // page renders perfectly. Status alone is not enough (see the suite
      // above), but a control that cannot resolve at all is worth naming
      // separately.
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

/**
 * A link-shaped control in the first stored row of each row shape. "Basel" is
 * not a web address, and a draft keeps it as typed all the same.
 */
const TYPED_LINKS = [
  {
    section: "dataset",
    field: "spatialCoverage",
    name: "spatialCoverage.r0.ref.url",
  },
  {
    section: "dataset",
    field: "temporalCoverage",
    name: "temporalCoverage.r0.ref.url",
  },
  { section: "dataset", field: "disciplines", name: "disciplines.r0.ref.url" },
  {
    section: "publications",
    field: "publications",
    name: "publications.r0.pid",
  },
  { section: "access", field: "funding", name: "funding.r0.url" },
];

const TYPED = "Basel";

test.describe("a typed value survives the row controls", () => {
  for (const { section, field, name } of TYPED_LINKS) {
    test(`${field}: Add and Remove keep what was typed`, async ({ page }) => {
      await page.goto(sectionUrl(TYPED_SHORTCODE, section));
      const before = await rowCount(page, field);
      const control = page.locator(`[name="${name}"]`);
      await control.fill(TYPED);

      // The body tells "never posted" from "posted, then lost on the way back".
      const posted = page.waitForRequest(
        (request) =>
          request.method() === "POST" &&
          request.url().endsWith(`/fields/${field}/add`),
      );
      await page
        .locator(`[formaction$="/fields/${field}/add"]`)
        .first()
        .click();
      const body = new URLSearchParams((await posted).postData() ?? "");
      expect(body.get(name), `${name} was not posted by Add`).toBe(TYPED);
      // The count first: until the re-render lands, the control still shows
      // what was typed into it.
      await expect.poll(() => rowCount(page, field)).toBe(before + 1);
      await expect(
        control,
        `${name} was posted, then emptied by the re-render`,
      ).toHaveValue(TYPED);

      await page
        .locator(`[formaction*="/fields/${field}/"][formaction$="/remove"]`)
        .last()
        .click();
      await expect.poll(() => rowCount(page, field)).toBe(before);
      await expect(control, `${name} was emptied by Remove`).toHaveValue(TYPED);

      await page.goto(sectionUrl(TYPED_SHORTCODE, section));
      await expect(
        page.locator(`[name="${name}"]`),
        `${name} did not reach the stored draft`,
      ).toHaveValue(TYPED);
    });
  }
});

test.describe("a variant switch survives the autosave", () => {
  test.beforeEach(() => {
    test.skip(
      test.info().project.name !== "chromium-js",
      "the autosave is a Datastar trigger",
    );
  });

  /** Resolves once the autosave has answered and the region is patched. */
  const autosaved = (page: Page) =>
    expect(page.locator('div[aria-live="polite"]').first()).toContainText(
      "Draft saved",
    );

  test("a new row switched to a recognised entry stays, on that branch", async ({
    page,
  }) => {
    await page.goto(sectionUrl(VARIANT_SHORTCODE, "dataset"));
    const before = await rowCount(page, "disciplines");
    await page.locator('[formaction$="/fields/disciplines/add"]').click();
    await expect.poll(() => rowCount(page, "disciplines")).toBe(before + 1);

    const added = page.locator(
      "fieldset#disciplines > ol.repeatable-rows > li.repeatable-row",
    );
    await added.last().getByLabel("A recognised entry").check();
    await autosaved(page);

    expect(await rowCount(page, "disciplines")).toBe(before + 1);
    await expect(added.last().getByLabel("Link")).toBeVisible();
  });
});
