import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { formatViolations } from "./axe-helpers";

const BASE_URL = "http://localhost:4000";

// Every distinct page type in DPE.
const pages = [
  { name: "Home", path: "/dpe/" },
  { name: "Projects listing", path: "/dpe/projects" },
  { name: "Project detail", path: "/dpe/projects/0803" },
  { name: "About", path: "/dpe/about" },
];

test.describe("DPE accessibility — WCAG 2.1 Level AA", () => {
  // WCAG 2.1 Level AA — required by EU Directive 2019/882 (EAA) via EN 301 549
  for (const { name, path } of pages) {
    test(`${name} (${path}) has no violations`, async ({ page }) => {
      await page.goto(`${BASE_URL}${path}`);

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
});
