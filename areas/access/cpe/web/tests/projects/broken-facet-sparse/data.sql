-- broken-facet-sparse data.
-- 'Thing' class has no 'role' property.
-- 'Other' class has a 'role' property with value_type='list_value'.
-- 'role' exists in the data (on Other) but not on Thing → SparseFacetProperty
-- warning. The project should load successfully (warning is non-fatal).
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('r1', 'name', 0, 'text', 'Test Thing');

INSERT INTO resources (id, class) VALUES ('o1', 'Other');
INSERT INTO lists (id, name) VALUES ('roles', 'Roles');
INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES ('n1', 'roles', NULL, 0);
INSERT INTO list_node_labels (node_id, lang, label) VALUES ('n1', 'en', 'Main');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('o1', 'role', 0, 'list_value', 'n1');
