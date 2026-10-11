-- theta data: three Widgets with a universal 'name' text property (search
-- target). No list-value/facet-relevant property is declared at all — the
-- facet-panel has zero `facet` children in config.kdl, so it never queries
-- data for facets; this file only needs to satisfy the search-bar.
-- "RabbIT Widget" prefix-matches a `q=rab` search.
INSERT INTO resources (id, class) VALUES
    ('w1', 'Widget'),
    ('w2', 'Widget'),
    ('w3', 'Widget');
INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
    ('w1', 'name', 0, 'text', 'RabbIT Widget',      'en'),
    ('w2', 'name', 0, 'text', 'Quantum Cat Widget', 'en'),
    ('w3', 'name', 0, 'text', 'Teapot Widget',      'en');
