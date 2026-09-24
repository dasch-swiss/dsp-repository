-- Migration 0003 — an append-only record of what was actually sent, so the
-- daily send caps count sends instead of inferring them from live rows.
--
-- Same rules as 0001 and 0002: applied inside the one BEGIN IMMEDIATE
-- transaction that also bumps `user_version`, forward-only, never edited once
-- released.

-- One row per message the transport accepted. `login_codes` was the previous
-- stand-in and could not be an honest one: a code rolled back after a failed
-- delivery disappears (correctly — nothing was sent) and a user's unspent codes
-- are deleted when they sign in (incorrectly — those were mailed). So the count
-- sat below the real send volume, and a control named for sends measured
-- something else.
--
-- No explicit primary key: nothing references a send, and the rowid SQLite
-- gives every table is identity enough for an append-only counter. Rows leave
-- only through the hourly prune, which drops everything past the caps' window.
CREATE TABLE mail_sends (
    -- Who it went to, for the per-account cap. Nullable, and SET NULL rather
    -- than CASCADE deliberately: deleting an account must not rewrite how much
    -- of the shared relay budget has already been spent, which is exactly what
    -- the global cap counts. The send happened; the account may go.
    user_id TEXT REFERENCES users (id) ON DELETE SET NULL,
    sent_at TEXT NOT NULL
) STRICT;

-- The global cap counts across all users over a time window, and the prune
-- deletes by the same column.
CREATE INDEX mail_sends_sent_at ON mail_sends (sent_at);
-- The per-account cap counts one user's sends over that window, so `user_id`
-- leads and `sent_at` covers the range. It also indexes the foreign key, which
-- is what keeps `ON DELETE SET NULL` from scanning the table.
CREATE INDEX mail_sends_user_id_sent_at ON mail_sends (user_id, sent_at);

-- `login_codes_created_at` existed for exactly one query — the global cap's
-- `count(*) ... WHERE created_at >= ?` — which `mail_sends` now answers.
-- Nothing else filters or orders on `created_at` alone: the cooldown's
-- compare-and-set and `find_active_for_user` both lead with `user_id`, and the
-- sweep filters `expires_at`. An index nothing reads is still written on every
-- login attempt.
DROP INDEX login_codes_created_at;
