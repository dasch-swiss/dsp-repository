---
title: "feat: hide a project from crawlers and from every overview"
type: feat
date: 2026-11-19
author: "Balduin Landolt"
status: implemented
repository: dasch-swiss/dsp-repository
linear: DEV-7919
linear_project: Integrate da-tei into dsp repository
---

# feat: hide a project from crawlers and from every overview

## Overview

Some presentations must run on stage and prod before their project goes public: Keller (083C) first, and the
Sephardic Bibles (086A) once deployed. Such a project should be reachable only through its link.

This plan adds the **Hidden project**. An operator lists shortcodes in one environment variable on
`access-server`. Every Access capability that presents a listed project then:

1. sends `X-Robots-Tag: noindex, nofollow` on every response under the project's prefix;
2. writes `<meta name="robots" content="noindex, nofollow">` into each HTML document it renders from the loaded
   project;
3. leaves the project out of every overview it renders;
4. otherwise answers exactly as it would for a project that is not hidden.

CPE implements this in this PR. DaTEI has no code on `main` and adopts it under DEV-7574. DPE is out of scope.

The work ships as one PR off `main`, in one commit.

## Problem Statement / Motivation

`dsp-repository` has no way to serve a presentation without advertising it. CPE's overview at `<CPE_BASE>` lists
every project that loaded. Nothing stops a crawler that finds a link from indexing a project's pages. DEV-7935
(Keller on stage, hidden) is blocked on this.

## Owner decisions (2026-11-19)

These were settled before planning and are not open for this plan to revisit.

- **Presentations only.** DPE is unchanged: its metadata pages (`/dpe/projects/{shortcode}`), the project list
  and search, `/dpe/api/v2`, OAI-PMH and ARK resolution. 083C and 086A are already public there as metadata
  records, and DPE links to no presentation.
- **Where the flag lives.** There is one per-environment variable on `access-server`, `ACCESS_HIDDEN_PROJECTS`.
  It is a comma-separated list of shortcodes that ops-deploy sets per environment. The composition root reads
  it once and hands the set to each capability. It is not deposited, and it is not part of a capability's own
  configuration file.
- **How noindex is signalled.** The header goes on every response under the prefix, and the meta tag goes in the
  HTML documents. There is no `robots.txt` entry, for two reasons. A `Disallow` stops the crawl that would see
  `noindex`. A public `robots.txt` would also advertise the hidden URL. No sitemap exists today, and none is added.
- **Capability-agnostic.** The variable, its parsing and the term are the Access Area's. Each capability
  receives the set and owns the four obligations above.
- **Wording.** The ticket says "landing page" for a listing. In `areas/access/CONTEXT.md`, **Landing page** is
  the page an ARK resolves to. This plan says **overview** for a listing.

## Proposed Solution

### The variable, read once by the composition root

`areas/access/server/src/serve.rs` gains a pure function:

```rust
/// `ACCESS_HIDDEN_PROJECTS`, split, trimmed and upper-cased; the entries that are
/// not a shortcode come back separately, for the caller to log.
fn parse_hidden_projects(value: Option<&str>) -> (BTreeSet<String>, Vec<String>)
```

`serve()` calls it with `std::env::var("ACCESS_HIDDEN_PROJECTS").ok().as_deref()`, beside its existing raw reads
of `DPE_SITE_ADDR`, `SYNC_DATA_DIR` and `MEDIA_DATA_DIR` (`serve.rs:216-221`). The rules are:

- **Unset or empty.** A value that is unset, empty or only whitespace means no hidden project. A value that is
  set but empty counts as unset (see
  `docs/learnings/configuration-errors/empty-composite-input-beats-compose-env-file.md`).
- **Splitting.** Split on `,`, trim each entry and drop the empty ones. `",083c, ,086A,"` gives `{083C, 086A}`.
- **Validation.** Upper-case each entry, then check it with `cpe_ports::is_shortcode`, which accepts exactly four
  of `0-9A-F`. `access-server` already depends on `cpe_ports`. That rule is stricter than the contract Shortcode
  in the root `CONTEXT.md`, which allows the five `0801a`–`0801e`. None of those is a presentation, so the
  stricter rule costs nothing.
- **Rejected entries.** `serve()` logs each rejected entry once as a warning that names it, then ignores it. The
  binary still starts. Reading this variable must never stop or delay DPE, just as `CpeConfig::load` must not.
