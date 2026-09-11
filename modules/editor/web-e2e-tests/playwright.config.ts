import * as fs from "node:fs";
import * as path from "node:path";
import { defineConfig, devices } from "@playwright/test";

const workspaceRoot = path.resolve(__dirname, "../../..");
const serverBinary = path.resolve(
  workspaceRoot,
  "target/release/editor-server",
);

if (!fs.existsSync(serverBinary)) {
  throw new Error(
    `Server binary not found at ${serverBinary}\n` +
      `Run 'cargo build -p editor-server --release' (and 'just css-editor-release') before running E2E tests.`,
  );
}

/**
 * The server's stdout, which is also the mail transport.
 *
 * With `EDITOR_SMTP_HOST` unset the editor uses its console mailer and writes
 * every login code to the log, so this file is the only way a test can sign in.
 * `tests/support/server-log.ts` reads it; do not redirect the server elsewhere.
 */
export const SERVER_LOG = path.resolve(__dirname, "test-results/server.log");

/** Where `auth.setup.ts` parks the signed-in cookie jars. */
export const RDU_STATE = path.resolve(__dirname, "test-results/.auth/rdu.json");
export const DEPOSITOR_STATE = path.resolve(
  __dirname,
  "test-results/.auth/depositor.json",
);

/** Seeded through `EDITOR_RDU_EMAILS`, so this account exists at startup. */
export const RDU_EMAIL = "rdu@dasch.swiss";
/** Created by `auth.setup.ts` through the RDU interface — nothing seeds it. */
export const DEPOSITOR_EMAIL = "depositor@example.test";
/**
 * Projects the depositor may edit. All exist in `modules/dpe/server/data/projects`.
 *
 * One per mutating purpose, and that is not spare capacity: a draft is per
 * (user, project), so two tests editing the same project share one draft. The
 * sweep that clicks every row control leaves blank rows behind, and a blank row
 * makes the next Add a no-op — which reads exactly like the inert-control bug
 * this suite exists to catch. Give a new mutating spec its own entry here.
 */
export const DEPOSITOR_SHORTCODES = [
  "0103",
  "0105",
  "0106",
  "0101",
  "0102",
] as const;

/** Row add/remove assertions. */
export const DEPOSITOR_SHORTCODE = DEPOSITOR_SHORTCODES[0];
/** The sweep that clicks every control; leaves the draft dirty by design. */
export const SWEEP_SHORTCODE = DEPOSITOR_SHORTCODES[1];
/** The agent picker and repeated round trips. */
export const PICKER_SHORTCODE = DEPOSITOR_SHORTCODES[2];
/** The sticky save-notice measurement. */
export const NOTICE_SHORTCODE = DEPOSITOR_SHORTCODES[3];
/**
 * The end-to-end edit → submit → review → approve journey. Its own project
 * because submitting locks the form: any other spec sharing it would find a
 * read-only page from the moment this one runs.
 */
export const JOURNEY_SHORTCODE = DEPOSITOR_SHORTCODES[4];

const PORT = 4101;
export const BASE_URL = `http://127.0.0.1:${PORT}`;

export default defineConfig({
  testDir: "./tests",
  timeout: 30 * 1000,
  expect: { timeout: 5000 },
  // Serial, deliberately. Every test drives the same depositor's draft of the
  // same project against one shared server, so two tests adding rows at once
  // read each other's changes as their own. Parallelism here does not fail
  // loudly — it makes row-count assertions flaky.
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: "html",
  use: {
    actionTimeout: 0,
    baseURL: BASE_URL,
    trace: "on-first-retry",
  },

  projects: [
    // Signs in as RDU, creates the depositor through the RDU interface, signs
    // in as the depositor, and saves both cookie jars. A setup *project* rather
    // than `globalSetup` deliberately: a setup project runs after `webServer` is
    // up, which `globalSetup` is not guaranteed to.
    {
      name: "setup",
      testMatch: /.*\.setup\.ts/,
    },

    // The JavaScript-enabled pass. This is the one that catches an inert
    // Datastar control: with JS off, a submit button carrying `formaction`
    // works by default, so the no-JS pass below cannot see that breakage.
    // Both passes are required — neither subsumes the other.
    {
      name: "chromium-js",
      dependencies: ["setup"],
      testIgnore: /.*\.setup\.ts/,
      use: { ...devices["Desktop Chrome"] },
    },

    // The discipline pass. The one-handler-two-renderings technique removes the
    // code divergence, not the risk that someone adds a `data-on:click` control
    // with no submit button behind it. Nothing but this project catches that.
    {
      name: "chromium-nojs",
      dependencies: ["setup"],
      // The accessibility spec is excluded, not skipped: axe injects its own
      // script into the page and cannot run at all without JavaScript. It is
      // not a coverage gap — the markup axe reads is server-rendered and
      // identical in both passes.
      testIgnore: [/.*\.setup\.ts/, /accessibility\.spec\.ts/],
      use: { ...devices["Desktop Chrome"], javaScriptEnabled: false },
    },
  ],

  webServer: {
    // stdout is teed to SERVER_LOG because it carries the login codes; the
    // suite cannot authenticate without reading them back.
    command: `mkdir -p "${path.dirname(SERVER_LOG)}" && "${serverBinary}" serve >"${SERVER_LOG}" 2>&1`,
    port: PORT,
    // Run from the workspace root so the relative EDITOR_* paths below resolve.
    cwd: workspaceRoot,
    reuseExistingServer: !process.env.CI,
    env: {
      EDITOR_SITE_ADDR: `127.0.0.1:${PORT}`,
      // DEV, not PROD: PROD refuses to start without EDITOR_SMTP_HOST, because a
      // console mailer in production is a standing credential leak. The suite
      // needs exactly that console mailer to read its login codes back.
      EDITOR_ENV: "DEV",
      EDITOR_PUBLIC_DIR: "modules/editor/public",
      // No default in the config, so the editor will not start without it.
      EDITOR_DATA_DIR: "modules/dpe/server/data",
      // Unset EDITOR_DB_DIR means in-memory SQLite — a fresh database per run.
      EDITOR_RDU_EMAILS: RDU_EMAIL,
      // 1, not 0: `EditorConfig::validate` refuses a zero cooldown outright
      // ("must be at least 1"), so the issue's "set the cooldown to zero"
      // cannot be followed literally. 1 second is the floor and is enough to
      // keep a retried sign-in from tripping the resend throttle.
      EDITOR_LOGIN_COOLDOWN_SECS: "1",
      // The console mailer logs codes at WARN, so `error` would hide them.
      RUST_LOG: "warn",
    },
  },
});
