-- Minimal Thing data. ReservedFacetProperty fires purely from config (the
-- facet's declared property name shadowing a reserved query key), before any
-- data lookup, so no 'sort' property data is needed to trigger it.
INSERT INTO resources (id, class) VALUES ('t1', 'Thing');
