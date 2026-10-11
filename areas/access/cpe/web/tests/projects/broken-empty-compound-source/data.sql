-- broken-empty-compound-source: ST_001 exists as a resource but has no
-- compound_parts rows. This triggers the EmptyCompoundSource warning.
INSERT INTO resources (id, class) VALUES ('ST_001', 'Story');
