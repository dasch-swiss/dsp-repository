//! Column mapping shared by the repository implementations.
//!
//! Ids and enums are stored as TEXT (see the migration for why), so reading them
//! back is a parse that can fail. These helpers turn that failure into a
//! `rusqlite` error naming the column, rather than a panic or a silent default —
//! a stored `role` the code does not recognise must not become `Depositor`.

use std::str::FromStr;

use rusqlite::types::Type;
use rusqlite::Row;
use uuid::Uuid;

/// Read a TEXT column as a [`Uuid`].
pub(super) fn uuid_column(row: &Row<'_>, index: usize) -> rusqlite::Result<Uuid> {
    let raw: String = row.get(index)?;
    Uuid::parse_str(&raw).map_err(|e| rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(e)))
}

/// Read a nullable TEXT column as an optional [`Uuid`].
pub(super) fn optional_uuid_column(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<Uuid>> {
    let raw: Option<String> = row.get(index)?;
    raw.map(|raw| {
        Uuid::parse_str(&raw).map_err(|e| rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(e)))
    })
    .transpose()
}

/// Read a TEXT column through its [`FromStr`] — the stored form of `role` and
/// `state`, both of which also carry a `CHECK` constraint in the schema.
pub(super) fn parsed_column<T>(row: &Row<'_>, index: usize) -> rusqlite::Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let raw: String = row.get(index)?;
    raw.parse()
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(e)))
}

/// A `count(*)`/`changes()` result as `u64`. SQLite counts are signed and never
/// negative, so a negative value would mean the query was not a count.
pub(super) fn row_count(counted: i64) -> u64 {
    counted.max(0).unsigned_abs()
}

/// A stored non-negative counter as `u32`.
pub(super) fn counter(stored: i64) -> u32 {
    stored.clamp(0, i64::from(u32::MAX)) as u32
}

/// `query_row` reports an absent row as `QueryReturnedNoRows`, which every
/// `find` in this layer wants as `Ok(None)` — a cookie for a session that has
/// been deleted is the ordinary case, not an error.
pub(super) trait OptionalRow<T> {
    fn optional_row(self) -> rusqlite::Result<Option<T>>;
}

impl<T> OptionalRow<T> for rusqlite::Result<T> {
    fn optional_row(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(other) => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_count_never_goes_negative() {
        assert_eq!(row_count(0), 0);
        assert_eq!(row_count(7), 7);
        assert_eq!(row_count(-1), 0);
    }

    #[test]
    fn test_optional_row_turns_a_missing_row_into_none() {
        let missing: rusqlite::Result<i64> = Err(rusqlite::Error::QueryReturnedNoRows);
        assert_eq!(missing.optional_row().unwrap(), None);
        assert_eq!(Ok::<_, rusqlite::Error>(7).optional_row().unwrap(), Some(7));
        // Any other error still has to surface.
        let broken: rusqlite::Result<i64> = Err(rusqlite::Error::InvalidColumnIndex(3));
        assert!(broken.optional_row().is_err());
    }

    #[test]
    fn test_counter_saturates_instead_of_wrapping() {
        // A wrapped counter would hand a locked-out account a fresh budget.
        assert_eq!(counter(0), 0);
        assert_eq!(counter(3), 3);
        assert_eq!(counter(-5), 0);
        assert_eq!(counter(i64::from(u32::MAX) + 10), u32::MAX);
    }
}
