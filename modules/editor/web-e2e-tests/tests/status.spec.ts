import { expect, test } from "@playwright/test";

import {
  DEPOSITOR_STATE,
  RDU_STATE,
  STATUS_SHORTCODES,
} from "../playwright.config";

/**
 * The depositor-visible states (REQ-2.1), the waiting-for-release notice
 * (REQ-2.5), the state-explanation page (REQ-2.6) and the forbidden vocabulary
 * (REQ-2.2) — driven through a real browser.
 *
 * `editor-web` already asserts all four against rendered markup, and that is
 * not this file's duplicate. A rendering test reads what one view function
 * returns; only a browser reads the page a server assembles — and the state
 * column is filled from three database reads no view makes. Both passes run it:
 * with JavaScript off, a state that only appeared after a Datastar patch would
 * be missing here.
 */

// One ordered sequence: each step leaves the record in the state the next one
// reads. Serial for the same reason `journey.spec.ts` is.
test.describe.configure({ mode: "serial" });

/**
 * This pass's project. Read per test rather than at module scope, because
 * `test.info()` is only defined inside a running test.
 */
function shortcode() {
  const code = STATUS_SHORTCODES[test.info().project.name];
  if (!code) {
    throw new Error(
      `no status project for Playwright project ${test.info().project.name}; ` +
        "add one to STATUS_SHORTCODES in playwright.config.ts",
    );
  }
  return code;
}

const overview = () => `/projects/${shortcode()}/sections/overview`;
const editedName = () => `Status spec ${shortcode()}`;

/** REQ-2.2. Lowercased; `pull request` is the phrase, not the bare word. */
const FORBIDDEN = ["export", "json", "transfer", "commit", "pull request"];

/**
 * The page's visible text, which is what REQ-2.2 bounds.
 *
 * `innerText` rather than `textContent`: it is the rendered text, so it omits
 * `<script>` bodies and anything hidden. With JavaScript disabled it still
 * works — the property is computed by the browser's layout, not by scripts.
 */
async function visibleText(page: import("@playwright/test").Page) {
  return (await page.locator("body").innerText()).toLowerCase();
}

async function expectNoForbiddenVocabulary(
  page: import("@playwright/test").Page,
  where: string,
) {
  const text = await visibleText(page);
  // The assertion below is an absence, and an absence passes just as happily
  // when the page never rendered. Every current call site happens to assert
  // real content first; this makes the helper safe regardless of the next one.
  expect(
    text.length,
    `REQ-2.2 check read an empty page on ${where}`,
  ).toBeGreaterThan(0);
  const found = FORBIDDEN.filter((word) => text.includes(word));
  expect(found, `REQ-2.2: forbidden words on ${where}`).toEqual([]);
}

/** The "Your changes" cell for the spec's project on the list page. */
async function stateCell(page: import("@playwright/test").Page) {
  await page.goto("/projects");
  const row = page.locator("tr", { hasText: shortcode() });
  return (await row.locator("td").last().innerText()).trim();
}

/**
 * Claim this project's review from the queue.
 *
 * Scoped to the row holding the shortcode, never `.first()`: more than one
 * project can be pending at once, and a `.first()` here would claim whichever
 * happened to sort first — silently reviewing another spec's record.
 */
async function claimReview(page: import("@playwright/test").Page) {
  await page.goto("/review");
  await page
    .locator("tr", { hasText: shortcode() })
    .getByRole("button", { name: /start review/i })
    .click();
  await expect(page).toHaveURL(new RegExp(`/review/${shortcode()}`));
}

