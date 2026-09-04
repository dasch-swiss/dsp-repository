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
    -- The note RDU leaves when it returns the project to the depositor.
    --
    -- On the draft rather than on the submission, because request-changes turns
    -- the submission *into* a draft: `submissions.reviewer_note` is deleted with
    -- its row at exactly the moment the depositor needs to read it, so a note
    -- kept only there could never reach the person it is addressed to.
    reviewer_note TEXT,
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
