-- broken-facet-incompatible-kind data.
-- 'Thing' class has a 'role' property of value_type='text'. The facet-panel
-- declares kind="list", which requires value_type='list_value' → IncompatibleFacetKind.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('r1', 'role', 0, 'text', 'Main Character');
