-- A border strip (b1) and a Page (p1) that links to it through
-- `strip-left`. DuplicateIncomingLinks is config-internal (checked by
-- `lower_resource_detail` before data validation), so this data never
-- needs to be exercised — it exists only so the fixture mirrors the shape
-- of the other broken-incoming-links-* fixtures.
INSERT INTO resources (id, class) VALUES ('b1', 'BorderStrip');
INSERT INTO resources (id, class) VALUES ('p1', 'Page');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b1', 'name', 0, 'text', 'Randleiste 1');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('p1', 'strip-left', 0, 'link', 'b1');
