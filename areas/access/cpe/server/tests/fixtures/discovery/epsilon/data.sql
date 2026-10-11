INSERT INTO resources (id, class) VALUES
    ('res-01', 'Thing');

INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
    ('res-01', 'name',  0, 'text', 'The Featured Resource', 'en'),
    ('res-01', 'quote', 0, 'text', 'A static literal-id quote', 'en');
