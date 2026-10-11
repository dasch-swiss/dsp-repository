INSERT INTO resources (id, class) VALUES ('p1', 'Page');

-- x + w = 1.2: the box leaves the image.
INSERT INTO annotations (id, resource_id, shape, x, y, w, h, color)
    VALUES ('a1', 'p1', 'rectangle', 0.8, 0.1, 0.4, 0.2, '#ff3333');