- **Startup log.** `serve()` logs the accepted set once at `info`, e.g. `hidden projects: 083C, 086A` or `none`.
  Shortcodes are not secret, since they are public DPE metadata.

`serve()` declares `let mut cpe_config = CpeConfig::load();` and sets `cpe_config.hidden_shortcodes` before
`Cpe::new(&cpe_config)` (`serve.rs:213-214`). `Cpe::new` copies the config into an `Arc`. DaTEI will get the same
set through its own config value. Either way it is a value and never a process-global (ADR-0005, ADR-0007).

**Why the root reads it, and not `CpeConfig::load`.** ADR-0003 and ADR-0007 give a capability's facade its own
configuration, and `CpeConfig` reads `cpe.toml` and `CPE_*`. This variable is not CPE's: it is one Access-Area
setting shared by every capability that presents projects. If each facade read it, the parsing rules would exist
once per capability, and a capability could drift from them. The root already reads the variables that are not
one capability's (`SYNC_DATA_DIR`, `MEDIA_DATA_DIR`). Parsing a list does not count as "adapter logic", which is
what ADR-0003 keeps out of the root.

A listed shortcode that no capability presents has no effect, so a project can be listed before it is deployed.

### CPE

**Config.** `CpeConfig` (`areas/access/cpe/server/src/config.rs`) gains this field:

```rust
/// The shortcodes of the Hidden projects (`ACCESS_HIDDEN_PROJECTS`, which the
/// host reads; CPE never does). Upper case.
#[serde(skip)]
pub hidden_shortcodes: BTreeSet<String>,
```

`#[serde(skip)]` keeps the field out of `cpe.toml` and `CPE_*`, the same way `load_error` is kept out. The
`.only([...])` list stays unchanged. `Default` gives an empty set. Every existing `CpeConfig { .. }` literal
already ends in `..`, and `BTreeSet` is a std type, so `no_pub_item_names_another_cpe_crate` (`lib.rs:510`) still
holds.

**Which project ids are hidden.** `activation::load_all` (`activation.rs:94-154`) is the one place where a
discovered project's `id` and its `shortcode` (`Option<String>`, upper case) are both in scope, for both served
and unavailable projects. Before the loop moves `discovered`, it collects
`hidden_ids: Arc<BTreeSet<String>>`: the ids whose `shortcode` is in `config.hidden_shortcodes`. It then logs:

- the hidden ids at `info`, e.g. `hidden project ids: keller`;
- each listed shortcode that matches no discovered folder, also at `info`, since listing 086A before it is
  deployed is legitimate. An operator who types `083D` for `083C` can see that from the log.

Two edge cases need no special handling:

- **Folders with no shortcode.** `from_sql_folders` and legacy folders have no shortcode (`discovery.rs:327`), so
  they can never be hidden. None of them is deployed.
- **Folders that share a shortcode.** Two folders that share a listed shortcode both become hidden ids. Both are
  refused already and answer 503, now with the header too.

`Project` (`cpe_web::project::Project`) does not change. The hidden ids are a routing fact, and the router
already keys everything by project id.

**One helper for "which project does this path belong to".** `fallback_handler` already takes the project id from
the first segment of `req.uri().path()`. The match is byte-for-byte on the undecoded path, with an empty-segment
guard (`router.rs:492-498`). Lift that into one `pub(crate)` function in `router.rs`, for example
`project_segment(path: &str) -> Option<&str>`. `fallback_handler` and the new layer both call it, so the two can
never disagree about which project a path belongs to. The match is case-sensitive and does not decode, so
`/THINGS/` and `/%74hings/` belong to no project.

**Header.** `load_all` adds a third router-level layer:

```rust
.layer(axum::middleware::from_fn_with_state(Arc::clone(&hidden_ids), request::robots_for_hidden_projects))
```

Put it in `request.rs`, beside `resolve_request_prefs` and `report_matched_route`. Add it before those two
`.layer` calls so that it is the innermost of the three. Every route and the fallback, including the 404 and the
503 of an unavailable project, sit inside all three. Then:

- axum's `nest` strips the base before CPE's router sees the request (axum 0.8.9, `StripPrefix`), so the layer
  matches CPE's own paths;
