import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

import {
  A11Y_CREATED_SHORTCODE,
  A11Y_REVIEW_SHORTCODE,
  A11Y_STATES_SHORTCODE,
  DEPOSITOR_EMAIL,
  DEPOSITOR_SHORTCODE,
  DEPOSITOR_STATE,
  RDU_STATE,
  RDU2_STATE,
} from "../playwright.config";
import { formatViolations } from "./support/axe-helpers";
import { depositorRow, throwawayEmail } from "./support/depositors";
import { proposePerson } from "./support/proposals";
import { claimReview, submitAs } from "./support/review";

// WCAG 2.1 Level AA — required by EU Directive 2019/882 (EAA) via EN 301 549.
async function expectNoViolations(
  page: import("@playwright/test").Page,
): Promise<void> {
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();

  const count = results.violations.length;
  expect(
    count,
    `${count} accessibility violation(s) found:\n\n${formatViolations(results.violations)}`,
  ).toBe(0);
}

test.describe("signed out", () => {
  test.use({ storageState: { cookies: [], origins: [] } });

  for (const { name, path } of [
    { name: "Sign in", path: "/login" },
    { name: "Not found", path: "/no-such-page" },
  ]) {
    test(`${name} (${path}) has no violations`, async ({ page }) => {
      await page.goto(path);
      await expectNoViolations(page);
    });
  }

  test("Enter the code (/login/code) has no violations", async ({ page }) => {
    // An address with no account: the page renders for any address.
    await page.goto("/login");
    await page.fill('[name="email"]', throwawayEmail("a11y-code"));
    await page.getByRole("button", { name: "Send me a code" }).click();
    await page.waitForURL(/\/login\/code/);
    await expect(
      page.getByRole("heading", { name: "Enter your code", level: 1 }),
    ).toBeVisible();
    await expectNoViolations(page);
  });
});

test.describe("depositor — the form screens", () => {
  test.use({ storageState: DEPOSITOR_STATE });

  const sections = [
    "overview",
    "dataset",
    "publications",
    "contributors",
    "access",
    "image",
  ];

  test("the project list has no violations", async ({ page }) => {
    await page.goto("/projects");
    await expectNoViolations(page);
  });

  // The state-explanation page. A new full-page route is exactly what this
  // sweep exists to cover, and its `<dl>` of state names and descriptions is a
  // structure nothing else in the editor uses.
  test("the state-explanation page has no violations", async ({ page }) => {
    await page.goto("/states");
    await expectNoViolations(page);
  });

  for (const section of sections) {
    test(`the ${section} section has no violations`, async ({ page }) => {
      await page.goto(`/projects/${DEPOSITOR_SHORTCODE}/sections/${section}`);
      await expectNoViolations(page);
    });
  }

  test("the forbidden page has no violations", async ({ page }) => {
    // A depositor reaching an RDU-only route gets a rendered 403, not a bare
    // status — so it is a page, and it is scanned like one.
    await page.goto("/review");
    await expectNoViolations(page);
  });
});

