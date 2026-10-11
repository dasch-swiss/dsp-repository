-- Only r1 exists; 'ghost' (the narrative-for id) does not, so
-- UnknownNarrativeForResource fires.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
