import { expect, type Page } from "@playwright/test";

/** Lowercased; `pull request` is the phrase, not the bare word. */
const FORBIDDEN = ["export", "json", "transfer", "commit", "pull request"];

/**
 * The page's visible text, which is what the vocabulary rule bounds.
 *
 * `innerText` rather than `textContent`: it is the rendered text, so it omits
 * `<script>` bodies and anything hidden. With JavaScript disabled it still
 * works — the property is computed by the browser's layout, not by scripts.
 */
export async function visibleText(page: Page) {
  return (await page.locator("body").innerText()).toLowerCase();
}

/** Fails when the page's visible text holds a forbidden word; `where` names the page in the message. */
export async function expectNoForbiddenVocabulary(page: Page, where: string) {
  const text = await visibleText(page);
  // The assertion below is an absence, and an absence passes just as happily
  // when the page never rendered. Every current call site happens to assert
  // real content first; this makes the helper safe regardless of the next one.
  expect(
    text.length,
    `REQ-2.2 check read an empty page on ${where}`,
  ).toBeGreaterThan(0);
  const found = FORBIDDEN.filter((word) => text.includes(word));
  expect(found, `REQ-2.2: forbidden words on ${where}`).toEqual([]);
}
