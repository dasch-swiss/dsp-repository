-- broken-search-sparse data.
-- 'Thing' class has no 'description' property.
-- 'Other' class has a 'description' property with value_type='text'.
-- 'description' exists in the data (on Other) but not on Thing →
-- SparseSearchProperty warning. The project should load successfully
-- (warning is non-fatal).
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('r1', 'name', 0, 'text', 'Test Thing');

INSERT INTO resources (id, class) VALUES ('o1', 'Other');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('o1', 'description', 0, 'text', 'A description on another class.');
