import * as fs from "node:fs";
import * as path from "node:path";
import type { Page } from "@playwright/test";
import { expect, test } from "./support/test";

// The compound viewer's contents rail (`.cv-toc`) beside its reader, under a narrative, at every
// width from phone to wide desktop. The fixture project has no such page, so the page is the
// `pages_rail` snapshot in the shell `cpe-web`'s `page.rs` puts around a narrative and the viewer
// after it; keep the two in step. No script runs; layout needs none.

const SNAPSHOT = path.resolve(
  __dirname,
  "../../components/src/component/snapshots/cpe_components__component__compound_viewer__tests__snapshot_compound_viewer_pages_rail.snap",
);

// CPE's stylesheets, in the order `document_head` links them.
const STYLESHEETS = [
  "tokens",
  "shell",
  "editorial",
  "record",
  "grid",
  "media",
  "incunabula",
];

const WIDTHS = [320, 768, 861, 1024, 1180, 1280, 1440, 2200];

// The snapshot's markup, without the YAML header insta writes above it.
function viewerMarkup(): string {
  const snapshot = fs.readFileSync(SNAPSHOT, "utf8");
  const body = snapshot.replace(/^---\n[\s\S]*?\n---\n/, "");
  expect(body, "the snapshot holds the viewer's article").toMatch(
    /^<article class="compound-viewer[^"]*page"/,
  );
  return body;
}

function pageMarkup(baseURL: string): string {
  const links = STYLESHEETS.map(
    (name) =>
      `<link rel="stylesheet" href="${baseURL}/cpe/incunabula/_static/cpe/${name}.css">`,
  ).join("");
  const intro =
    "The full text is here as a linked dataset. Page through the parts and follow the links " +
    "to the records that appear in each one.";
  return `<!DOCTYPE html>
<html lang="en" data-direction="edition" data-accent="ochre" data-bg="tint" data-pick="newsreader" data-narr="text" data-narr-side="end">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">${links}</head>
<body><div class="shell"><main class="shell__main" id="main" tabindex="-1">
<article class="editorial page"><div class="narr"><header class="editorial__head"><h1>A book</h1></header>
<div class="prose editorial__intro"><p class="editorial__lede"><em>Read it part by part.</em></p><p class="editorial__open">${intro}</p></div></div></article>
${viewerMarkup()}
</main></div></body></html>`;
}

// How far the document scrolls sideways, in CSS pixels. Zero or less is no sideways scroll: where the
// vertical scrollbar takes layout space, the content is narrower than the viewport.
function sidewaysScroll(page: Page): Promise<number> {
  return page.evaluate(
    () =>
      document.documentElement.scrollWidth -
      document.documentElement.clientWidth,
  );
}

async function left(page: Page, selector: string): Promise<number> {
  const box = await page.locator(selector).boundingBox();
  expect(box, `${selector} is laid out`).not.toBeNull();
  return box?.x ?? Number.NaN;
}

test.beforeEach(async ({ page, baseURL }) => {
  expect(baseURL, "the stylesheets come from the running server").toBeTruthy();
  // Unstyled, every box starts at the body's edge and nothing overflows: the checks below
  // would pass on a page whose stylesheets did not load.
  const loaded: string[] = [];
  page.on("response", (response) => {
    if (response.ok() && response.url().endsWith(".css")) {
      loaded.push(response.url());
    }
  });
  await page.setContent(pageMarkup(baseURL ?? ""));
  expect(loaded).toHaveLength(STYLESHEETS.length);
});

for (const width of WIDTHS) {
  test(`at ${width}px the page does not scroll sideways`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    expect(await sidewaysScroll(page)).toBeLessThanOrEqual(0);
  });

  // The page description is one line, cut with an ellipsis: unbroken, it is wider than the
  // reader's column, and the column must not grow to fit it.
  test(`at ${width}px a reader line wider than its column does not widen the page`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.locator(".cv-aside__text").evaluate((el) => {
      el.textContent = "A description of the page that runs on. ".repeat(12);
    });
    expect(await sidewaysScroll(page)).toBeLessThanOrEqual(0);
  });

  // The member's properties under the reader, as a `contents` layout renders them, with a keyword
  // set (`resource_detail`'s markup) holding a hierarchical chip longer than a phone's line.
  test(`at ${width}px a keyword chip longer than the line does not widen the page`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.locator(".cv-toc__reader").evaluate((el) => {
      el.insertAdjacentHTML(
        "beforeend",
        '<div class="cv-member__props"><div class="prop"><div class="prop__label">Keyword</div>' +
          '<div class="prop__value"><ul class="keyword-set">' +
          '<li class="keyword"><span class="path">Theme</span><span class="path">Identity</span>' +
          '<span class="path">Growth</span><span class="leaf">Physical Growth</span></li>' +
          '<li class="keyword"><span class="path">Theme</span><span class="leaf">Fantasy</span></li>' +
          "</ul></div></div></div>",
      );
    });
    expect(await sidewaysScroll(page)).toBeLessThanOrEqual(0);
  });

  test(`at ${width}px the narrative's intro and the contents rail share their left edge`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 });
    const intro = await left(page, ".editorial__intro");
    const rail = await left(page, ".cv-toc");
    expect(Math.abs(intro - rail)).toBeLessThanOrEqual(1);
  });
}
