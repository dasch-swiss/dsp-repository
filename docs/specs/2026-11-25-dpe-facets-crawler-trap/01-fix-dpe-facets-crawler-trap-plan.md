---
title: DPE facets stop offering crawlers an unbounded number of links
type: fix
date: 2026-11-25
author: "Ivan Subotic"
status: reviewed
repository: dasch-swiss/dsp-repository
linear: DEV-8065
---

# DPE facets stop offering crawlers an unbounded number of links

## Overview

The facets on `/dpe/projects` stop being one `<a href>` per option. They become one GET form with real checkboxes and an Apply button. With JavaScript, Datastar submits the form on every change, patches the results and sets the address bar to the canonical query string the server built. The mobile filter panel opens from a button instead of a link. A `/robots.txt` keeps crawlers off the facet query parameters, and filtered pages carry a canonical link to `/dpe/projects`.

## Problem Statement / Motivation

On 2026-11-24 a crawler burst against the project list took `repository.dasch.swiss` down for about 1.5 minutes (Traefik OOM-killed; post-mortem `docs/post-mortems/2026-11-24-repository-traefik-oom.md` in the infra repository). The facets are a crawler trap:

- every facet option is a link (`filter_checkbox_group.rs`), with its href built by toggling the option in the current query (`domain/project.rs`, `pages/projects/page.rs`);
- facets are multi-select and comma-joined (`data_language=ka,x-cpa,it,…`), so the number of distinct URLs is practically unbounded, each a subset of the same few hundred projects;
- the mobile panel is a link to `?dialog=true`, which doubles that URL space;
- `/robots.txt` is 404, and no page has a canonical link.

Filtered views must stay shareable and bookmarkable through the query string, and a filtered URL opened directly must render on the server.

## Proposed Solution

1. **Query contract (dpe-web, dpe-server, shared-datastar).** `ProjectQuery` is parsed from the raw query string by a hand-written parser that accepts both repeated keys (what a checkbox form submits) and comma-joined values (every URL bookmarked so far). Its canonical form, `to_query_string`, joins multi-select values with commas, percent-encodes every value, and never emits `dialog`. A predicate `has_filter_params()` says whether a facet, `search` or `dialog` is set. `shared_datastar::Get` gains `.form()`.
2. **One facet form (dpe-web).** The four facet groups become `<fieldset>`s of `<label>` + `<input type="checkbox" name value>` in a single `<form id="project-filters" method="get" action="/dpe/projects">`, with an Apply submit button and the current search as a hidden input. The form is rendered once: it is the sidebar at `lg` and the slide-in panel below `lg`. The mobile trigger is a submit button carrying `dialog=true`. No link from the unfiltered page carries a facet value or `dialog`; the only links that still do are the pagination links of an already filtered page, which get `rel="nofollow"` and are disallowed by robots.txt.
3. **Datastar enhancement (dpe-web, dpe-server).** `data-on:change` and `data-on:submit__prevent` on the form run `@get('/dpe/projects/results', {contentType: 'form', retry: 'never', retryMaxCount: 0})`. The new fragment handler patches the results region, the result count and the "Clear all" slot, then sends an `ExecuteScript` with `history.replaceState` to `/dpe/projects` plus the canonical query string. The server is the only writer of the URL. The mobile panel opens and closes on a client-only signal.
4. **Crawler hints (dpe-web public, dpe-server).** `areas/access/dpe/public/robots.txt` disallows each facet parameter, `search`, `dialog` and the two fragment routes, but not `page`. Filtered pages get `<link rel="canonical" href="<public base URL>/dpe/projects">`.

## Alternative Approaches Considered

| Option | Why not |
|---|---|
| Keep the anchors, add `rel="nofollow"` | The links stay; the incident crawler ignores hints, and a well-behaved one still discovers every combination. Owner decision 2026-11-25. |
| Drop the no-JS fallback | ADR-0004 requires a working fallback for a Datastar-enhanced control. An Apply button costs nothing and is no link. |
| `<meta name="robots" content="noindex">` on filtered pages | A robots-disallowed URL is never fetched, so its noindex is never read. Owner decision 2026-11-25, which overrides the issue's noindex item. The canonical link is read under the same condition; it is kept for crawlers that ignore robots.txt but honour canonicals, and for link-preview fetchers, not for compliant search crawlers. |
| Drop the facets from pagination links on a filtered page | Paging a filtered list must keep the filter. These links exist only on filtered pages, which no link from the unfiltered page reaches; `rel="nofollow"` and robots.txt cover them. |
| Hash fragments, POST for filtering, a share token, 410/404 for old query URLs | Each breaks bookmarking, direct server rendering of a filtered URL, or GET semantics. Owner decision 2026-11-25. |
| Datastar Pro `data-replace-url` | Not in open-source Datastar. `ExecuteScript` with `history.replaceState` does the same, as CPE's reader already does (`areas/access/cpe/server/src/router.rs`, the `pushState` script). |
| `axum_extra::extract::Query` (`serde_html_form`) for repeated keys | A new dependency. `serde_urlencoded` cannot build a sequence from repeated keys, and `url::form_urlencoded` is already a workspace crate. |
| `cpe_core::util::js_string_escape` for the `replaceState` script | DPE may not depend on CPE (`areas/access/server/CLAUDE.md`, Dependency boundary). A canonical query string whose values are all percent-encoded holds no character that needs escaping in a single-quoted JS string, so no escaper is needed. |
| Native `<dialog>` with `showModal()` for the mobile panel | Opening it needs script, so the no-JS path would lose the panel. |
| `<details>`/`<summary>` for the mobile panel | One form cannot be a closed disclosure below `lg` and always open at `lg` without `::details-content` styling, which not every supported browser has, or two copies of the form, whose checkbox states would drift. |
| Mosaic `checkbox_group` for the facet groups | Its group shell (legend plus `field-choices`, form-field styling) changes the sidebar look, and it has no slot for the access-rights info link. The facet rows use the same native pattern (`fieldset`, `legend`, `label for`, `input type="checkbox" name value`) locally. Whether a facet group becomes a Mosaic tile is H3. |
| `Disallow: /dpe/projects?` in robots.txt | It also blocks `?page=N`, and the paginated project list must stay crawlable. |
| Patch the whole form on each change | The morph would replace the focused checkbox. The fragment patches only the results region and the "Clear all" slot, so focus stays where it is. |

