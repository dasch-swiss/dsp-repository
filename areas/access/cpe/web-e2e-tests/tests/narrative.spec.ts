import { expect, test } from "./support/test";

test("the front page renders its narrative and its two ways in", async ({
  page,
}) => {
  await page.goto("/cpe/incunabula/");

  await expect(
    page.getByRole("heading", {
      name: "The image sequences of Basel's early prints",
      level: 1,
    }),
  ).toBeVisible();
  for (const name of ["Two ways in", "Timeline", "The printing offices"]) {
    await expect(page.getByRole("heading", { name, level: 2 })).toBeVisible();
  }

  await page.getByRole("link", { name: /^The re-used woodcut/ }).click();
  await expect(page).toHaveURL("/cpe/incunabula/woodcuts");
});

test("the primary navigation reaches every page of the project", async ({
  page,
}) => {
  await page.goto("/cpe/incunabula/");
  const primary = page.getByRole("navigation", { name: "Primary" });

  for (const [name, path] of [
    ["Books", "/cpe/incunabula/books"],
    ["Narrenschiff", "/cpe/incunabula/narrenschiff"],
    ["Woodcuts", "/cpe/incunabula/woodcuts"],
    ["Border strips", "/cpe/incunabula/strips"],
    ["Home", "/cpe/incunabula/"],
  ]) {
    await primary.getByRole("link", { name, exact: true }).click();
    await expect(page).toHaveURL(path);
  }
});

test("the Narrenschiff page renders its narrative and opens an edition", async ({
  page,
}) => {
  await page.goto("/cpe/incunabula/narrenschiff");

  await expect(
    page.getByRole("heading", { name: "The Narrenschiff", level: 1 }),
  ).toBeVisible();
  for (const name of ["Three editions", "The border strips"]) {
    await expect(page.getByRole("heading", { name, level: 2 })).toBeVisible();
  }

  await page
    .getByRole("link", { name: /German.*\[Das\] Narrenschiff \(dt\.\)/ })
    .click();
  await expect(page).toHaveURL("/cpe/incunabula/books/narrenschiff-dt");
});

test("the woodcuts page renders its narrative and opens a woodcut on its page", async ({
  page,
}) => {
  await page.goto("/cpe/incunabula/woodcuts");

  await expect(
    page.getByRole("heading", { name: "The re-used woodcut", level: 1 }),
  ).toBeVisible();
  for (const name of [
    "Between the editions of the Narrenschiff",
    "Printed four times",
    "Inside one book",
    "Between books",
  ]) {
    await expect(page.getByRole("heading", { name, level: 2 })).toBeVisible();
  }

  await page
    .getByRole("link", { name: /Holzschnitt zu Kap\. 13: Von der Buhlerei\./ })
    .first()
    .click();
  await expect(page).toHaveURL(
    /\/cpe\/incunabula\/books\/narrenschiff-dt\?page=narrenschiff-dt-c1v#annotation-/,
  );
});
