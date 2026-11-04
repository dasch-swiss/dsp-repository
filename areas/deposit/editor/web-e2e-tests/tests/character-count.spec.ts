import { expect, test } from "@playwright/test";

import { DEPOSITOR_SHORTCODE, DEPOSITOR_STATE } from "../playwright.config";

test.use({ storageState: DEPOSITOR_STATE });

// Unicode scalar values, as the counter counts them; `.length` counts an emoji as two.
const scalars = (text: string) => [...text].length;

const sentence = (remaining: number) => {
  const n = Math.abs(remaining);
  return `You have ${n} ${n === 1 ? "character" : "characters"} ${remaining < 0 ? "too many" : "remaining"}`;
};

test("the short description counter counts characters and never truncates", async ({
  page,
}) => {
  test.skip(
    test.info().project.name !== "chromium-js",
    "the counter is Datastar; without JavaScript it is hidden and only the server check remains",
  );

  await page.goto(`/projects/${DEPOSITOR_SHORTCODE}/sections/overview`);

  const textarea = page.locator('textarea[name="shortDescription"]');
  const counter = page.locator("#shortDescription-counter");
  const visible = counter.locator('[aria-hidden="true"]');
  const live = counter.locator('[aria-live="polite"]');

  const shown = (await visible.textContent()) ?? "";
  const match = shown.match(/^You have (\d+) characters? (remaining|too many)$/);
  expect(match, shown).not.toBeNull();
  const remainingOnLoad = Number(match![1]) * (match![2] === "too many" ? -1 : 1);
  const original = await textarea.inputValue();
  const cap = remainingOnLoad + scalars(original);

  await expect(visible).toHaveText(sentence(remainingOnLoad));
  await expect(visible).toHaveClass(
    remainingOnLoad < 0 ? /field-error/ : /^(?!.*field-error)/,
  );
  await expect(live).toHaveText("");

  // `fill` types without blurring, so no `change` fires and nothing is saved.
  await textarea.fill("a".repeat(cap - 10));
  await expect(visible).toHaveText("You have 10 characters remaining");
  await expect(visible).not.toHaveClass(/field-error/);
  await expect(live).toHaveText("You have 10 characters remaining");

  const over = `\u{1F600}${"a".repeat(cap + 11)}`;
  await textarea.fill(over);
  await expect(visible).toHaveText("You have 12 characters too many");
  await expect(visible).toHaveClass(/field-error/);
  await expect(live).toHaveText("You have 12 characters too many");

  const kept = await textarea.inputValue();
  expect(scalars(kept)).toBe(cap + 12);
  expect(kept).toBe(over);

  const autosaved = () =>
    page.waitForResponse(
      (r) => r.request().method() === "POST" && r.url().includes("/sections/"),
    );

  let saved = autosaved();
  await textarea.fill("abc   ");
  await textarea.press("Tab");
  await saved;
  await expect(visible).toBeVisible();
  await expect(visible).toHaveText(sentence(cap - 3));

  saved = autosaved();
  await textarea.fill(original);
  await textarea.press("Tab");
  await saved;
  await expect(visible).toHaveText(sentence(remainingOnLoad));
});
