-- good-media: one Character with a 'portrait' link to one Image resource
-- that carries a representations row.
INSERT INTO resources (id, class) VALUES ('r1', 'Character'), ('img1', 'Image');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('r1', 'portrait', 0, 'link', 'img1');
INSERT INTO representations (resource_id, kind, uri, seq)
    VALUES ('img1', 'image', 'https://example.org/full/max/0/default.jpg', 0);
