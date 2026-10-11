//! The crate's one integration test binary.
//!
//! The project's mapping, the vendored snapshot, the store built from it and that store's file
//! are statics: one load per test run, and a failed load poisons the static instead of being
//! retried by every test that reads it. A test that touches none of them never pays for the
//! load.

use std::sync::LazyLock;

use cpe_core::remodel::Remodel;
use cpe_ports::{ArchiveProjection, ProjectSnapshot};
use cpe_store_build::error::BuildErrors;
use cpe_store_build::hooks::resolve;
use cpe_store_build::store::{Store, Table};
use cpe_store_build::{sql, SCHEMA_SQL};
use rusqlite::Connection;
use sync_store::LiveArchiveProjection;

mod annotations;
mod books;
mod committed_0803;
mod hook_rules;
mod hookless;
mod hooks;
mod hygiene;
mod pages;
mod rebuild;
mod refusals;
mod strips;
mod support;
mod worked_example;

/// The folder of the project these tests build, which is its name in the table of hooks.
const PROJECT: &str = "incunabula";

/// The project's mapping, lowered once from its committed config.
static MAPPING: LazyLock<Remodel> = LazyLock::new(|| {
    let config = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../projects/incunabula/config.kdl"));
    support::lowered(config)
});

/// The shortcode the project's mapping declares.
fn shortcode() -> &'static str {
    MAPPING.shortcode.as_str()
}

/// The project's store of `snapshot`: built under its mapping, with its hook. The test modules
/// that build the project's own snapshots call this; those over a fixture project call the
/// library's `build` with the fixture's mapping.
fn build(snapshot: &ProjectSnapshot) -> Result<Store, BuildErrors> {
    cpe_store_build::build(snapshot, &MAPPING, resolve(PROJECT))
}

/// The vendored data directory, which the binary reads by default.
const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../sync/data");

static SNAPSHOT: LazyLock<ProjectSnapshot> = LazyLock::new(|| {
    LiveArchiveProjection::new(DATA_DIR)
        .snapshot(shortcode())
        .expect("the vendored 0803 data is served")
});

static STORE: LazyLock<Store> = LazyLock::new(|| build(&SNAPSHOT).expect("the vendored 0803 snapshot builds"));

/// The header the binary writes the project's store file under.
static HEADER: LazyLock<String> = LazyLock::new(|| sql::header(PROJECT, &MAPPING));

/// The whole store file, as the binary writes it.
static SQL: LazyLock<String> =
    LazyLock::new(|| sql::to_sql(&STORE, &MAPPING, &HEADER).expect("every table of the built store has a row"));

fn count(connection: &Connection, table: Table) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {}", table.name()), [], |row| row.get(0))
        .expect("the table is in the schema")
}

#[test]
fn test_list_tables_load_under_the_schema_with_foreign_keys_on() {
    let tables = [Table::Lists, Table::ListNodes, Table::ListNodeLabels];
    let sql = sql::to_sql_tables(&STORE, &MAPPING, &HEADER, &tables).expect("the list tables have rows");
    let mut connection = Connection::open_in_memory().expect("an in-memory database opens");
    connection.pragma_update(None, "foreign_keys", true).expect("the pragma is set");
    connection.execute_batch(SCHEMA_SQL).expect("the schema loads");

    let transaction = connection.transaction().expect("a transaction opens");
    transaction.execute_batch(&sql).expect("the list tables load");
    transaction.commit().expect("the load commits");

    assert_eq!(tables.map(|table| count(&connection, table)), [2, 7, 14]);
    let dangling = connection.execute("INSERT INTO list_nodes (id, list_id) VALUES ('node', 'no-such-list')", []);
    assert!(dangling.is_err(), "foreign keys are enforced: {dangling:?}");
}
