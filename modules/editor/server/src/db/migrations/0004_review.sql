-- Migration 0004 — the two columns the review surface writes (US-4).
--
-- Same rules as 0001–0003: applied inside the one BEGIN IMMEDIATE transaction
-- that also bumps `user_version`, forward-only, never edited once released.

-- The per-field decisions and substitutions RDU has recorded on this submission
-- (REQ-4.3), as a JSON object keyed by project member name. Nullable, and null
-- until a reviewer decides something: an empty object and "nothing decided" are
-- the same state, and storing one of them would make a reload able to tell them
-- apart when nothing else can.
--
-- A reviewer's substituted value goes here and NOT into `payload`. Overwriting
-- the payload would be the shorter path and would destroy the evidence the
-- depositor has to be shown: REQ-4.4 waives the second approver, so the value
-- RDU put in place of theirs is seen by nobody unless the submitted one
-- survives beside it.
--
-- Opaque to this layer, like `payload` — the reviewing handler parses it.
ALTER TABLE submissions ADD COLUMN review_state TEXT;

-- The note RDU left when it returned the project to the depositor (REQ-4.5).
--
-- On the draft rather than on the submission, because request-changes turns the
-- submission *into* a draft: `submissions.reviewer_note` is deleted with its row
-- at exactly the moment the depositor needs to read it, so a note kept only
-- there could never reach the person it is addressed to.
ALTER TABLE drafts ADD COLUMN reviewer_note TEXT;