## Technical Considerations

**Design context.** No Claude Design artefact exists for this change, and none is needed: the current rendering is the visual design context, and the facet rows keep their look. The only new visible controls are the Apply button (hidden with JavaScript) and the mobile trigger and Close as buttons instead of links, all with existing DPE button classes (`btn btn-outline`, `btn btn-ghost`).

**Query parsing.** `ProjectQuery::from_query_str(&str)` in `areas/access/dpe/web/src/domain/project.rs`, on `url::form_urlencoded::parse` (add `@crates//:url` to `areas/access/dpe/web/BUILD.bazel`; `check-capability-deps` allows it for a web crate). Rules:

- `type_of_data`, `data_language`, `access_rights`: every occurrence is split on `,`, empty parts are dropped, duplicates are removed keeping first-seen order; the field stays the existing comma-joined `Option<String>` (`None` when empty), so the filtering in `domain/projects.rs` is untouched.
- `ongoing`, `finished`, `dialog`: `true` when the last occurrence is `true`, else unset. `page`: the last occurrence parsed as a positive integer, else unset. `search`: the last occurrence.
- Unknown keys are ignored. A malformed value is ignored rather than answered with 400, so a mangled bookmark still renders.
- `+` decodes to a space (`form_urlencoded` does this; `urlencoding::decode` does not).

A `%2C` decodes to a comma before splitting. The `dialog` field stays on `ProjectQuery` (only `with_dialog` goes). `has_filter_params()` is true when any multi-select field, `ongoing`, `finished`, `search` or `dialog` is set; `page` alone does not count.

The only `Query<ProjectQuery>` extractor, in `projects_page_handler` (`areas/access/dpe/server/src/shell.rs`), takes `axum::extract::RawQuery` and calls `from_query_str`. The fuzz shim `areas/access/dpe/server/fuzz/shims/query_params.rs` follows.

**Canonical query string.** `to_query_string` keeps its parameter order (`ongoing`, `finished`, `search`, `page` when above 1, `type_of_data`, `data_language`, `access_rights`), percent-encodes every value with `urlencoding::encode` (each value on its own, before joining, so the separator stays a literal `,` and never becomes `%2C`), joins multi-select values with a literal `,`, and, from Phase 2 on, never emits `dialog`. Phase 1 still emits it, because the mobile link that needs it goes only in Phase 2, and each layer of the stack keeps a working no-JS mobile panel. Its output therefore holds only `A-Z a-z 0-9 - _ . ~ % , & = ?`. No facet option value contains a comma; a test over the committed corpus pins that.

**URL authority.** Only the server writes the URL: the fragment handler sends `history.replaceState(null, '', '/dpe/projects<canonical query string>')`, a relative URL (an absolute one on another origin throws `SecurityError`). Filtering adds no history entries; Back leaves the page.

**Form and fragment.** The form omits `page`, so any change starts at page 1, and carries `autocomplete="off"`, so a Back navigation restored from the browser cache shows the boxes the server rendered for the URL, not stale ones. The fragment route is `GET /dpe/projects/results`, registered in `areas/access/dpe/server/src/router.rs` beside `/dpe/projects/search` (a static segment, so it wins over `/dpe/projects/{id}`). Its handler in `fragments.rs` parses the request's own query string (Datastar puts the form fields there for a GET; any other key, such as a `datastar` parameter, is ignored), renders with the same functions the page uses, and sends four events: `PatchElements` for `#project-results` (outer), `PatchElements` for `#project-filters-clear` (outer; the span renders empty when no facet is set, so the slot never disappears), `PatchElements` for `#project-results-count` (inner, so the live-region node itself survives), and the `ExecuteScript`. It carries the same `tracing::instrument` fields as the search fragment. The action is `datastar::get("/dpe/projects/results").form().retry_never()`, rendering exactly `@get('/dpe/projects/results', {contentType: 'form', retry: 'never', retryMaxCount: 0})`: one options object. On a fetch error or `retries-failed`, the form's `data-on:datastar-fetch` calls `el.submit()`, a native submission that bypasses the `submit` listener. Datastar v1.0.2 registers `datastar-fetch` handlers on `document`, so every fetch on the page reaches it: the expression is guarded with `evt.detail.el === el`, as the tab container guards with `evt.detail.el.closest('#project-tabs')`. The visitor gets the full page instead of a dead control (the tab pattern in `docs/src/dpe/architecture.md`, applied to a form); that submission carries no `dialog`, so an open mobile panel closes.

