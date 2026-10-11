-- Two resources of class Thing, both carrying only a 'name' property.
-- The parameterised resource-detail lists 'missing-prop' in show-properties
-- and 'nonexistent-prop' as title-property; neither is present on ANY Thing,
-- so both trigger SparseProperty warnings computed over the class.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r1', 'name', 0, 'text', 'First Thing');
INSERT INTO resources (id, class) VALUES ('r2', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r2', 'name', 0, 'text', 'Second Thing');
