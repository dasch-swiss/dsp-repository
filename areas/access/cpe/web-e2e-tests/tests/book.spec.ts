import type { Page } from "@playwright/test";

import { expect, test } from "./support/test";

/** The book these cases read: 194 pages, page ids `melusine-1` to `melusine-194`. */
const BOOK = {
  path: "/cpe/incunabula/books/melusine",
  title: "Geschichte von der schönen Melusine",
  pages: 194,
};

/** The reader, named by its page heading ("5 / 194"). */
function reader(page: Page, n: number) {
  return page.getByRole("article", {
    name: `${n} / ${BOOK.pages}`,
    exact: true,
  });
}

function pager(page: Page) {
  return page.getByRole("navigation", { name: "Turn pages" });
}

/** The page picker, a disclosure in the pager. */
function picker(page: Page) {
  return page.locator("details#cv-picker");
}

/** The `_compound` requests `page` makes from now on, in order. */
function compoundLoads(page: Page) {
  const loads: string[] = [];
  page.on("request", (request) => {
    if (request.url().includes("/_compound")) loads.push(request.url());
  });
  return loads;
}

/**
 * Turns from page 5 to 6 with Next and checks that the turn was the only
 * `_compound` request: one the action before it made would come first.
 */
async function expectOnlyTheNextTurn(page: Page, loads: string[]) {
  await pager(page).getByRole("link", { name: /Next/ }).click();
  await expect(reader(page, 6)).toBeVisible();
  expect(loads).toHaveLength(1);
}

/** The URL of page `n` of the book, as the reader's own links write it. */
function pageUrl(n: number) {
  return `${BOOK.path}?page=melusine-${n}`;
}

/**
 * Moves to an in-page anchor and back, and checks that the reader was not
 * reloaded: Back reloads only when the address differs from the one the
 * reader last recorded, so this fails when that record is missing or stale.
 */
async function expectNoReloadOnInPageBack(page: Page, url: string) {
  const loads: string[] = [];
  page.on("request", (request) => {
    if (request.isNavigationRequest()) loads.push(request.url());
  });
  await page.evaluate(() => Object.assign(window, { cpeSpecMark: true }));
  await page.evaluate(() => {
    location.hash = "in-page";
  });
  await page.goBack();
  await expect(page).toHaveURL(url);
  const marked = await page.evaluate(
    () =>
      new Promise<boolean>((resolve) =>
        requestAnimationFrame(() =>
          requestAnimationFrame(() => resolve("cpeSpecMark" in window)),
        ),
      ),
  );
  expect(loads).toEqual([]);
  expect(marked).toBe(true);
}

test.describe("opening a book", () => {
  test("a book opens from the list at its first page", async ({ page }) => {
    await page.goto("/cpe/incunabula/books");

    await page.getByRole("link", { name: new RegExp(BOOK.title) }).click();

    await expect(page).toHaveURL(BOOK.path);
    await expect(
      page.getByRole("heading", { name: BOOK.title, level: 1 }),
    ).toBeVisible();
    await expect(reader(page, 1)).toBeVisible();
  });
});

test.describe("paging through the reader", () => {
  test("Next and Previous turn the page and the URL with it", async ({
    page,
  }) => {
    await page.goto(pageUrl(5));
    await expect(reader(page, 5)).toBeVisible();

    await pager(page).getByRole("link", { name: /Next/ }).click();
    await expect(reader(page, 6)).toBeVisible();
    await expect(page).toHaveURL(pageUrl(6));

    await pager(page)
      .getByRole("link", { name: /Previous/ })
      .click();
    await expect(reader(page, 5)).toBeVisible();
    await expect(page).toHaveURL(pageUrl(5));
  });

  test("Back and Forward keep the reader and the URL in step", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "With JavaScript off every page turn is a full navigation; the history is the browser's own.",
    );

    await page.goto(pageUrl(5));
    await pager(page).getByRole("link", { name: /Next/ }).click();
    await expect(reader(page, 6)).toBeVisible();
    await pager(page).getByRole("link", { name: /Next/ }).click();
    await expect(reader(page, 7)).toBeVisible();

    await page.goBack();
    await expect(page).toHaveURL(pageUrl(6));
    await expect(reader(page, 6)).toBeVisible();

    await page.goForward();
    await expect(page).toHaveURL(pageUrl(7));
    await expect(reader(page, 7)).toBeVisible();
  });

  test("a reload keeps the page turned to, and Back still turns back", async ({
    page,
  }) => {
    await page.goto(pageUrl(5));
    await expect(reader(page, 5)).toBeVisible();
    await pager(page).getByRole("link", { name: /Next/ }).click();
    await expect(reader(page, 6)).toBeVisible();

    await page.reload();
    await expect(reader(page, 6)).toBeVisible();

    await page.goBack();
    await expect(page).toHaveURL(pageUrl(5));
    await expect(reader(page, 5)).toBeVisible();
  });

  test("Back over an in-page move after a reload does not reload the reader", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "Without JavaScript the reader keeps no address of its own.",
    );

    await page.goto(pageUrl(5));
    await expect(reader(page, 5)).toBeVisible();
    await pager(page).getByRole("link", { name: /Next/ }).click();
    await expect(reader(page, 6)).toBeVisible();
    await page.reload();
    await expect(reader(page, 6)).toBeVisible();

    await expectNoReloadOnInPageBack(page, pageUrl(6));
  });
});