- before calling `next.run(request)`, which consumes the request, the layer captures
  `project_segment(request.uri().path())` and checks it against the set;
- on a hidden project it calls `response.headers_mut().insert(X_ROBOTS_TAG, NOINDEX)`, never `append`;
- `X_ROBOTS_TAG` is `HeaderName::from_static("x-robots-tag")` and `NOINDEX` is
  `HeaderValue::from_static("noindex, nofollow")`, both `const`s in `request.rs`;
- the layer neither logs nor opens a span.

The layer covers everything under the project's prefix:

- `/<id>` and `/<id>/` (both are routes, with no redirect between them);
- every page route, parameterised ones included;
- the 303s the handlers answer when they drop a query key (`?appearance=`, `?accent=`), and `canonical_page_redirect`'s (`?page=`);
- the SSE fragments;
- `/<id>/_assets/*` and `/<id>/_static/*`;
- the project's 404;
- the 503 of a hidden project that failed to load;
- HEAD requests.

It does not cover the following, and correctly so:

- the overview at `<base>` and `<base>/`, and CPE's shared `/_static/*`, which belong to no project;
- the 503 that every path answers before activation (`lib.rs:199`), when no project id is known yet. A crawler
  does not index a 503.

**Meta tag.** `cpe_web::page` gains this enum:

```rust
/// Whether a document may be indexed; a Hidden project's pages may not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Indexing { Indexable, NoIndex }
```

`document_head` (`page.rs:58`) gains an `indexing: Indexing` parameter. Inside its `html!`, right after the
viewport meta, it writes this:

```rust
@if indexing == Indexing::NoIndex {
    meta name="robots" content="noindex, nofollow";
}
```

This needs no `PreEscaped`. `document_head` has three callers, all in `page.rs`:

- `render_page` (`:454`) and `render_not_found_project` (`:548`) gain an `indexing: Indexing` parameter and pass it
  through.
- `site_page` (`:726`) passes `Indexing::Indexable`. `site_page` serves the overview (`render_site_index`) and
  CPE's own not-found (`render_not_found_site`).

`router::build` (`router.rs:105`) gains a `hidden_ids: &BTreeSet<String>` parameter, so a caller with nothing
hidden passes `&BTreeSet::new()`. From it:

- `make_handler` (both registrations, `router.rs:147` and `:156`) and `make_parameterized_handler` (`:273`) gain an
  `indexing: Indexing` argument: `NoIndex` when `hidden_ids` holds the project's id, else `Indexable`. They pass it
  to `render_page` (`:1037`, `:1148`).
- `fallback_handler` (`:484`) is one closure for all projects, so it gets the set itself (as
  `Arc<BTreeSet<String>>`) and derives `Indexing` from `matched_project.id()` (`:525-529`) before calling
  `render_not_found_project` (`:563`).

`build`'s other callers pass an empty set: the trial build in `with_routes_that_register` (`activation.rs:300`),
about 26 tests in `router.rs` and one in `static_files.rs`. The existing callers of `render_page` and `render_not_found_project` in tests pass
`Indexing::Indexable` (about 61 sites in `page.rs` tests, a mechanical edit). The meta is written only for
`NoIndex`, so every existing snapshot stays byte-identical. Update the head's doc comment (`page.rs:30-41`) to
list the new meta.

Two kinds of response carry the header and no meta:

- **SSE fragments.** They are not documents.
- **The 503 of a hidden project that failed to load.** `unavailable::page()` (`unavailable.rs:37-58`) renders a
  head of its own from nothing project-specific, because the project did not load.

**Overview.** `router::build` passes `site_index_handler` only the projects whose id is not in `hidden_ids`. The
fallback and every project's routes keep the full `served` list. The count text (`index_count_text`) and the
empty state follow from the filtered list.

