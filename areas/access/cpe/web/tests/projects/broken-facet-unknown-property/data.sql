-- broken-facet-unknown-property data.
-- 'Thing' class has a 'name' property only. The facet-panel declares
-- 'no-such-prop' which exists nowhere in the data → UnknownFacetProperty.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('r1', 'name', 0, 'text', 'Test Thing');
