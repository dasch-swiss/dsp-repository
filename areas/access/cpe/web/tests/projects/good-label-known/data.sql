-- Resource r1 of class Thing exists, with a 'name' property. Both the
-- 'name' property label and the 'Thing' class label above name real,
-- present data — neither should produce an UnknownLabel warning.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r1', 'name', 0, 'text', 'Test Resource');
