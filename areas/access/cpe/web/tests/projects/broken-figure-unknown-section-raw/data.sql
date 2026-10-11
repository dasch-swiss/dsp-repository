-- Timeline fixture data (Incunabula Slice 4), copied verbatim from
-- good-timeline/ for the figure-block slot fixtures (the slotted component
-- is a timeline). Two Books carrying a `pub-date` (date), an `office`
-- lane (list_value), an `extent` (integer) and a `name` (text); one Page
-- carrying `scan-date` (a date present in the data but on no Book) and
-- `page-count` (an integer present in the data but on no Book).
INSERT INTO lists (id, name) VALUES ('list-office', 'office');
INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES
    ('office-amerbach', 'list-office', NULL, 0),
    ('office-other', 'list-office', NULL, 1);
INSERT INTO list_node_labels (node_id, lang, label) VALUES
    ('office-amerbach', 'de', 'Amerbach'),
    ('office-amerbach', 'en', 'Amerbach'),
    ('office-other', 'de', 'Andere'),
    ('office-other', 'en', 'Other');

INSERT INTO resources (id, class) VALUES
    ('bereitung', 'Book'),
    ('de-generatione', 'Book'),
    ('p1', 'Page');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES
    ('bereitung', 'name', 0, 'text', 'Bereitung zu dem Heiligen Sakrament'),
    ('de-generatione', 'name', 0, 'text', 'De generatione hominis'),
    ('p1', 'name', 0, 'text', 'a1r');
INSERT INTO properties (resource_id, name, seq, value_type, date_start, date_end, date_precision) VALUES
    ('bereitung', 'pub-date', 0, 'date', '1489', '1489', 'year'),
    ('de-generatione', 'pub-date', 0, 'date', '1487', '1490', 'year'),
    ('p1', 'scan-date', 0, 'date', '2010-05-01', '2010-05-01', 'day');
INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
    ('bereitung', 'office', 0, 'list_value', 'office-amerbach'),
    ('de-generatione', 'office', 0, 'list_value', 'office-other');
INSERT INTO properties (resource_id, name, seq, value_type, value_int) VALUES
    ('bereitung', 'extent', 0, 'integer', 511),
    ('de-generatione', 'extent', 0, 'integer', 120),
    ('p1', 'page-count', 0, 'integer', 2);
