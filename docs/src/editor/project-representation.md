# The Project Representation

What a draft is, where the published set comes from, and how a project file is written. The decision behind the draft's shape is `areas/deposit/ADR-0002`.

The editor's path is `ProjectRaw` -> draft -> `ProjectRaw`, never through DPE's `Project` view model: `impl From<&Project> for ProjectRaw` rewrites `url` into the object form and hardcodes `clusters: None`, both lossy in exactly the places the editor is required to preserve.

`editor_core::draft::ProjectDraft` is the project's JSON members rather than a struct mirroring `ProjectRaw` with 36 `Option` fields. Three requirements pull that way at once: a draft must hold a field the depositor has not filled in and a value that is present but invalid, it must carry every field the editor does not manage unchanged, and it must survive a field being added to the contract without an editor change. An absent key is a missing field, any value is retained whether it validates or not, and validity is decided once, at `to_raw`, which is the submission boundary.

The three `#[serde(untagged)]` enums therefore need no stored variant tag. Untagged deserialization takes the first variant that fits, but a value that keeps its JSON kind verbatim cannot be forced into the wrong one: a string can only be `Funding::Text`, because `Grants` needs an array. `funding_shape` and the two `*_shapes` accessors derive the variant in serde's own attempt order, so what they report can never disagree with what the written file is built from.

`url` keeps the form it was read in. Zero of the 85 committed files use the structured object form (36 hold a one-element string array, 38 a two-element array, 11 omit `url`), so writing the object form would rewrite 74 files. It is used only where there was no prior value: new projects, and those 11 files.

## The published set

`editor_core::published::PublishedProjects` reads `$EDITOR_DATA_DIR/projects/*.json` once at startup and holds them in memory, keyed by case-folded shortcode. The set cannot change without a redeployment, so nothing polls and nothing invalidates. It is not behind a repository port: the ports exist because the editor writes through them and a test has to be able to make a write fail, and this is a read of an immutable snapshot, so a trait would buy an indirection with one implementation.

Three properties of the committed corpus decide the shape, each measured over all 85 files rather than sampled:

- **The `shortcode` field is the key, not the filename.** Five files disagree with the shortcode they hold — `projects/0801_bebb.json` is project `0801d`, and its siblings under `0801_*` are `0801a` through `0801e`. Keying on the filename stem would file all five under `0801`, which no project actually has, so all five would be unreachable by the code they are addressed by and four would be dropped as duplicates.
- **Lookup folds case.** 24 shortcodes are mixed case (`080C`, `081B`, `085F`), and no two collide when folded. This matches `Actor::may_reach`, which folds for the same reason. Two files claiming one folded shortcode is reported rather than resolved, so which project answers can never depend on directory order.
- **Nothing about the load is fatal.** An unset `EDITOR_DATA_DIR` is a configured state (the PR preview has no snapshot), and one malformed file among 85 is a problem with the image rather than a reason to refuse every request. Both are reported at `warn` with a count and one line per failing file, because "84 of 85" is findable where an exited process says only that it exited.

`get` returning `None` does **not** mean the project does not exist. A project may exist only locally, created by RDU or merely assigned, in which case its form opens from its draft or blank, so a 404 needs the draft and submission records too — which is why `/projects/{shortcode}` answers 200 for an unpublished shortcode.

## Created projects

