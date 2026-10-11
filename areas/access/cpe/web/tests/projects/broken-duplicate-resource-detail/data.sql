-- broken-duplicate-resource-detail data. Both literal resource ids referenced
-- by the two resource-detail components exist, so the only diagnostic
-- produced is DuplicateSingletonComponent (no unrelated UnknownResource).
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO resources (id, class) VALUES ('r2', 'Thing');
