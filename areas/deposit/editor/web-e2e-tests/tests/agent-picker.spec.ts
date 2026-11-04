import { expect, type Page, test } from "@playwright/test";

import { DEPOSITOR_STATE, PICKER_SHORTCODE } from "../playwright.config";
import {
  type AgentField,
  appendAgent,
  appendBox,
  appendOptions,
  appendResults,
  cardName,
  cards,
  checkRole,
  isJs,
  optionId,
  optionName,
  searchToAppend,
  trimCards,
} from "./support/agents";

test.use({ storageState: DEPOSITOR_STATE });

const CONTRIBUTORS = `/projects/${PICKER_SHORTCODE}/sections/contributors`;

const FIELDS: AgentField[] = ["attributions", "contactPoint"];

const NO_ROLE = "Choose at least one role";

/**
 * Every test appends to the picker project's draft and removes what it added
 * before it ends, so none depends on what an earlier one left. A test that
 * fails midway leaves its card behind; the count-relative assertions still hold.
 */

for (const field of FIELDS) {
  test.describe(`${field}: the append search`, () => {
    test("a chosen match becomes a card, empties the box and survives a reload", async ({
      page,
    }) => {
      await page.goto(CONTRIBUTORS);
      const before = await cards(page, field).count();

      const added = await appendAgent(page, field);

      await expect(
        cardName(cards(page, field).last()),
        "the new card does not show the chosen name",
      ).toHaveText(added.name);
      await expect(
        appendBox(page, field),
        "the search box was not emptied by the choice",
      ).toHaveValue("");
      await expect(
        appendOptions(page, field),
        "the list stayed open after a choice",
      ).toHaveCount(0);

      await page.goto(CONTRIBUTORS);
      await expect(cards(page, field)).toHaveCount(before + 1);
      await expect(
        cardName(cards(page, field).last()),
        `${added.name} did not survive a reload`,
      ).toHaveText(added.name);

      await trimCards(page, field, before);
    });

    test("Remove takes a card away again", async ({ page }) => {
      await page.goto(CONTRIBUTORS);
      const before = await cards(page, field).count();
      await appendAgent(page, field);

      await trimCards(page, field, before);

      await page.goto(CONTRIBUTORS);
      await expect(cards(page, field)).toHaveCount(before);
    });

    test("Change on a card swaps its agent", async ({ page }) => {
      await page.goto(CONTRIBUTORS);
      const before = await cards(page, field).count();
      const first = await appendAgent(page, field);

      const card = () => cards(page, field).last();
      await card()
        .locator('button[name="intent"][value^="change-agent:"]')
        .click();
      const box = card().locator('input[role="combobox"]');
      await box.fill("a");
      if (!isJs()) {
        await card()
          .locator('button[name="intent"][value="find-agent"]')
          .click();
        await page.waitForLoadState("domcontentloaded");
      }
      const options = card().locator(
        'button[role="option"][name="intent"][value^="choose-agent:"]',
      );
      await expect(options.first()).toBeVisible();

      // Pick a match that is not the agent the card already holds.
      const ids = await Promise.all(
        (await options.all()).map((option) => optionId(option)),
      );
      const index = ids.findIndex((id) => id !== first.id);
      expect(index, "no match differs from the card's agent").toBeGreaterThan(
        -1,
      );
      const other = options.nth(index);
      const otherName = await optionName(other);
      await other.click();

      await expect(
        cardName(card()),
        "the card does not show the agent that was chosen",
      ).toHaveText(otherName);
      await expect(card().locator('input[role="combobox"]')).toHaveCount(0);

      await page.goto(CONTRIBUTORS);
      await expect(cards(page, field)).toHaveCount(before + 1);
      await expect(
        cardName(card()),
        "the changed agent did not survive a reload",
      ).toHaveText(otherName);

      await trimCards(page, field, before);
    });
  });
}

test.describe("a contributor's roles", () => {
  const ROLES = ["Data Collector", "Data Curator"];

  const roleBox = (page: Page, role: string) =>
    cards(page, "attributions")
      .last()
      .locator(`input[type="checkbox"][value="${role}"]`);

  test("Remove on a role chip drops that role", async ({ page }) => {
    await page.goto(CONTRIBUTORS);
    const before = await cards(page, "attributions").count();
    await appendAgent(page, "attributions");

    for (const role of ROLES) {
      await checkRole(page, cards(page, "attributions").last(), role);
    }

    await page.goto(CONTRIBUTORS);
    const chips = () =>
      cards(page, "attributions")
        .last()
        .getByRole("list", { name: /^Roles of / });
    for (const role of ROLES) {
      await expect(chips().getByText(role, { exact: true })).toBeVisible();
    }

    const removed = ROLES[0];
    const remove = cards(page, "attributions")
      .last()
      .getByRole("button", {
        name: new RegExp(`^Remove role ${removed} from `),
      });
    await expect(
      remove,
      "the chip's Remove does not carry its role",
    ).toHaveAttribute(
      "value",
      new RegExp(`^remove-role:attributions\\.r\\d+:${removed}$`),
    );
    await remove.click();
    if (!isJs()) {
      await page.waitForLoadState("domcontentloaded");
    }
    await expect(chips().getByText(removed, { exact: true })).toHaveCount(0);

    await page.goto(CONTRIBUTORS);
    await expect(
      chips().getByText(removed, { exact: true }),
      `${removed} came back after a reload`,
    ).toHaveCount(0);
    await expect(chips().getByText(ROLES[1], { exact: true })).toBeVisible();
    await expect(roleBox(page, removed)).not.toBeChecked();

    await trimCards(page, "attributions", before);
  });

  test("a contributor with no role is marked incomplete on a fresh load", async ({
    page,
  }) => {
    await page.goto(CONTRIBUTORS);
    const before = await cards(page, "attributions").count();
    await appendAgent(page, "attributions");

    // The response to the choice that added the card does not scold; only the
    // JavaScript pass renders that response in place.
    if (isJs()) {
      await expect(
        cards(page, "attributions").last().getByText(NO_ROLE),
      ).toHaveCount(0);
    }

    await page.goto(CONTRIBUTORS);
    await expect(
      cards(page, "attributions").last().getByText(NO_ROLE),
    ).toBeVisible();

    // Choosing a role clears it.
    await checkRole(page, cards(page, "attributions").last(), ROLES[0]);
    await page.goto(CONTRIBUTORS);
    await expect(
      cards(page, "attributions").last().getByText(NO_ROLE),
    ).toHaveCount(0);

    await trimCards(page, "attributions", before);
  });
});

