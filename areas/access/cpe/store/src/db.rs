//! A project's in-memory database and the type that owns it: [`ProjectStore::open`] opens a
//! connection, enables foreign-key enforcement, applies the store schema and loads the project's
//! store text.

use std::sync::{Mutex, MutexGuard};

use cpe_core::ir::{DataArks, ValidatedConfig};
use rusqlite::Connection;

use crate::error::SqliteError;
use crate::query::reads::ProjectReads;
use crate::query::{self, LiveProjectFacts, QueryError};

/// The store schema DDL, embedded at compile time. `db.rs` lives at
/// `store/src/`, so `../schema.sql` resolves to `store/schema.sql`.
const SCHEMA_SQL: &str = include_str!("../schema.sql");

/// What can go wrong while building a project's in-memory database.
/// Each variant pins the step that failed and carries the
/// underlying SQLite error, wrapped, as its source.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("failed to open in-memory database")]
    Open(#[source] SqliteError),
    #[error("failed to enable foreign-key enforcement")]
    Pragma(#[source] SqliteError),
    #[error("failed to apply the store's schema")]
    Schema(#[source] SqliteError),
    #[error("failed to load project data")]
    Data(#[source] SqliteError),
}

/// One project's store: the in-memory database and the only handle on its connection.
///
/// The connection sits in a `Mutex` so that the store is `Sync` (a `rusqlite::Connection` is `Send`
/// but not `Sync`) and can be shared across request handlers. The field is private and
/// `ProjectStore::lock` is crate-private, so no guard leaves this crate. The lock rule is in
/// `query::reads`' module doc.
#[derive(Debug)]
pub struct ProjectStore {
    conn: Mutex<Connection>,
}

impl ProjectStore {
    /// Open a project's store from its SQL text: a fresh in-memory database with foreign keys
    /// enabled, the store schema applied and `data_sql` executed.
    ///
    /// `data_sql` is trusted project data (the store text built at activation, or a project's
    /// committed `data.sql`), never request input: it is executed verbatim via `execute_batch`.
    pub fn open(data_sql: &str) -> Result<Self, DbError> {
        Ok(Self { conn: Mutex::new(open_project_db(data_sql)?) })
    }

    /// The validator's facts over this store. Each fact locks for its own query.
    pub fn facts(&self) -> LiveProjectFacts<'_> {
        LiveProjectFacts::new(self)
    }

    /// Build the FTS5 index of every `search-bar` in `config`. A load step: it runs once, after
    /// validation and before the first read.
    pub fn build_search_indexes(&self, config: &ValidatedConfig) -> Result<(), QueryError> {
        query::build_search_indexes(&self.lock(), config)
    }

    /// Every non-NULL `ark` of `resources` and of `annotations`, keyed by id. The values are
    /// whatever is stored; their shape is the `MalformedDataArk` boot check's.
    pub fn data_arks(&self) -> Result<DataArks, QueryError> {
        query::data_arks(&self.lock())
    }

    /// The composite reads over this store.
    pub fn reads(&self) -> ProjectReads<'_> {
        ProjectReads::new(self)
    }

    /// Lock the connection. Keep this crate-private and return no guard from a `pub` function: a
    /// caller that needs another query gets a method on `ProjectReads`. Nothing that holds the
    /// guard may call back into this store: the `Mutex` is not reentrant.
    pub(crate) fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .expect("conn mutex is never poisoned — no panic path holds the guard")
    }
}