**Search and facets.** The hidden `search` input holds the search the server applied for this URL; text typed into the search box but not submitted does not reach the facet form. Submitting the search form still drops the facets, as today, and "Clear all" still goes to plain `/dpe/projects`, dropping the search too; both are unchanged behaviour.

**Results region.** A new `project_results(query, ctx)` in `pages/projects/components/` wraps the existing list and pagination in `div#project-results`. Outside it, between the search card and the results, sits `p#project-results-count[role="status"][aria-live="polite"]` with "N projects", so the patch changes its text and never replaces the node. The page and the fragment render both from the same functions. A full-page fallback announces nothing; the page title and list are the feedback.

**Pagination on a filtered page.** `project_pagination` keeps the facets in its links, so paging keeps the filter, and adds `rel="nofollow"` to every link when `has_filter_params()` is true.

**Mobile panel.** One container holds the form. Below `lg` it is a fixed slide-in panel (`max-lg:fixed …`), hidden with `max-lg:hidden` unless open; at `lg` it is the static sidebar. It is a non-modal disclosure, as today: no `role="dialog"`, no focus trap; the backdrop is a visual dimmer and a click target only. Without JavaScript, "open" is `dialog=true` from the request: the trigger is `<button type="submit" form="project-filters" name="dialog" value="true">` with `aria-expanded` rendered from `dialog`, Close is `<button type="submit" form="project-filters">` with `autofocus` when the panel renders open, and both Close and Apply submit without `dialog`, so they apply the selection and close the panel (intended). With JavaScript, `data-signals` seeds `_filters_open` from `dialog`; the trigger and Close (both `data-on:click__prevent`, so neither submits) and the backdrop (a `div`, no longer a link, always rendered and `lg:hidden`) set it; `data-class="{'max-lg:hidden': !$_filters_open}"` toggles the panel and the backdrop; Escape (`data-on:keydown__window`) closes it only while `$_filters_open` is true. A `dialog=true` bookmark opened at `lg` renders the normal sidebar. Every class name is a complete literal (Tailwind scans the Rust sources as text).

**Apply button.** `data-show="false"` hides it once Datastar has started; until then (a few hundred milliseconds on load) it is visible. Accepted: a CSS-only `js` class would need a script in the head for one button.

**Crawler hints.** `areas/access/dpe/public/robots.txt` is served at `/robots.txt` by the existing `ServeDir` fallback, and the `//areas/access/dpe:public` filegroup globs it into the image and the tests with no Bazel change. Content:

```
User-agent: *
Disallow: /dpe/projects?*ongoing=
Disallow: /dpe/projects?*finished=
Disallow: /dpe/projects?*search=
Disallow: /dpe/projects?*type_of_data=
Disallow: /dpe/projects?*data_language=
Disallow: /dpe/projects?*access_rights=
Disallow: /dpe/projects?*dialog=
Disallow: /dpe/projects/results
Disallow: /dpe/projects/search
```

The file names no CPE path: the hidden-projects plan (`docs/specs/2026-11-19-hidden-projects/01-feat-hidden-projects-plan.md`) keeps hidden CPE projects out of `robots.txt` so the file never advertises them, and ADR-0007 leaves the site root to DPE. `*` in a path is a Google and Bing extension, not RFC 9309; a crawler that does not support it ignores those lines. The search dropdown's `?search=` links (`fragments.rs`) arrive only through the search fragment and are covered by `*search=`. The tab fragment `/dpe/projects/{id}/tab/{tab}` is fetched by Datastar from tab links whose `href` is the crawlable `?tab=` page, so it needs no rule.

The canonical link goes into `view::HeadExtras` in `areas/access/dpe/server/src/shell.rs` (today an empty `html! {}`), as an absolute URL from `AppState.public_base_url`, when `has_filter_params()` is true, page number or not. An unfiltered or page-only URL gets none.

**Datastar gate.** `data-on:change`, `data-on:submit__prevent`, `data-on:datastar-fetch`, `data-on:keydown__window`, `data-on:click__prevent`, keyless `data-class`, `data-signals`, `data-show` and keyed `data-attr:aria-expanded` all fit the grammar in `.github/scripts/check-datastar-attributes.sh`; the actions come from `shared_datastar`.

### Accessibility

