-- One resource of class Thing carrying only a 'name' property. The
-- parameterised resource-detail lists 'missing-prop', which no Thing has —
-- a SparseProperty warning that must survive the failed site lowering.
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES ('r1', 'name', 0, 'text', 'First Thing');
