-- broken-two-compound-viewers: both compound-viewers use a $id param
-- source, so no data-dependent check fires — a bare resource suffices.
INSERT INTO resources (id, class) VALUES ('b1', 'Book');
