import { expect, type Locator, type Page, test } from "@playwright/test";

import {
  DEPOSITOR_SHORTCODE,
  DEPOSITOR_STATE,
  SWEEP_SHORTCODE,
  TYPED_SHORTCODE,
  VARIANT_SHORTCODE,
} from "../playwright.config";
import { appendAgent, openRoles } from "./support/agents";

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

/**
 * Every field on this page that renders an Add control, by field name.
 *
 * `attributions` and `contactPoint` render none: their rows are appended by the
 * search under the cards, which `agent-picker.spec.ts` drives, Remove included.
 */
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
      test.skip(
        fields.length === 0,
        `${section} renders no field with an Add control`,
      );

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

test.describe("an \"Add another\" value lands, clears, and can be removed", () => {
  /**
   * Runs `edit`, then waits for the save it leads to. Waits on the response
   * rather than the "Draft saved" text, which an earlier autosave on the page
   * already left up.
   */
  async function saveAfter(page: Page, edit: () => Promise<void>) {
    if (test.info().project.name === "chromium-js") {
      const saved = page.waitForResponse(
        (r) => r.request().method() === "POST",
      );
      await edit();
      await page.keyboard.press("Tab");
      await saved;
    } else {
      await edit();
      await page.getByRole("button", { name: "Save draft" }).click();
      await page.waitForLoadState("domcontentloaded");
    }
  }

  /**
   * Saves from inside `box` without leaving it: the JS pass fires the change
   * event the autosave listens for while the box keeps focus, the no-JS pass
   * clicks "Save draft".
   */
  async function saveFromBox(page: Page, box: Locator) {
    if (test.info().project.name === "chromium-js") {
      const saved = page.waitForResponse(
        (r) => r.request().method() === "POST",
      );
      await box.focus();
      await box.dispatchEvent("change");
      await saved;
    } else {
      await page.getByRole("button", { name: "Save draft" }).click();
      await page.waitForLoadState("domcontentloaded");
    }
  }

  test("a custom data language", async ({ page }) => {
    const add = (p: Page) => p.getByLabel("Add another", { exact: true });
    const hu = (p: Page) => p.locator('input[name="dataLanguage"][value="hu"]');

    await page.goto(sectionUrl(VARIANT_SHORTCODE, "dataset"));
    await expect(hu(page)).toHaveCount(0);
    await saveAfter(page, () => add(page).fill("hu"));

    await expect(add(page)).toHaveValue("");
    await expect(hu(page)).toBeChecked();

    await saveAfter(page, () => hu(page).uncheck());
    await page.goto(sectionUrl(VARIANT_SHORTCODE, "dataset"));
    await expect(hu(page)).toHaveCount(0);
  });

  test("typing a language that is already held still clears the box", async ({
    page,
  }) => {
    const add = (p: Page) => p.getByLabel("Add another", { exact: true });
    const hu = (p: Page) => p.locator('input[name="dataLanguage"][value="hu"]');

    await page.goto(sectionUrl(VARIANT_SHORTCODE, "dataset"));
    await saveAfter(page, () => add(page).fill("hu"));
    await expect(hu(page)).toBeChecked();

    // The held list is unchanged, so only the response's own token tells the
    // morph to replace the box rather than patch the typed text in place.
    await add(page).fill("Hungarian");
    await saveFromBox(page, add(page));
    await expect(add(page)).toHaveValue("");
    await expect(hu(page)).toHaveCount(1);
    await expect(hu(page)).toBeChecked();
    // Saved from inside the box, so the replacement keeps the focus it had.
    if (test.info().project.name === "chromium-js") {
      await expect(add(page)).toBeFocused();
    }

    await saveAfter(page, () => hu(page).uncheck());
    await page.goto(sectionUrl(VARIANT_SHORTCODE, "dataset"));
    await expect(hu(page)).toHaveCount(0);
  });

  test("tabbing away from the box leaves the focus where it went", async ({
    page,
  }) => {
    test.skip(
      test.info().project.name !== "chromium-js",
      "focus moves on an autosave only with JavaScript",
    );
    const add = (p: Page) => p.getByLabel("Add another", { exact: true });
    const hu = (p: Page) => p.locator('input[name="dataLanguage"][value="hu"]');

    await page.goto(sectionUrl(VARIANT_SHORTCODE, "dataset"));
    await saveAfter(page, () => add(page).fill("hu"));
    await expect(hu(page)).toBeChecked();

    await add(page).fill("Hungarian");
    const saved = page.waitForResponse((r) => r.request().method() === "POST");
    await page.keyboard.press("Tab");
    await saved;
    await expect(add(page)).toHaveValue("");
    await page.waitForTimeout(300);
    await expect(add(page)).not.toBeFocused();
    expect(
      await page.evaluate(
        () => document.activeElement && document.activeElement !== document.body,
      ),
    ).toBe(true);

    await saveAfter(page, () => hu(page).uncheck());
    await page.goto(sectionUrl(VARIANT_SHORTCODE, "dataset"));
    await expect(hu(page)).toHaveCount(0);
  });

  test("unticking one language while adding another clears the box", async ({
    page,
  }) => {
    const add = (p: Page) => p.getByLabel("Add another", { exact: true });
    const hu = (p: Page) => p.locator('input[name="dataLanguage"][value="hu"]');
    const fi = (p: Page) => p.locator('input[name="dataLanguage"][value="fi"]');

    await page.goto(sectionUrl(VARIANT_SHORTCODE, "dataset"));
    await saveAfter(page, () => add(page).fill("hu"));
    await expect(hu(page)).toBeChecked();

    // One value out and one in: the stored count is the same before and after.
    await saveAfter(page, async () => {
      await hu(page).uncheck();
      await add(page).fill("Finnish");
    });
    await expect(add(page)).toHaveValue("");

    // Without a reload: the checkbox that took hu's slot is not left unticked
    // by hu's untick, or the next save would drop Finnish.
    await expect(hu(page)).toHaveCount(0);
    await expect(fi(page)).toBeChecked();
    await saveAfter(page, () => add(page).fill("Estonian"));
    await expect(fi(page)).toBeChecked();

    await page.goto(sectionUrl(VARIANT_SHORTCODE, "dataset"));
    await expect(fi(page)).toBeChecked();
    await expect(hu(page)).toHaveCount(0);

    await expect(page.locator('input[name="dataLanguage"][value="et"]')).toBeChecked();

    await saveAfter(page, async () => {
      await fi(page).uncheck();
      await page.locator('input[name="dataLanguage"][value="et"]').uncheck();
    });
    await page.goto(sectionUrl(VARIANT_SHORTCODE, "dataset"));
    await expect(fi(page)).toHaveCount(0);
  });

  test("a custom contributor role", async ({ page }) => {
    const row = (p: Page) =>
      p
        .locator(
          "fieldset#attributions > ol.repeatable-rows > li.repeatable-row",
        )
        .first();
    const add = (p: Page) => row(p).getByLabel("Add another role");
    const custom = (p: Page) =>
      row(p).locator('input[type="checkbox"][value="Gardener"]');

    await page.goto(sectionUrl(VARIANT_SHORTCODE, "contributors"));
    if ((await rowCount(page, "attributions")) === 0) {
      await appendAgent(page, "attributions");
    }
    // Closed while the row holds a role, so open it before reaching in.
    await openRoles(row(page));
    await expect(custom(page)).toHaveCount(0);
    await saveAfter(page, () => add(page).fill("Gardener"));

    await expect(add(page)).toHaveValue("");
    await expect(custom(page)).toBeChecked();

    await openRoles(row(page));
    await saveAfter(page, () => custom(page).uncheck());
    await page.goto(sectionUrl(VARIANT_SHORTCODE, "contributors"));
    await expect(custom(page)).toHaveCount(0);
  });
});
