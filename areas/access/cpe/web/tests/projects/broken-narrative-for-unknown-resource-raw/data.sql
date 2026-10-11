-- Only r1 exists; 'ghost' (the narrative-for id) does not, so
-- UnknownNarrativeForResource fires via the Err/raw branch of
-- validate_with_data (the config-internal MissingSiteTitle error forces
-- that path).
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
