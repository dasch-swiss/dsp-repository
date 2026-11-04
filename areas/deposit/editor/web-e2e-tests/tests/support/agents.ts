import { expect, type Locator, type Page, test } from "@playwright/test";

/**
 * The fields of the contributors section that list agents. Each is a list of
 * cards with one search under it that appends a card; neither has an Add button.
 */
export type AgentField = "attributions" | "contactPoint";

export const isJs = () => test.info().project.name === "chromium-js";

/** The cards of one list. The append search sits outside the fieldset. */
export const cards = (page: Page, field: AgentField): Locator =>
  page.locator(`fieldset#${field} > ol.repeatable-rows > li.repeatable-row`);

/** The name a card shows: its first bold run, not a roles chip or a badge. */
export const cardName = (card: Locator): Locator =>
  card.locator('div[role="group"] p span.font-semibold').first();

/** The append search's input, by name: the page renames its id on every reset. */
export const appendBox = (page: Page, field: AgentField): Locator =>
  page.locator(`input[name="${field}.new.q"]`);

/** The append search's results container. Its id holds a dot, so not `#`. */
export const appendResults = (page: Page, field: AgentField): Locator =>
  page.locator(`[id="${field}.new-results"]`);

export const appendOptions = (page: Page, field: AgentField): Locator =>
  appendResults(page, field).locator(
    `button[role="option"][name="intent"][value^="add-agent:${field}:"]`,
  );

/** What an option shows as the agent's name, without its badge or detail line. */
export const optionName = async (option: Locator): Promise<string> =>
  (await option.locator("span.font-semibold").innerText()).trim();

/** The agent id an option's value ends with (`kind:...:{id}`). */
export const optionId = async (option: Locator): Promise<string> => {
  const value = (await option.getAttribute("value")) ?? "";
  return value.slice(value.lastIndexOf(":") + 1);
};

/**
 * Types `query` into the append search of `field` and waits for its options.
 * With JavaScript the options arrive as the box is typed in; without, they
 * arrive when the Search button next to the box is pressed.
 */
export async function searchToAppend(
  page: Page,
  field: AgentField,
  query = "a",
): Promise<void> {
  await appendBox(page, field).fill(query);
  if (!isJs()) {
    await page
      .locator("div.combobox")
      .filter({ has: appendBox(page, field) })
      .locator('button[name="intent"][value="find-agent"]')
      .click();
    await page.waitForLoadState("domcontentloaded");
  }
  await expect(
    appendOptions(page, field).first(),
    `the search for "${query}" offered nothing to append to ${field}`,
  ).toBeVisible();
}

/**
 * Appends one agent to `field` through its search and returns who was added.
 * `index` picks among the options, so a second call can take a different agent.
 */
export async function appendAgent(
  page: Page,
  field: AgentField,
  index = 0,
): Promise<{ id: string; name: string }> {
  const before = await cards(page, field).count();
  await searchToAppend(page, field);
  const option = appendOptions(page, field).nth(index);
  const added = { id: await optionId(option), name: await optionName(option) };
  await option.click();
  await expect
    .poll(() => cards(page, field).count(), {
      message: `choosing ${added.name} did not append a card to ${field} (was ${before})`,
    })
    .toBe(before + 1);
  return added;
}

export async function trimCards(
  page: Page,
  field: AgentField,
  count: number,
): Promise<void> {
  let present = await cards(page, field).count();
  while (present > count) {
    await cards(page, field)
      .last()
      .locator("button.repeatable-remove")
      .click();
    await expect.poll(() => cards(page, field).count()).toBe(present - 1);
    present -= 1;
  }
}

export async function openRoles(card: Locator): Promise<void> {
  const details = card.locator("details").first();
  const open = await details.evaluate((el) => (el as HTMLDetailsElement).open);
  if (!open) {
    await details.locator("summary").click();
  }
}

/**
 * Runs `edit`, then waits for the save it leads to. With JavaScript the edit's
 * change event autosaves and the response is awaited; without, "Save draft" is
 * pressed.
 */
export async function saveAfter(
  page: Page,
  edit: () => Promise<void>,
): Promise<void> {
  if (isJs()) {
    const saved = page.waitForResponse((r) => r.request().method() === "POST");
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
 * Ticks `role` on a card and waits for the save. Opens the disclosure itself:
 * every saving check reloads the page without JavaScript, which closes it again.
 */
export async function checkRole(
  page: Page,
  card: Locator,
  role: string,
): Promise<void> {
  await openRoles(card);
  await saveAfter(page, () =>
    card.locator(`input[type="checkbox"][value="${role}"]`).check(),
  );
}
