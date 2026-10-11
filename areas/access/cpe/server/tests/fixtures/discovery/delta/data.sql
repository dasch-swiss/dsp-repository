INSERT INTO resources (id, class) VALUES
    ('alice',        'Character'),
    ('white-rabbit', 'Character');

INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
    ('alice',        'name',  0, 'text', 'Alice',           'en'),
    ('alice',        'name',  1, 'text', 'Alice',           'de'),
    ('alice',        'quote', 0, 'text', 'Curiouser and curiouser!', 'en'),
    ('alice',        'quote', 1, 'text', 'Immer merkwürdiger!',      'de'),
    ('white-rabbit', 'name',  0, 'text', 'The White Rabbit', 'en'),
    ('white-rabbit', 'name',  1, 'text', 'Das Weiße Kaninchen', 'de'),
    ('white-rabbit', 'quote', 0, 'text', 'Oh dear! I shall be late!', 'en'),
    ('white-rabbit', 'quote', 1, 'text', 'Oh Gott! Ich werde zu spät kommen!', 'de');
