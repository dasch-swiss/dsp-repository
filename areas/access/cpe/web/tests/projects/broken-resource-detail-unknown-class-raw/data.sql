-- A single Character resource. The config's resource-detail declares class
-- "Charakter" (typo'd), so UnknownResourceDetailClass fires and the
-- show-properties sparse check must be skipped entirely (class unknown).
INSERT INTO resources (id, class) VALUES ('c1', 'Character');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('c1', 'name', 0, 'text', 'Alice');