test.describe("RDU — the review screens", () => {
  test.use({ storageState: RDU_STATE });

  for (const { name, path } of [
    { name: "Review queue", path: "/review" },
    { name: "Depositor accounts", path: "/depositors" },
    { name: "New depositor", path: "/depositors/new" },
    { name: "Approved records", path: "/approved" },
    { name: "New project", path: "/projects/new" },
  ]) {
    test(`${name} (${path}) has no violations`, async ({ page }) => {
      await page.goto(path);
      await expectNoViolations(page);
    });
  }

  for (const [action, heading] of [
    ["Edit", /^Edit /],
    ["Remove", /^Remove .+\?$/],
  ] as const) {
    test(`the depositor's ${action} page has no violations`, async ({
      page,
    }) => {
      await page.goto("/depositors");
      await depositorRow(page, DEPOSITOR_EMAIL)
        .getByRole("link", { name: action })
        .click();
      await expect(page).toHaveURL(
        new RegExp(`/depositors/[^/]+/${action.toLowerCase()}$`),
      );
      await expect(
        page.getByRole("heading", { name: heading, level: 1 }),
      ).toBeVisible();
      await expectNoViolations(page);
    });
  }

  test("a created project's remove page has no violations", async ({
    page,
  }) => {
    await page.goto("/projects/new");
    await page
      .getByLabel("Shortcode", { exact: true })
      .fill(A11Y_CREATED_SHORTCODE);
    await page
      .getByLabel("Name", { exact: true })
      .fill(`Accessibility sweep ${A11Y_CREATED_SHORTCODE}`);
    await page
      .locator("main")
      .getByRole("button", { name: "Create project" })
      .click();
    // Created, or on a retry refused as already created: either way it exists.
    await expect(page.locator("main")).toContainText(
      new RegExp(
        `Accessibility sweep ${A11Y_CREATED_SHORTCODE}|Project ${A11Y_CREATED_SHORTCODE} has already been created`,
      ),
    );

    await page.goto("/projects");
    await page
      .getByRole("link", { name: `Remove project ${A11Y_CREATED_SHORTCODE}` })
      .click();
    await expect(
      page.locator("main").getByRole("button", { name: "Remove project" }),
    ).toBeVisible();
    await expectNoViolations(page);
  });
});

test.describe("depositor — the section page in later states", () => {
  // The second test reads the record the first one submitted.
  test.describe.configure({ mode: "serial" });

  const overview = `/projects/${A11Y_STATES_SHORTCODE}/sections/overview`;

  test("the submitted, locked section has no violations", async ({
    browser,
  }) => {
    const context = await browser.newContext({ storageState: DEPOSITOR_STATE });
    try {
      const page = await context.newPage();
      await page.goto(overview);
      // Absent on a re-run or CI retry: already submitted, so edit and submit are skipped.
      const saveDraft = page.getByRole("button", { name: "Save draft" });
      if ((await saveDraft.count()) > 0) {
        await page.fill(
          'input[name="name"]',
          `A11y states ${A11Y_STATES_SHORTCODE}`,
        );
        await saveDraft.click();
        await page.getByRole("button", { name: "Submit for review" }).click();
      }

      // A fresh load, so the scan reads the locked page and not the submit flash.
      await page.goto(overview);
      await expect(page.locator("main")).toContainText(
        "You can't edit until RDU has looked at it",
      );
      await expect(
        page.getByRole("button", { name: "Save draft" }),
        "the form must be locked before it is scanned",
      ).toHaveCount(0);
      await expectNoViolations(page);
    } finally {
      await context.close();
    }
  });

  test("the awaiting-release section has no violations", async ({
    browser,
  }) => {
    const rdu = await browser.newContext({ storageState: RDU_STATE });
    const depositor = await browser.newContext({
      storageState: DEPOSITOR_STATE,
    });
    try {
      const rduPage = await rdu.newPage();
      await rduPage.goto("/review");
      const startReview = rduPage
        .locator("tr", { hasText: A11Y_STATES_SHORTCODE })
        .getByRole("button", { name: /start review/i });
      // Absent on a re-run: already claimed and approved, so claim and approve are skipped.
      if ((await startReview.count()) > 0) {
        // The queue with a Waiting row, before the claim removes it.
        await expectNoViolations(rduPage);
        await startReview.click();
        await expect(rduPage).toHaveURL(
          new RegExp(`/review/${A11Y_STATES_SHORTCODE}`),
        );
        const acceptAll = rduPage.getByRole("button", { name: /accept all/i });
        if ((await acceptAll.count()) > 0) {
          await acceptAll.first().click();
        }
        await rduPage
          .getByRole("button", { name: "Approve", exact: true })
          .first()
          .click();
        await rduPage.getByRole("button", { name: "Yes, approve" }).click();
      }

      const page = await depositor.newPage();
      await expect(async () => {
        await page.goto(overview);
        await expect(page.locator("main")).toContainText(
          "RDU approved this project",
          { timeout: 1000 },
        );
      }).toPass();
      await expect(page.locator("main")).toContainText(/same working day/i);
      await expectNoViolations(page);
    } finally {
      await rdu.close();
      await depositor.close();
    }
  });

  test("the approved record's discard confirmation has no violations", async ({
    browser,
  }) => {
    const context = await browser.newContext({ storageState: RDU_STATE });
    try {
      const page = await context.newPage();
      await page.goto("/approved");
      await page
        .locator("tr", { hasText: A11Y_STATES_SHORTCODE })
        .getByRole("link", { name: "Discard" })
        .click();
      await expect(page).toHaveURL(/\/approved\/[^/]+\/discard$/);
      await expect(
        page.getByRole("heading", { name: "Discard this record?", level: 1 }),
      ).toBeVisible();
      await expectNoViolations(page);
    } finally {
      await context.close();
    }
  });
});

