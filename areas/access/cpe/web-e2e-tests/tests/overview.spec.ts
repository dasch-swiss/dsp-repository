import { expect, test } from "./support/test";

test("the overview lists Incunabula and leads to its front page", async ({
  page,
}) => {
  await page.goto("/cpe/");
  await expect(
    page.getByRole("heading", { name: "Projects", level: 2 }),
  ).toBeVisible();

  await page
    .getByRole("link", { name: /The image sequences of Basel's early prints/ })
    .click();

  await expect(page).toHaveURL("/cpe/incunabula/");
  await expect(
    page.getByRole("heading", {
      name: "The image sequences of Basel's early prints",
      level: 1,
    }),
  ).toBeVisible();
});

test("a path that names no project answers the not-found page with 404", async ({
  page,
}) => {
  const response = await page.goto("/cpe/no-such-project/");
  expect(response?.status()).toBe(404);
  await expect(
    page.getByRole("heading", {
      name: "This address does not name a project.",
      level: 1,
    }),
  ).toBeVisible();

  await page.getByRole("link", { name: "All projects" }).click();
  await expect(page).toHaveURL("/cpe/");
});

test("a path inside Incunabula that names nothing answers its not-found page with 404", async ({
  page,
}) => {
  const response = await page.goto("/cpe/incunabula/no-such-page");
  expect(response?.status()).toBe(404);
  await expect(
    page.getByRole("heading", { name: "This page does not exist.", level: 1 }),
  ).toBeVisible();

  await page.getByRole("main").getByRole("link", { name: "Books" }).click();
  await expect(page).toHaveURL("/cpe/incunabula/books");
});
