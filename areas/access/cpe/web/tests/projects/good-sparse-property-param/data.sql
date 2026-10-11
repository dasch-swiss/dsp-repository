-- Two Things: both carry 'name', only r1 carries 'subtitle' (1/2 — uneven,
-- but present on the class, so no SparseProperty warning).
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r1', 'name', 0, 'text', 'First Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r1', 'subtitle', 0, 'text', 'A subtitle');
INSERT INTO resources (id, class) VALUES ('r2', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r2', 'name', 0, 'text', 'Second Thing');