test.describe("the pages behind a proposal, a draft, a submission and a claim", () => {
  // A retry would start from the state the failed attempt left. Runs once per
  // server, so a local re-run needs a fresh one.
  test.describe.configure({ mode: "serial", retries: 0 });
  test.use({ storageState: DEPOSITOR_STATE });

  const overview = `/projects/${A11Y_REVIEW_SHORTCODE}/sections/overview`;

  test("an entity proposal page has no violations", async ({ page }) => {
    const form = await proposePerson(
      page,
      A11Y_REVIEW_SHORTCODE,
      `A11y ${A11Y_REVIEW_SHORTCODE}`,
    );

    await page.goto(form);
    await expect(
      page.getByRole("textbox", { name: "Given name 1", exact: true }),
    ).toHaveValue("Ada");
    await expectNoViolations(page);
  });

  test("the discard-draft confirmation has no violations", async ({ page }) => {
    await page.goto(overview);
    await page.fill(
      'input[name="name"]',
      `A11y draft ${A11Y_REVIEW_SHORTCODE}`,
    );
    await page.getByRole("button", { name: "Save draft" }).click();
    await expect(page.locator("main")).toContainText("Draft saved");

    await page.goto(overview);
    await page.getByRole("button", { name: "Discard draft" }).click();
    await expect(
      page.getByRole("heading", { name: "Discard the draft?" }),
    ).toBeVisible();
    await expectNoViolations(page);
  });

  test("the take-back confirmation has no violations", async ({
    browser,
    page,
  }) => {
    await submitAs(browser, A11Y_REVIEW_SHORTCODE, (form) =>
      form.fill(
        'input[name="name"]',
        `A11y submitted ${A11Y_REVIEW_SHORTCODE}`,
      ),
    );

    await page.goto(overview);
    await page
      .getByRole("button", { name: "Take the submission back" })
      .click();
    await expect(
      page.getByRole("heading", { name: "Take the submission back?" }),
    ).toBeVisible();
    await expectNoViolations(page);
  });

  test("a review another reviewer holds, with a proposed person, has no violations", async ({
    browser,
  }) => {
    const rdu = await browser.newContext({ storageState: RDU_STATE });
    const second = await browser.newContext({ storageState: RDU2_STATE });
    try {
      await claimReview(await rdu.newPage(), A11Y_REVIEW_SHORTCODE);

      const page = await second.newPage();
      await page.goto(`/review/${A11Y_REVIEW_SHORTCODE}`);
      const main = page.locator("main");
      await expect(main).toContainText("Somebody else is reviewing this");
      await expect(
        main.getByRole("heading", {
          name: "Proposed people and organisations",
        }),
      ).toBeVisible();
      await expectNoViolations(page);
    } finally {
      await rdu.close();
      await second.close();
    }
  });
});

test.describe("error announcement", () => {
  test.use({ storageState: { cookies: [], origins: [] } });

  test("the live region pre-exists its first message", async ({ page }) => {
    // A live region inserted *together with* its first message is not announced
    // by a screen reader — the region has to be in the accessibility tree
    // before the text arrives. axe cannot see this: an empty region and an
    // absent one look identical to a static scan, and a populated one looks
    // correct. So it is asserted directly, before any error exists.
    await page.goto("/login");
    const liveRegions = page.locator(
      '[aria-live], [role="alert"], [role="status"]',
    );
    await expect(
      liveRegions.first(),
      "the sign-in form must render its live region before the first error, or the error is never announced",
    ).toBeAttached();
  });
});
