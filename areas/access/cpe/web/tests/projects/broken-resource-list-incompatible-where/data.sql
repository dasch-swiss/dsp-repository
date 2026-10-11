-- 'Thing' class has 'name' stored only as value_text (never value_int).
-- The config's `where property="name" equals=5` compares an integer, which
-- can never match a text-only column → IncompatibleWhereValue warning.
INSERT INTO resources (id, class) VALUES ('t1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('t1', 'name', 0, 'text', 'A Thing');
