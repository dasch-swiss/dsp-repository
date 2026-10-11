-- A border strip (b1, the page's own host class) and a Page (p1) that
-- links to it via `strip-left`. Target class of `strip-left` on Page is
-- exactly BorderStrip, matching the page's own host class.
INSERT INTO resources (id, class) VALUES ('b1', 'BorderStrip');
INSERT INTO resources (id, class) VALUES ('p1', 'Page');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b1', 'name', 0, 'text', 'Randleiste 1');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('p1', 'name', 0, 'text', 'a1r');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('p1', 'strip-left', 0, 'link', 'b1');
