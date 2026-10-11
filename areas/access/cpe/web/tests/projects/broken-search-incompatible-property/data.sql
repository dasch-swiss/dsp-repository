-- broken-search-incompatible-property data.
-- 'Thing' class has a 'views' property of value_type='integer'. The
-- search-bar declares it as searchable, but FTS5 search requires 'text' or
-- 'richtext' → IncompatibleSearchProperty.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_int)
    VALUES ('r1', 'views', 0, 'integer', 42);
