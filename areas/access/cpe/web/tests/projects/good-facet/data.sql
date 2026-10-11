-- good-facet data.
-- 'Thing' class with a 'category' property of value_type='list_value' present
-- on every resource of the class (universal). The facet-panel declares
-- kind="list" on 'category' → no IncompatibleFacetKind, no UnknownFacetProperty,
-- and no SparseFacetProperty (every Thing has the property).
INSERT INTO lists (id, name) VALUES ('categories', 'Categories');
INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES
    ('cat-a', 'categories', NULL, 0),
    ('cat-b', 'categories', NULL, 1);
INSERT INTO list_node_labels (node_id, lang, label) VALUES
    ('cat-a', 'en', 'Category A'),
    ('cat-a', 'de', 'Kategorie A'),
    ('cat-b', 'en', 'Category B'),
    ('cat-b', 'de', 'Kategorie B');

INSERT INTO resources (id, class) VALUES
    ('t1', 'Thing'),
    ('t2', 'Thing'),
    ('t3', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
    ('t1', 'category', 0, 'list_value', 'cat-a'),
    ('t2', 'category', 0, 'list_value', 'cat-a'),
    ('t3', 'category', 0, 'list_value', 'cat-b');
