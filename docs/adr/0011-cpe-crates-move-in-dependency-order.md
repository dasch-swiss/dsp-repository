---
status: accepted
date: 2026-10-11
---

# CPE's crates move in dependency order

ADR-0007 brings CPE into `areas/access/cpe`, and its amendment of 2026-10-02 (DEV-7400) splits the move into
one crate at a time. That amendment also says when a crate moves: "A crate moves when its churn in the
incubator has dropped, not in dependency order". Its reason is in the incubator's go-live plan
(`dsp-incubator/cpe/docs/INCUNABULA_GO_LIVE.md`, "Moving"): "moving a still-busy crate turns every prototype
change into a cross-repo change". The amendment of 2026-10-11 (DEV-7464) made one exception, for `cpe-core`,
and said: "The rule stands for the crates that follow. […] Dropping the rule for them would be a changed
decision and a new record (ADR-0006)." This is that record. It replaces the churn sentence of the amendment of
2026-10-02, quoted first above, and nothing else of ADR-0007.

The owner's reasons for dropping the rule (2026-10-11):

- **Churn is planned, not observed.** The rule treats churn as something to wait out. It is the result of
  what the team chooses to work on. The recent churn in CPE's crates was the work that prepares the move to
  production (the crate split, the store built from the port, the remodelling), and with that done much less
  is expected. Measuring past commits says little about the next weeks.
- **The go-live schedule.** Serving Incunabula from this repository is the priority. Waiting for a crate to
  look quiet delays DEV-7465 and the production path by more than the cross-repository changes cost.
- **Vendoring back has proved cheap.** The cost the rule feared is a pin move and an empty `just vendor-diff`
  in the incubator, as `cpe-ports` and `sync-store` already pay at each pin move.

The decision, phrased so a violation is describable in code:

- **A CPE crate moves when the next piece of work needs it here, in dependency order.** A crate moves after
  the CPE crates it depends on, and its churn in the incubator is not a condition.
- **The store and page assembly still move last, with the wiring** and the retirement of the prototype's
  binary, as the amendment of 2026-10-02 has it.
- **Every move still ends with the incubator vendoring the crate back** under its record
  `0013-new-projects-are-prototyped-against-a-vendored-monorepo-cpe.md`, and with an empty `just vendor-diff`
  there. From the move on, a change to the crate is a PR here.
- **The cost the old rule avoided is accepted.** The rule's reason is not denied: a change to a moved crate is
  a PR here followed by a pin move in the incubator.

## Considered Options

- **Crates move in dependency order, when the next piece of work needs them (chosen).**
- **Keep the churn rule and wait for each crate to go quiet** — rejected for the three reasons above.
- **Keep the rule and make a second exception, for the component library** — rejected: with `cpe-core`,
  the image viewer and the component library moved, the only crates left are the store and page assembly,
  which move last under either rule. A rule that every remaining case is an exception to decides nothing.
- **Amend ADR-0007 again, without a new record** — rejected: ADR-0006 gives a decision that changes rather
  than narrows a record of its own, and the amendment of 2026-10-11 (DEV-7464) names this case as one.

## Consequences

- ADR-0007 stays `accepted` and is not superseded as a whole: one sentence of one amendment is replaced, and
  every other clause stands. It takes a dated amendment that points here, and a dated note under the sentence
  of the DEV-7464 amendment that says the rule stands.
- A change to a moved crate is made here and reaches the incubator with its next pin move, also while the
  incubator's work on that crate is still busy.
- The churn measurements in the amendment of 2026-10-11 (DEV-7464) stay as the record of why `cpe-core` was
  an exception at the time; no later move repeats them.

Enforced by: review, for a crate that moves into `areas/access/cpe` before a CPE crate it depends on, and
for a move that does not end with the incubator's re-vendor and an empty `just vendor-diff` there.
