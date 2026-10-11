-- Minimal Thing data. GridWithoutMedia fires purely from config (grid=#true
-- with no `media` attribute), before any data lookup, so no particular
-- property data is needed to trigger it.
INSERT INTO resources (id, class) VALUES ('t1', 'Thing');
