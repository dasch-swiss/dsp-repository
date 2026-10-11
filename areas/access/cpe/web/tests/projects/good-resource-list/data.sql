-- 'Thing' class with 'name' property — matches the config's class and sort/where.
-- No resource-list diagnostics should fire.
INSERT INTO resources (id, class) VALUES ('t1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('t1', 'name', 0, 'text', 'Test Thing');