| Control or view | Design-system component | Keyboard path | Name / role / state | State choice | Announcements |
|-----------------|-------------------------|---------------|---------------------|--------------|---------------|
| Facet group | Native `fieldset` + `legend` (DPE `dpe-subtitle`); no Mosaic tile (see Alternatives) | Tab enters the group's first checkbox | Group named by its legend | Always enabled | None |
| Facet option | Native `label for` + `input type="checkbox"` | Tab per checkbox, Space toggles; after the patch focus stays on the toggled checkbox (the form is not morphed) | Name: the option label; role checkbox; state: `checked` | Always enabled | Result count through `#project-results-count` |
| Access-rights info link | Existing link, kept | In Tab order after the legend | "More information", opens in a new tab | Enabled | None |
| Apply button | `btn` | After the last group; hidden with JavaScript (`data-show="false"`) | "Apply filters"; native submit | Hidden (`display: none`, out of the accessibility tree), not disabled, with JavaScript; visible without | Full page load |
| Clear all | Existing link to `/dpe/projects` | In the header, before the groups | "Clear all" | Shown when any facet is set; the fragment patches its slot | Full page load |
| Results region | Existing project list and pagination | Not focused by the patch | `#project-results` | — | None itself; see the count |
| Result count | `p` with `role="status"`, outside `#project-results` | Not focusable | "N projects" | Always rendered | `aria-live="polite"`; the patch replaces its text, not the node |
| Mobile Filters trigger | `btn btn-outline` button | Focusable below `lg`; on open, focus moves to the panel (`tabindex="-1"`) | "Filters"; `aria-controls="project-filters-panel"`, `aria-expanded` rendered from `dialog` and then bound to `_filters_open` | Hidden at `lg` | None |
| Mobile panel | App-local slide-in, as today; non-modal | Escape (only while open) and Close return focus to the trigger; without JavaScript, Close has `autofocus` when the panel renders open | Region labelled by its "Filters" heading | Hidden below `lg` when closed; never hidden at `lg` | None |
| Backdrop | `div`, was a link | Not focusable, `aria-hidden="true"` | None | Always rendered; `lg:hidden`, and `max-lg:hidden` unless the panel is open, toggled with the panel | None |
| Close filters | `btn btn-ghost` button | Inside the panel, first control | "Close filters" | Shown in the open panel | None |

Visible focus: checkboxes, the trigger, Close and Apply keep a visible focus indicator (the browser default or the existing `btn` focus style); the panel shows none, since it is a programmatic focus target. Contrast: label and button text at least 4.5:1, checkbox border and focus indicator at least 3:1 against their background, checked by `just test-a11y-dpe` on `/dpe/projects?ongoing=true`. Target size: each label row at least 24 px high, so the label enlarges the checkbox's hit area. Reflow: the panel is full width below `md` today and stays so; at 320 px width and 400 % zoom the form scrolls inside the panel with no horizontal scroll.

## Implementation Phases

#### Phase 1: Query contract

depends on: none

- [ ] Tests first in `areas/access/dpe/web/src/domain/` for `ProjectQuery::from_query_str`: repeated keys and comma-joined values give the same query; mixed forms merge; `+` decodes to a space; `%2C` splits like a comma; empty parts and duplicates are dropped; malformed `page`, `ongoing`, `finished` and unknown keys (a `datastar` parameter among them) are ignored. They catch the 400 a checkbox form's repeated keys would get from `Query<ProjectQuery>`.
- [ ] Implement `ProjectQuery::from_query_str` per Technical Considerations, Query parsing; add `@crates//:url` to `areas/access/dpe/web/BUILD.bazel`.
- [ ] Tests first, then `ProjectQuery::has_filter_params()`: true for each facet, `ongoing`, `finished`, `search` and `dialog`; false for the default query and for `page` alone.
- [ ] Tests first for `to_query_string`: every value percent-encoded (a value with a space, `'`, `<` and `&`), multi-select joined with a literal `,`, output limited to the character set in Technical Considerations. They catch a raw value reaching an `href` or a script.
- [ ] Change `to_query_string` to pass those tests, still emitting `dialog`, and update the existing tests that assert its raw output.
- [ ] A test over the committed test corpus that no type-of-data, data-language or access-rights option value contains a comma; it catches an option the comma-joined form could not round-trip.
- [ ] Replace every `Query<ProjectQuery>` extractor in `areas/access/dpe/server/src/` with `RawQuery` plus `from_query_str`; add a test in the `tests` module of `areas/access/dpe/server/src/router.rs` that `GET /dpe/projects?type_of_data=a&type_of_data=b` answers 200 (it is 400 today) and update `areas/access/dpe/server/fuzz/shims/query_params.rs`.
- [ ] Add `Get::form()` to `shared/datastar/src/lib.rs`, emitting `{contentType: 'form'}`, with tests beside `Post::form`'s that `.form().retry_never()` renders exactly `@get('/dpe/projects/results', {contentType: 'form', retry: 'never', retryMaxCount: 0})`, one options object, in either call order.
- [ ] Verify: `bazel test //areas/access/dpe/... //shared/datastar/...`, then the gate `just check && just test`.
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: One facet form, no facet links

depends on: Phase 1 (the form submits repeated keys, which only `from_query_str` accepts)

