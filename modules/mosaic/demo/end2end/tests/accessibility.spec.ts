import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import type { Result } from "axe-core";

function formatViolations(violations: Result[]): string {
  return violations
    .map((v, i) => {
      const nodes = v.nodes
        .map((n) => {
          const target = n.target.join(", ");
          const summary = n.failureSummary ?? "(no summary)";
          return `    - ${target}\n      ${summary.replace(/\n/g, "\n      ")}`;
        })
        .join("\n");
      return (
        `[${i + 1}] ${v.id} (${v.impact ?? "unknown impact"})\n` +
        `    ${v.help}\n` +
        `    ${v.helpUrl}\n` +
        `  Affected nodes:\n${nodes}`
      );
    })
    .join("\n\n");
}

test.describe("Mosaic demo accessibility", () => {
  test("home page has no WCAG violations", async ({ page }) => {
    await page.goto("http://localhost:3000/");
    // WCAG 2.1 Level AA — required by EU Directive 2019/882 (EAA) via EN 301 549
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"]) // remove this lines with specific tags for testing against all of W3C's WCAG including best practice
      .analyze();
    const count = results.violations.length;
    expect(count, `${count} accessibility violation(s) found:\n\n${formatViolations(results.violations)}`).toBe(0);
  });
});
