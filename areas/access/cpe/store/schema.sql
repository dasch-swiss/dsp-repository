-- CPE engine schema. The single source of truth for the data shape
-- every project's data.sql populates. Transcribed from dsp-incubator cpe/docs/SCHEMA.md;
-- keep the two in sync (SCHEMA.md documents the value-type semantics
-- and the constraints validated at boot rather than in DDL).
--
-- PRAGMA foreign_keys = ON is applied in code (db.rs) before this file
-- runs, not here: the pragma is per-connection and a no-op inside a
-- transaction.

CREATE TABLE resources (
    id     TEXT PRIMARY KEY,
    class  TEXT NOT NULL,
    -- the resource's plain DSP data ARK URL, or NULL; never ''. Its shape is
    -- the boot check MalformedDataArk; see dsp-incubator cpe/docs/SCHEMA.md.
    ark    TEXT
);
CREATE INDEX idx_resources_class ON resources(class);

CREATE TABLE properties (
    resource_id     TEXT NOT NULL REFERENCES resources(id),
    name            TEXT NOT NULL,
    seq             INTEGER NOT NULL DEFAULT 0,
    value_type      TEXT NOT NULL CHECK (value_type IN (
                        'text','richtext','integer','decimal','boolean',
                        'date','link','list_value'
                    )),
    value_text      TEXT,
    value_lang      TEXT,
    value_int       INTEGER,
    value_real      REAL,
    date_start      TEXT,
    date_end        TEXT,
    date_precision  TEXT CHECK (date_precision IN ('year','month','day')),
    -- date only: curated display string, written in value_lang; range-ness is derived from date_end, not a precision.
    date_display    TEXT,
    -- date only: the calendar the ISO bounds were recorded in (stored, never computed with).
    date_calendar   TEXT CHECK (date_calendar IN ('julian','gregorian')),
    target_id       TEXT,
    PRIMARY KEY (resource_id, name, seq)
);
CREATE INDEX idx_properties_name      ON properties(name);
CREATE INDEX idx_properties_name_text ON properties(name, value_text);
CREATE INDEX idx_properties_name_int  ON properties(name, value_int);
CREATE INDEX idx_properties_name_date ON properties(name, date_start, date_end);
-- (target_id, resource_id), not (target_id) alone. The trailing
-- resource_id is what makes facet narrowing cheap: every `where`/facet
-- filter lowers to `EXISTS (SELECT 1 FROM properties pW WHERE
-- pW.resource_id = r.id AND pW.name = ? AND pW.target_id IN (…))`
-- (`query.rs`, `build_where`'s `TargetIn` arm), a correlated subquery
-- re-run once per candidate row. With a single-column index SQLite
-- seeks target_id and then SCANS the whole bucket for that target
-- looking for resource_id — O(candidate rows x rows sharing the target),
-- which on HdM's 8,942 Performances measured 15.6 s for `?source=ib`.
-- With resource_id in the index it is one equality seek per row (69 ms
-- for that same request; 61-75 ms across all six measured routes).
-- A plan regression is caught by `target_in_where_uses_composite_target_index`
-- in `query.rs`. See dsp-incubator cpe/docs/SCHEMA.md.
CREATE INDEX idx_properties_target    ON properties(target_id, resource_id);

CREATE TABLE lists (
    id    TEXT PRIMARY KEY,
    name  TEXT NOT NULL
);

CREATE TABLE list_nodes (
    id         TEXT PRIMARY KEY,
    list_id    TEXT NOT NULL REFERENCES lists(id),
    parent_id  TEXT REFERENCES list_nodes(id),
    seq        INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_list_nodes_list   ON list_nodes(list_id);
CREATE INDEX idx_list_nodes_parent ON list_nodes(parent_id, seq);

CREATE TABLE list_node_labels (
    node_id  TEXT NOT NULL REFERENCES list_nodes(id),
    lang     TEXT NOT NULL,
    label    TEXT NOT NULL,
    PRIMARY KEY (node_id, lang)
);

CREATE TABLE representations (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    resource_id  TEXT NOT NULL REFERENCES resources(id),
    kind         TEXT NOT NULL CHECK (kind IN ('image','audio','video','document')),
    uri          TEXT NOT NULL,
    width        INTEGER,
    height       INTEGER,
    alt          TEXT,
    attribution  TEXT,
    seq          INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_representations_resource ON representations(resource_id, seq);

CREATE TABLE compound_parts (
    parent_id  TEXT NOT NULL REFERENCES resources(id),
    child_id   TEXT NOT NULL REFERENCES resources(id),
    seq        INTEGER NOT NULL,
    PRIMARY KEY (parent_id, child_id)
);
CREATE INDEX idx_compound_parts_parent ON compound_parts(parent_id, seq);
CREATE INDEX idx_compound_parts_child  ON compound_parts(child_id);

-- Annotations (dsp-incubator cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md): one shape on one resource's displayed image.
-- Box, points, colour and link-target rules are boot checks
-- (MalformedAnnotation), not DDL; see dsp-incubator cpe/docs/SCHEMA.md.
CREATE TABLE annotations (
    id          TEXT PRIMARY KEY,
    -- the annotated resource, not its Image: the geometry is relative to
    -- that resource's displayed image
    resource_id TEXT NOT NULL REFERENCES resources(id),
    shape       TEXT NOT NULL CHECK (shape IN ('rectangle','polygon','circle')),
    -- bounding box in 0..1 of the image; a circle is the ellipse inscribed in it
    x           REAL NOT NULL,
    y           REAL NOT NULL,
    w           REAL NOT NULL,
    h           REAL NOT NULL,
    -- polygon only: SVG `points` text "x,y x,y …" in 0..1; NULL otherwise
    points      TEXT,
    -- '#rrggbb', lowercase
    color       TEXT,
    remark      TEXT,
    remark_lang TEXT,
    -- the annotation's own plain DSP data ARK URL (a DSP Region's), or NULL;
    -- never ''. Boot check MalformedDataArk, as for resources.ark.
    ark         TEXT
);
CREATE INDEX idx_annotations_resource ON annotations(resource_id);

-- A named link joining annotations or resources (at least one target).
CREATE TABLE annotation_links (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    name_lang    TEXT,
    comment      TEXT,
    comment_lang TEXT
);

-- One row per link target: exactly one of an annotation or a resource.
CREATE TABLE annotation_link_targets (
    link_id       TEXT NOT NULL REFERENCES annotation_links(id),
    seq           INTEGER NOT NULL,
    annotation_id TEXT REFERENCES annotations(id),
    resource_id   TEXT REFERENCES resources(id),
    PRIMARY KEY (link_id, seq),
    CHECK ((annotation_id IS NULL) <> (resource_id IS NULL))
);
CREATE INDEX idx_alt_annotation ON annotation_link_targets(annotation_id);
CREATE INDEX idx_alt_resource   ON annotation_link_targets(resource_id);