- [ ] Tests first in `filter_checkbox_group.rs`, `project_filters_content.rs`, `mobile_filters_button.rs`, `project_pagination.rs` and `page.rs`: the filter form and the mobile trigger hold no `<a` whose `href` contains `/dpe/projects?`; the whole unfiltered page holds no `href` carrying a facet parameter or `dialog=`, with `dialog=true` set too; pagination links on a filtered page keep the facets and carry `rel="nofollow"`, on an unfiltered page they carry no `rel`; the form is `id="project-filters" method="get" action="/dpe/projects" autocomplete="off"`; four `fieldset`s with `legend`s; each checkbox has `id`, `name`, `value`, a `label for` and `checked` when set; status options are `name="ongoing" value="true"` and `name="finished" value="true"`; data-language values are codes and labels are display names; the hidden `search` input appears only when a search is set; there is an Apply submit button; `#project-filters-clear` renders, empty when no facet is set, and otherwise holds exactly one link, `href="/dpe/projects"`, "Clear all"; the form has no `page` input; the access-rights legend keeps its "More information" link with `target="_blank"`; the form is rendered once per page; with `dialog=true` the panel renders open, the trigger has `aria-expanded="true"` and `aria-controls="project-filters-panel"`, the panel has `id="project-filters-panel"`, `tabindex="-1"` and `aria-labelledby` its "Filters" heading, the backdrop is a `div` with `aria-hidden="true"` and `lg:hidden` and Close has `autofocus`; without `dialog` the panel and the backdrop render with `max-lg:hidden`, so the JavaScript path can show both without a request; no checkbox, trigger, Close or Apply carries `outline-none` or `focus:outline-none`, so the focus indicator stays visible. They catch a facet link coming back, a second form whose state drifts and a lost focus ring.
- [ ] Rewrite `filter_checkbox_group` to the native fieldset pattern, taking a UI-local `FilterOption { label, value, checked }` instead of `(label, checked, href)`.
- [ ] Rewrite `project_filters_content` as the form: header with the "Clear all" link in `span#project-filters-clear`, the four groups, hidden `search`, Apply button.
- [ ] In `domain/project.rs`, replace `status_filter_items` and `access_rights_filter_items` with option builders that carry values, delete the `with_*_toggled` methods and `with_dialog` once nothing calls them, with their tests, and stop `to_query_string` emitting `dialog`, with a test.
- [ ] Rewrite `page.rs` and `mobile_filters_button.rs`: one container renders the form, static sidebar at `lg`, slide-in panel below `lg` that is open when `dialog=true`; trigger and Close are the submit buttons from Technical Considerations, Mobile panel, with the panel attributes the tests above name; the backdrop is a `div`; `project_filters.rs` is removed, with its entry in `pages/projects/components/mod.rs`, if the container replaces it. The public signature `projects_page(query, ctx)` stays unchanged, so `shell.rs` needs no edit in this unit.
- [ ] Add `rel="nofollow"` to the pagination links when `has_filter_params()` is true, in `project_pagination.rs`, with tests for a facet, for `search` alone and for `dialog` alone.
- [ ] Add `just test-e2e-dpe` beside `test-a11y-dpe` in `justfile`, built the same way, running `npx playwright test tests/project-filters.spec.ts --project=chromium --project=chromium-no-js`. In `areas/access/dpe/web-e2e-tests/playwright.config.ts`, add a `chromium-no-js` project (`devices["Desktop Chrome"]`, `javaScriptEnabled: false`, `testMatch: /project-filters\.spec\.ts/`, `grep: /@no-js/`) and give the existing `chromium` project `grepInvert: /@no-js/`; the no-JS tests carry the `@no-js` tag. `just test-a11y-dpe` stays unchanged and still runs `accessibility.spec.ts` under `chromium`. Add a step running `just test-e2e-dpe` to `.github/workflows/a11y-dpe.yml` after the accessibility step.
- [ ] Write the JS-off tests, tagged `@no-js`, in `tests/project-filters.spec.ts`: check a box and Apply filters the list and puts the parameter in the URL; a comma-joined URL opened directly renders with its boxes checked; Apply with a search set keeps the search; on a filtered page, following the page-2 link keeps the filter; at a 375 px viewport the Filters button opens the panel and Close closes it. They catch a dead no-JS path.
- [ ] Add `{ name: "Projects listing, filtered", path: "/dpe/projects?ongoing=true" }` to `tests/accessibility.spec.ts`, plus a case that scans `/dpe/projects?dialog=true` at a 375 px viewport, so the open mobile panel is checked too.
- [ ] Document `just test-e2e-dpe`, the `chromium-no-js` project and the `@no-js` tag in `docs/src/dpe/testing-strategy.md` (E2E layer) and in the Playwright line of `areas/access/dpe/CLAUDE.md`.
- [ ] Verify: `bazel test //areas/access/dpe/...`, `just test-e2e-dpe`, `just test-a11y-dpe`, then the gate `just check && just test`.
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: Datastar enhancement

depends on: Phase 2 (the fragment patches the form's `#project-filters-clear` slot and the trigger targets the form's container); Phase 1 (`Get::form()` and the canonical query string)

