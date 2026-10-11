-- eta data: three Widgets, all sharing one 'category' list-value facet
-- (universal → no SparseFacetProperty) and a universal 'name' text property
-- (search target). "RabbIT Widget" prefix-matches a `q=rab` search.
INSERT INTO lists (id, name) VALUES ('categories', 'Categories');
INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES
    ('cat-a', 'categories', NULL, 0);
INSERT INTO list_node_labels (node_id, lang, label) VALUES
    ('cat-a', 'en', 'Category A'),
    ('cat-a', 'de', 'Kategorie A');

INSERT INTO resources (id, class) VALUES
    ('w1', 'Widget'),
    ('w2', 'Widget'),
    ('w3', 'Widget');
INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
    ('w1', 'name', 0, 'text', 'RabbIT Widget',      'en'),
    ('w2', 'name', 0, 'text', 'Quantum Cat Widget', 'en'),
    ('w3', 'name', 0, 'text', 'Teapot Widget',      'en');
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
    ('w1', 'category', 0, 'list_value', 'cat-a'),
    ('w2', 'category', 0, 'list_value', 'cat-a'),
    ('w3', 'category', 0, 'list_value', 'cat-a');
