-- good-compound-viewer: ST_001 exists as a compound resource with two member
-- chapters. The compound-viewer pointing at ST_001 should produce no
-- EmptyCompoundSource warning.
INSERT INTO resources (id, class) VALUES ('ST_001', 'Story');
INSERT INTO resources (id, class) VALUES ('SCH_001', 'StoryChapter');
INSERT INTO resources (id, class) VALUES ('SCH_002', 'StoryChapter');
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('ST_001', 'SCH_001', 0);
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('ST_001', 'SCH_002', 1);
