use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite_migration::{Migrations, M};

const MIGRATIONS_SLICE: &[M<'_>] = &[
    M::up("CREATE TABLE friend(name TEXT NOT NULL);"),
    M::up("INSERT INTO friend(name) VALUES ('one');"),
];
const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATIONS_SLICE);

pub fn setup_db() -> Pool<SqliteConnectionManager> {
    let manager = SqliteConnectionManager::memory();
    let pool = r2d2::Pool::new(manager).unwrap();

    let mut conn = pool.get().unwrap();

    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(())).unwrap();

    MIGRATIONS.to_latest(&mut conn).unwrap();

    pool
}
