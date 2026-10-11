-- broken-facet-multi data.
-- 'Thing' class has a 'role' text property and a 'name' text property.
-- 'ghost-prop' does not exist anywhere → UnknownFacetProperty.
-- 'role' exists but as 'text' not 'list_value' → IncompatibleFacetKind.
-- Both errors must be reported in the same validation pass (accumulation).
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('r1', 'role', 0, 'text', 'Main Character');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('r1', 'name', 1, 'text', 'Test Thing');
