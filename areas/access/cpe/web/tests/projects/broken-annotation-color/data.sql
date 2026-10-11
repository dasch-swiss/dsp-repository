INSERT INTO resources (id, class) VALUES ('p1', 'Page');

-- '#FF3333' is not lowercase #rrggbb.
INSERT INTO annotations (id, resource_id, shape, x, y, w, h, color)
    VALUES ('a1', 'p1', 'rectangle', 0.1, 0.1, 0.4, 0.2, '#FF3333');
