-- r1 of class Thing has a 'name' property (present, so no SparseProperty);
-- the group's label 'not-declared' is not a chrome string id.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r1', 'name', 0, 'text', 'Test Resource');