test.describe("the arrow keys", () => {
  test("Right and Left turn the page while focus is in the reader", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "The arrow keys are bound by script.",
    );

    await page.goto(pageUrl(5));
    await reader(page, 5).focus();

    await page.keyboard.press("ArrowRight");
    await expect(reader(page, 6)).toBeVisible();
    await expect(page).toHaveURL(pageUrl(6));

    await reader(page, 6).focus();
    await page.keyboard.press("ArrowLeft");
    await expect(reader(page, 5)).toBeVisible();
    await expect(page).toHaveURL(pageUrl(5));
  });

  test("an arrow key in the open page picker turns no page", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "The arrow keys are bound by script.",
    );

    await page.goto(pageUrl(5));
    const loads = compoundLoads(page);
    await picker(page).locator("summary").click();
    await picker(page).getByRole("link", { name: "5", exact: true }).focus();

    await page.keyboard.press("ArrowLeft");
    await expectOnlyTheNextTurn(page, loads);
  });

  test("an arrow key with a modifier turns no page", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "The arrow keys are bound by script.",
    );

    await page.goto(pageUrl(5));
    const loads = compoundLoads(page);
    await reader(page, 5).focus();

    await page.keyboard.press("Shift+ArrowLeft");
    await expectOnlyTheNextTurn(page, loads);
  });
});

test.describe("the page picker", () => {
  test("opening it scrolls the current page's entry into the panel", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "The scroll on opening is made by script; without it the panel opens at its top.",
    );

    await page.goto(pageUrl(150));

    await picker(page).locator("summary").click();

    const current = picker(page).getByRole("link", {
      name: "150",
      exact: true,
    });
    await expect(current).toHaveAttribute("aria-current", "page");
    await expect(current).toBeInViewport();
  });

  test("choosing a page turns to it and moves the URL", async ({ page }) => {
    await page.goto(pageUrl(5));

    await picker(page).locator("summary").click();
    await picker(page).getByRole("link", { name: "9", exact: true }).click();

    await expect(reader(page, 9)).toBeVisible();
    await expect(page).toHaveURL(pageUrl(9));
  });

  test("choosing the current page only closes the picker", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "Without JavaScript every entry is a plain link.",
    );

    await page.goto(pageUrl(5));
    const loads = compoundLoads(page);
    await picker(page).locator("summary").click();
    await picker(page).getByRole("link", { name: "5", exact: true }).click();

    await expect(picker(page)).not.toHaveAttribute("open");
    await expect(picker(page).locator("summary")).toBeFocused();
    await expect(page).toHaveURL(pageUrl(5));
    await expectOnlyTheNextTurn(page, loads);
  });

  test("a group's mark moves to its heading inside the panel", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "Without JavaScript a mark is a plain in-page link.",
    );

    await page.goto(pageUrl(5));
    await picker(page).locator("summary").click();
    const mark = picker(page).getByRole("link", {
      name: "Pages 121–140",
      exact: true,
    });
    const target = await mark.getAttribute("href");
    await mark.click();

    const heading = picker(page).locator(`${target}`);
    await expect(heading).toBeFocused();
    await expect(heading).toBeInViewport();
    await expect(page).toHaveURL(pageUrl(5));
  });
});

