-- 'Character' class has a 'name' property only. The 'media' attribute
-- declares 'no-such-prop' which exists nowhere in the data.
INSERT INTO resources (id, class) VALUES ('r1', 'Character');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('r1', 'name', 0, 'text', 'Alice');
