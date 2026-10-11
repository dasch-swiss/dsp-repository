-- r1 and r2 are both Character; r1.colleague links to r2. No resource in
-- the database carries a representations row anywhere.
INSERT INTO resources (id, class) VALUES ('r1', 'Character'), ('r2', 'Character');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('r1', 'colleague', 0, 'link', 'r2');
