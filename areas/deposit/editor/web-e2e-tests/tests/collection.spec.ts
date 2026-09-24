import type { APIRequestContext } from "@playwright/test";
import { expect, test } from "@playwright/test";

import {
  COLLECTION_SHORTCODE,
  COLLECTION_TOKEN,
  DEPOSITOR_STATE,
  RDU_STATE,
} from "../playwright.config";

// One arc across two roles and the collection-report API, so the steps share
// one record's state: edit, submit, approve, then report failed / closed /
// merged and discard.
test.describe.configure({ mode: "serial" });

const OVERVIEW = `/projects/${COLLECTION_SHORTCODE}/sections/overview`;
const EDITED_NAME = `Edited by the collection E2E suite ${COLLECTION_SHORTCODE}`;
const PULL_REQUEST = "https://github.com/dasch-swiss/dsp-repository/pull/90210";
const FAILURE_REASON = "GitHub API returned 500 while opening the pull request";

// The approved record's id, read back from the unauthenticated listing and
// cached. `GET /api/v1/approved-records` carries a tight per-IP burst
// (`router.rs`, `APPROVED_RECORDS_BURST`) sized for an occasional CI poller, so
// fetching it once per test trips that limit before the arc finishes.
let cachedRecordId: string | undefined;

async function recordId(request: APIRequestContext): Promise<string> {
  if (cachedRecordId) {
    return cachedRecordId;
  }
  const response = await request.get("/api/v1/approved-records");
  expect(
    response.ok(),
    `approved-records responded ${response.status()}`,
  ).toBeTruthy();
  const body = await response.json();
  const record = (body.records as { id: string; shortcode: string }[]).find(
    (candidate) => candidate.shortcode === COLLECTION_SHORTCODE,
  );
  expect(record, `no approved record for ${COLLECTION_SHORTCODE}`).toBeTruthy();
  cachedRecordId = (record as { id: string }).id;
  return cachedRecordId;
}

// Posts a collection report with the suite's bearer token.
async function reportCollection(
  request: APIRequestContext,
  id: string,
  body: Record<string, unknown>,
) {
  const response = await request.post("/api/v1/collection-report", {
    headers: { Authorization: `Bearer ${COLLECTION_TOKEN}` },
    data: {
      record: id,
      pullRequest: null,
      state: null,
      failure: null,
      ...body,
    },
  });
  expect(
    response.status(),
    `collection-report responded ${response.status()}: ${await response.text()}`,
  ).toBe(204);
}

test.describe("collection: report and discard", () => {
  test("a depositor edits and submits", async ({ browser }) => {
    const context = await browser.newContext({ storageState: DEPOSITOR_STATE });
    try {
      const page = await context.newPage();
      await page.goto(OVERVIEW);
      await page.fill('input[name="name"]', EDITED_NAME);
      await page.getByRole("button", { name: "Save draft" }).click();
      await page.getByRole("button", { name: "Submit for review" }).click();
    } finally {
      await context.close();
    }
  });

  test("RDU finds it in the queue and approves it", async ({ browser }) => {
    const context = await browser.newContext({ storageState: RDU_STATE });
    try {
      const page = await context.newPage();
      await page.goto("/review");
      await page
        .locator("tr", { hasText: COLLECTION_SHORTCODE })
        .getByRole("button", { name: /start review/i })
        .click();
      await expect(page).toHaveURL(
        new RegExp(`/review/${COLLECTION_SHORTCODE}`),
      );

      const acceptAll = page.getByRole("button", { name: /accept all/i });
      if ((await acceptAll.count()) > 0) {
        await acceptAll.first().click();
      }

      await page
        .getByRole("button", { name: "Approve", exact: true })
        .first()
        .click();
      // Approving is a two-step confirmation, deliberately.
      await page.getByRole("button", { name: "Yes, approve" }).click();

      await expect
        .poll(
          async () =>
            ((await page.locator("main").textContent()) ?? "").includes(
              "Approved",
            ),
          {
            message:
              "the record does not read as Approved after the confirmation",
          },
        )
        .toBe(true);
    } finally {
      await context.close();
    }
  });

  test("the surface shows it awaiting collection, never reported on", async ({
    browser,
  }) => {
    const context = await browser.newContext({ storageState: RDU_STATE });
    try {
      const page = await context.newPage();
      await page.goto("/collection");
      const row = page.locator("tr", { hasText: COLLECTION_SHORTCODE });
      await expect(
        row,
        "a differing approved record must read as awaiting collection",
      ).toContainText("Awaiting collection");
      await expect(
        row,
        "a record never reported on must say so, not render a blank cell",
      ).toContainText("Never reported on");
    } finally {
      await context.close();
    }
  });

  test("a reported failure shows on the surface with its reason", async ({
    request,
    browser,
  }) => {
    const id = await recordId(request);
    await reportCollection(request, id, { failure: FAILURE_REASON });

    const context = await browser.newContext({ storageState: RDU_STATE });
    try {
      const page = await context.newPage();
      await page.goto("/collection");
      const row = page.locator("tr", { hasText: COLLECTION_SHORTCODE });
      // A badge alone would not name the reason; the reason text is the thing
      // to assert here.
      await expect(row).toContainText(FAILURE_REASON);
    } finally {
      await context.close();
    }
  });

  test("a closed pull request reads as closed, will retry", async ({
    request,
    browser,
  }) => {
    const id = await recordId(request);
    await reportCollection(request, id, {
      pullRequest: PULL_REQUEST,
      state: "closed",
    });

    const context = await browser.newContext({ storageState: RDU_STATE });
    try {
      const page = await context.newPage();
      await page.goto("/collection");
      const row = page.locator("tr", { hasText: COLLECTION_SHORTCODE });
      await expect(row).toContainText("Closed, will retry");
    } finally {
      await context.close();
    }
  });

  test("a merged pull request over differing data strands the record, and discard removes it", async ({
    request,
    browser,
  }) => {
    const id = await recordId(request);
    // The depositor's edit is what makes this stranded rather than published:
    // the pull request merged, but the published set this deployment carries
    // still holds the unedited project.
    await reportCollection(request, id, {
      pullRequest: PULL_REQUEST,
      state: "merged",
    });

    const context = await browser.newContext({ storageState: RDU_STATE });
    try {
      const page = await context.newPage();
      await page.goto("/collection");
      const row = page.locator("tr", { hasText: COLLECTION_SHORTCODE });
      await expect(row).toContainText("Merged, still differs");

      // A link with a real `href`, not a JS-only control — it has to open with
      // JavaScript off too.
      const discard = row.getByRole("link", { name: "Discard" });
      await expect(discard).toHaveAttribute(
        "href",
        `/collection/${id}/discard`,
      );
      await discard.click();

      await expect(page.locator("main")).toContainText("Discard this record?");
      await page.getByRole("button", { name: "Discard permanently" }).click();

      await expect(page).toHaveURL(/\/collection$/);
      await expect(
        page.locator("tr", { hasText: COLLECTION_SHORTCODE }),
      ).toHaveCount(0);
    } finally {
      await context.close();
    }
  });
});
