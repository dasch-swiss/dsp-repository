-- B1 (Book) compounds one Page member (P1), which carries BOTH an 'image'
-- and a 'scan' link property to distinct image-like Image resources. Each
-- link is individually valid (link-typed, present, image-like target) so
-- only the cross-declaration conflict should fire.
INSERT INTO resources (id, class) VALUES ('B1', 'Book'), ('P1', 'Page'), ('img1', 'Image'), ('img2', 'Image');
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('B1', 'P1', 0);
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
    ('P1', 'image', 0, 'link', 'img1'),
    ('P1', 'scan', 0, 'link', 'img2');
INSERT INTO representations (resource_id, kind, uri, seq) VALUES
    ('img1', 'image', 'https://example.org/full/max/0/default.jpg', 0),
    ('img2', 'image', 'https://example.org/full/max/1/default.jpg', 0);
