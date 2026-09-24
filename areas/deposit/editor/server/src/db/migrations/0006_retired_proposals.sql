-- Migration 0006 — when an accepted proposal's entity was observed published.
--
-- Same rules as 0001 through 0005: applied inside the one BEGIN IMMEDIATE
-- transaction that also bumps `user_version`, forward-only, never edited once
-- released.

-- NULL until the startup reconciliation pass observes this accepted proposal's
-- entity in the published set; from then on the proposal must not ride along
-- with a new approved record for its project.
ALTER TABLE entity_proposals ADD COLUMN retired_at TEXT;
