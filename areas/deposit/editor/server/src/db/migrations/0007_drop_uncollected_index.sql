-- Migration 0007 — drop the partial index over uncollected approved records.
--
-- Same rules as 0001 through 0006: applied inside the one BEGIN IMMEDIATE
-- transaction that also bumps `user_version`, forward-only, never edited once
-- released.

-- Nothing reads it: every approved-record query leads with `id` or `shortcode`,
-- or enumerates every record unfiltered.
DROP INDEX approved_records_uncollected;
