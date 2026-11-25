Fixes LINEAR-ID, LINEAR-ID, ...

## Motivation
Why this work was needed. What problem it solves for users.

## Summary
1-3 bullet points of user-visible changes.

## Key Changes
### [Topic]
- change details

## Review guide
<!-- Multi-phase PRs only; delete otherwise. One section per phase. -->
### Phase N — [name]
- **Review:** the files and seams to look at
- **Verified:** what the tests, the browser check and the reviewers covered
- **Decision:** what was settled here and why

## Challenges and Decisions
What was tried, what failed, and key architecture decisions.
Structure as sub-sections when multiple challenges exist:

### [Challenge title]
**Problem:** description of the issue encountered
**Tried:** approaches that didn't work and why
**Solution:** what worked and why it's the right approach

## Gotchas
Things future developers should know. Each gotcha should be
actionable — not just "this is hard" but "do X instead of Y".

## Learnings
The files added to `docs/learnings/`, or why there are none.

## Test Plan
<!-- Say how each line of "Before a merge" in docs/src/git-conventions.md was met, or why it does not apply. -->
- [ ] verification steps

## Commit hygiene
<!-- This repo rebase-merges: every commit lands on `main` verbatim. One commit per scope per slice, at most 100 — see docs/src/git-conventions.md. Every commit needs a type AND a scope: `type(scope): subject`. Check with `just commit-lint`. -->
- [ ] I have curated the branch history (fixups folded, bodies written for the agent that reads them, comments cleaned) so it reads well on `main`
