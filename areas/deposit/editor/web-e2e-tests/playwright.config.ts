import * as fs from "node:fs";
import * as path from "node:path";
import { defineConfig, devices } from "@playwright/test";

const workspaceRoot = path.resolve(__dirname, "../../../..");
const serverBinary = path.resolve(
  workspaceRoot,
  "target/release/deposit-server",
);

/** DPE's published data set, relative to `workspaceRoot`. */
const DPE_DATA_DIR = "areas/access/dpe/server/data";

if (!fs.existsSync(serverBinary)) {
  throw new Error(
    `Server binary not found at ${serverBinary}\n` +
      `Run 'cargo build -p deposit-server --release' (and 'just css-editor-release') before running E2E tests.`,
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
export const RDU2_STATE = path.resolve(
  __dirname,
  "test-results/.auth/rdu2.json",
);

/** Seeded through `EDITOR_RDU_EMAILS`, so this account exists at startup. */
export const RDU_EMAIL = "rdu@dasch.swiss";
/**
 * The second reviewer, seeded with `RDU_EMAIL`, for a review one account takes
 * over from the other. The display name is the address's local part, so it must
 * not contain `rdu`: a name match on `rdu` would find both.
 */
export const RDU2_EMAIL = "reviewer-b@dasch.swiss";
/** Created by `auth.setup.ts` through the RDU interface — nothing seeds it. */
export const DEPOSITOR_EMAIL = "depositor@example.test";
/**
 * Projects the depositor may edit. All exist in `areas/access/dpe/server/data/projects`, except
 * the `CREATED_SHORTCODES` and `SUBMITTED_CREATED_SHORTCODES`, which RDU creates during the run.
 *
 * One per mutating purpose, and that is not spare capacity: a draft is per
 * (user, project), so two tests editing the same project share one draft. The
 * sweep that clicks every row control leaves blank rows behind, and a blank row
 * makes the next Add a no-op — which reads exactly like the inert-control bug
 * this suite exists to catch. Give a new mutating spec its own entry here.
 *
 * Every entry's name is listed on the depositor's `/projects`, which the
 * vocabulary check reads: a project whose text holds a word
 * `tests/support/vocabulary.ts` forbids ("Committee", "Transfers") fails it.
 *
 * Codes the suite takes outside this list: `REMOVED_SHORTCODES`, `A11Y_CREATED_SHORTCODE`, and
 * `0813`, the published code `projects.spec.ts` tries to create.
 *
 * Adding a spec: append its entry at the end, since the constants below index this list. A spec
 * that walks a lifecycle from a starting state in both passes gets a per-pass
 * `Partial<Record<…>>` map, read with `perPass`. Only a spec whose assertions hold whatever another
 * spec left in the draft may share `PICKER_SHORTCODE`, except the one its doc names. A code from
 * the data set needs a project JSON with no word the vocabulary check forbids (above).
 */
export const DEPOSITOR_SHORTCODES = [
  "0103",
  "0105",
  "0106",
  "0101",
  "0102",
  "0107",
  "0108",
  "0110",
  "0112",
  "0117",
  "E2E1",
  "E2E2",
  "0111",
  "0113",
  "0115",
  "0811",
  "0812",
  "0118",
  "0119",
  "0120",
  "0121",
  "0803",
  "0804",
  "0805",
  "0806",
  "0807",
  "0810",
  "E2E3",
  "E2E4",
  "080C",
  "080E",
  "0816",
] as const;

/** Row add/remove assertions. */
export const DEPOSITOR_SHORTCODE = DEPOSITOR_SHORTCODES[0];
/** The sweep that clicks every control; leaves the draft dirty by design. */
export const SWEEP_SHORTCODE = DEPOSITOR_SHORTCODES[1];
/**
 * The agent picker and repeated round trips. `proposals.spec.ts` is the
 * exception to the sharing rule above: it asserts that the project shows no
 * proposals, so a spec that leaves a live proposal here breaks it.
 */
export const PICKER_SHORTCODE = DEPOSITOR_SHORTCODES[2];
/** The sticky save-notice measurement. */
export const NOTICE_SHORTCODE = DEPOSITOR_SHORTCODES[3];
/**
 * The end-to-end edit → submit → review → approve journey. Its own project
 * because submitting locks the form: any other spec sharing it would find a
 * read-only page from the moment this one runs.
 */
export const JOURNEY_SHORTCODE = DEPOSITOR_SHORTCODES[4];
/**
 * The depositor-visible state sequence and the approved box —
 * **one project per pass**, not one per spec.
 *
 * Every other mutating spec is idempotent enough to run twice against one
 * server, because it asserts a transition rather than a starting point. This
 * one walks the whole lifecycle from Online, so the second pass would begin
 * wherever the first left off and read Approved where it expects Online. Both
 * passes share a server, so the only way to give each a pristine project is to
 * give each a different one. Keyed by Playwright project name.
 */
export const STATUS_SHORTCODES: Partial<Record<string, string>> = {
  "chromium-js": DEPOSITOR_SHORTCODES[5],
  "chromium-nojs": DEPOSITOR_SHORTCODES[6],
};
/**
 * The Approved records page's arc: edit, submit, approve, discard. Its
 * own project for the same reason as `JOURNEY_SHORTCODE` — a draft is per
 * (user, project), so sharing one with another mutating spec would find that
 * spec's leftover draft state instead of a pristine project.
 */
export const APPROVED_SHORTCODE = DEPOSITOR_SHORTCODES[7];
/** Switching a row's variant ahead of the autosave. */
export const VARIANT_SHORTCODE = DEPOSITOR_SHORTCODES[8];
/** A typed value surviving a row control: the first row of every row shape holds a link. */
export const TYPED_SHORTCODE = DEPOSITOR_SHORTCODES[9];

/**
 * The project RDU creates and the depositor then edits, one per pass for the reason
 * `STATUS_SHORTCODES` gives: the second pass would find the first one's project already created.
 * Keyed by Playwright project name.
 */
export const CREATED_SHORTCODES: Partial<Record<string, string>> = {
  "chromium-js": DEPOSITOR_SHORTCODES[10],
  "chromium-nojs": DEPOSITOR_SHORTCODES[11],
};
/**
 * Created by RDU and submitted by the depositor, so its removal is refused. One
 * per pass for the reason `CREATED_SHORTCODES` gives.
 */
export const SUBMITTED_CREATED_SHORTCODES: Partial<Record<string, string>> = {
  "chromium-js": DEPOSITOR_SHORTCODES[27],
  "chromium-nojs": DEPOSITOR_SHORTCODES[28],
};
/**
 * Created and removed by RDU, one per pass for the reason `CREATED_SHORTCODES`
 * gives. Absent from the data set like them, and assigned to no depositor:
 * nobody else opens them.
 */
export const REMOVED_SHORTCODES: Partial<Record<string, string>> = {
  "chromium-js": "E2E5",
  "chromium-nojs": "E2E6",
};
/**
 * The accessibility sweep's submitted-locked and awaiting-release pages, and its
 * approved record's discard page. Not shared: submitting locks the form, so any
 * other spec would find it read-only.
 */
export const A11Y_STATES_SHORTCODE = DEPOSITOR_SHORTCODES[12];
/**
 * The refused submit: its draft holds a malformed link on purpose, so any other spec would
 * find its own save warned about that link.
 */
export const REFUSED_SUBMIT_SHORTCODE = DEPOSITOR_SHORTCODES[13];
/** A language-row line saved with no language. */
export const LANGUAGE_ROWS_SHORTCODE = DEPOSITOR_SHORTCODES[14];
/**
 * The accessibility sweep's pages that need a proposal, a draft, a submission
 * and a claim. Not shared: it is submitted and claimed. One, not one per pass:
 * `accessibility.spec.ts` runs only in `chromium-js`.
 */
export const A11Y_REVIEW_SHORTCODE = DEPOSITOR_SHORTCODES[29];
/**
 * Created by RDU for the accessibility sweep's remove page, and never removed.
 * Assigned to no depositor, like `REMOVED_SHORTCODES`; one, for the reason
 * `A11Y_REVIEW_SHORTCODE` gives.
 */
export const A11Y_CREATED_SHORTCODE = "E2E7";

/*
 * One project per pass, for the reason `STATUS_SHORTCODES` gives: each spec walks
 * a lifecycle from a published project. Read with `perPass`
 * (`tests/support/per-pass.ts`).
 */

/**
 * Discard draft and take the submission back: each ends the project's draft or
 * submission, which a spec sharing the project would still need.
 */
export const LIFECYCLE_SHORTCODES: Partial<Record<string, string>> = {
  "chromium-js": DEPOSITOR_SHORTCODES[15],
  "chromium-nojs": DEPOSITOR_SHORTCODES[16],
};
/**
 * Per-field review decisions; the approved record is read back by shortcode, so
 * it holds only this spec's edits.
 */
export const DECISIONS_SHORTCODES: Partial<Record<string, string>> = {
  "chromium-js": DEPOSITOR_SHORTCODES[17],
  "chromium-nojs": DEPOSITOR_SHORTCODES[18],
};
/**
 * Request changes, whose returned draft keeps accepted fields read-only: a
 * field another spec edits would be locked.
 */
export const CHANGES_REQUESTED_SHORTCODES: Partial<Record<string, string>> = {
  "chromium-js": DEPOSITOR_SHORTCODES[19],
  "chromium-nojs": DEPOSITOR_SHORTCODES[20],
};
/**
 * Reject; apart from `CHANGES_REQUESTED_SHORTCODES` because each outcome closes
 * the review round the other needs open.
 */
export const REJECTED_SHORTCODES: Partial<Record<string, string>> = {
  "chromium-js": DEPOSITOR_SHORTCODES[21],
  "chromium-nojs": DEPOSITOR_SHORTCODES[22],
};
/**
 * One reviewer taking over another's claim; on a shared project the claim taken
 * over would be another spec's review.
 */
export const TAKEOVER_SHORTCODES: Partial<Record<string, string>> = {
  "chromium-js": DEPOSITOR_SHORTCODES[23],
  "chromium-nojs": DEPOSITOR_SHORTCODES[24],
};
/**
 * Decisions on an entity proposal; the approved record is read back for the
 * proposed person, so only this spec's proposal may be in it.
 */
export const ENTITY_DECISIONS_SHORTCODES: Partial<Record<string, string>> = {
  "chromium-js": DEPOSITOR_SHORTCODES[25],
  "chromium-nojs": DEPOSITOR_SHORTCODES[26],
};
/**
 * Proposing a new person and organisation. Its own project, since a proposal it
 * leaves would break `proposals.spec.ts` on `PICKER_SHORTCODE`; one per pass,
 * since a proposal the other pass left would satisfy its listing assertion.
 */
export const PROPOSALS_NEW_SHORTCODES: Partial<Record<string, string>> = {
  "chromium-js": DEPOSITOR_SHORTCODES[30],
  "chromium-nojs": DEPOSITOR_SHORTCODES[31],
};

const PORT = Number(process.env.EDITOR_E2E_PORT ?? 4101);
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
    // Not "on-first-retry": the serial groups run with `retries: 0`, so a
    // failure there is never retried and would leave no trace.
    trace: "retain-on-failure",
  },

  projects: [
    // Signs in as RDU, creates the depositor through the RDU interface, signs
    // in as the depositor and the second reviewer, and saves each cookie
    // jar. A setup *project* rather than `globalSetup` deliberately: a setup
    // project runs after `webServer` is up, which `globalSetup` is not
    // guaranteed to.
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
      EDITOR_PUBLIC_DIR: "areas/deposit/editor/public",
      // No default in the config, so the editor will not start without it.
      EDITOR_DATA_DIR: DPE_DATA_DIR,
      // Unset EDITOR_DB_DIR means in-memory SQLite — a fresh database per run.
      EDITOR_RDU_EMAILS: `${RDU_EMAIL},${RDU2_EMAIL}`,
      // 1, not 0: `EditorConfig::validate` refuses a zero cooldown outright, and
      // 1 second is the floor — enough to keep a retried sign-in from tripping
      // the resend throttle.
      EDITOR_LOGIN_COOLDOWN_SECS: "1",
      // The console mailer logs codes at WARN, so `error` would hide them.
      RUST_LOG: "warn",
    },
  },
});
