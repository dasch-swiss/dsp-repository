-- r1 of class Thing carries both 'name' (show-properties) and 'creator'
-- (the group member) — both present, so no SparseProperty; r1 also exists,
-- so the narrative-for id resolves cleanly.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r1', 'name', 0, 'text', 'Test Resource');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r1', 'creator', 0, 'text', 'Test Creator');