test.describe("keyboard use of the append search (JavaScript only)", () => {
  test.beforeEach(() => {
    test.skip(
      !isJs(),
      "the arrow-key script is JavaScript; without it Tab is the way in",
    );
  });

  const field: AgentField = "attributions";

  test("the arrow keys move focus along the options and save nothing", async ({
    page,
  }) => {
    const added: string[] = [];
    page.on("request", (request) => {
      if (
        request.method() === "POST" &&
        request.postData()?.includes("add-agent")
      ) {
        added.push(request.url());
      }
    });

    await page.goto(CONTRIBUTORS);
    const before = await cards(page, field).count();
    await searchToAppend(page, field);
    const options = appendOptions(page, field);
    const count = await options.count();
    expect(count, "the search offered fewer than two options").toBeGreaterThan(
      1,
    );

    const box = appendBox(page, field);
    await box.focus();
    await page.keyboard.press("ArrowDown");
    await expect(options.nth(0)).toBeFocused();
    await page.keyboard.press("ArrowDown");
    await expect(options.nth(1)).toBeFocused();
    await page.keyboard.press("ArrowUp");
    await expect(options.nth(0)).toBeFocused();
    await page.keyboard.press("ArrowUp");
    await expect(
      box,
      "ArrowUp on the first option did not return to the input",
    ).toBeFocused();

    // The last option is a stop: one ArrowDown too many stays on it.
    for (let i = 0; i <= count; i++) {
      await page.keyboard.press("ArrowDown");
    }
    await expect(options.nth(count - 1)).toBeFocused();

    expect(added, "an arrow key posted an add").toEqual([]);
    await page.goto(CONTRIBUTORS);
    await expect(
      cards(page, field),
      "an arrow key changed the card count",
    ).toHaveCount(before);
  });

  test("Escape empties the results and returns to the input", async ({
    page,
  }) => {
    await page.goto(CONTRIBUTORS);
    await searchToAppend(page, field);
    const box = appendBox(page, field);

    await box.focus();
    await page.keyboard.press("Escape");
    await expect(appendOptions(page, field)).toHaveCount(0);
    await expect(appendResults(page, field)).toBeEmpty();
    await expect(box).toBeFocused();

    // From an option rather than the input.
    await box.fill("e");
    await expect(appendOptions(page, field).first()).toBeVisible();
    await box.focus();
    await page.keyboard.press("ArrowDown");
    await expect(appendOptions(page, field).first()).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(appendOptions(page, field)).toHaveCount(0);
    await expect(
      box,
      "focus did not return to the input after Escape on an option",
    ).toBeFocused();
  });

  test("Enter on a focused option adds that card", async ({ page }) => {
    await page.goto(CONTRIBUTORS);
    const before = await cards(page, field).count();
    await searchToAppend(page, field);

    await appendBox(page, field).focus();
    await page.keyboard.press("ArrowDown");
    const option = appendOptions(page, field).first();
    await expect(option).toBeFocused();
    const name = await optionName(option);
    await page.keyboard.press("Enter");

    await expect
      .poll(() => cards(page, field).count(), {
        message: "Enter on an option did not append a card",
      })
      .toBe(before + 1);
    await expect(cardName(cards(page, field).last())).toHaveText(name);

    await trimCards(page, field, before);
  });

  test("Enter in the input searches and adds nothing", async ({ page }) => {
    await page.goto(CONTRIBUTORS);
    await appendAgent(page, field);
    const before = await cards(page, field).count();

    await appendBox(page, field).fill("a");
    await expect(appendOptions(page, field).first()).toBeVisible();
    const posts: string[] = [];
    page.on("request", (request) => {
      if (request.method() === "POST") {
        posts.push(request.url());
      }
    });
    await appendBox(page, field).press("Enter");
    // With the type-ahead on, Enter only keeps the form from submitting: left
    // alone it would press some card's control, the form's first submit button.
    await page.waitForTimeout(1500);
    expect(posts, "Enter in the input posted the form").toEqual([]);
    await expect(appendOptions(page, field).first()).toBeVisible();

    await page.goto(CONTRIBUTORS);
    await expect(cards(page, field)).toHaveCount(before);

    await trimCards(page, field, before - 1);
  });

  test("typing in the append box and leaving it posts no autosave", async ({
    page,
  }) => {
    await page.goto(CONTRIBUTORS);
    const posts: string[] = [];
    page.on("request", (request) => {
      if (request.method() === "POST") {
        posts.push(request.url());
      }
    });

    await appendBox(page, field).fill("a");
    await appendBox(page, field).press("Tab");
    // The form autosaves 1s after a change; a combobox's change is not one.
    await page.waitForTimeout(1500);
    expect(posts, "the append search's change event autosaved").toEqual([]);
  });
});
