-- broken-member-links-contents-layout: a `$id` source with no compound
-- parts gives the member-links data check no member classes, so only the
-- config-internal PagesLayoutOnly fires — a bare resource suffices.
INSERT INTO resources (id, class) VALUES ('b1', 'Book');
