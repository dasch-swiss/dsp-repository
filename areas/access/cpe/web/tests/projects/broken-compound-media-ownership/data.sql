-- broken-compound-media-ownership: Book carries a valid `image` link
-- property (host→image, dsp-incubator cpe/docs/adr/0008-images-are-embedded-only-resources.md) so the resource-detail's own media
-- validation passes cleanly, isolating CompoundMediaOwnership as the only
-- diagnostic. No compound_parts rows are needed — the compound-viewer's
-- source is a $id param, which skips the data-dependent EmptyCompoundSource
-- check entirely.
INSERT INTO resources (id, class) VALUES ('b1', 'Book'), ('img1', 'Image');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('b1', 'image', 0, 'link', 'img1');
INSERT INTO representations (resource_id, kind, uri, seq)
    VALUES ('img1', 'image', 'https://example.org/full/max/0/default.jpg', 0);
