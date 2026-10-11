-- r1 of class Thing has only a 'name' property; the group member
-- 'missing-prop' is absent from the class, triggering SparseProperty.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r1', 'name', 0, 'text', 'Test Resource');