**Doc comments.** `lib.rs:34-35` ("CPE's own list of them is the page at its base") and `lib.rs:205-206` ("The
page at the base lists the same projects") are no longer true. They change to say that `Cpe::projects()` includes
Hidden projects and the overview does not. `Cpe::projects()` / `ProjectSummary` is the host-facing list. It is
never rendered, and the host uses it only in a test (`serve.rs:1225`), so it stays as it is.

### Vocabulary and documentation

- **`areas/access/CONTEXT.md`** gains **Hidden project**. The entry says:
  - It is a project whose Shortcode is in `ACCESS_HIDDEN_PROJECTS`.
  - Each capability that presents it keeps the four obligations.
  - It is a search-index signal, not access control. Anyone with the link reaches it. AI-training crawlers that
    follow only `robots.txt` are not addressed. Images served from the IIIF host are not covered. DPE's metadata
    record for the same Shortcode stays public. No public page should link to a Hidden project, since a link lets
    crawlers learn the URL even when they do not index it.
  - _Avoid_: private, unlisted, unpublished, embargoed. These terms suggest access control, which ADR-0013's
    decision function owns.
- **`areas/access/datei/CONTEXT.md` → Boundary commitments** gains one entry. The Presenter honours a Hidden
  project with the same four obligations, using the set the composition root hands it. That is how DEV-7574
  adopts the mechanism.
- **`areas/access/cpe/CONTEXT.md` → Overview** gains one line: the overview leaves Hidden projects out.
- **`docs/src/dpe/operations.md` → Environment Variables:** a row for `ACCESS_HIDDEN_PROJECTS` in the table
  (`:72`). The row says it is read by `access-server`, honoured by CPE and ignored by DPE, and that it is the
  deployment's to set.
- **`docs/src/cpe/operations.md`:**
  - In `## Configuration`, the sentence about "the five variables" names the sixth variable, which CPE honours but
    does not read, and links to its row in DPE's table.
  - The failure-mode table gains a row: an entry that is not a shortcode is logged as a warning and ignored, and
    the binary serves.
  - A short "Hiding and un-hiding a project" paragraph covers three points. The variable is read at start, so a
    change needs a restart or a redeploy. Search engines notice un-hiding only when they next crawl. HTTP caches
    keep `noindex` responses until they expire.
- **`areas/access/server/CLAUDE.md`:** one bullet. The root reads `ACCESS_HIDDEN_PROJECTS` and hands the set to
  each capability. The image does not set it, because it is the deployment's to set, as `CPE_PUBLIC_ORIGIN` is.
- **`ARCH-MAP.md`:** the `areas/access/server` entry (`:149`) names the variable among those the root reads. The
  `areas/access/cpe` entry (`:1050`) names `CpeConfig::hidden_shortcodes` and `Indexing`. Leave the fingerprints
  to `dune:dune-map` and do not hand-edit them.

There is no ADR. The decision is small and reversible, and the term records it. If the dune reviewer finds it is a
cross-capability commitment that needs one, the executor writes it.

## Alternative Approaches Considered

- **`robots.txt` `Disallow`.** Rejected by the owner decision. It blocks the crawl that would see `noindex`, a URL
  known from elsewhere can still be indexed without its content, and the file advertises the path.
- **A header layer in `access-server` over the prefixes the capabilities report.** This would give every
  capability one shared layer. Each capability would have to export `(shortcode, prefix)` pairs, unavailable
  projects included, which `ProjectSummary` does not carry. It would also put path matching into the composition
  root, which ADR-0003 says is wiring. Each capability would still filter its own overview and write the meta. The
  shared layer saves one of four obligations at the cost of a new contract. Rejected.
- **`CPE_HIDDEN_PROJECTS` read by `CpeConfig`.** This is the cheapest option for CPE. DaTEI would then need its own
  variable, and ops-deploy would keep two lists in sync. Rejected in favour of one Access-Area variable.
- **A `hidden` flag on `cpe_web::project::Project`.** `load()` wraps the project in an `Arc` before `load_all` sees
  it. `Project::load` has 56 call sites. And the renderers do not take a `Project`. The router already keys
  everything by id, so the id set is enough.
- **A field on `RenderContext` instead of an `Indexing` parameter.** Its literal is built in about 71 places, more
  than the 61 renderer call sites. It would also give a per-project fact to a per-request context. Rejected.
- **A `Shortcode` newtype.** CPE carries shortcodes as `String`, validated by `cpe_ports::is_shortcode`
  everywhere. A newtype for this one set would cross three crates for no new invariant. Rejected.
- **A shared crate for the parsing and the header value.** One function and two constants do not earn a new
  `shared-*` crate.
- **A per-project marker in `media`'s stand-in directory or in `config.kdl`.** Rejected by the owner decision.
  Visibility is a per-environment publication decision, and the archive will feed the stand-in.
- **`Referrer-Policy` on hidden pages.** Browsers default to `strict-origin-when-cross-origin`, so a hidden path
  does not leak to Google Fonts or the IIIF host. Not added.

## Technical Considerations

- **ADR-0005.** CPE pages inherit the landing-page obligations once an ARK resolves to them (ADR-0007). None
  resolves to CPE today. A Hidden project is a temporary state before publication, and `noindex` removes none of
  the metadata a page embeds.
- **Security.** This is not access control. A hidden project's pages, assets and fragments are public to anyone
  with the link. The term and the operations docs say so.
- **Snapshots.** Existing snapshots stay unchanged, because the meta is written only for `NoIndex`. New
  assertions pin the hidden case.
- **`<base>/`.** `mount_cpe` answers `<base>/` with `oneshot` on CPE's root router, which is the overview. That
  path is never a project's, so the layer is not needed there.
- **Accessibility.** No control or view changes. The meta tag is not visible, and the overview lists fewer
  teasers with the same markup.

## Implementation Phases

#### Phase 1: Hidden projects in `access-server` and CPE

Tests come first. Each one names the regression it catches. Test names follow `CONVENTIONS.md`. A test that loops
over paths names the path in each assertion message.

Fixtures:

- [x] Parameterise `things_snapshot` by shortcode in `areas/access/server/src/serve.rs`, as `activation.rs`'s `snapshot(shortcode, name)` does.
- [x] Extend `things_files` in `serve.rs` to a third shortcode, `0003`, with a `site.title` that differs per shortcode. Two served projects must be told apart by title.
- [x] In `areas/access/cpe/server/src/base_tests.rs`, add a second `Cpe` over Incunabula, built from `config()` with `hidden_shortcodes = {"0803"}`. The shared `cpe()` is a `OnceLock` built without the set, so it cannot be reused.

Tests in `serve.rs` (the parser and the assembled app):

- [x] `parse_hidden_projects` gives an empty set and no rejects for `None`, `""`, `"  "` and `" , "`. Catches an empty value read as a list.
- [x] `parse_hidden_projects(Some(",083c, 086A ,,"))` gives `{083C, 086A}`. Catches whitespace, stray commas and case left unnormalised.
- [x] `parse_hidden_projects(Some("083C,83C,0803x,/cpe/x"))` gives `{083C}` and rejects `["83C", "0803x", "/cpe/x"]` as typed, trimmed but not upper-cased, so the warning names what the operator wrote. Catches one bad entry that rejects the whole list, or that passes through.
- [x] With `things` (0001, hidden), `other` (0003, visible) and `offline` (0002, hidden, unavailable): GET and HEAD on `/cpe/things` and `/cpe/things/`, GET `/cpe/things/?lang=de`, a 303 from `?appearance=dark`, and a 404 under `/cpe/things/` each carry `X-Robots-Tag: noindex, nofollow`. Catches a layer that covers some answers of a project and misses others.
- [x] The same paths under `/cpe/other/`, the overview at `/cpe` and `/cpe/`, `/cpe/THINGS/` and `/dpe/projects` carry no `X-Robots-Tag`. `/cpe//things/` does carry it: axum's `nest` hands CPE `/things/`, so that URL serves the hidden page (provisional, decided in execution). Catches a header that leaks outside the hidden project, or a path match that differs from `fallback_handler`'s.
- [x] The body of `/cpe/things/` contains `<meta name="robots" content="noindex, nofollow">`, and so does its 404. The body of `/cpe/other/` does not. Catches the header without the meta, or the meta on every project.
- [x] The overview at `/cpe` lists `other` by title and link, its count text is the one-project string `index_count_text` gives, and it does not mention `things`. Catches an overview that still lists a hidden project, or counts it.
- [x] `/cpe/things/` answers with the same status as with nothing hidden, and the same body once the robots meta line is removed. Catches hiding that changes anything beyond the header and the meta.
- [x] `/cpe/offline/` answers 503 with `X-Robots-Tag`. Catches an unavailable hidden project whose shortcode was never mapped to its id.
- [x] Under a base of `/presentations`, `/presentations/things/` carries the header and the overview at `/presentations` omits `things`. Catches a layer that matches the full path instead of the path CPE sees once the base is stripped.

Tests in `cpe-server` and `cpe-web`:

- [x] In `base_tests.rs`, on the hidden Incunabula: `/cpe/incunabula/books/_sort` (SSE) and `/cpe/incunabula/_assets/assets/cover.webp` answer with a success status (`_sort` sent as a Datastar request, `answer(…, true)`) and carry `X-Robots-Tag`. On the shared `cpe()` (not hidden), the same two paths do not carry it. The status check stops the test from passing on a 404, which carries the header too. Catches a layer that covers pages but not fragments or assets.
- [x] In `activation.rs`'s tests, beside `folders_that_share_a_shortcode_both_answer_503_and_nothing_is_fetched`: two folders that share a hidden shortcode both answer 503 with the header. Catches hidden ids collected from served projects only.
- [x] In `areas/access/cpe/web/src/page.rs`: `document_head` with `Indexing::NoIndex` contains the robots meta, and with `Indexing::Indexable` it does not. Catches a parameter that is wired but ignored.

Implementation:

- [x] Add `parse_hidden_projects` to `serve.rs`.
- [x] In `serve()`, make `cpe_config` mutable, set `hidden_shortcodes`, log the set at info and each rejected entry as a warning. The tests set `hidden_shortcodes` directly, so this one line is checked only by H2.
- [x] Add `hidden_shortcodes` to `CpeConfig` with `#[serde(skip)]`, and set it to an empty set in `Default`.
- [x] In `activation::load_all`, collect `hidden_ids` before the loop over served and unavailable projects, and log the hidden ids and the listed shortcodes with no folder at info.
- [x] Lift the segment extraction of `fallback_handler` into `router::project_segment`, and use it there.
- [x] Add `request::robots_for_hidden_projects` with the two `const`s, and layer it in `load_all` as the innermost of the three layers.
- [x] Add `Indexing` to `cpe_web::page` and the `indexing` parameter to `document_head`. `site_page` passes `Indexable`. Update the head's doc comment.
- [x] Add the `indexing` parameter to `render_page` and `render_not_found_project`.
- [x] Update the existing test callers of `render_page` and `render_not_found_project` to pass `Indexing::Indexable`.
- [x] Add the `hidden_ids` parameter to `router::build`. `make_handler` and `make_parameterized_handler` get their `Indexing`, and `fallback_handler` gets the set.
- [x] Update `router::build`'s other callers (the trial build at `activation.rs:300`, the tests in `router.rs` and `static_files.rs`) to pass an empty set.
- [x] In `router::build`, filter the projects given to `site_index_handler` to those that are not hidden.
- [x] Update the doc comments at `cpe/server/src/lib.rs:34-35` and `:205-206`.

Documentation:

- [x] Add the **Hidden project** term to `areas/access/CONTEXT.md`.
- [x] Add the boundary commitment to `areas/access/datei/CONTEXT.md`.
- [x] Add the line to the Overview entry of `areas/access/cpe/CONTEXT.md`.
- [x] Add the variable's row to the environment table of `docs/src/dpe/operations.md`.
- [x] In `docs/src/cpe/operations.md`, update the Configuration sentence.
- [x] In `docs/src/cpe/operations.md`, add the failure-mode row.
- [x] In `docs/src/cpe/operations.md`, add the hiding and un-hiding paragraph.
- [x] Add the bullet to `areas/access/server/CLAUDE.md`.
- [x] Add `cpe_ports::is_shortcode` to the imports named in the dependency-boundary paragraph of `areas/access/server/CLAUDE.md` (`:75-78`).
- [x] Update the `areas/access/server` and `areas/access/cpe` entries of `ARCH-MAP.md`.

Verification:

- [x] Run `just check && just test`; it passes.
- [x] Run `eng:test-browser` against `ACCESS_HIDDEN_PROJECTS=0803 just run`. `/cpe/` shows the empty state ("No projects are being presented yet."). `/cpe/incunabula/` renders, its response carries the header, and its head carries the meta.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

The commit is `feat(access-server,cpe-server,cpe-web): hide a project from crawlers and from the overview
(DEV-7919)`. One commit stays within CI's one-commit cap.

Reviewers:

- from `eng.yaml`: `eng:review:ivan-reviewer`, `eng:review:security-reviewer` and `eng:review:accessibility-reviewer`;
- `eng:review:dune-reviewer`, for `ARCH-MAP.md` and `CONTEXT.md`;
- `eng:review:rust-reviewer`;
- `eng:review:consistency-reviewer`.

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Set `ACCESS_HIDDEN_PROJECTS=083C,086A` for `access-server` on stage and prod in ops-deploy, before Keller is deployed there (DEV-7935) | Owner | after ship | ops-deploy is another repository, and what is hidden in which environment is a deployment decision |
| H2 | After a deploy, check three things: `curl -sI <host>/cpe/<keller id>/` shows the header, `/cpe/` does not list Keller, and the start log names Keller's id as hidden | Owner | after ship | Needs the deployed environment |

## Acceptance Criteria

- [ ] When `ACCESS_HIDDEN_PROJECTS` lists a CPE project's shortcode, every response under that project's prefix carries `X-Robots-Tag: noindex, nofollow`. This includes redirects, the project's 404 and the 503 of a project that failed to load.
- [ ] Every HTML document CPE renders from that loaded project, its pages and its 404, carries `<meta name="robots" content="noindex, nofollow">`.
- [ ] CPE's overview does not list the project, and its count matches the projects it lists.
- [ ] The project answers at its own URL exactly as it would without the variable, apart from the header and the meta.
- [ ] When no listed shortcode matches a discovered project, every response is identical to before. Otherwise, only the hidden project's responses and the overview's list of teasers differ. DPE's responses never differ: `Dpe::new` never receives the set, so the structure guarantees this and no test is needed.
- [ ] An unset, empty or malformed value never stops the binary. A rejected entry is logged as a warning. The hidden ids, and the listed shortcodes that have no folder, are logged at info.
- [ ] The tests listed in Phase 1 exist and pass under `just test`.
- [ ] The term, both operations pages, `areas/access/server/CLAUDE.md` and `ARCH-MAP.md` describe the mechanism, including how DaTEI adopts it.

## Dependencies & Risks

- **No dependency on the DaTEI stack.** The work builds off `main`.
- **Risk: someone reads "hidden" as "protected".** Mitigation: the term's _Avoid_ line and the operations docs
  say it is not access control.
- **Risk: a typo in the variable leaves the project indexable.** Mitigation: the start log names the hidden ids
  and the listed shortcodes with no folder, and H2 checks it.
- **Risk: DaTEI forgets an obligation.** Mitigation: the boundary commitment in `datei/CONTEXT.md`, which
  DEV-7574's plan reads.
- **Residual: images on the IIIF host carry no header.** An image linked from elsewhere can still be indexed.
  This is out of scope, and the term says so.

## Success Metrics

- Phase 1's tests pass. The existing CPE and DPE snapshot suites pass with no snapshot updated.
- On stage after H1, Keller answers with the header and is absent from `/cpe/`. This is DEV-7935's check.

## References

- Ticket: DEV-7919. Blocks: DEV-7935. The DaTEI mount: DEV-7574.
- Composition root: `areas/access/server/src/serve.rs`. Functions: `serve` (`:180`), `data_dir` (`:19`),
  `mount_cpe` (`:114`). Test helpers: `cpe_config` (`:363`), `things_snapshot` (`:391`), `projects_folder`
  (`:415`), `things_files` (`:426`), `things_and_offline` (`:437`), `get` (`:480`).
- CPE: `areas/access/cpe/server/src/`, the files `config.rs` (`:17-65`), `activation.rs` (`load_all`
  `:94-154`), `router.rs` (`build` `:105`, `fallback_handler` `:437-529`), `request.rs`, `unavailable.rs` and
  `lib.rs`. Also `areas/access/cpe/web/src/page.rs` (`document_head` `:58`, `site_page` `:726`).
- ADR-0003 (the composition root), ADR-0005 (no process-global configuration; landing-page obligations),
  ADR-0007 (CPE in the Access Area), ADR-0009 (DaTEI), ADR-0013 (authorisation, which this is not).
- Learnings: `docs/learnings/configuration-errors/empty-composite-input-beats-compose-env-file.md`,
  `docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`.
- Google Search Central, "Robots meta tag, data-nosnippet, and X-Robots-Tag specifications":
  https://developers.google.com/search/docs/crawling-indexing/robots-meta-tag
