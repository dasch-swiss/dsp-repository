-- 'Thing' class with a 'name' (text) property and a 'description' (richtext)
-- property — both are the search-bar's "searches" targets.
--
-- t1 'RabbIT' — a prefix search on "rab" must match this via FTS5's
-- unicode61 tokenizer casefolding (case-insensitive prefix match).
-- t2 'Quantum Cat' — has no relation to "rab"/"caterpillar"; distinguishes
-- a real non-match from an over-broad query.
-- t3 'Absolem' — the word "Caterpillar" appears ONLY in its richtext
-- description, never in its name — exercises full-text-only matching (the
-- richtext markdown must be stripped to plain text and indexed).
-- No resource anywhere contains "xyzzy" — used to assert a genuine
-- zero-match query.
INSERT INTO resources (id, class) VALUES
    ('t1', 'Thing'),
    ('t2', 'Thing'),
    ('t3', 'Thing');

INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES
    ('t1', 'name', 0, 'text', 'RabbIT'),
    ('t1', 'description', 0, 'richtext', 'A quantum cat costume.'),
    ('t2', 'name', 0, 'text', 'Quantum Cat'),
    ('t2', 'description', 0, 'richtext', 'A curious feline.'),
    ('t3', 'name', 0, 'text', 'Absolem'),
    ('t3', 'description', 0, 'richtext', 'A wise **Caterpillar** on a mushroom.');
