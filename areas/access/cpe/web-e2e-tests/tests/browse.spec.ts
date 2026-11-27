import type { Page } from "@playwright/test";

import { expect, test } from "./support/test";

// Incunabula's book list has a sort order and a list or grid view; it declares no facet panel and
// no search bar, so there is neither to exercise here.

/** The list of books in `/books`, told apart from the breadcrumb's list by a book it holds. */
function bookList(page: Page) {
  return page
    .getByRole("main")
    .getByRole("list")
    .filter({
      has: page.getByRole("link", {
        name: /Geschichte von der schönen Melusine/,
      }),
    });
}

test("the book list shows every book, in date order", async ({ page }) => {
  await page.goto("/cpe/incunabula/books");

  await expect(
    page.getByRole("heading", { name: "Books", level: 1 }),
  ).toBeAttached();
  await expect(page.getByText("19 books")).toBeVisible();
  const books = bookList(page).getByRole("link");
  await expect(books).toHaveCount(19);
  await expect(books.first()).toContainText(
    "Geschichte von der schönen Melusine",
  );
});

test("a sort order reorders the list", async ({ page }) => {
  await page.goto("/cpe/incunabula/books");

  await page.getByLabel("Order: By date").click();
  await page.getByRole("button", { name: "Title A–Z" }).click();

  await expect(page.getByLabel("Order: Title A–Z")).toBeVisible();
  const books = bookList(page).getByRole("link");
  await expect(books.first()).toContainText(
    "Bereitung zu dem Heiligen Sakrament",
  );
  await expect(books).toHaveCount(19);
});

test("a sort order is a link of its own", async ({ page }) => {
  await page.goto("/cpe/incunabula/books?sort=name-desc");

  await expect(page.getByLabel("Order: Title Z–A")).toBeVisible();
  await expect(bookList(page).getByRole("link").first()).toContainText(
    "Zeitglöcklein des Lebens und Leidens Christi [1492]",
  );
});

test("the grid view lists the same books", async ({ page }) => {
  await page.goto("/cpe/incunabula/books");

  await page
    .getByRole("group", { name: "View" })
    .getByRole("link", { name: "Grid" })
    .click();

  await expect(page).toHaveURL("/cpe/incunabula/books?view=grid");
  await expect(
    page.getByRole("group", { name: "View" }).getByRole("link", {
      name: "Grid",
    }),
  ).toHaveAttribute("aria-current", "true");
  await expect(bookList(page).getByRole("link")).toHaveCount(19);
});
