//! In-memory project database construction: open a connection, enable
//! foreign-key enforcement, apply the store schema, then load a
//! project's `data.sql`. The single entry point is [`open_project_db`].

use rusqlite::Connection;

/// The store schema DDL, embedded at compile time. `db.rs` lives at
/// `store/src/`, so `../schema.sql` resolves to `store/schema.sql`.
const SCHEMA_SQL: &str = include_str!("../schema.sql");

/// What can go wrong while building a project's in-memory database.
/// Each variant pins the boot step that failed and carries the
/// underlying SQLite error as its source.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("failed to open in-memory database")]
    Open(#[source] rusqlite::Error),
    #[error("failed to enable foreign-key enforcement")]
    Pragma(#[source] rusqlite::Error),
    #[error("failed to apply the store's schema")]
    Schema(#[source] rusqlite::Error),
    #[error("failed to load project data")]
    Data(#[source] rusqlite::Error),
}

/// Open a fresh in-memory database, enable foreign keys, apply the
/// store schema, then execute the project's `data.sql`, returning the
/// populated connection.
///
/// Foreign keys are enabled *before* any DDL runs: the `foreign_keys`
/// pragma is per-connection and a no-op once a transaction is open, so
/// it must be set on the bare connection first (see `dsp-incubator cpe/docs/SCHEMA.md`).
///
/// `data_sql` is trusted project data (build-embedded at boot), never
/// request input — it is executed verbatim via `execute_batch`.
pub fn open_project_db(data_sql: &str) -> Result<Connection, DbError> {
    let mut conn = Connection::open_in_memory().map_err(DbError::Open)?;
    conn.pragma_update(None, "foreign_keys", true).map_err(DbError::Pragma)?;
    conn.execute_batch(SCHEMA_SQL).map_err(DbError::Schema)?;
    {
        let tx = conn.transaction().map_err(DbError::Data)?;
        tx.execute_batch(data_sql).map_err(DbError::Data)?;
        tx.commit().map_err(DbError::Data)?;
    }
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixture exercising the seven non-annotation tables, inserted in foreign-key
    /// order (parents before children).
    const FIXTURE: &str = "
        INSERT INTO resources (id, class) VALUES
            ('r1', 'Book'), ('r2', 'Page'), ('r3', 'Page');

        INSERT INTO lists (id, name) VALUES ('l1', 'genre');
        INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES
            ('n1', 'l1', NULL, 0);
        INSERT INTO list_node_labels (node_id, lang, label) VALUES
            ('n1', 'en', 'Fiction'), ('n1', 'de', 'Belletristik');

        INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_int) VALUES
            ('r1', 'title', 0, 'text', 'The Book', NULL),
            ('r1', 'pageCount', 0, 'integer', NULL, 42),
            ('r2', 'title', 0, 'text', 'Page One', NULL);

        INSERT INTO representations (resource_id, kind, uri, seq) VALUES
            ('r1', 'image', 'https://iiif.example/r1', 0);

        INSERT INTO compound_parts (parent_id, child_id, seq) VALUES
            ('r1', 'r2', 0), ('r1', 'r3', 1);
    ";

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
            .expect("count query should succeed")
    }

    #[test]
    fn schema_applies_cleanly() {
        let conn = open_project_db("").expect("empty data should load");
        let tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN \
                 ('resources','properties','lists','list_nodes','list_node_labels',\
                 'representations','compound_parts','annotations','annotation_links',\
                 'annotation_link_targets')",
                [],
                |row| row.get(0),
            )
            .expect("sqlite_master query should succeed");
        assert_eq!(tables, 10, "all ten schema tables should exist");

        // Guards against a dropped CREATE INDEX in schema.sql.
        let indexes: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name LIKE 'idx_%'",
                [],
                |row| row.get(0),
            )
            .expect("sqlite_master query should succeed");
        assert_eq!(indexes, 14, "all fourteen named indexes should exist");
    }

    #[test]
    fn annotation_link_target_names_exactly_one_target() {
        let base = "
            INSERT INTO resources (id, class) VALUES ('p1', 'Page');
            INSERT INTO annotations (id, resource_id, shape, x, y, w, h)
                VALUES ('a1', 'p1', 'rectangle', 0.1, 0.1, 0.2, 0.2);
            INSERT INTO annotation_links (id, name) VALUES ('l1', 'same block');
        ";
        let one = format!(
            "{base} INSERT INTO annotation_link_targets (link_id, seq, annotation_id) \
             VALUES ('l1', 0, 'a1');"
        );
        assert!(open_project_db(&one).is_ok(), "one target must load");
        let neither = format!("{base} INSERT INTO annotation_link_targets (link_id, seq) VALUES ('l1', 0);");
        assert!(open_project_db(&neither).is_err(), "no target must fail the CHECK");
        let both = format!(
            "{base} INSERT INTO annotation_link_targets (link_id, seq, annotation_id, resource_id) \
             VALUES ('l1', 0, 'a1', 'p1');"
        );
        assert!(open_project_db(&both).is_err(), "two targets must fail the CHECK");
    }

    #[test]
    fn data_loads_and_reads_back() {
        let conn = open_project_db(FIXTURE).expect("fixture should load");
        assert_eq!(count(&conn, "resources"), 3);
        assert_eq!(count(&conn, "properties"), 3);
        assert_eq!(count(&conn, "lists"), 1);
        assert_eq!(count(&conn, "list_nodes"), 1);
        assert_eq!(count(&conn, "list_node_labels"), 2);
        assert_eq!(count(&conn, "representations"), 1);
        assert_eq!(count(&conn, "compound_parts"), 2);
    }

    #[test]
    fn foreign_keys_enforced() {
        // A property referencing a resource that doesn't exist must be
        // rejected — proves the pragma took effect (one representative
        // FK; the pragma is binary, so this covers all of them).
        let orphan = "INSERT INTO properties (resource_id, name, seq, value_type, value_text) \
                      VALUES ('ghost', 'title', 0, 'text', 'x');";
        let err = open_project_db(orphan).expect_err("orphan FK should be rejected");
        assert!(matches!(err, DbError::Data(_)));
    }

    #[test]
    fn check_constraints_fire() {
        let bad_value_type = "INSERT INTO resources (id, class) VALUES ('r1', 'Book'); \
            INSERT INTO properties (resource_id, name, seq, value_type, value_text) \
            VALUES ('r1', 'title', 0, 'bogus', 'x');";
        assert!(matches!(
            open_project_db(bad_value_type).expect_err("bad value_type should be rejected"),
            DbError::Data(_)
        ));

        let bad_precision = "INSERT INTO resources (id, class) VALUES ('r1', 'Book'); \
            INSERT INTO properties (resource_id, name, seq, value_type, date_start, date_end, date_precision) \
            VALUES ('r1', 'published', 0, 'date', '1480', '1480', 'century');";
        assert!(matches!(
            open_project_db(bad_precision).expect_err("bad date_precision should be rejected"),
            DbError::Data(_)
        ));

        let bad_calendar = "INSERT INTO resources (id, class) VALUES ('r1', 'Book'); \
            INSERT INTO properties (resource_id, name, seq, value_type, date_start, date_calendar) \
            VALUES ('r1', 'published', 0, 'date', '1480', 'hebrew');";
        assert!(matches!(
            open_project_db(bad_calendar).expect_err("bad date_calendar should be rejected"),
            DbError::Data(_)
        ));

        let good_calendar = "INSERT INTO resources (id, class) VALUES ('r1', 'Book'); \
            INSERT INTO properties (resource_id, name, seq, value_type, date_start, date_display, date_calendar) \
            VALUES ('r1', 'published', 0, 'date', '1480', '1480', 'gregorian');";
        open_project_db(good_calendar).expect("a known date_calendar must load");
    }
}
