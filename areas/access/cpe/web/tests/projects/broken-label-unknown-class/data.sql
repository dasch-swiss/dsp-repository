-- Resource r1 of class Thing exists. The 'labels' block names 'UnknownClass',
-- which is absent from the data entirely — triggers an UnknownLabel warning.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r1', 'name', 0, 'text', 'Test Resource');
