-- Migration 0002 — the two columns the login flow needs and 0001 could not
-- have known the shape of.
--
-- Same rules as 0001: applied inside the one BEGIN IMMEDIATE transaction that
-- also bumps `user_version`, forward-only, never edited once released.

-- When the account-level counter last went up. `failed_logins` alone cannot
-- express a lockout that ends: NIST SP 800-63B-4 requires the count survive a
-- new secret and reset only on success, which means a capped account can never
-- clear itself by succeeding — it is locked out. Throttling therefore has to be
-- time-based, and that needs an instant to measure from. Cleared with the
-- counter on a successful authentication.
ALTER TABLE users ADD COLUMN failed_login_at TEXT;

-- The pre-auth binding: the opaque token handed to the browser that asked for
-- this code, and the only browser that may spend it. It blocks the main attack
-- email one-time codes are exposed to — an attacker triggers a login for the
-- victim, talks them into reading the code out, and spends it from their own
-- machine.
--
-- Nullable rather than `NOT NULL DEFAULT ''`: a default would give any row
-- predating this migration the empty token, and an empty cookie would then
-- match it. NULL matches nothing, so the fail-closed reading is the one the
-- column type already enforces.
--
-- Stored unhashed for the same reason as `code`: it lives ten minutes, and
-- anyone who can read this table already holds `sessions`.
ALTER TABLE login_codes ADD COLUMN browser_token TEXT;

-- UNIQUE, not a plain index: it is the lookup key for verification, and the
-- uniqueness is the guarantee that one token addresses one code. SQLite allows
-- many NULLs in a unique index, so pre-migration rows do not collide.
CREATE UNIQUE INDEX login_codes_browser_token ON login_codes (browser_token);
