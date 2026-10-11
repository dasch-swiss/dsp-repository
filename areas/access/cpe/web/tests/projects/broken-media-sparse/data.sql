-- 'portrait' is a link property present on Location (with a valid
-- image-like target), but absent from Character entirely.
INSERT INTO resources (id, class) VALUES
    ('r1', 'Character'), ('loc1', 'Location'), ('img1', 'Image');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('loc1', 'portrait', 0, 'link', 'img1');
INSERT INTO representations (resource_id, kind, uri, seq)
    VALUES ('img1', 'image', 'https://example.org/full/max/0/default.jpg', 0);
