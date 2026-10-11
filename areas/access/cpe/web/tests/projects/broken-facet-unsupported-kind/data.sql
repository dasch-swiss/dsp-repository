-- broken-facet-unsupported-kind data.
-- 'Thing' class has a 'year' property so the config-level facet would otherwise
-- be valid. The UnsupportedFacetKind check fires before the data checks for
-- kind compatibility, but we still need valid data for the project to load far
-- enough to reach the data-dependent check.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_int)
    VALUES ('r1', 'year', 0, 'integer', 1450);
