import * as fs from "node:fs";
import * as path from "node:path";
import { defineConfig, devices } from "@playwright/test";

const serverBinary = path.resolve(
  __dirname,
  "../../..",
  "target/release/dpe-server",
);

if (!fs.existsSync(serverBinary)) {
  throw new Error(
    `Server binary not found at ${serverBinary}\n` +
      `Run 'cargo build -p dpe-server --release' (and 'just css-release') before running E2E tests.`,
  );
}

/**
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
  testDir: "./tests",
  timeout: 30 * 1000,
  expect: {
    timeout: 5000,
  },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: "html",
  use: {
    actionTimeout: 0,
    trace: "on-first-retry",
  },

  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
      },
    },

    {
      name: "firefox",
      use: {
        ...devices["Desktop Firefox"],
      },
    },

    {
      name: "webkit",
      use: {
        ...devices["Desktop Safari"],
      },
    },

    /* Test against branded browsers. */
    // {
    //   name: 'Microsoft Edge',
    //   use: {
    //     channel: 'msedge',
    //   },
    // },
    // {
    //   name: 'Google Chrome',
    //   use: {
    //     channel: 'chrome',
    //   },
    // },
  ],

  /* Start the DPE server before running tests */
  webServer: {
    command: `${serverBinary} serve`,
    port: 4000,
    // Run from the workspace root so the server's relative data dir
    // (modules/dpe/server/data, resolved in config.rs) resolves. Without this the
    // project cache is empty and every page renders without its expected content.
    cwd: path.resolve(__dirname, "../../.."),
    reuseExistingServer: !process.env.CI,
    env: {
      DPE_SITE_ADDR: "127.0.0.1:4000",
      DPE_ENV: "PROD",
      RUST_LOG: "error",
    },
  },
});
