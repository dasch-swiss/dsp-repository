-- Two Books (with `name` and `pub-date`) and a Map carrying `extent` — so
-- `extent` exists in the data but on no Book (sparse on Book).
INSERT INTO resources (id, class) VALUES ('b1', 'Book');
INSERT INTO resources (id, class) VALUES ('b2', 'Book');
INSERT INTO resources (id, class) VALUES ('m1', 'Map');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b1', 'name', 0, 'text', 'Buch 1');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b1', 'pub-date', 0, 'text', '1485');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b2', 'name', 0, 'text', 'Buch 2');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b2', 'pub-date', 0, 'text', '1492');
INSERT INTO properties (resource_id, name, seq, value_type, value_int)
    VALUES ('m1', 'extent', 0, 'integer', 2);
