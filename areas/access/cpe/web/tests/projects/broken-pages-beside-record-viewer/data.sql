-- broken-pages-beside-record-viewer: "other_book" carries a valid `image`
-- link property so resource-detail's own media validation passes cleanly.
-- "b1" has no compound_parts rows, so the compound-viewer's own media check
-- is skipped entirely (empty member-class set, per good-compound-media-
-- no-members) — isolating MultipleImageViewers as the only diagnostic.
INSERT INTO resources (id, class) VALUES ('b1', 'Book'), ('other_book', 'Book'), ('img1', 'Image');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('other_book', 'image', 0, 'link', 'img1');
INSERT INTO representations (resource_id, kind, uri, seq)
    VALUES ('img1', 'image', 'https://example.org/full/max/0/default.jpg', 0);
