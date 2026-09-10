-- Migration 0001 — initial schema.
--
-- Applied inside one BEGIN IMMEDIATE transaction together with the
-- `user_version` bump, so a crash part-way leaves the database at the previous
-- version rather than half-migrated. Forward-only: once released, this file is
-- never edited — a change is a new numbered file.
--
-- STRICT on every table. Without it SQLite accepts any value in any column and
-- coerces silently, so a mapping bug surfaces as wrong data instead of an error.
--
-- Timestamps are TEXT in rusqlite's chrono format
-- ("YYYY-MM-DD HH:MM:SS.SSS+00:00"): fixed-width and always UTC, so the
-- lexicographic ordering SQLite gives TEXT is chronological ordering, and
-- `expires_at > ?` works. Ids are TEXT in hyphenated UUID form rather than
-- 16-byte blobs, so the database stays legible to an operator reading it by
-- hand — which is the only way in, the image having no shell.
--
-- Every foreign key is indexed. Without an index on the child column, SQLite
-- scans the whole child table for each parent row deleted, so `ON DELETE
-- CASCADE` on `users` would degrade with the number of sessions.

CREATE TABLE users (
    id               TEXT    NOT NULL PRIMARY KEY,
    -- As entered, plaintext (PRD Constraints: the app must decrypt to send, so
    -- a key would sit beside the data).
    email            TEXT    NOT NULL,
    -- Lowercased. Carries the uniqueness constraint (REQ-7.4) and every lookup,
    -- so `A@x.test` cannot shadow `a@x.test`.
    email_normalized TEXT    NOT NULL UNIQUE,
    name             TEXT    NOT NULL,
    role             TEXT    NOT NULL CHECK (role IN ('depositor', 'rdu')),
    -- Account-level consecutive failures. NIST SP 800-63B-4: a new secret SHALL
    -- NOT reset the count, so it lives here and not on the code.
    failed_logins    INTEGER NOT NULL DEFAULT 0 CHECK (failed_logins >= 0),
    last_code_at     TEXT,
    created_at       TEXT    NOT NULL
) STRICT;

-- Project assignments (REQ-1.2, REQ-7.3). A child table rather than a JSON
-- column on `users`, so "who holds shortcode X" is answerable — needed when
-- removing a shortcode from someone who has a draft on it.
CREATE TABLE user_shortcodes (
    user_id   TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    shortcode TEXT NOT NULL,
    PRIMARY KEY (user_id, shortcode)
) STRICT;

CREATE INDEX user_shortcodes_shortcode ON user_shortcodes (shortcode);

CREATE TABLE sessions (
    -- The opaque token the cookie carries, not a UUID: how it is minted is the
    -- auth layer's decision.
    id           TEXT NOT NULL PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at   TEXT NOT NULL,
    -- Advanced on use, for the idle timeout.
    last_seen_at TEXT NOT NULL,
    -- Absolute expiry, set at creation and never extended.
    expires_at   TEXT NOT NULL
) STRICT;

CREATE INDEX sessions_user_id ON sessions (user_id);
CREATE INDEX sessions_expires_at ON sessions (expires_at);

CREATE TABLE login_codes (
    id          TEXT    NOT NULL PRIMARY KEY,
    user_id     TEXT    NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Unhashed on purpose: it lives ten minutes, and anyone who can read this
    -- table already holds `sessions` (PRD Constraints).
    code        TEXT    NOT NULL,
    -- Wrong entries against this code; three invalidates it (REQ-6.4).
    attempts    INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    created_at  TEXT    NOT NULL,
    expires_at  TEXT    NOT NULL,
    -- Set on acceptance. A code authenticates once (NIST §3.1.3.2).
    consumed_at TEXT
) STRICT;

CREATE INDEX login_codes_user_id ON login_codes (user_id);
-- The global daily send cap counts across all users over a time window.
CREATE INDEX login_codes_created_at ON login_codes (created_at);

