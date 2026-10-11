-- Minimal data: contains a resource, but NOT 'ghost-resource'.
INSERT INTO resources (id, class) VALUES ('real-resource', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('real-resource', 'name', 0, 'text', 'Real Resource');