test.describe("depositor-visible status", () => {
  test("the state-explanation page names every state and the expected wait", async ({
    browser,
  }) => {
    // REQ-2.6, and the link that makes it findable — a page reachable only by
    // typing its URL explains nothing.
    const context = await browser.newContext({ storageState: DEPOSITOR_STATE });
    try {
      const page = await context.newPage();
      await page.goto("/projects");
      await page
        .getByRole("link", { name: /what do these states mean/i })
        .click();
      await expect(page).toHaveURL(/\/states$/);

      const main = page.locator("main");
      for (const label of [
        "Draft",
        "Submitted",
        "In review",
        "Approved",
        "Online",
      ]) {
        await expect(main, `${label} is not explained`).toContainText(label);
      }
      await expect(main, "REQ-2.6 requires the expected wait").toContainText(
        /few weeks/i,
      );
      await expectNoForbiddenVocabulary(page, "/states");
    } finally {
      await context.close();
    }
  });

  test("a published project with nothing pending reads Online", async ({
    browser,
  }) => {
    // The resting state, and the only place a depositor ever sees the result of
    // REQ-2.4's discard: the record is deleted at startup, so by the time any
    // page loads there is nothing left but this.
    const context = await browser.newContext({ storageState: DEPOSITOR_STATE });
    try {
      const page = await context.newPage();
      await page.goto("/projects");
      await expect(page.locator("main")).toContainText("Your changes");
      expect(await stateCell(page)).toBe("Online");
      await expectNoForbiddenVocabulary(page, "/projects");
    } finally {
      await context.close();
    }
  });

  test("saving a draft moves the project to Draft", async ({ browser }) => {
    const context = await browser.newContext({ storageState: DEPOSITOR_STATE });
    try {
      const page = await context.newPage();
      await page.goto(overview());
      await page.fill('input[name="name"]', editedName());
      await page.getByRole("button", { name: "Save draft" }).click();

      // Polled: the state is read back on a fresh request, and the save's own
      // re-render is not that request.
      await expect
        .poll(() => stateCell(page), {
          message: "a saved draft must take the project out of Online",
        })
        .toBe("Draft");
      await expectNoForbiddenVocabulary(page, "the form after a save");
    } finally {
      await context.close();
    }
  });

  test("submitting moves the project to Submitted", async ({ browser }) => {
    const context = await browser.newContext({ storageState: DEPOSITOR_STATE });
    try {
      const page = await context.newPage();
      await page.goto(overview());
      await page.getByRole("button", { name: "Submit for review" }).click();

      await expect.poll(() => stateCell(page)).toBe("Submitted");
    } finally {
      await context.close();
    }
  });

  test("a claimed review moves the project to In review", async ({
    browser,
  }) => {
    const rdu = await browser.newContext({ storageState: RDU_STATE });
    const depositor = await browser.newContext({
      storageState: DEPOSITOR_STATE,
    });
    try {
      const rduPage = await rdu.newPage();
      await claimReview(rduPage);

      const page = await depositor.newPage();
      await expect
        .poll(() => stateCell(page), {
          message: "claiming a review must be visible to the depositor",
        })
        .toBe("In review");
    } finally {
      await rdu.close();
      await depositor.close();
    }
  });

  test("an approved change reads Approved and says it is waiting for a release", async ({
    browser,
  }) => {
    // REQ-2.5. The record is approved but the published set this deployment
    // carries does not hold it, so it is waiting — and the form stays editable,
    // because approve is the only outcome that does not hand the project back.
    const rdu = await browser.newContext({ storageState: RDU_STATE });
    const depositor = await browser.newContext({
      storageState: DEPOSITOR_STATE,
    });
    try {
      const rduPage = await rdu.newPage();
      await rduPage.goto(`/review/${shortcode()}`);
      const acceptAll = rduPage.getByRole("button", { name: /accept all/i });
      if ((await acceptAll.count()) > 0) {
        await acceptAll.first().click();
      }
      await rduPage
        .getByRole("button", { name: "Approve", exact: true })
        .first()
        .click();
      await rduPage.getByRole("button", { name: "Yes, approve" }).click();

      const page = await depositor.newPage();
      await expect.poll(() => stateCell(page)).toBe("Approved");

      await page.goto(overview());
      const main = page.locator("main");
      await expect(main, "REQ-2.5 notice is missing").toContainText(
        "Waiting for the next release",
      );
      await expect(main, "REQ-2.6 wait is missing from it").toContainText(
        /few weeks/i,
      );
      // Informational, not a lock: the form has to stay editable.
      await expect(
        page.getByRole("button", { name: "Save draft" }),
        "an approval must not lock the form",
      ).toHaveCount(1);

      await expectNoForbiddenVocabulary(page, "the waiting-for-release form");
    } finally {
      await rdu.close();
      await depositor.close();
    }
  });
});
