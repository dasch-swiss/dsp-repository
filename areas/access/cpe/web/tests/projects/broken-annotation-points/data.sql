INSERT INTO resources (id, class) VALUES ('p1', 'Page');

-- A polygon needs at least three vertices.
INSERT INTO annotations (id, resource_id, shape, x, y, w, h, points, color)
    VALUES ('a1', 'p1', 'polygon', 0.1, 0.1, 0.4, 0.4, '0.1,0.1 0.5,0.5', '#ff3333');