RDU creates a project at `/projects/new` with a shortcode and a name; everything else is derived or the depositor's to fill in. The shortcode is upper-cased (ASCII), DSP's spelling, before anything is derived from it, so the draft's `shortcode`, `id` and `pid`, the file name and the URLs spell it so; the rows are keyed by the folded shortcode, as every other table is. It is refused, with an error at the field, when `shared_metadata::is_valid_shortcode` rejects it, when it is `new` in any case (the static segment of `/projects/new`, which would shadow the project's own `/projects/{shortcode}`), when the published set carries it, when a published project already holds the `pid` it derives (`creation::pid_holder`: `0801a` to `0801e` share `0801`'s ARK, so `0801` is refused), or when the editor already holds a registration, a draft, a submission or an approved record for it: an assigned depositor can save a draft for an unpublished shortcode before RDU creates it, and creation never overwrites one. A refusal for a draft alone names `/projects/{shortcode}`, where the draft can be opened and discarded. `editor_core::creation` holds the derivation:

- `id` is the shortcode and `pid` is `https://ark.dasch.swiss/ark:/72163/1/<shortcode>`, the form every committed project whose ARK names its own shortcode has. The ARK is derived, not minted, so it resolves only once DSP knows the project.
- `howToCite` is seeded as `<name> [database] (DaSCH), <pid>.` and `legalInfo` as `[]`. Both are display-only, so neither the depositor nor RDU can fill them in on the form, and `ProjectDraft::to_raw` needs both at submit. RDU completes the citation and adds the licence in the collection pull request, which `editor-core`'s `published_licence` test, over every committed project, refuses to let merge with an empty `legalInfo`.

The registration is a row of its own, `created_projects` (shortcode, name, creator, time), written with the seeded draft in one transaction; the draft alone cannot be the registration, because "Discard draft" removes a draft. On a created project that control rewrites the draft to the seed instead, without the name, since the shortcode, `id`, `pid` and citation are display-only and nobody could enter them again. RDU removes a created project from the "New projects" group on `/projects` while it has no submission and no approved record; removal deletes the registration, the draft and the project's review rounds and withdraws every entity proposal of it that is live or accepted and not yet published, in one transaction that re-checks for a submission or record, so a project created again under the shortcode inherits none of them. The removal redirects to `/projects?removed=<shortcode>`, which confirms it ([Routing](./routing.md)). A created project's shortcode cannot be changed; it is removed and created again.

Once its record is released through a Pull-request run, a created project is a new file: the run writes `projects/{shortcode}_{slug(name)}.json`, named with the project's own shortcode spelling, and the corpus manifest counts it. The startup pass deletes the registration once the published set carries the shortcode ([Status](./status.md#created-projects)).

## Canonical form

`editor_core::canonical::write_project` is the single decision about what a `projects/*.json` file looks like: members in `ProjectRaw`'s declaration order at every depth, `null` members dropped recursively, language keys alphabetical, four-space indent, a trailing newline, non-ASCII unescaped. An approved submission is then byte-comparable with what is committed, so a review diff shows only what the depositor changed.

Two things make that work and are easy to undo by accident:

- The workspace enables `serde_json`'s **`preserve_order`**. The writer round-trips through `serde_json::Value` to strip nulls, and `Value` is `BTreeMap`-backed without that feature, which would alphabetise every key in every file. Under the feature, `Map::remove` is swap-remove: use `retain` or `shift_remove`.
- Multilingual fields are `shared_metadata::utils::Multilingual` (a `BTreeMap`), not `HashMap`. Under `preserve_order` a `HashMap` field serializes in its own randomised iteration order, which would make the round-trip test flaky.

`ProjectRaw` deliberately carries no `skip_serializing_if`: `dpe-server`'s `fragments.rs` serializes it through `axum::Json`, so the attribute would drop null members from DPE's API responses too. Stripping happens in the writer instead.

The 85-file round-trip test (`editor-core/tests/canonical_round_trip.rs`) asserts `load -> draft -> write` is byte-identical for the whole published corpus, and regenerates it under `CANONICALIZE_PROJECT_FILES=1`. Generating the corpus from the writer rather than a sibling script is the point: a script has to agree with the writer by inspection, and a near-miss surfaces later as a failing round-trip that looks like a writer bug.

## Submission checks

`editor_core::submission::unresolved_temporal_coverage` applies the rule that every `temporalCoverage` entry must resolve to a structured date, which `access-server validate` does not block on and OAI-PMH needs. It reuses `shared_metadata::temporal_coverage::completeness_gap`, the same decision `validate` and `dpe-api-oai`'s `every_committed_temporal_coverage_resolves` apply, and adds the entry index so the form can mark a row rather than the whole field. The open question is settled as refusal: a depositor who needs a period the enrichment table does not know uses the `Reference` variant, which always resolves — and since the variant chooser landed that escape route is one a depositor can actually take, where before the refusal named a way out the form did not offer.
