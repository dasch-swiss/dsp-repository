import type { Result } from "axe-core";

export function formatViolations(violations: Result[]): string {
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
