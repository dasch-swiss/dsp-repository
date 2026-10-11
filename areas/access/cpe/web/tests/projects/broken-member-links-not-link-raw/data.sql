-- One Book compound with one Page member (the good-compound-pages shape),
-- plus a `strip-left` link to a BorderStrip and a text `caption`.
INSERT INTO resources (id, class) VALUES ('b1', 'Book'), ('p1', 'Page'), ('img1', 'Image'), ('s1', 'BorderStrip');
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('b1', 'p1', 0);
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('p1', 'image', 0, 'link', 'img1');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('p1', 'strip-left', 0, 'link', 's1');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('p1', 'caption', 0, 'text', 'Titelblatt');
INSERT INTO representations (resource_id, kind, uri, seq)
    VALUES ('img1', 'image', 'https://example.org/full/max/0/default.jpg', 0);
