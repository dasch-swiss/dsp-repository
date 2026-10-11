-- r1 exists, so the resource-existence half of narrative-for stays silent;
-- only DuplicateNarrativeFor should fire (both entries name the same id).
INSERT INTO resources (id, class) VALUES ('r1', 'Thing');
