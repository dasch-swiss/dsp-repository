// CPE's own script: the longer logic its DataStar attributes call (`cpe-components`'
// `annotation::arrival_init_expr`, and `PICKER_TOGGLE_EXPR`, `picker_click_expr` and
// `PAGES_KEYS_EXPR` in `compound_viewer`), which consume the return values. A function here
// sees neither DataStar's signals nor its actions, so it takes `el` and `evt` and returns
// values.
//
// The markup this file reads, written by the Rust side, one selector per line: a tag, a
// `.class` or an `[attribute]`/`[attribute=value]`, or a tag followed by one of the latter two.
// Element ids are passed in from Rust instead. cpe-web's test
// `cpe_js_header_selectors_occur_in_a_reader_page` renders a reader page and finds each; a line
// in this list that is not such a selector fails it.
//   .anno__entry
//   .anno__entry--page
//   a[data-anno-page]
//   [data-anno-first]
//   [data-anno-note]
//   .record__tracks
//   details
//   summary
//   .pk__panel
//   .pk__band
//   [aria-current=page]
//   a[rel=prev]
//   a[rel=next]
//   .rv
//   image-viewer
//
// Loaded with `defer` before DataStar's module script, so `window.cpe` exists when `data-init`
// runs. A `data-init` calls through `window.cpe?.`: one that throws stops the page's other
// `data-init`s.
window.cpe = {
  /**
   * The arrival on a reader page: a `#annotation-<id>` hash for an entry of the register
   * emphasises it, unless it is a page-level entry. On a `navigate` load (the referrer survives a
   * reload and Back/Forward) whose same-origin referrer is a partner page (its path, with its
   * `page=` id, or no `page=` for a book's first page), the partner link's `data-anno-note` is the
   * note. Writes nothing to the URL or storage.
   *
   * A `navigate` load also scrolls once, a frame later, when the note the caller writes is in the
   * DOM: beside a sticky reader the entry to the window's end, else, in one track, the reader's
   * head (the parent of the title). `record.css` decides both layouts.
   *
   * `el` is the record article; `ids` holds `register`, `entryPrefix`, `reader` and `title`.
   * Returns `{arrive, note}`, the entry id to emphasise and the note, each `''` when none.
   */
  arrival(el, ids) {
    const result = { arrive: "", note: "" };
    let target = null;
    try {
      target = document.getElementById(
        decodeURIComponent(location.hash.slice(1)),
      );
    } catch {
      return result;
    }
    const at = target?.closest(`#${ids.register} .anno__entry`);
    if (!at) return result;
    if (!at.classList.contains("anno__entry--page")) {
      result.arrive = at.id.slice(ids.entryPrefix.length);
    }
    const nav = performance.getEntriesByType("navigation")[0];
    const ref =
      nav?.type === "navigate" && document.referrer
        ? new URL(document.referrer)
        : null;
    if (ref?.origin === location.origin) {
      const page = ref.searchParams.get("page");
      const from = [...at.querySelectorAll("a[data-anno-page]")].find(
        (a) =>
          new URL(a.href).pathname === ref.pathname &&
          (page === null
            ? "annoFirst" in a.dataset
            : page === a.dataset.annoPage),
      );
      if (from) result.note = from.dataset.annoNote ?? "";
    }
    if (nav?.type === "navigate") {
      requestAnimationFrame(() => {
        const reader = document.getElementById(ids.reader);
        const tracks = el.querySelector(".record__tracks");
        if (reader && getComputedStyle(reader).position === "sticky") {
          at.scrollIntoView({ block: "end" });
        } else if (
          tracks &&
          getComputedStyle(tracks).gridTemplateColumns.split(" ").length === 1
        ) {
          document
            .getElementById(ids.title)
            ?.parentElement?.scrollIntoView({ block: "start" });
        }
      });
    }
    return result;
  },

  /**
   * On opening, the page picker `el` scrolls its panel so the current entry sits a third of the
   * way down. Instant, never `scrollIntoView`: that would scroll the window and drag the sticky
   * reader.
   */
  pickerToggle(el) {
    if (!el.open) return;
    const panel = el.querySelector(".pk__panel");
    const current = panel.querySelector("[aria-current=page]");
    if (current) panel.scrollTop = current.offsetTop - panel.clientHeight / 3;
  },

  /**
   * The page picker panel `el`'s one delegated click. A jump mark (`href="#…"`) scrolls the panel
   * to its group's heading below the sticky band and focuses it; the window never scrolls. Any
   * other link is an entry: the picker closes and focus returns to its summary. A modified click
   * (Ctrl, Cmd, Shift, Alt) is left to the browser, so "open in a new tab" still works.
   *
   * Returns the entry's query for the caller's `_compound` swap, or `null`: for a mark, a
   * modified click, a click outside a link, and the current page's entry, whose swap would push
   * the address already shown and leave a Back press that goes nowhere.
   */
  pickerTarget(evt, el) {
    const a = evt.target.closest("a[href]");
    if (!a || evt.ctrlKey || evt.metaKey || evt.shiftKey || evt.altKey) {
      return null;
    }
    evt.preventDefault();
    const href = a.getAttribute("href");
    if (href.startsWith("#")) {
      const heading = document.getElementById(href.slice(1));
      if (heading) {
        el.scrollTop =
          heading.offsetTop - el.querySelector(".pk__band").offsetHeight;
        heading.focus({ preventScroll: true });
      }
      return null;
    }
    const picker = el.closest("details");
    picker.open = false;
    picker.querySelector("summary").focus({ preventScroll: true });
    if (a.getAttribute("aria-current") === "page") return null;
    return a.search;
  },

  /**
   * Left and Right page the reader `el` while focus is inside it, by clicking its own
   * `a[rel=prev]` or `a[rel=next]`, so the steps' handler runs; at an inert end there is no link.
   * Stands aside for a held key (one page per press), an event already handled, any modifier
   * (Alt+Left is the browser's Back), and focus in a form control, a slider, the page picker or
   * the viewer (the canvas pans with the arrows; its shadow DOM retargets to `image-viewer`).
   */
  pageKey(evt, el) {
    if (evt.key !== "ArrowLeft" && evt.key !== "ArrowRight") return;
    if (
      evt.repeat ||
      evt.defaultPrevented ||
      evt.shiftKey ||
      evt.ctrlKey ||
      evt.altKey ||
      evt.metaKey
    ) {
      return;
    }
    if (
      evt.target.closest(
        "input,select,textarea,[contenteditable],[role=slider],details,image-viewer,.rv",
      )
    ) {
      return;
    }
    el.querySelector(
      evt.key === "ArrowLeft" ? "a[rel=prev]" : "a[rel=next]",
    )?.click();
  },
};
