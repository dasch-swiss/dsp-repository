-- 'Thing' class has 'name'; 'Widget' class has only 'label'.
-- Config lists class="Widget" with WHERE property="name", which exists (on Thing)
-- but not on Widget → SparseResourceListProperty warning.
-- Mirrors broken-resource-list-sparse but for a WHERE clause instead of sort.
INSERT INTO resources (id, class) VALUES ('t1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('t1', 'name', 0, 'text', 'A Thing');

INSERT INTO resources (id, class) VALUES ('w1', 'Widget');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('w1', 'label', 0, 'text', 'A Widget');