- [ ] Tests first in `areas/access/dpe/server/src/fragments.rs` for `GET /dpe/projects/results`: `text/event-stream`; an outer patch for `#project-results`, an outer patch for `#project-filters-clear` (an empty span when no facet is set), an inner patch for `#project-results-count`; an `ExecuteScript` with `history.replaceState(null, '', '/dpe/projects?…')` equal to the canonical query string, which holds no `page` even when the request carries `page=3`; repeated keys accepted; the response is the fragment, not a project page, so `/dpe/projects/{id}` does not capture `results`; a value holding `'` and `</script>` appears in the script only percent-encoded. They catch a URL the server did not build and a script injection.
- [ ] Add `project_results` (the results region) and the `role="status"` count outside it, export them from `pages/projects/components/mod.rs`, and render both from `page.rs`.
- [ ] Add the fragment handler and route per Technical Considerations, Form and fragment.
- [ ] Add to the form: `data-on:change` and `data-on:submit__prevent` with `datastar::get("/dpe/projects/results").form().retry_never()`, the `data-on:datastar-fetch` fallback to `el.submit()` guarded by `evt.detail.el === el`, and `data-show="false"` on the Apply button.
- [ ] Add the mobile panel's behaviour per Technical Considerations, Mobile panel: `data-signals` seeding `_filters_open` from `dialog`; `data-attr:aria-expanded` bound to it on the trigger; `data-class` toggling `max-lg:hidden` on the panel and on the backdrop; trigger click opens and moves focus to the panel; Close (`data-on:click__prevent`, so it neither submits the form nor fetches) and backdrop clicks close and return focus to the trigger; `data-on:keydown__window` closes on Escape only while `$_filters_open` is true, and returns focus to the trigger.
- [ ] Write the JS-on tests in `tests/project-filters.spec.ts`: toggling a box updates the list without a navigation; the URL then equals the canonical form and a reload shows the same state; focus stays on the toggled box; the count region reads the new count and is the same DOM node as before the patch; the Apply button is hidden; after filtering, opening a project and pressing Back, the checked boxes match the URL; at 375 px the trigger opens the panel with `aria-expanded="true"` and no network request (`page.on('request')`), focus is in the panel, Escape closes it and focus returns to the trigger, the same holds for Close (which sends no request) and for a backdrop click, and Escape with the panel closed changes nothing; at 320 px with the panel open the page has no horizontal scroll (`scrollWidth <= clientWidth`) and every facet label row is at least 24 px high; with `/dpe/projects/results` answering 500 (`page.route`), toggling a box falls back to a full page load with the filter applied. They catch focus loss, a stale URL and a dead control.
- [ ] Update `docs/src/dpe/architecture.md` (the filter form pattern, the server-written URL, and the degradation rule extended to forms: a Datastar-enhanced form has a real `action` and a submit button) and the fragment list in `areas/access/dpe/CLAUDE.md`.
- [ ] Verify: `bazel test //areas/access/dpe/... //areas/access/server/...`, `just test-e2e-dpe`, `just test-a11y-dpe`, then the gate `just check && just test`.
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: Crawler hints

depends on: Phase 1 (the canonical link is decided by `has_filter_params()`, and it goes into the handler whose extractor Phase 1 changes)

- [ ] Tests first in a new `#[cfg(test)] mod tests` in `areas/access/dpe/server/src/shell.rs` (a file the `form` unit does not edit), building the router as the `router.rs` tests do but with the real public directory (`//areas/access/dpe:public`, already in `_TEST_DATA` of `areas/access/dpe/server/BUILD.bazel`) instead of `NO_PUBLIC_DIR`: `GET /robots.txt` answers 200 `text/plain` with each `Disallow` line of Technical Considerations, Crawler hints, no line that disallows `/dpe/projects?page=` or `/dpe/projects?` alone, and no line naming `/cpe`; a request with any facet, `search` or `dialog` renders `<link rel="canonical" href="<public base URL>/dpe/projects">`; `/dpe/projects` and `/dpe/projects?page=2` render none; no projects page renders `<meta name="robots"`. They catch the 404 robots.txt and a canonical on the paginated list.
- [ ] Add `areas/access/dpe/public/robots.txt`.
- [ ] Add the canonical link to the projects handler's `HeadExtras` in `areas/access/dpe/server/src/shell.rs`, decided by `has_filter_params()`. This phase changes no file under `areas/access/dpe/web/`, so it cannot collide with the `form` unit.
- [ ] Document robots.txt and the canonical link in a new "Crawlers" section of `docs/src/dpe/operations.md`, after the configuration table, stating that the file names no `/cpe` path because hidden projects stay out of it (the `ACCESS_HIDDEN_PROJECTS` row).
- [ ] Verify: `bazel test //areas/access/dpe/... //areas/access/server/...` (the `serve.rs` parity test covers `/robots.txt` with CPE mounted), then the gate `just check && just test`.
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Threads

Up to 2 units in parallel: `form` and `hints` both start from `query` and touch disjoint files.

- `query`: `areas/access/dpe/web/src/domain/project.rs` (and `domain/projects.rs` tests), `areas/access/dpe/web/BUILD.bazel`, `areas/access/dpe/server/src/shell.rs` (extractor only), `areas/access/dpe/server/src/router.rs` (tests only), `areas/access/dpe/server/fuzz/shims/query_params.rs`, `shared/datastar/src/lib.rs`.
- `form`: `areas/access/dpe/web/src/domain/project.rs`, `areas/access/dpe/web/src/pages/projects/**`, `areas/access/dpe/server/src/{fragments.rs,router.rs}`, `areas/access/dpe/web-e2e-tests/**`, `justfile`, `.github/workflows/a11y-dpe.yml`, `docs/src/dpe/{architecture.md,testing-strategy.md}`, `areas/access/dpe/CLAUDE.md`.
- `hints`: `areas/access/dpe/server/src/shell.rs` (canonical link and its tests), `areas/access/dpe/public/robots.txt`, `docs/src/dpe/operations.md`.

No other doc needs a change: `docs/src/dpe/project_structure.md` ("projects (with filters and pagination)") still holds, and neither `ARCH-MAP.md` nor `CONTEXT.md` names the facet files.

