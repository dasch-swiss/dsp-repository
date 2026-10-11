INSERT INTO resources (id, class) VALUES
    ('thing-01', 'Thing'),
    ('thing-02', 'Thing'),
    ('thing-03', 'Thing'),
    ('thing-04', 'Thing');

INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
    ('thing-01', 'name', 0, 'text', 'RabbIT',       'en'),
    ('thing-02', 'name', 0, 'text', 'Quantum Cat',  'en'),
    ('thing-03', 'name', 0, 'text', 'Looking Glass', 'en'),
    ('thing-04', 'name', 0, 'text', 'Teapot',       'en');
