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
