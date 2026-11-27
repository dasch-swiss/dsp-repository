import { test as base } from "@playwright/test";

export { expect } from "@playwright/test";

/**
 * Playwright's `test`, with every request to another origin aborted.
 *
 * Images come from an external IIIF host and fonts from Google; the suite must pass with both
 * unreachable, so it never reaches them. Without JavaScript Chromium also ignores
 * `loading="lazy"` and fetches every image before `load`, which an external host makes slow.
 */
export const test = base.extend<{ sameOriginOnly: undefined }>({
  sameOriginOnly: [
    async ({ context, baseURL }, use) => {
      const origin = new URL(baseURL ?? "").origin;
      await context.route(
        (url) => url.origin !== origin,
        (route) => route.abort(),
      );
      await use(undefined);
    },
    { auto: true },
  ],
});
