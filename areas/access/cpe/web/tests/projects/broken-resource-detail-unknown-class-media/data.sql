-- A Character resource carrying an 'image' link to an image-like class (has
-- 'representations' rows), so the media four-check would otherwise pass cleanly
-- if it ran against the (nonexistent) 'Charakter' class — it must not run at all.
INSERT INTO resources (id, class) VALUES ('c1', 'Character'), ('img1', 'Image');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('c1', 'name', 0, 'text', 'Alice');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('c1', 'image', 0, 'link', 'img1');
INSERT INTO representations (resource_id, kind, uri, seq)
    VALUES ('img1', 'image', 'https://example.org/full/max/0/default.jpg', 0);
