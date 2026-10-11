-- 'bio' is a text property on Character, not link-typed.
INSERT INTO resources (id, class) VALUES ('r1', 'Character');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('r1', 'bio', 0, 'text', 'A curious girl.');
