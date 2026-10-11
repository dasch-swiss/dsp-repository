-- r1 (Character) carries both link properties, each individually valid
-- (link-typed, present, targeting an image-like class) so only the
-- cross-declaration conflict should fire.
INSERT INTO resources (id, class) VALUES ('r1', 'Character'), ('img1', 'Image');
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
    ('r1', 'portrait', 0, 'link', 'img1'),
    ('r1', 'headshot', 0, 'link', 'img1');
INSERT INTO representations (resource_id, kind, uri, seq)
    VALUES ('img1', 'image', 'https://example.org/full/max/0/default.jpg', 0);
