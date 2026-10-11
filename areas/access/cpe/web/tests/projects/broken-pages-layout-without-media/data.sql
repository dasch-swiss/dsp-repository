-- broken-pages-layout-without-media: $id param source skips the
-- EmptyCompoundSource check, so a bare resource is all that's needed.
INSERT INTO resources (id, class) VALUES ('b1', 'Book');
