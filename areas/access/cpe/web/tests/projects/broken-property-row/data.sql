-- Minimal resources to satisfy FK constraints.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');

-- Violation 1: text row with NULL value_text.
INSERT INTO properties (resource_id, name, seq, value_type)
    VALUES ('r1', 'label', 0, 'text');

-- Violation 2: date row with NULL date_start.
INSERT INTO properties (resource_id, name, seq, value_type)
    VALUES ('r1', 'created', 0, 'date');

-- Violation 3: link row with NULL target_id.
INSERT INTO properties (resource_id, name, seq, value_type)
    VALUES ('r1', 'related', 0, 'link');
