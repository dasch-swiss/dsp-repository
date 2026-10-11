-- One Book compound with one Page member linking to one Image with a
-- representations row (dsp-incubator cpe/docs/adr/0008-images-are-embedded-only-resources.md: host→image link, never a representation on
-- the Page itself) — mirrors good-compound-media's shape.
INSERT INTO resources (id, class) VALUES ('b1', 'Book'), ('p1', 'Page'), ('img1', 'Image');
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('b1', 'p1', 0);
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('p1', 'image', 0, 'link', 'img1');
INSERT INTO representations (resource_id, kind, uri, seq)
    VALUES ('img1', 'image', 'https://example.org/full/max/0/default.jpg', 0);
