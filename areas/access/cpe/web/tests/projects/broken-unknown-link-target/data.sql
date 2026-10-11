-- broken-unknown-link-target: one resource with a link property pointing at
-- an absent target id. This triggers the UnknownLinkTarget warning.
INSERT INTO resources (id, class) VALUES ('src_resource', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, target_id)
    VALUES ('src_resource', 'related', 0, 'link', 'nonexistent_target');
