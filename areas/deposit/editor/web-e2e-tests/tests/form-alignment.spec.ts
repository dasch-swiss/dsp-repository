import { expect, type Locator, test } from "@playwright/test";

import { DEPOSITOR_SHORTCODE, DEPOSITOR_STATE } from "../playwright.config";

test.use({ storageState: DEPOSITOR_STATE });

// Read-only on purpose: nothing here edits, blurs or submits, so no draft is touched.
test.beforeEach(() => {
  test.skip(
    test.info().project.name !== "chromium-js",
    "geometry does not depend on JavaScript; one pass measures it",
  );
});

const SECTION = (name: string) =>
  `/projects/${DEPOSITOR_SHORTCODE}/sections/${name}`;

const box = async (locator: Locator) => {
  const b = await locator.boundingBox();
  expect(b, "element has no box").not.toBeNull();
  return b!;
};

const near = (actual: number, expected: number) =>
  expect(Math.abs(actual - expected)).toBeLessThanOrEqual(1);

test("a group's rows start and end where a plain field does", async ({
  page,
}) => {
  await page.goto(SECTION("overview"));

  const field = await box(page.locator('input[name="name"]'));
  const row = await box(page.locator(".repeatable-row").first());

  near(row.x, field.x);
  near(row.x + row.width, field.x + field.width);
});

test("a select's text and chevron sit as far in from their edges as a text field's", async ({
  page,
}) => {
  await page.goto(SECTION("overview"));

  // Measured against the text input's own padding, not a pinned number, so a change to the
  // shared inset moves all three together.
  const text = await page
    .locator('input[name="name"]')
    .evaluate((el) => parseFloat(getComputedStyle(el).paddingLeft));
  const select = await page
    .locator("select.field-select")
    .first()
    .evaluate((el) => {
      const s = getComputedStyle(el);
      return {
        appearance: s.appearance,
        paddingLeft: parseFloat(s.paddingLeft),
        paddingRight: parseFloat(s.paddingRight),
        backgroundPosition: s.backgroundPosition,
        chevronWidth: parseFloat(s.backgroundSize),
      };
    });

  expect(select.appearance).toBe("none");
  expect(select.paddingLeft).toBe(text);

  // Computed as `calc(100% - <inset>) <y>`: the inset is the chevron's gap from the right edge.
  const inset = /^calc\(100% - ([\d.]+)px\)/.exec(select.backgroundPosition);
  expect(inset, select.backgroundPosition).not.toBeNull();
  expect(parseFloat(inset?.[1] ?? "")).toBe(text);

  expect(select.paddingRight).toBeGreaterThanOrEqual(
    text + select.chevronWidth,
  );
});

test("the publications list is named by its fieldset, and only its obligation is shown", async ({
  page,
}) => {
  await page.goto(SECTION("publications"));

  const legend = page.locator("#publications > legend");

  // `sr-only` clips to 1px rather than hiding, so `toBeVisible` would still pass.
  const name = await box(legend.locator(".sr-only"));
  expect(name.width).toBeLessThanOrEqual(1);
  expect(name.height).toBeLessThanOrEqual(1);

  const pill = legend.getByText("Optional", { exact: true });
  await expect(pill).toBeVisible();
  const pillBox = await box(pill);
  expect(pillBox.width).toBeGreaterThan(1);
  expect(pillBox.height).toBeGreaterThan(1);

  await expect(
    page.getByRole("group", { name: /^Publications/ }),
  ).toHaveAttribute("id", "publications");
});

test("a choice group inside a row stays within the row's border and lines up with its other fields", async ({
  page,
}) => {
  await page.goto(SECTION("dataset"));

  const row = page.locator(".repeatable-row:has(.field-group)").first();
  const group = row.locator(".field-group").first();
  const rowBox = await box(row);
  const groupBox = await box(group);
  const BORDER = 1;
  expect(groupBox.x).toBeGreaterThanOrEqual(rowBox.x + BORDER);

  // The group's own left rule is pulled outside its content, so the legend, not the group,
  // is what has to line up with the row's other fields.
  const legend = await box(group.locator("legend"));
  const sibling = await box(
    row.locator(".field:not(.field-group):visible").first(),
  );
  near(legend.x, sibling.x);
});

test("a combobox input and its Search button share one top and one height", async ({
  page,
}) => {
  await page.goto(SECTION("contributors"));

  const combobox = page.locator(".combobox").first();
  const input = await box(combobox.locator("input[role=combobox]"));
  const button = await box(combobox.getByRole("button", { name: "Search" }));

  near(button.y, input.y);
  near(button.height, input.height);
});

test("at 320px a combobox input keeps its minimum width and the page does not scroll sideways", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await page.goto(SECTION("contributors"));

  const combobox = page.locator(".combobox").first();
  const input = await box(combobox.locator("input[role=combobox]"));
  const button = await box(combobox.getByRole("button", { name: "Search" }));

  // 10rem is the input's floor in combobox.css; the button may sit beside the input or below it.
  const floor = await page.evaluate(
    () => 10 * parseFloat(getComputedStyle(document.documentElement).fontSize),
  );
  expect(input.width).toBeGreaterThanOrEqual(floor - 1);
  const beside = Math.abs(button.y - input.y) <= 1;
  const below = button.y >= input.y + input.height;
  expect(beside || below, "Search overlaps the input").toBe(true);

  const overflow = await page.evaluate(
    () =>
      document.documentElement.scrollWidth -
      document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
});
