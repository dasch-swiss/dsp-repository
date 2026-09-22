-- Migration 0005 — when a collection run last reported on a record.
--
-- Same rules as 0001 through 0004: applied inside the one BEGIN IMMEDIATE
-- transaction that also bumps `user_version`, forward-only, never edited once
-- released.

-- NULL until a collection run has reported on this record; moved by every
-- report thereafter, whatever it said, so it is the age of the state beside
-- it rather than the moment of first dispatch.
ALTER TABLE approved_records ADD COLUMN reported_at TEXT;
