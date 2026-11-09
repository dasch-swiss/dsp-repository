import { test } from "@playwright/test";

/** A value per Playwright project, as the `*_SHORTCODES` maps in `playwright.config.ts` hold. */
export type PerPass = Partial<Record<string, string>>;

/**
 * This pass's entry of a per-pass map, passed under its own name so a missing
 * entry can say which map lacks it: `perPass({ STATUS_SHORTCODES })`.
 *
 * Call it inside a test, never at module scope: `test.info()` is only defined
 * while a test runs.
 */
export function perPass(named: Record<string, PerPass>): string {
  const entries = Object.entries(named);
  if (entries.length !== 1) {
    throw new Error(
      "perPass takes exactly one map: perPass({ SOME_SHORTCODES })",
    );
  }
  const [name, map] = entries[0];
  const pass = test.info().project.name;
  const value = map[pass];
  if (!value) {
    throw new Error(
      `${name} has no entry for Playwright project ${pass}; add one in playwright.config.ts`,
    );
  }
  return value;
}
