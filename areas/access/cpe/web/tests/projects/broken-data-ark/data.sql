INSERT INTO resources (id, class, ark) VALUES
    -- well formed, and NULL: no diagnostic
    ('p1', 'Page', 'https://ark.dasch.swiss/ark:/72163/1/0803/twPlI6hTUzCbBJr1IYKQawd'),
    ('i1', 'Image', NULL),
    -- a version ARK
    ('p2', 'Page', 'https://ark.dasch.swiss/ark:/72163/1/0803/wk5fLgvtUXupW8=ZfJRn=Q1.20180604T085622Z'),
    -- http:
    ('b1', 'Book', 'http://ark.dasch.swiss/ark:/72163/1/0803/cpQ3=JfqVZOkd7hUQ26kTg7');

INSERT INTO annotations (id, resource_id, shape, x, y, w, h, ark) VALUES
    -- well formed, and NULL: no diagnostic
    ('a0', 'p1', 'rectangle', 0.1, 0.1, 0.2, 0.2, 'https://ark.dasch.swiss/ark:/72163/1/0803/bsX4cDKkW3GMC8_Ms2tOpw5'),
    ('a5', 'p1', 'rectangle', 0.1, 0.1, 0.2, 0.2, NULL),
    -- a foreign host
    ('a1', 'p1', 'rectangle', 0.1, 0.1, 0.2, 0.2, 'https://ark.example.org/ark:/72163/1/0803/wO=l4gfTVsGnKtusKEomhQn'),
    -- a trailing /
    ('a2', 'p1', 'rectangle', 0.1, 0.1, 0.2, 0.2, 'https://ark.dasch.swiss/ark:/72163/1/0803/EDqDFUAsXtSuBreIXyuCiQE/'),
    -- a lowercase shortcode
    ('a3', 'p1', 'rectangle', 0.1, 0.1, 0.2, 0.2, 'https://ark.dasch.swiss/ark:/72163/1/081c/XTtkp_hEUDWO828r2pb2PAm'),
    -- the empty string, never a stand-in for NULL
    ('a4', 'p1', 'rectangle', 0.1, 0.1, 0.2, 0.2, '');
