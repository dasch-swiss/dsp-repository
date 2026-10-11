-- Showcase fixture data (Incunabula Slice 7, woodcut-cases): two Books,
-- four Pages with an `image` link each, three annotations (a woodcut on
-- c1v and c8r, a note on c1v), an annotation link joining the two woodcuts
-- and a page-level link joining h8r and c6v.
INSERT INTO resources (id, class) VALUES
    ('bk-dt', 'Book'),
    ('bk-aug', 'Book'),
    ('pg-c1v', 'Page'),
    ('pg-c8r', 'Page'),
    ('pg-h8r', 'Page'),
    ('pg-c6v', 'Page'),
    ('img-c1v', 'Image'),
    ('img-c8r', 'Image'),
    ('img-h8r', 'Image'),
    ('img-c6v', 'Image');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES
    ('bk-dt', 'name', 0, 'text', '[Das] Narrenschiff (dt.)'),
    ('bk-aug', 'name', 0, 'text', '[Das] Narrenschiff (lat.) [Aug. 1497]'),
    ('pg-c1v', 'name', 0, 'text', 'c1v'),
    ('pg-c1v', 'description', 0, 'text', 'Beginn Kapitel 13.
Holzschnitt zu Kap. 13: Von der Buhlerei.
11.7 x 8.4 cm.'),
    ('pg-c1v', 'comment', 0, 'text', 'Schramm, Bd. 22, Abb. 1124.'),
    ('pg-c8r', 'name', 0, 'text', 'c8r'),
    ('pg-c8r', 'description', 0, 'text', 'Holzschnitt zu Kap. 13: Von der Buhlerei.'),
    ('pg-h8r', 'name', 0, 'text', 'h8r'),
    ('pg-h8r', 'description', 0, 'text', 'Holzschnitt: Anbetung des Kindes.
Masse: 75 x 61 mm.'),
    ('pg-c6v', 'name', 0, 'text', 'c6v');
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
    ('pg-c1v', 'image', 0, 'link', 'img-c1v'),
    ('pg-c8r', 'image', 0, 'link', 'img-c8r'),
    ('pg-h8r', 'image', 0, 'link', 'img-h8r'),
    ('pg-c6v', 'image', 0, 'link', 'img-c6v');
INSERT INTO representations (resource_id, kind, uri, width, height, seq) VALUES
    ('img-c1v', 'image', 'https://iiif.example/c1v.jp2/full/max/0/default.jpg', 3322, 5113, 0),
    ('img-c8r', 'image', 'https://iiif.example/c8r.jp2/full/max/0/default.jpg', 3564, 5304, 0),
    ('img-h8r', 'image', 'https://iiif.example/h8r.jp2/full/max/0/default.jpg', 1360, 2028, 0),
    ('img-c6v', 'image', 'https://iiif.example/c6v.jp2/full/max/0/default.jpg', 1360, 2028, 0);
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES
    ('bk-dt', 'pg-c1v', 1),
    ('bk-dt', 'pg-h8r', 2),
    ('bk-aug', 'pg-c8r', 1),
    ('bk-aug', 'pg-c6v', 2);
INSERT INTO annotations (id, resource_id, shape, x, y, w, h, points, color, remark, remark_lang) VALUES
    ('an-c1v', 'pg-c1v', 'rectangle', 0.18, 0.16, 0.63, 0.55, NULL, '#ff3333', NULL, NULL),
    ('an-c8r', 'pg-c8r', 'rectangle', 0.08, 0.22, 0.54, 0.52, NULL, '#ff3333', NULL, NULL),
    ('an-note', 'pg-c1v', 'rectangle', 0.21, 0.59, 0.34, 0.09, NULL, '#ff3333', 'Handschriftliche Bemerkung in der Zeichnung', 'de');
INSERT INTO annotation_links (id, name, name_lang, comment, comment_lang) VALUES
    ('L-e1', 'identischer Holzschnitt', 'de', 'Der Holzschnitt wird auf Seite c1v und c8r verwendet.', 'de'),
    ('L-page', 'Holzschnitte identisch?', 'de', 'Holzschnitte identisch?', 'de');
INSERT INTO annotation_link_targets (link_id, seq, annotation_id, resource_id) VALUES
    ('L-e1', 0, 'an-c1v', NULL),
    ('L-e1', 1, 'an-c8r', NULL),
    ('L-page', 0, NULL, 'pg-h8r'),
    ('L-page', 1, NULL, 'pg-c6v');