test.describe("arriving on an annotation", () => {
  /** A page of another book whose register holds one annotation, and no link. */
  const ANNOTATED = {
    url: "/cpe/incunabula/books/de-generatione?page=de-generatione-a2r",
    entry: "annotation-fh-snkNvV9eEOCzY2m2dMw",
  };

  /** Where the window shows `selector`, once the arrival's frame has passed. */
  async function boxAfterArrival(page: Page, selector: string) {
    return page.evaluate(
      (sel) =>
        new Promise<{ top: number; bottom: number; height: number }>(
          (resolve) =>
            requestAnimationFrame(() =>
              requestAnimationFrame(() => {
                const box = document
                  .querySelector(sel)
                  ?.getBoundingClientRect() ?? { top: NaN, bottom: NaN };
                resolve({
                  top: box.top,
                  bottom: box.bottom,
                  height: innerHeight,
                });
              }),
            ),
        ),
      selector,
    );
  }

  test("its fragment emphasises the entry and brings it into view", async ({
    page,
  }, testInfo) => {
    await page.goto(`${ANNOTATED.url}#${ANNOTATED.entry}`);

    const entry = page.locator(`#${ANNOTATED.entry}`);
    await expect(entry).toBeInViewport();
    if (testInfo.project.name === "chromium-nojs") {
      await expect(entry).not.toHaveClass(/\bis-em\b/);
    } else {
      await expect(entry).toHaveClass(/\bis-em\b/);
    }
  });

  test("beside a sticky reader the entry is scrolled to the window's end", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "The arrival scroll is made by script; without it the browser scrolls to the fragment.",
    );
    await page.setViewportSize({ width: 1600, height: 900 });

    await page.goto(`${ANNOTATED.url}#${ANNOTATED.entry}`);
    await expect(page.locator(`#${ANNOTATED.entry}`)).toHaveClass(/\bis-em\b/);

    const box = await boxAfterArrival(page, `#${ANNOTATED.entry}`);
    // One pixel of slack: the box is laid out at fractional pixels.
    expect(box.bottom).toBeLessThanOrEqual(box.height + 1);
    expect(box.bottom).toBeGreaterThan(box.height - 20);
  });

  test("in one track the reader's head is scrolled to the top", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "The arrival scroll is made by script; without it the browser scrolls to the fragment.",
    );
    await page.setViewportSize({ width: 600, height: 900 });

    await page.goto(`${ANNOTATED.url}#${ANNOTATED.entry}`);
    await expect(page.locator(`#${ANNOTATED.entry}`)).toHaveClass(/\bis-em\b/);

    const box = await boxAfterArrival(page, ".cv-member__head");
    expect(box.top).toBeGreaterThanOrEqual(0);
    expect(box.top).toBeLessThan(150);
  });

  test("without a fragment no entry is emphasised", async ({
    page,
  }, testInfo) => {
    await page.goto(ANNOTATED.url);

    if (testInfo.project.name === "chromium-js") {
      // The toggle is shown by DataStar, after the record's `data-init` ran.
      await expect(
        page.getByRole("button", { name: "Show on scan" }),
      ).toBeVisible();
    }
    await expect(page.locator("#annotations .anno__entry")).toHaveCount(1);
    await expect(page.locator("#annotations .is-em")).toHaveCount(0);
  });
});

test.describe("without CPE's own script", () => {
  test.beforeEach(async ({ page }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "Without JavaScript no script runs at all.",
    );
    await page.route("**/_static/cpe/cpe.js", (route) => route.abort());
  });

  test("a stale address on an annotated book is still corrected", async ({
    page,
  }) => {
    await page.goto(`${BOOK.path}?page=abc`);
    await expect(reader(page, 1)).toBeVisible();
    await expect(page).toHaveURL(BOOK.path);
  });

  test("choosing a page in the picker still turns to it", async ({ page }) => {
    await page.goto(pageUrl(5));

    await picker(page).locator("summary").click();
    await picker(page).getByRole("link", { name: "9", exact: true }).click();

    await expect(reader(page, 9)).toBeVisible();
    await expect(page).toHaveURL(pageUrl(9));
  });
});

test.describe("a page parameter that names no page", () => {
  for (const value of ["abc", "9999", "melusine-9999"]) {
    test(`?page=${value} lands on the first page`, async ({ page }) => {
      const response = await page.goto(`${BOOK.path}?page=${value}`);
      expect(response?.status()).toBe(200);
      await expect(reader(page, 1)).toBeVisible();
    });
  }

  test("the address is corrected to the page shown", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "The correction is made by script; without it the address stays as typed.",
    );

    await page.goto(`${BOOK.path}?page=melusine-9999`);
    await expect(reader(page, 1)).toBeVisible();
    await expect(page).toHaveURL(BOOK.path);
  });

  test("the corrected address keeps the other parameters as written", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "The correction is made by script; without it the address stays as typed.",
    );
    // `$name` and `@name(` are what DataStar rewrites in an expression; `&`
    // is what the attribute escapes.
    const kept = `${BOOK.path}?x=$expand@get(1)&y=2`;

    await page.goto(`${BOOK.path}?page=abc&x=$expand@get(1)&y=2`);
    await expect(reader(page, 1)).toBeVisible();
    await expect(page).toHaveURL(kept);

    await expectNoReloadOnInPageBack(page, kept);
  });

  test("a page turned after the correction moves the URL to that page", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "chromium-nojs",
      "The correction is made by script; without it the address stays as typed.",
    );

    await page.goto(`${BOOK.path}?page=abc`);
    await expect(page).toHaveURL(BOOK.path);

    await pager(page).getByRole("link", { name: /Next/ }).click();
    await expect(reader(page, 2)).toBeVisible();
    await expect(page).toHaveURL(pageUrl(2));

    await page.goBack();
    await expect(page).toHaveURL(BOOK.path);
    await expect(reader(page, 1)).toBeVisible();
  });
});
