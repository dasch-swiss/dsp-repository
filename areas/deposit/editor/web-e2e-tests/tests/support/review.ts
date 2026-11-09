import {
  type APIRequestContext,
  type Browser,
  expect,
  type Page,
} from "@playwright/test";

import { DEPOSITOR_STATE } from "../../playwright.config";

/**
 * Claim `shortcode`'s review from the queue and land on its review page.
 *
 * Scoped to the row holding the shortcode, never `.first()`: more than one
 * project can be pending at once, and `.first()` would claim whichever sorts
 * first.
 */
export async function claimReview(
  page: Page,
  shortcode: string,
): Promise<void> {
  await page.goto("/review");
  await page
    .locator("tr", { hasText: shortcode })
    .getByRole("button", { name: /start review/i })
    .click();
  await expect(page).toHaveURL(new RegExp(`/review/${shortcode}`));
}

/** The "Your changes" cell for `code` on the depositor's `/projects`, which the caller has loaded. */
export const stateCell = (page: Page, code: string) =>
  page.locator("tr", { hasText: code }).locator("td").last();

/** What a depositor's form says while a submission waits for RDU. */
export const LOCKED = "You can't edit until RDU has looked at it";

/**
 * As the shared depositor, apply `edit` to `shortcode`'s overview, save the
 * draft and submit it, then wait until a fresh load shows the form locked.
 *
 * `edit` has to change something: a draft identical to the published record
 * leaves nothing to submit.
 */
export async function submitAs(
  browser: Browser,
  shortcode: string,
  edit: (page: Page) => Promise<void>,
): Promise<void> {
  const overview = `/projects/${shortcode}/sections/overview`;
  const context = await browser.newContext({ storageState: DEPOSITOR_STATE });
  try {
    const page = await context.newPage();
    await page.goto(overview);
    await edit(page);
    await page.getByRole("button", { name: "Save draft" }).click();
    await page.getByRole("button", { name: "Submit for review" }).click();

    // Polled on fresh loads: with JavaScript the submit is a background post,
    // and closing the context before it lands would drop it. The locked text
    // is asserted too, since a missing button alone also fits an error page.
    await expect(async () => {
      await page.goto(overview);
      await expect(page.locator("main")).toContainText(LOCKED, {
        timeout: 1000,
      });
      await expect(
        page.getByRole("button", { name: "Save draft" }),
        "the form is still editable after Submit for review",
      ).toHaveCount(0, { timeout: 1000 });
    }).toPass({ timeout: 15_000 });
  } finally {
    await context.close();
  }
}

/**
 * On a claimed review page, Approve and its confirmation.
 *
 * Decides no field and asserts no outcome: an approval and a refusal both
 * render on this page, and which one is expected is the caller's to say.
 */
export async function approve(page: Page): Promise<void> {
  await page
    .locator("main")
    .getByRole("button", { name: "Approve", exact: true })
    .click();
  await page.getByRole("button", { name: "Yes, approve" }).click();
}

/** The parts of an approved record the specs read; the route serves more. */
export type ApprovedRecord = {
  shortcode: string;
  project: { name?: string; shortDescription?: string };
  entities: { kind: string; operation: string; id: string }[];
};

/**
 * `shortcode`'s record from `GET /api/v1/approved-records`, read once.
 *
 * Never polled: the route is rate-limited per IP (the burst in
 * `web/src/routes.rs`), both passes share that budget, and a new caller has to
 * fit what the existing callers leave of it.
 */
export async function approvedRecord(
  request: APIRequestContext,
  shortcode: string,
): Promise<ApprovedRecord | undefined> {
  const res = await request.get("/api/v1/approved-records");
  expect(res.ok()).toBe(true);
  const { records } = (await res.json()) as { records: ApprovedRecord[] };
  return records.find((r) => r.shortcode === shortcode);
}
