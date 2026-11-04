import { expect, test } from "@playwright/test";

import { DEPOSITOR_STATE, LANGUAGE_ROWS_SHORTCODE } from "../playwright.config";

test.use({ storageState: DEPOSITOR_STATE });

const OVERVIEW = `/projects/${LANGUAGE_ROWS_SHORTCODE}/sections/overview`;
const LANGUAGE_MISSING = "Choose a language for this line.";

// Clicking Save straight after typing blurs the text box, so the form's debounced autosave fires
// about a second after the save has answered and re-renders the form. The flag must survive it.
test("a line saved with no language keeps its error through the autosave after the save", async ({
  page,
}) => {
  await page.goto(OVERVIEW);
  const field = page.locator("fieldset#alternativeNames");
  await field.locator('[formaction$="/fields/alternativeNames/add"]').click();
  const line = field.locator("li.repeatable-row").last();
  const text = line.locator('input[name$=".text"]');
  await expect(text).toBeVisible();
  await expect(line.locator("select")).toHaveValue("");

  await text.fill("asdasdasd");
  await page
    .locator("main")
    .getByRole("button", { name: "Save draft" })
    .click();

  await expect(field.getByText(LANGUAGE_MISSING)).toBeVisible();
  // Past the autosave's one-second debounce and its round trip.
  await page.waitForTimeout(2500);
  await expect(field.getByText(LANGUAGE_MISSING)).toBeVisible();
  await expect(line.locator("select")).toHaveAttribute("aria-invalid", "true");
  await expect(text).toHaveValue("asdasdasd");
});
