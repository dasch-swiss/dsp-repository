import * as fs from "node:fs";
import * as path from "node:path";
import { defineConfig, devices } from "@playwright/test";

/** The release server Bazel built, set by `just test-e2e-cpe` and `just test-a11y-cpe`. */
const serverBinary = process.env.ACCESS_SERVER_BIN ?? "";

if (!serverBinary || !fs.existsSync(serverBinary)) {
  throw new Error(
    `Server binary not found (ACCESS_SERVER_BIN='${serverBinary}')\n` +
      `Run the suite with 'just test-e2e-cpe' or 'just test-a11y-cpe', which build the server and set it.`,
  );
}

const PORT = 4102;

export default defineConfig({
  testDir: "./tests",
  timeout: 30 * 1000,
  expect: { timeout: 5000 },
  // Every spec only reads: CPE has no write path, so tests share the server safely.
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: "html",
  use: {
    actionTimeout: 0,
    baseURL: `http://127.0.0.1:${PORT}`,
    // CPE picks its language from `Accept-Language`; the specs read the English labels.
    locale: "en-US",
    // CPE declares cross-document view transitions on reader pages, only under
    // `prefers-reduced-motion: no-preference`. While one runs, the root takes every click, so a
    // click right after a page turn lands on it; reduced motion turns them off, and the fades
    // with them.
    reducedMotion: "reduce",
    trace: "retain-on-failure",
  },

  projects: [
    {
      name: "chromium-js",
      use: { ...devices["Desktop Chrome"] },
    },

    // Every journey again with JavaScript off: each page must work as plain links and forms
    // (ADR-0004). The accessibility spec is excluded, not skipped: axe injects its own script and
    // cannot run without JavaScript, and the markup it reads is the same server rendering.
    {
      name: "chromium-nojs",
      testIgnore: /accessibility\.spec\.ts/,
      use: { ...devices["Desktop Chrome"], javaScriptEnabled: false },
    },
  ],

  webServer: {
    command: `"${serverBinary}" serve`,
    // Not `port`, and not `/healthz`: the server binds and answers `/healthz` before CPE has
    // activated, and CPE answers 503 until it has. Incunabula's front page answers 200 only once
    // the project is served.
    url: `http://127.0.0.1:${PORT}/cpe/incunabula/`,
    timeout: 120 * 1000,
    // From the workspace root, so the defaults of `CPE_PROJECTS_DIR`, `SYNC_DATA_DIR` and
    // `MEDIA_DATA_DIR` resolve to the committed data of project 0803.
    cwd: path.resolve(__dirname, "../../../.."),
    reuseExistingServer: !process.env.CI,
    env: {
      CPE_BASE: "/cpe",
      DPE_SITE_ADDR: `127.0.0.1:${PORT}`,
      RUST_LOG: "error",
    },
  },
});
