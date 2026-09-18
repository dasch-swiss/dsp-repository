-- Migration 0004 — advisory fields the collection pull request reports back.
--
-- Same rules as 0001 through 0003: applied inside the one BEGIN IMMEDIATE
-- transaction that also bumps `user_version`, forward-only, never edited once
-- released.

-- All three are NULL until a collection run has something to say, and stay
-- NULL forever for a record no report has touched yet.
ALTER TABLE approved_records ADD COLUMN pull_request_url TEXT;
ALTER TABLE approved_records ADD COLUMN pull_request_state TEXT
    CHECK (pull_request_state IN ('open', 'merged', 'closed'));
-- NULL when the last report carried no failure — including every record no
-- report has named yet, which is indistinguishable from one that last
-- succeeded. Nothing here needs to tell those apart.
ALTER TABLE approved_records ADD COLUMN last_failure TEXT;
