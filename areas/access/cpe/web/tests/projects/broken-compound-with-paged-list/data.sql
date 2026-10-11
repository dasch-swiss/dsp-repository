-- CompoundViewerWithPagedList is config-internal (checked by
-- `check_and_lower_pages` before the success/raw fork), so this data never
-- needs to be exercised — it exists only so the fixture mirrors the shape
-- of the other Layer-4 fixtures (a minimal Book resource).
INSERT INTO resources (id, class) VALUES ('b1', 'Book');
INSERT INTO properties (resource_id, name, seq, value_type, value_text)
    VALUES ('b1', 'name', 0, 'text', 'Book One');
