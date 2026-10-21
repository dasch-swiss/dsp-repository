-- Migration 0001: the baseline schema of identity's database, `identity.sqlite`.
--
-- Applied inside one BEGIN IMMEDIATE transaction together with the
-- `user_version` bump, so a crash part-way leaves the database at the previous
-- version rather than half-migrated.
--
-- Edited in place until the first production deployment (DEV-6921). After
-- it, frozen: the `user_version` guard never re-runs an applied migration, so a
-- change is a new numbered file.
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
    -- As entered, plaintext (the app must decrypt to send, so
    -- a key would sit beside the data).
    email            TEXT    NOT NULL,
    -- Lowercased. Carries the uniqueness constraint and every lookup,
    -- so `A@x.test` cannot shadow `a@x.test`.
    email_normalized TEXT    NOT NULL UNIQUE,
    name             TEXT    NOT NULL,
    role             TEXT    NOT NULL CHECK (role IN ('depositor', 'rdu')),
    -- Account-level consecutive failures. NIST SP 800-63B-4: a new secret SHALL
    -- NOT reset the count, so it lives here and not on the code.
    failed_logins    INTEGER NOT NULL DEFAULT 0 CHECK (failed_logins >= 0),
    last_code_at     TEXT,
    created_at       TEXT    NOT NULL,
    -- When `failed_logins` last went up, so a lockout can expire: the count
    -- resets only on success (NIST SP 800-63B-4). Cleared with the counter.
    failed_login_at  TEXT
) STRICT;

-- Assigned projects. A child table rather than a JSON
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
    -- table already holds `sessions`.
    code        TEXT    NOT NULL,
    -- Wrong entries against this code; three invalidates it.
    attempts    INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    created_at  TEXT    NOT NULL,
    expires_at  TEXT    NOT NULL,
    -- Set on acceptance. A code authenticates once (NIST §3.1.3.2).
    consumed_at TEXT,
    -- The pre-auth binding: only the browser that asked for this code may spend
    -- it, so a code read out to an attacker is useless. Nullable, not `DEFAULT ''`:
    -- NULL matches no cookie, an empty string matches an empty one. Unhashed like
    -- `code`.
    browser_token TEXT
) STRICT;

CREATE INDEX login_codes_user_id ON login_codes (user_id);
-- UNIQUE: one token addresses one code. SQLite allows many NULLs in it.
CREATE UNIQUE INDEX login_codes_browser_token ON login_codes (browser_token);

-- One row per message the transport accepted; the daily send caps count these.
-- `login_codes` cannot stand in: signing in deletes codes that were mailed. No
-- primary key, since nothing references a send; the hourly prune removes rows
-- past the caps' window.
CREATE TABLE mail_sends (
    -- SET NULL, not CASCADE: deleting an account must not refund the relay
    -- budget the global cap counts.
    user_id TEXT REFERENCES users (id) ON DELETE SET NULL,
    sent_at TEXT NOT NULL
) STRICT;

-- The global cap and the prune.
CREATE INDEX mail_sends_sent_at ON mail_sends (sent_at);
-- The per-account cap; also the foreign-key index `ON DELETE SET NULL` needs.
CREATE INDEX mail_sends_user_id_sent_at ON mail_sends (user_id, sent_at);