-- One draft per project, not per user: per-user multiple drafts are out of
-- scope and concurrency is last-write-wins.
CREATE TABLE drafts (
    shortcode  TEXT NOT NULL PRIMARY KEY,
    -- JSON. The permissive draft representation is Phase 4's; this layer never
    -- interprets the body.
    payload    TEXT NOT NULL,
    -- SET NULL, not CASCADE: removing an account must not destroy the project's
    -- work. "Last editor" then reads as unknown rather than dangling.
    updated_by TEXT REFERENCES users (id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE INDEX drafts_updated_by ON drafts (updated_by);
CREATE INDEX drafts_updated_at ON drafts (updated_at);

CREATE TABLE submissions (
    id            TEXT NOT NULL PRIMARY KEY,
    -- UNIQUE is PRD Constraints' "one pending submission per project", enforced
    -- here rather than left as something handlers must remember.
    shortcode     TEXT NOT NULL UNIQUE,
    payload       TEXT NOT NULL,
    -- REQ-2.1's Draft and Online are absent by design: a draft is a `drafts`
    -- row, and Online is derived at startup, at which point the local record is
    -- discarded (REQ-2.4).
    state         TEXT NOT NULL CHECK (state IN ('submitted', 'in_review', 'approved')),
    submitted_by  TEXT REFERENCES users (id) ON DELETE SET NULL,
    submitted_at  TEXT NOT NULL,
    reviewed_by   TEXT REFERENCES users (id) ON DELETE SET NULL,
    reviewed_at   TEXT,
    -- Carried back to the depositor when RDU requests changes.
    reviewer_note TEXT,
    -- The per-field decisions and substitutions RDU has recorded on this
    -- submission, as a JSON object keyed by project member name. Null until a
    -- reviewer decides something: an empty object and "nothing decided" are the
    -- same state, and storing one of them would make a reload able to tell them
    -- apart when nothing else can.
    --
    -- A reviewer's substituted value goes here and NOT into `payload`.
    -- Overwriting the payload would be the shorter path and would destroy the
    -- evidence the depositor has to be shown: a depositor's submission needs no
    -- second approver, so the value RDU put in place of theirs is seen by nobody
    -- unless the submitted one survives beside it.
    --
    -- Opaque to this layer, like `payload` — the reviewing handler parses it.
    review_state  TEXT
) STRICT;

-- The review queue is oldest first (REQ-4.1).
CREATE INDEX submissions_submitted_at ON submissions (submitted_at);
CREATE INDEX submissions_submitted_by ON submissions (submitted_by);
CREATE INDEX submissions_reviewed_by ON submissions (reviewed_by);

-- Finished review rounds: what was decided about a submission, by whom, and
-- what the depositor has to be told (REQ-4.4 to REQ-4.7).
--
-- Every outcome deletes the `submissions` row, so this is the only thing left
-- saying what happened. REQ-2.1 fixes the state list at five and has no
-- Rejected, so a terminated round cannot be a submission state without adding a
-- sixth.
--
-- Four things read it, and each would otherwise want a column of its own:
-- a rejection being visible at all (REQ-4.6 discards and notifications are out
-- of scope); a returned draft being distinguishable from one never submitted;
-- the per-field accepted state surviving the return (REQ-4.5), which is what
-- makes locking an accepted field possible; and the depositor seeing what RDU
-- substituted for their value before approving. The last two read
-- `review_state`.
--
-- Append-only: a row is written by the transition that ends the round and never
-- updated, so the rows for one project are its review history. Repeated rounds
-- leave a trail rather than overwriting one field.
CREATE TABLE review_rounds (
    id            TEXT NOT NULL PRIMARY KEY,
    shortcode     TEXT NOT NULL,
    -- The submission this round ended. Not a foreign key: that row is deleted
    -- by the same transaction, so a reference would either fail or have to be
    -- nulled, and this is how two rounds recorded in the same second are told
    -- apart.
    submission_id TEXT NOT NULL,
    -- `withdrawn` is the depositor's own discard (REQ-4.7). It is a round like
    -- the others because it answers the same question — what became of the
    -- submission — and answering it from two places would leave the
    -- depositor's own action the one with no trace.
    outcome       TEXT NOT NULL CHECK (outcome IN ('approved', 'changes_requested', 'rejected', 'withdrawn')),
    -- What the depositor is told. Null is allowed for every outcome: requiring
    -- one is the handler's rule, since a withdrawal has nobody to address.
    note          TEXT,
    -- The `submissions.review_state` snapshot as the round ended, or null where
    -- nothing was decided. A snapshot and not a reference, for the reason
    -- `submission_id` is not a foreign key: nothing else can answer which
    -- fields were accepted or what was put in place of the depositor's values.
    review_state  TEXT,
    -- SET NULL, not CASCADE, for the reason `drafts.updated_by` is: removing an
    -- account must not destroy the record of what was decided.
    actor         TEXT REFERENCES users (id) ON DELETE SET NULL,
    at            TEXT NOT NULL
) STRICT;

-- The depositor's form reads the newest round for one project, so this is the
-- query the index exists for. `at` descending is served by an ascending index
-- read backwards, so no second index is needed.
CREATE INDEX review_rounds_shortcode_at ON review_rounds (shortcode, at);
CREATE INDEX review_rounds_actor ON review_rounds (actor);

CREATE TABLE approved_records (
    id           TEXT NOT NULL PRIMARY KEY,
    shortcode    TEXT NOT NULL,
    payload      TEXT NOT NULL,
    approved_by  TEXT REFERENCES users (id) ON DELETE SET NULL,
    approved_at  TEXT NOT NULL,
    -- NULL while uncollected. A failed collection leaves it NULL, which is what
    -- makes the next run retry it (REQ-5.7).
    collected_at TEXT
) STRICT;

-- Partial index: the collection endpoint only ever asks for the uncollected
-- ones, and collected rows stay out of the index entirely.
CREATE INDEX approved_records_uncollected ON approved_records (approved_at) WHERE collected_at IS NULL;
CREATE INDEX approved_records_shortcode ON approved_records (shortcode);
CREATE INDEX approved_records_approved_by ON approved_records (approved_by);

-- Entity proposals (US-3): a person or organisation a depositor proposes to
-- create, or a change to one their project already references.
--
-- A table of its own rather than a member of `drafts.payload`, and not by
-- preference: `ProjectDraft` is `#[serde(transparent)]` over the project's JSON
-- members and `to_raw` deserializes it into `ProjectRaw`, so a non-contract
-- member would be dropped on the way to a file with nothing saying so.
--
-- It is deliberately NOT a child of `submissions`. REQ-3.3 carries a proposal
-- inside the project's pending submission, but every review outcome deletes
-- that row while the proposal outlives it — an accepted one is on its way to a
-- pull request, and a returned one is still the depositor's to finish. Keyed by
-- shortcode for the same reason `review_rounds` is.
CREATE TABLE entity_proposals (
    id           TEXT NOT NULL PRIMARY KEY,
    shortcode    TEXT NOT NULL,
    -- The allocated `person-NNN` / `organization-NNN`, or, for a `change`, the
    -- id of the entity being changed. One namespace across both stores, because
    -- `contactPoint`, `attributions[].contributor` and `funding[].funders` each
    -- accept either kind and resolve through one lookup.
    entity_id    TEXT NOT NULL,
    kind         TEXT NOT NULL CHECK (kind IN ('person', 'organization')),
    operation    TEXT NOT NULL CHECK (operation IN ('new', 'change')),
    -- JSON: a `platform_metadata::Person` or `Organization` body. Opaque to
    -- this layer, like `drafts.payload` — a half-filled proposal cannot
    -- deserialize as the contract type yet, and deciding that is submit's job.
    --
    -- It does NOT carry `id`. That is `entity_id` beside it, which the
    -- allocator and the uniqueness index work on, so a second copy in the
    -- payload would be free to drift from the one actually claimed. Readers
    -- fill it in from `entity_id` and overwrite whatever they find.
    payload      TEXT NOT NULL,
    status       TEXT NOT NULL CHECK (status IN ('draft', 'submitted', 'accepted', 'rejected', 'withdrawn')),
    -- What RDU recorded about this proposal in the round now running, separate
    -- from `status`, which is the lifecycle. The same split
    -- `submissions.review_state` makes for a project field: a decision is taken
    -- during the review and only becomes a status when the round ends, so
    -- request-changes can hand the proposal back as a draft while retaining
    -- what was decided — REQ-4.5 requires exactly that for fields, and a
    -- proposal reviewed on the same surface must not lose it.
    --
    -- Null while undecided. An explicitly-undecided value would be the same
    -- state written twice, which is the argument `review::Decision`'s docs make.
    decision     TEXT CHECK (decision IN ('accept', 'reject')),
    -- SET NULL, not CASCADE, for the reason `drafts.updated_by` is: removing an
    -- account must not destroy the project's work.
    proposed_by  TEXT REFERENCES users (id) ON DELETE SET NULL,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    -- Who accepted or rejected it, and when. The decision lives here and not in
    -- `submissions.review_state`, which is keyed by project member: a proposal
    -- is not one, and the submission carrying the state is deleted by the
    -- transition that ends the round.
    decided_by   TEXT REFERENCES users (id) ON DELETE SET NULL,
    decided_at   TEXT
) STRICT;

-- An allocated id is claimed exactly once, whatever became of the proposal that
-- claimed it. This is the guard REQ-3.6 needs and REQ-5.4 does not give: REQ-5.4
-- renumbers on collision with the repository, and nothing in it stops two
-- proposals inside the editor both taking the next free id. Terminal rows stay
-- in the index on purpose — an id handed to a rejected proposal must not be
-- handed to a different entity later, because a sibling collection pull request
-- may already carry it.
--
-- Partial, on `new` only: a `change` names an id somebody else allocated, and
-- several projects may propose changes to one entity (last collected wins).
CREATE UNIQUE INDEX entity_proposals_allocated_id ON entity_proposals (entity_id) WHERE operation = 'new';

-- One live proposal per project per entity. Without it a depositor who proposes
-- changes to the same organisation twice has two rows the review surface would
-- show as separate decisions over one file, and whichever applied last would
-- silently win.
CREATE UNIQUE INDEX entity_proposals_live_per_entity
    ON entity_proposals (shortcode, entity_id) WHERE status IN ('draft', 'submitted');

-- The form and the review surface both read one project's proposals.
CREATE INDEX entity_proposals_shortcode_status ON entity_proposals (shortcode, status);
-- Allocation takes the highest number for a kind, on the write path.
CREATE INDEX entity_proposals_kind_entity_id ON entity_proposals (kind, entity_id);
CREATE INDEX entity_proposals_proposed_by ON entity_proposals (proposed_by);
CREATE INDEX entity_proposals_decided_by ON entity_proposals (decided_by);
