-- r1 of class Thing has a 'name' property, named by both show-properties
-- and the group — triggers DuplicateGroupedProperty.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r1', 'name', 0, 'text', 'Test Resource');
