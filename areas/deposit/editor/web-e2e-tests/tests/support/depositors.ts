import { expect, type Locator, type Page, test } from "@playwright/test";

/**
 * An address for a throwaway depositor, unique per pass and per retry.
 *
 * A sign-in, a sign-out or a wrong code spends a budget (the per-address daily
 * send cap, the account lockout) that lives as long as the server, which both
 * passes and every retry share. An account of its own per attempt keeps one
 * attempt's spending from refusing the next. Never use `DEPOSITOR_EMAIL` or
 * `RDU_EMAIL` for such a test.
 */
export function throwawayEmail(purpose: string): string {
  const { project, retry } = test.info();
  return `${purpose}-${project.name}-r${retry}@example.test`;
}

/**
 * As RDU on `page`, create a depositor through `/depositors/new` and wait for
 * its row on `/depositors`.
 *
 * Fails on an address that already has an account rather than reusing it: a
 * reused throwaway carries the spent budgets it exists to avoid. Locally, a
 * server kept by `reuseExistingServer` from an earlier run already holds it;
 * stop that server first.
 */
export async function createDepositor(
  page: Page,
  email: string,
  shortcodes: readonly string[],
): Promise<void> {
  await page.goto("/depositors/new");
  await page.fill('[name="name"]', `E2E ${email.split("@")[0]}`);
  await page.fill('[name="email"]', email);
  await page.fill('[name="shortcodes"]', shortcodes.join(", "));
  await page
    .locator("main")
    .getByRole("button", { name: "Create depositor" })
    .click();

  // Not the URL: the form posts to `/depositors`, so a refusal renders there too.
  await expect(
    depositorRow(page, email),
    `${email} is not listed after Create depositor; the form shows why`,
  ).toHaveCount(1);
}

/** The `/depositors` row of the account with `email`. */
export function depositorRow(page: Page, email: string): Locator {
  return page.locator("tr", { has: page.getByText(email, { exact: true }) });
}
