-- One Book compound with one Page member; 'note' is a text property on Page.
INSERT INTO resources (id, class) VALUES ('b1', 'Book'), ('p1', 'Page');
INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('b1', 'p1', 0);
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('p1', 'note', 0, 'text', 'Title page.');
