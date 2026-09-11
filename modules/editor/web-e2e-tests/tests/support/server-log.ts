import * as fs from "node:fs";

import { SERVER_LOG } from "../../playwright.config";

/**
 * How many bytes the server log already holds.
 *
 * Take this *before* triggering a send, then pass it to {@link waitForLoginCode}
 * so only the tail written afterwards is searched. Without the offset a reused
 * server (`reuseExistingServer` is on locally) hands back a code from an earlier
 * run, and the sign-in fails with a code that was already spent.
 */
export function logOffset(): number {
  try {
    return fs.statSync(SERVER_LOG).size;
  } catch {
    return 0;
  }
}

function readFrom(offset: number): string {
  let fd: number | undefined;
  try {
    fd = fs.openSync(SERVER_LOG, "r");
    const size = fs.fstatSync(fd).size;
    if (size <= offset) return "";
    const buffer = Buffer.alloc(size - offset);
    fs.readSync(fd, buffer, 0, buffer.length, offset);
    return buffer.toString("utf8");
  } catch {
    return "";
  } finally {
    if (fd !== undefined) fs.closeSync(fd);
  }
}

/**
 * The six-digit login code the console mailer wrote after `offset`.
 *
 * The editor runs with `EDITOR_SMTP_HOST` unset, so `ConsoleMailer` writes the
 * message body to the log at WARN instead of sending it — the log is the only
 * channel a test has. The body reads "…code for the DaSCH Metadata Editor
 * is:\n\n    123456", and `tracing` escapes those newlines, so the code is
 * matched relative to the "is:" marker rather than by scanning for any six
 * digits (a timestamp or a port would also match that).
 */
export async function waitForLoginCode(
  offset: number,
  timeoutMs = 15_000,
): Promise<string> {
  const deadline = Date.now() + timeoutMs;
  let tail = "";
  while (Date.now() < deadline) {
    tail = readFrom(offset);
    const matches = [...tail.matchAll(/Editor is:(?:\\n|\s)*(\d{6})/g)];
    const last = matches.at(-1);
    if (last) return last[1];
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(
    `No login code appeared in ${SERVER_LOG} within ${timeoutMs}ms.\n` +
      `If the tail below is empty the server is logging below WARN or to another sink.\n` +
      `--- tail ---\n${tail.slice(-2000)}`,
  );
}
