-- Two Books: `name`, `pub-place` (text), `extent` (integer) and `office`
-- (a list_value into a one-node list).
INSERT INTO lists (id, name) VALUES ('list-office', 'office');
INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES ('office-a', 'list-office', NULL, 0);
INSERT INTO resources (id, class) VALUES ('b1', 'Book');
INSERT INTO resources (id, class) VALUES ('b2', 'Book');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b1', 'name', 0, 'text', 'Buch 1');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b2', 'name', 0, 'text', 'Buch 2');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b1', 'pub-place', 0, 'text', 'Basel');
INSERT INTO properties (resource_id, name, seq, value_type, value_int)
    VALUES ('b1', 'extent', 0, 'integer', 80);
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('b1', 'office', 0, 'list_value', 'office-a');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('b2', 'office', 0, 'list_value', 'office-a');