```yaml
plan_branch: plan/2026-11-25-dpe-facets-crawler-trap
max_parallel: 2
units:
  - unit: query
    phases: [1]
    depends_on: []
    base: plan
    branch: impl/dpe-facets-query
  - unit: form
    phases: [2, 3]
    depends_on: [query]
    base: query
    branch: impl/dpe-facets-form
  - unit: hints
    phases: [4]
    depends_on: [query]
    base: query
    branch: impl/dpe-facets-hints
  - unit: final
```

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Read the plan PR, then answer the start card | Ivan | before start | Accepting the plan is the owner's |
| H2 | Run `gh stack merge --rebase <top PR>` on the plan's top PR | Ivan | after ship | Merging is the owner's |
| H3 | Decide whether the facet group (fieldset, legend, info link, checkbox rows) becomes a Mosaic tile | Ivan | after ship | A design-system addition is the owner's call |
| H4 | One week after the deploy, compare the access logs' request count on `/dpe/projects?` with facet parameters against the week before the incident | Infra (Sämi) | after ship | Needs production log access |

## Requirements

- REQ-1 [B] (Ubiquitous): The projects page shall render the facets as one `<form method="get" action="/dpe/projects">` with one checkbox per facet option, named after its query parameter. Check: `page.rs` and `project_filters_content.rs` unit tests.
- REQ-2 [B] (Ubiquitous): The unfiltered projects page shall contain no `<a>` whose `href` carries a facet value or `dialog`, and the facet form and mobile trigger shall contain no link to `/dpe/projects?` on any page. Check: `page.rs` unit tests, with and without facets and `dialog=true`.
- REQ-3 [B] (Event-driven): When a visitor opens a filtered URL with comma-joined or repeated-key values, the DPE shall render the filtered list with those options checked. Check: `from_query_str` unit tests, the `dpe-server` repeated-key test, the JS-off E2E test.
- REQ-4 [B] (Event-driven): When a visitor without JavaScript submits the facet form, the DPE shall render the list filtered by the checked options. Check: JS-off E2E test.
- REQ-5 [B] (Event-driven): When a visitor submits the facet form while a search is set, the DPE shall keep the search. Check: JS-off E2E test.
- REQ-6 [B] (Event-driven): When the facet selection changes, the DPE shall show page 1 of the result. Check: form unit test (no `page` input) and fragment test.
- REQ-7 [B] (Complex): While JavaScript is enabled, when a visitor toggles a facet checkbox, the DPE shall replace the results region without a page navigation. Check: JS-on E2E test.
- REQ-8 [B] (Complex): While JavaScript is enabled, when the results region is replaced, the DPE shall set the address bar to `/dpe/projects` plus the canonical query string the server built. Check: fragment test and JS-on E2E reload test.
- REQ-9 (Ubiquitous): The canonical query string shall percent-encode every value and join multi-select values with commas, in the fixed parameter order. Check: `to_query_string` unit tests.
- REQ-10 (Unwanted-behaviour): If a query value holds a character outside the unreserved set, then the `replaceState` script shall carry it only percent-encoded. Check: fragment test with `'` and `</script>`.
- REQ-11 [B] (Unwanted-behaviour): If the results request fails, then the facet form shall fall back to a native submission. Check: JS-on E2E test with the fragment answering 500.
- REQ-12 [B] (Event-driven): When a visitor without JavaScript activates the mobile Filters button, the DPE shall render the page with the filter panel open. Check: JS-off E2E test at 375 px.
- REQ-13 [B] (Complex): While JavaScript is enabled, when a visitor activates the mobile Filters button, the DPE shall open the filter panel without a request. Check: JS-on E2E test at 375 px.
- REQ-14 [B] (Event-driven): When a crawler requests `/robots.txt`, the DPE shall answer 200 with a `Disallow` rule for each facet parameter, `search`, `dialog`, `/dpe/projects/results` and `/dpe/projects/search`, and with no rule matching `/dpe/projects?page=N`. Check: `dpe-server` robots test.
- REQ-15 [B] (Event-driven): When the projects page is requested with a facet, `search` or `dialog` parameter, the DPE shall render `<link rel="canonical">` to the absolute `/dpe/projects` URL. Check: `dpe-server` canonical tests.
- REQ-16 (State-driven): While the projects page is requested without a facet, `search` or `dialog` parameter, the DPE shall render no canonical link. Check: `dpe-server` canonical tests for `/dpe/projects` and `?page=2`.
- REQ-17 (Ubiquitous): Each facet group shall be a `fieldset` named by its `legend`, and each facet checkbox shall be named by a `label`. Check: unit tests, and zero axe violations from `just test-a11y-dpe` on the filtered page.
- REQ-18 (Event-driven): When the results region is replaced, the DPE shall leave keyboard focus on the toggled checkbox. Check: JS-on E2E `toBeFocused()`.
- REQ-19 (Event-driven): When the results region is replaced, the DPE shall announce the result count through a polite live region. Check: JS-on E2E test on `#project-results-count`.
- REQ-20 (Event-driven): When the mobile filter panel closes through Escape or Close, the DPE shall return focus to the Filters button, whose `aria-expanded` reflects the panel's state. Check: JS-on E2E test at 375 px.
- REQ-21 (State-driven): While a facet, `search` or `dialog` is set, the projects page shall mark every pagination link `rel="nofollow"`. Check: `project_pagination.rs` unit test.

