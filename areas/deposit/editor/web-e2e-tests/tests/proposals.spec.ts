import { expect, test } from "@playwright/test";

import { DEPOSITOR_STATE, PICKER_SHORTCODE } from "../playwright.config";
import { appendAgent, cards, trimCards } from "./support/agents";

test.use({ storageState: DEPOSITOR_STATE });

const CONTRIBUTORS = `/projects/${PICKER_SHORTCODE}/sections/contributors`;

/** Both passes: without JavaScript the section and the entity form redirect; with it they answer a script that navigates, which only a browser runs. */
test("proposing changes starts on the card and returns to it, and a discard ends there", async ({
  page,
}) => {
  await page.goto(CONTRIBUTORS);
  const before = await cards(page, "attributions").count();
  const added = await appendAgent(page, "attributions");
  const card = `attributions-card-${added.id}`;

  await cards(page, "attributions")
    .last()
    .getByRole("button", { name: /^Propose changes to this/ })
    .click();
  await expect(page).toHaveURL(
    new RegExp(
      `/entities/${added.id}\\?section=contributors&field=attributions$`,
    ),
  );
  const entity = page.url();
  await expect(page.locator("main h1")).toContainText(added.name);
  await expect(
    page.getByRole("button", { name: "Save draft" }),
  ).toBeVisible();

  await page.getByRole("link", { name: "Back to the project" }).click();
  await expect(page).toHaveURL(new RegExp(`/sections/contributors#${card}$`));
  await expect(page.locator(`[id="${card}"]`)).toBeVisible();

  await page.goto(entity);
  await page.getByRole("button", { name: "Discard this proposal" }).click();
  await page
    .getByRole("button", { name: "Yes, discard this proposal" })
    .click();
  await expect(page).toHaveURL(
    new RegExp(`/sections/contributors\\?discarded_proposal=${added.id}$`),
  );
  await expect(
    page.getByText(`Your proposed changes to ${added.name} were discarded.`),
  ).toHaveCount(1);
  await expect(
    page.getByRole("heading", { name: "Proposed persons and organisations" }),
    "the discarded proposal is still listed",
  ).toHaveCount(0);

  await trimCards(page, "attributions", before);
});
