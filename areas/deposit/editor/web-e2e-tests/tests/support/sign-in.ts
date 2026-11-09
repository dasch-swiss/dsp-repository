import { expect, type Page } from "@playwright/test";

import { logOffset, waitForLoginCode } from "./server-log";

/**
 * Drive the real two-step sign-in: post the address, read the code the console
 * mailer wrote to the log, post the code.
 *
 * Deliberately the whole flow rather than a forged session cookie. The session
 * cookie carries the `__Host-` prefix, which requires `Secure`, and whether a
 * browser accepts that over plain `http://127.0.0.1` rests on localhost being a
 * trustworthy origin — forging the jar would skip the one check that answers it.
 *
 * NEVER select a submit control as `button[type="submit"]` on a signed-in page:
 * the shell header renders `<form method="post" action="/logout">` above
 * `<main>`, so the first such button is Sign out. Name the button instead.
 */
export async function signIn(page: Page, email: string): Promise<void> {
  await page.goto("/login");
  await enterCode(page, await requestCode(page, email));

  // The destination differs by role, so assert on having left the login flow
  // rather than on a particular landing page.
  await expect(page).not.toHaveURL(/\/login/);
}

/** Post `email` on the address form `page` already shows and return the code sent. */
export async function requestCode(page: Page, email: string): Promise<string> {
  const offset = logOffset();
  await page.fill('[name="email"]', email);
  await page.getByRole("button", { name: "Send me a code" }).click();
  await page.waitForURL(/\/login\/code/);
  return waitForLoginCode(offset);
}

/** Enter `code` on the code form. */
export async function enterCode(page: Page, code: string): Promise<void> {
  await page.fill('[autocomplete="one-time-code"]', code);
  // Signed out at this point, so the header carries no Sign out button.
  await page.locator("main").getByRole("button", { name: "Sign in" }).click();
}
