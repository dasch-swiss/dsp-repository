-- A border strip (b1, the page's own host class), a Woodcut (w1), and a
-- Page (p1) that links to the Woodcut (not the BorderStrip) via
-- `strip-left`. `strip-left` is fully link-typed on Page, so every earlier
-- tier passes; its only stored target class is Woodcut, never BorderStrip.
INSERT INTO resources (id, class) VALUES ('b1', 'BorderStrip');
INSERT INTO resources (id, class) VALUES ('w1', 'Woodcut');
INSERT INTO resources (id, class) VALUES ('p1', 'Page');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b1', 'name', 0, 'text', 'Randleiste 1');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('w1', 'name', 0, 'text', 'Woodcut 1');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('p1', 'name', 0, 'text', 'a1r');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('p1', 'strip-left', 0, 'link', 'w1');