## Out of Scope

- CPE's facet panel (`areas/access/cpe/components/src/component/facet_panel.rs`) has the same trap. Follow-up Linear issue in DEV, filed by `final`.
- Rate limiting in Traefik: a separate action of the post-mortem.
- `<meta name="robots" content="noindex">` and `rel="nofollow"` on facets: replaced by robots.txt, the canonical link and the removal of the links (owner decision 2026-11-25).
- Redirecting repeated-key URLs to the comma-joined form: both render; only the JavaScript path rewrites the URL.
- Running the other DPE Playwright specs (`tab-switching.spec.ts`, `cover-image.spec.ts`, `accessibility-tabs.spec.ts`) in CI. `just test-e2e-dpe` runs only `project-filters.spec.ts`; the rest join it in a follow-up Linear issue in DEV, filed by `final`.
- The search input's own form and its live dropdown, beyond the hidden `search` input in the facet form.

## Normative Sources

1. The owner's decisions of 2026-11-25 (coordinator brief for DEV-8065): GET form with real checkboxes, Apply button, Datastar `@get` plus `replaceState` by the server, no hash, POST, share token or 410/404, mobile panel without links, robots.txt without `page`, canonical to `/dpe/projects`, no noindex, DPE only.
2. `docs/adr/0004-hypermedia-frontends.md` (progressive enhancement, JS on and off E2E, GET never writes).
3. Linear DEV-8065; where it differs from 1 (noindex, nofollow, dropping the no-JS fallback), 1 wins.
4. `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md` (the site root and its `ServeDir` fallback are DPE's) and `docs/specs/2026-11-19-hidden-projects/01-feat-hidden-projects-plan.md` (no `robots.txt` entry for a hidden CPE project).
5. `docs/src/dpe/architecture.md` (Datastar conventions) and `areas/access/dpe/CLAUDE.md`.
6. `eng/references/accessibility-standards.md`.

## Dependencies & Risks

- The incident crawler used rotating user agents and probably ignores robots.txt. Removing the links still removes the trap for anything that follows `<a href>` and does not submit forms; Traefik rate limiting covers the rest.
- Datastar v1.0.2 (`areas/access/dpe/public/vendor/datastar.js`) appends a GET form's fields to the URL's query string; a Datastar upgrade that changes this breaks the fragment's input. The fragment test pins the server side, the JS-on E2E test the client side.
- `just test-e2e-dpe` is a new CI step; its first run on the PR is read before the unit reports `ready` (`docs/learnings/build-errors/remote-download-minimal-leaves-toplevel-outputs-remote-in-ci.md`: a recipe copying from `bazel-bin` needs `--remote_download_toplevel` in CI).
- Playwright's `webServer` starts the server with the config directory as working directory; `robots.txt` and the public directory must resolve from the path the recipe passes, as the accessibility recipe already does.
- `form` and `hints` start from `query` and edit disjoint files, so the stack-on rebase of whichever finishes second has no conflict.
- The no-comma test covers the committed corpus only. A production option value with a comma would be split by the parser and match nothing; the corpus is curated metadata with controlled values, so the risk is accepted.

## Success Metrics

- On the unfiltered page, every `href="/dpe/projects?` link is a pagination link with `page=` alone (`curl -s <DPE>/dpe/projects | grep -o 'href="/dpe/projects?[^"]*"'`); today there is one link per facet option plus the dialog link.
- `curl -s -o /dev/null -w '%{http_code}' <DPE>/robots.txt` is 200; today 404.
- H4: requests to `/dpe/projects` with facet parameters fall well below the pre-incident week once crawlers re-read robots.txt.

## References

- `areas/access/dpe/web/src/pages/projects/components/filter_checkbox_group.rs` (one `<a href>` per option)
- `areas/access/dpe/web/src/pages/projects/components/project_filters_content.rs`, `mobile_filters_button.rs`, `page.rs`
- `areas/access/dpe/web/src/domain/project.rs` (`ProjectQuery`, `to_query_string`, `with_*_toggled`)
- `areas/access/dpe/server/src/shell.rs` (`projects_page_handler`, `HeadExtras`), `fragments.rs` (search and tab fragments, `replaceState` in the tab handler), `router.rs`
- `areas/access/dpe/web/src/pages/projects/components/project_search_input.rs` (existing Datastar form pattern)
- `areas/access/cpe/server/src/router.rs` (`pushState` through `ExecuteScript`, relative URL)
- `shared/datastar/src/lib.rs` (`Get`, `Post::form`)
- `areas/access/server/src/serve.rs` (`/robots.txt` parity test), `areas/access/dpe/BUILD.bazel` (`public` filegroup)
- `.github/scripts/check-datastar-attributes.sh`, `.github/workflows/a11y-dpe.yml`, `justfile` (`test-a11y-dpe`, `test-e2e-editor`)
- `dasch-specs/learnings/best-practices/visual-parity-oracle-worktree-ab-for-big-bang-ui-refactors.md`, `dasch-specs/learnings/build-errors/tailwind-source-none-silent-class-loss.md`, `dasch-specs/learnings/best-practices/maudfmt-adoption-no-check-mode-and-clippy-gotchas.md`
