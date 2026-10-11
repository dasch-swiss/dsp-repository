-- A 'Thing' class with a 'name' property. The config references 'no-such-prop'
-- and 'another-missing-prop' which exist nowhere in the data → hard-fail.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('r1', 'name', 0, 'text', 'Test Resource');
