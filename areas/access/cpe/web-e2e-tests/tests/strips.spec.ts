import { expect, test } from "./support/test";

test("the border strips list every strip and opens one", async ({ page }) => {
  await page.goto("/cpe/incunabula/strips");

  await expect(
    page.getByRole("heading", { name: "Border strips", level: 1 }),
  ).toBeAttached();
  await expect(page.getByText("38 border strips")).toBeVisible();

  await page.getByRole("link", { name: /^l_10\b/ }).click();

  await expect(page).toHaveURL("/cpe/incunabula/strips/l-10");
  await expect(
    page.getByRole("heading", { name: "l_10", level: 1 }),
  ).toBeVisible();
});

test("a strip leads to a page it is printed on", async ({ page }) => {
  await page.goto("/cpe/incunabula/strips/l-10");

  await page.getByRole("link", { name: "b6v", exact: true }).click();

  await expect(page).toHaveURL(
    "/cpe/incunabula/books/narrenschiff-dt?page=narrenschiff-dt-b6v",
  );
  await expect(
    page.getByRole("heading", { name: "[Das] Narrenschiff (dt.)", level: 1 }),
  ).toBeVisible();
});
