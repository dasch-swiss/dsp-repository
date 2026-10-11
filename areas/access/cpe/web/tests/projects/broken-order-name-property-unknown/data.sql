-- Two Books carrying `name` and `pub-date`; no resource carries `no-such-prop`.
INSERT INTO resources (id, class) VALUES ('b1', 'Book');
INSERT INTO resources (id, class) VALUES ('b2', 'Book');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b1', 'name', 0, 'text', 'Buch 1');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b1', 'pub-date', 0, 'text', '1485');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b2', 'name', 0, 'text', 'Buch 2');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b2', 'pub-date', 0, 'text', '1492');
