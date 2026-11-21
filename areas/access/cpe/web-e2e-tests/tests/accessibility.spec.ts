import AxeBuilder from "@axe-core/playwright";
import { formatViolations } from "./support/axe-helpers";
import { expect, test } from "./support/test";

// One page of each kind CPE renders for Incunabula. WCAG 2.1 Level AA, which EU Directive
// 2019/882 (EAA) requires via EN 301 549. `status` guards against scanning a not-found page
// in place of the page meant.
const PAGES: { name: string; path: string; status?: number }[] = [
  { name: "the overview", path: "/cpe/" },
  { name: "the front page", path: "/cpe/incunabula/" },
  { name: "a narrative page", path: "/cpe/incunabula/narrenschiff" },
  { name: "a narrative page of cases", path: "/cpe/incunabula/woodcuts" },
  { name: "the book list", path: "/cpe/incunabula/books" },
  { name: "the book list as a grid", path: "/cpe/incunabula/books?view=grid" },
  {
    name: "the reader",
    path: "/cpe/incunabula/books/melusine?page=melusine-5",
  },
  {
    name: "the reader on an annotated page",
    path: "/cpe/incunabula/books/narrenschiff-dt?page=narrenschiff-dt-c1v",
  },
  { name: "the strip list", path: "/cpe/incunabula/strips" },
  { name: "a strip", path: "/cpe/incunabula/strips/l-10" },
  {
    name: "the project's not-found page",
    path: "/cpe/incunabula/no-such-page",
    status: 404,
  },
  { name: "CPE's not-found page", path: "/cpe/no-such-project/", status: 404 },
];

for (const { name, path, status = 200 } of PAGES) {
  test(`${name} (${path}) has no violations`, async ({ page }) => {
    const response = await page.goto(path);
    expect(response?.status()).toBe(status);

    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
      .analyze();

    const count = results.violations.length;
    expect(
      count,
      `${count} accessibility violation(s) found:\n\n${formatViolations(results.violations)}`,
    ).toBe(0);
  });
}
