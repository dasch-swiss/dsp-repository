import { type Browser, expect, test } from "@playwright/test";

import {
  BASE_URL,
  DEPOSITOR_SHORTCODE,
  DEPOSITOR_STATE,
  RDU_STATE,
} from "../playwright.config";
import { createDepositor, throwawayEmail } from "./support/depositors";
import { enterCode, requestCode, signIn } from "./support/sign-in";

/**
 * Sign-in, sign-out, the header menus and the error pages, as a browser sees
 * them in both passes.
 *
 * Every test that signs in, signs out or enters a wrong code does it for a
 * throwaway depositor of its own (`throwawayEmail`): a sign-in or a wrong code
 * spends budgets that live as long as the server, and a sign-out with the
 * shared jars would sign every later spec out.
 */

const CODE_REJECTED =
  "That code is not valid, or it has expired. Request a new one.";

/** A throwaway depositor, created by RDU in a context of its own. */
async function throwaway(browser: Browser, purpose: string): Promise<string> {
  const email = throwawayEmail(purpose);
  const rdu = await browser.newContext({ storageState: RDU_STATE });
  try {
    await createDepositor(await rdu.newPage(), email, [DEPOSITOR_SHORTCODE]);
  } finally {
    await rdu.close();
  }
  return email;
}

test("a wrong code is refused and signs nobody in", async ({
  page,
  browser,
}) => {
  const email = await throwaway(browser, "wrong-code");
  await page.goto("/login");
  const code = await requestCode(page, email);
  const wrong = String((Number(code) + 1) % 1_000_000).padStart(6, "0");

  await enterCode(page, wrong);

  await expect(page).toHaveURL(/\/login\/code/);
  await expect(page.getByText(CODE_REJECTED)).toBeVisible();
  await page.goto("/projects");
  await expect(page).toHaveURL(/\/login\?next=\/projects$/);
});

test("signing in from a redirect returns to the page asked for", async ({
  page,
  browser,
}) => {
  const email = await throwaway(browser, "next");
  const section = `/projects/${DEPOSITOR_SHORTCODE}/sections/overview`;

  await page.goto(section);
  await expect(page).toHaveURL(new RegExp(`/login\\?next=${section}$`));
  // The form on the page the guard sent us to, not `signIn()`'s bare `/login`:
  // the destination rides on this form's action.
  const code = await requestCode(page, email);
  await enterCode(page, code);

  await expect(page).toHaveURL(`${BASE_URL}${section}`);
});

test("Sign out signs the browser out", async ({ page, browser }) => {
  const email = await throwaway(browser, "sign-out");
  await signIn(page, email);

  await page.getByRole("button", { name: "Sign out" }).click();

  await expect(page).toHaveURL(/\/login$/);
  await page.goto("/projects");
  await expect(page).toHaveURL(/\/login\?next=\/projects$/);
});

test("each role's header offers its own menu, and each link lands", async ({
  browser,
}) => {
  const depositor = await browser.newContext({ storageState: DEPOSITOR_STATE });
  try {
    const page = await depositor.newPage();
    await page.goto("/projects");
    await expect(
      page.getByRole("navigation", { name: "Main" }).getByRole("link"),
    ).toHaveText(["Projects"]);
  } finally {
    await depositor.close();
  }

  const rdu = await browser.newContext({ storageState: RDU_STATE });
  try {
    const page = await rdu.newPage();
    const menu = page.getByRole("navigation", { name: "Main" });
    await page.goto("/projects");
    await expect(menu.getByRole("link")).toHaveText([
      "Projects",
      /^Review queue/,
      "Approved records",
      "Accounts",
    ]);
    for (const [label, path] of [
      ["Projects", "/projects"],
      ["Review queue", "/review"],
      ["Approved records", "/approved"],
      ["Accounts", "/depositors"],
    ]) {
      // Never from the target itself, where the URL would hold without the click.
      await page.goto(path === "/projects" ? "/review" : "/projects");
      await menu.getByRole("link", { name: label }).click();
      await expect(page).toHaveURL(`${BASE_URL}${path}`);
    }
  } finally {
    await rdu.close();
  }
});

test("the Review queue label counts the pending submissions", async ({
  browser,
}) => {
  const rdu = await browser.newContext({ storageState: RDU_STATE });
  try {
    const page = await rdu.newPage();
    await page.goto("/review");

    const label =
      (
        await page
          .getByRole("navigation", { name: "Main" })
          .getByRole("link", { name: /^Review queue/ })
          .textContent()
      )?.trim() ?? "";
    const count = label.match(/^Review queue \((\d+)\)$/)?.[1];
    expect(count, `the label reads "${label}"`).toBeDefined();
    // The table is absent when nothing is pending, which reads as zero rows.
    await expect(
      page
        .getByRole("table", { name: "Pending submissions" })
        .locator("tbody")
        .getByRole("row"),
    ).toHaveCount(Number(count));
  } finally {
    await rdu.close();
  }
});

test.describe("signed in as the depositor", () => {
  test.use({ storageState: DEPOSITOR_STATE });

  test("/ redirects to the project list", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveURL(/\/projects$/);
  });

  test("the review queue is forbidden and shows no queue", async ({ page }) => {
    const response = await page.goto("/review");

    expect(response?.status()).toBe(403);
    await expect(
      page.getByRole("heading", {
        name: "You do not have access to this page",
      }),
    ).toBeVisible();
    await expect(page.getByRole("table")).toHaveCount(0);
  });
});

test("an unknown path is not found, signed in and signed out", async ({
  browser,
}) => {
  for (const storageState of [DEPOSITOR_STATE, { cookies: [], origins: [] }]) {
    const context = await browser.newContext({ storageState });
    try {
      const page = await context.newPage();
      const response = await page.goto("/no-such-page");

      expect(response?.status()).toBe(404);
      await expect(
        page.getByRole("heading", { name: "Page not found" }),
      ).toBeVisible();
    } finally {
      await context.close();
    }
  }
});
