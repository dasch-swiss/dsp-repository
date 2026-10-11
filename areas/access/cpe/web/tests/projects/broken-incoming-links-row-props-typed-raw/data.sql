-- One Person (pe1) and two Works (w1, w2) linking to it through `composer`.
-- `category` (text) is on both; `title` (text) only on w1; `count` is an
-- integer; `nope` exists nowhere.
INSERT INTO resources (id, class) VALUES ('pe1', 'Person');
INSERT INTO resources (id, class) VALUES ('w1', 'Work');
INSERT INTO resources (id, class) VALUES ('w2', 'Work');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('pe1', 'name', 0, 'text', 'Rossini');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('w1', 'name', 0, 'text', 'Il barbiere');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('w2', 'name', 0, 'text', 'Tancredi');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('w1', 'composer', 0, 'link', 'pe1');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('w2', 'composer', 0, 'link', 'pe1');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('w1', 'category', 0, 'text', 'Oper');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('w2', 'category', 0, 'text', 'Oper');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('w1', 'title', 0, 'text', 'Der Barbier');
INSERT INTO properties (resource_id, name, seq, value_type, value_int)
    VALUES ('w1', 'count', 0, 'integer', 3);
INSERT INTO properties (resource_id, name, seq, value_type, value_int)
    VALUES ('w2', 'count', 0, 'integer', 4);
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('w1', 'note', 0, 'text', 'Anmerkung');
