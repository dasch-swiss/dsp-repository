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
});

test.describe("a page parameter that names no page", () => {
  for (const value of ["abc", "9999", "melusine-9999"]) {
    test(`?page=${value} lands on the first page`, async ({ page }) => {
      const response = await page.goto(`${BOOK.path}?page=${value}`);
      expect(response?.status()).toBe(200);
      await expect(reader(page, 1)).toBeVisible();
    });
  }
});
