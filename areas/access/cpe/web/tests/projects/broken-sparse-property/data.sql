-- Resource r1 of class Thing exists, with only a 'name' property.
-- show-properties lists 'missing-prop' which is absent, and title-property
-- 'nonexistent-prop' is also absent — both trigger SparseProperty warnings.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r1', 'name', 0, 'text', 'Test Resource');
