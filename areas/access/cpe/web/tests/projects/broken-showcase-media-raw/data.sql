-- Showcase cards fixture data (Incunabula Slice 7, edition-cards and
-- strip-row): two Books with a cover image each, three BorderStrips with an
-- integer `page-uses` and a tile image.
INSERT INTO resources (id, class) VALUES
    ('bk-dt', 'Book'),
    ('bk-aug', 'Book'),
    ('st-l1', 'BorderStrip'),
    ('st-r1', 'BorderStrip'),
    ('st-r2', 'BorderStrip'),
    ('img-dt', 'Image'),
    ('img-aug', 'Image'),
    ('img-l1', 'Image');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES
    ('bk-dt', 'name', 0, 'text', '[Das] Narrenschiff (dt.)'),
    ('bk-dt', 'pub-place', 0, 'text', 'Basel'),
    ('bk-aug', 'name', 0, 'text', '[Das] Narrenschiff (lat.) [Aug. 1497]'),
    ('bk-aug', 'pub-place', 0, 'text', 'Basel'),
    ('st-l1', 'name', 0, 'text', 'l_1'),
    ('st-r1', 'name', 0, 'text', 'r_1'),
    ('st-r2', 'name', 0, 'text', 'r_2');
INSERT INTO properties (resource_id, name, seq, value_type, value_int) VALUES
    ('bk-dt', 'extent', 0, 'integer', 328),
    ('bk-aug', 'extent', 0, 'integer', 324),
    ('st-l1', 'page-uses', 0, 'integer', 15),
    ('st-r1', 'page-uses', 0, 'integer', 9),
    ('st-r2', 'page-uses', 0, 'integer', 1);
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
    ('bk-dt', 'image', 0, 'link', 'img-dt'),
    ('bk-aug', 'image', 0, 'link', 'img-aug'),
    ('st-l1', 'image', 0, 'link', 'img-l1');
INSERT INTO representations (resource_id, kind, uri, width, height, seq) VALUES
    ('img-dt', 'image', 'https://iiif.example/dt.jp2/full/max/0/default.jpg', 3227, 5105, 0),
    ('img-aug', 'image', 'https://iiif.example/aug.jp2/full/max/0/default.jpg', 3702, 5195, 0),
    ('img-l1', 'image', 'https://iiif.example/l1.jp2/full/max/0/default.jpg', 300, 2000, 0);
