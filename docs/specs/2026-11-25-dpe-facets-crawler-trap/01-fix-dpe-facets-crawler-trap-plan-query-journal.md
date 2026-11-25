---
plan: docs/specs/2026-11-25-dpe-facets-crawler-trap/01-fix-dpe-facets-crawler-trap-plan.md
target_repo: dasch-swiss/dsp-repository
base_commit: adf60272e3b293cdae41a0e581ee8d2faf4b16d2
branch: claude/project-thread-3ggdvm
started: 2026-11-25
unit: query
unit_state: implementing
started_from: adf60272e3b293cdae41a0e581ee8d2faf4b16d2
problem: >
  The DPE project list renders every facet option as a link built by toggling
  the option in the current query, so crawlers see an unbounded URL space.
  This unit lays the query contract the facet form needs: a parser that takes
  repeated keys and comma-joined values, a percent-encoded canonical query
  string, has_filter_params(), and Get::form() in shared-datastar.
symptoms:
  - "GET /dpe/projects?type_of_data=a&type_of_data=b answers 400"
status: in-progress
---

# Execution Journal: 01-fix-dpe-facets-crawler-trap-plan (unit query)

## Repos

| repo | base_commit | branch | merge_strategy | status | pr |
|------|-------------|--------|----------------|--------|----|
| dsp-repository | adf6027 | claude/project-thread-3ggdvm | rebase | in-progress | — |

## Phases

| phase | status | phase_base | review_fix_rounds |
|-------|--------|------------|-------------------|
| 1 | in-progress | dsp-repository@adf6027 | 0 |

## Chunk queue

- 1.1 dpe-web: `ProjectQuery::from_query_str`, `has_filter_params`, percent-encoded `to_query_string`, no-comma corpus test; `@crates//:url`
- 1.2 dpe-server: `RawQuery` + `from_query_str` in `projects_page_handler`, repeated-key router test, fuzz shim
- 1.3 shared-datastar: `Get::form()`

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|

## Deferrals

## Deviations

- Phase 1: the unit branch is `claude/project-thread-3ggdvm`, not the plan's `impl/dpe-facets-query`: this session may push only to its assigned branch. The coordinator gets the real name in `ready` and `done`.

## Side findings

## Owner decisions
