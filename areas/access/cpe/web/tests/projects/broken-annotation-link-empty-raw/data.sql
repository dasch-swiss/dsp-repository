INSERT INTO resources (id, class) VALUES ('p1', 'Page');

INSERT INTO annotations (id, resource_id, shape, x, y, w, h, color)
    VALUES ('a1', 'p1', 'rectangle', 0.1, 0.1, 0.4, 0.2, '#ff3333');

-- 'l1' is well formed and targets a1; 'l2' has no target row.
INSERT INTO annotation_links (id, name) VALUES ('l1', 'identischer Holzschnitt'), ('l2', 'nothing');
INSERT INTO annotation_link_targets (link_id, seq, annotation_id) VALUES ('l1', 0, 'a1');