/// The connection behind [`ProjectStore::open`].
///
/// Foreign keys are enabled *before* any DDL runs: the `foreign_keys`
/// pragma is per-connection and a no-op once a transaction is open, so
/// it must be set on the bare connection first (see `dsp-incubator cpe/docs/SCHEMA.md`).
fn open_project_db(data_sql: &str) -> Result<Connection, DbError> {
    let mut conn = Connection::open_in_memory().map_err(SqliteError::new).map_err(DbError::Open)?;
    conn.pragma_update(None, "foreign_keys", true)
        .map_err(SqliteError::new)
        .map_err(DbError::Pragma)?;
    conn.execute_batch(SCHEMA_SQL)
        .map_err(SqliteError::new)
        .map_err(DbError::Schema)?;
    {
        let tx = conn.transaction().map_err(SqliteError::new).map_err(DbError::Data)?;
        tx.execute_batch(data_sql).map_err(SqliteError::new).map_err(DbError::Data)?;
        tx.commit().map_err(SqliteError::new).map_err(DbError::Data)?;
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

    // ── ProjectStore ──────────────────────────────────────────────────────────

    use std::error::Error;

    use cpe_core::ir::{Pagination, WhereClause};
    use cpe_core::lang::Lang;
    use cpe_core::validate::context::ValidationContext;
    use cpe_core::validate::facts::ProjectFacts;

    /// A list of `Thing`s with a search bar over `name`.
    const SEARCH_CONFIG: &str = r#"
site {
    title "T"
    languages "de" "en"
}
page id="things" route="/things" {
    resource-list id="things" class="Thing"
    search-bar target="things" searches="name"
}
"#;

    const SEARCH_DATA: &str = "
        INSERT INTO resources (id, class) VALUES ('t1', 'Thing'), ('t2', 'Thing');
        INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES
            ('t1', 'name', 0, 'text', 'Rabbit'),
            ('t2', 'name', 0, 'text', 'Quantum Cat');
    ";

    fn english() -> Lang {
        cpe_core::lang::resolve_builtin(&["en".to_owned()])
    }

    /// Whether `error` or anything in its source chain is a type of the driver.
    fn names_the_driver(error: &(dyn Error + 'static)) -> bool {
        let mut current = Some(error);
        while let Some(error) = current {
            if error.is::<rusqlite::Error>() || error.is::<rusqlite::ffi::Error>() {
                return true;
            }
            current = error.source();
        }
        false
    }

    #[test]
    fn a_store_opens_from_sql_text_and_reads_a_resource() {
        let store = ProjectStore::open(FIXTURE).expect("the fixture loads");
        let book = store
            .reads()
            .get_resource("r1", &english())
            .expect("the read succeeds")
            .expect("r1 is stored");
        assert_eq!(book.class, "Book");
        assert!(store
            .reads()
            .get_resource("ghost", &english())
            .expect("the read succeeds")
            .is_none());
    }

    #[test]
    fn a_store_that_cannot_load_its_text_names_the_step_and_not_the_driver() {
        let orphan = "INSERT INTO properties (resource_id, name, seq, value_type, value_text) \
                      VALUES ('ghost', 'title', 0, 'text', 'x');";
        let error = ProjectStore::open(orphan).expect_err("an orphan row is refused");
        assert!(matches!(error, DbError::Data(_)));
        assert_eq!(error.to_string(), "failed to load project data");
        let source = error.source().expect("the step carries its cause");
        assert!(source.to_string().contains("FOREIGN KEY"), "{source}");
        assert!(source.source().is_none(), "the cause ends the chain");
        assert!(!names_the_driver(&error));
    }

    #[test]
    fn a_failed_query_does_not_hand_out_the_driver_error() {
        let store = ProjectStore::open("").expect("an empty store opens");
        store
            .lock()
            .execute_batch("PRAGMA foreign_keys = OFF; DROP TABLE resources;")
            .expect("the table drops");
        for error in [
            store.data_arks().expect_err("the ARKs cannot be read"),
            store
                .reads()
                .count_resources("Thing", &[])
                .expect_err("the count cannot be read"),
        ] {
            assert!(matches!(error, QueryError::Query(_)));
            assert!(error.to_string().contains("no such table: resources"), "{error}");
            assert!(!names_the_driver(&error));
        }
    }

    #[test]
    fn a_stores_facts_answer_from_its_data() {
        let store = ProjectStore::open(FIXTURE).expect("the fixture loads");
        let facts = store.facts();
        assert!(facts.class_exists("Book").expect("the fact is read"));
        assert!(!facts.class_exists("Map").expect("the fact is read"));
        assert_eq!(facts.class_of("r2").expect("the fact is read").as_deref(), Some("Page"));
        // A fact that kept its guard would block this call.
        assert!(store.data_arks().is_ok());
    }

    #[test]
    fn a_store_reads_its_data_arks() {
        let store = ProjectStore::open(
            "INSERT INTO resources (id, class, ark) VALUES
                ('b1', 'Book', 'https://ark.dasch.swiss/ark:/72163/1/0803/cpQ3=JfqVZOkd7hUQ26kTg7'),
                ('p1', 'Page', NULL);",
        )
        .expect("the data loads");
        let arks = store.data_arks().expect("the ARKs are read");
        assert_eq!(
            arks.resource("b1"),
            Some("https://ark.dasch.swiss/ark:/72163/1/0803/cpQ3=JfqVZOkd7hUQ26kTg7")
        );
        assert_eq!(arks.resource("p1"), None);
    }

    #[test]
    fn a_store_builds_the_search_indexes_its_reads_query() {
        let store = ProjectStore::open(SEARCH_DATA).expect("the data loads");
        let ctx = ValidationContext::for_tests("things");
        let (config, _) = cpe_core::validate::load_config(SEARCH_CONFIG, &ctx, &store.facts(), &[])
            .expect("the config validates against the store's facts");
        let search = WhereClause::Search {
            table: query::search_index_name("Thing", &["name".to_owned()]),
            query: "rab".to_owned(),
        };
        let list = |store: &ProjectStore| {
            store.reads().list_resources(
                "Thing",
                std::slice::from_ref(&search),
                &[],
                Pagination::PerPage(20),
                1,
                &english(),
            )
        };
        assert!(list(&store).is_err(), "no index before it is built");

        store.build_search_indexes(&config).expect("the index builds");

        let found = list(&store).expect("the search runs");
        let ids: Vec<&str> = found.items.iter().map(|resource| resource.id.as_str()).collect();
        assert_eq!(ids, vec!["t1"]);
    }
}
