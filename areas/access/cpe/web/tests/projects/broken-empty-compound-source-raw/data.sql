-- broken-empty-compound-source-raw: EMPTY_ST exists as a resource but has
-- no compound_parts rows (no seq/member_id entries). This triggers the
-- EmptyCompoundSource warning via the Err/raw branch of validate_with_data
-- (the config-internal MissingSiteTitle error forces that path).
INSERT INTO resources (id, class) VALUES ('EMPTY_ST', 'Story');
