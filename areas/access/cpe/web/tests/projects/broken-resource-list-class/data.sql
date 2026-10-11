-- Only a 'Thing' class exists; the config asks for 'NoSuchClass' → hard-fail.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('r1', 'name', 0, 'text', 'Test Resource');
