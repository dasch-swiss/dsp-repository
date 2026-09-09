import { expect, test } from "@playwright/test";

const BASE_URL = "http://localhost:4000/dpe";

// A cover image is optional by design: it is onboarded per project on request.
// 0803 (incunabula) has one; 0843 (woposs) has none, so it exercises the
// placeholder path.
const WITH_COVER = "0803";
const WITHOUT_COVER = "0843";

const COVER_URL_PREFIX = "/assets/images/";
// The hero cover sits in `figure > div.overflow-hidden`. Scoping to it keeps the
// sidebar's cc-licence badges (also served from /assets/images/) out of the way.
const HERO = "figure > div.overflow-hidden";

test.describe("Cover image: server-side fallback (DEV-7128)", () => {
  test("a project without a cover renders no <img> with JS disabled", async ({
    browser,
  }) => {
    const context = await browser.newContext({ javaScriptEnabled: false });
    const page = await context.newPage();

    await page.goto(`${BASE_URL}/projects/${WITHOUT_COVER}`);

    // The `onerror` handler cannot run here, so an <img> pointing at a missing
    // file would leave the browser's broken-image glyph on screen. The server
    // must decide instead: no cover means no <img> at all.
    const cover = page.locator(`${HERO} img`);
    await expect(cover).toHaveCount(0);

    // The placeholder stands in its place, and is actually visible rather than
    // carrying the `hidden` class it has when it sits behind an <img>.
    const placeholder = page.locator(`${HERO} > div.bg-gray-100`);
    await expect(placeholder).toHaveCount(1);
    await expect(placeholder).toBeVisible();

    await context.close();
  });

  test("a project with a cover still renders its <img> with JS disabled", async ({
    browser,
  }) => {
    const context = await browser.newContext({ javaScriptEnabled: false });
    const page = await context.newPage();

    await page.goto(`${BASE_URL}/projects/${WITH_COVER}`);

    // The positive canary for the test above: "no broken image" must not be
    // satisfied by having stopped rendering covers altogether.
    const cover = page.locator(`${HERO} img`);
    await expect(cover).toHaveCount(1);
    await expect(cover).toHaveAttribute(
      "src",
      `${COVER_URL_PREFIX}${WITH_COVER}.webp`,
    );
    await expect(cover).toBeVisible();

    await context.close();
  });

  test("the projects listing requests no cover image that 404s", async ({
    page,
  }) => {
    const notFound: string[] = [];
    page.on("response", (response) => {
      const url = response.url();
      if (url.includes(COVER_URL_PREFIX) && response.status() === 404) {
        notFound.push(`${response.status()} ${url}`);
      }
    });

    await page.goto(`${BASE_URL}/projects`);
    await page.waitForLoadState("networkidle");

    // Every cover the page asks for is one the server confirmed exists, so a 404
    // here means the render decision and the asset directory have drifted apart.
    expect(notFound).toEqual([]);

    // Guard against the assertion passing because nothing was requested at all.
    const covers = page.locator(`${HERO} img`);
    expect(await covers.count()).toBeGreaterThan(0);
  });
});
