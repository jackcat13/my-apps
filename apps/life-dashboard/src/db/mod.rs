//! Persistence layer: opens the per-user SQLite database, runs migrations, and exposes a
//! small repository function per table. All functions take `&SqlitePool` and return
//! [`DbError`] so callers (the UI layer) can match on failure kinds.

pub mod connections;
mod error;
pub mod preferences;

use std::path::Path;

use jiff::Timestamp;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

pub use error::DbError;

/// Opens (creating if needed) the SQLite database at `path` and runs pending migrations.
pub async fn connect(path: &Path) -> Result<SqlitePool, DbError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(DbError::Connect)?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(DbError::Migrate)?;
    Ok(pool)
}

/// Parses a timestamp stored in a `TEXT` timestamp column.
fn decode_timestamp(raw: &str) -> Result<Timestamp, DbError> {
    raw.parse().map_err(|source| DbError::InvalidTimestamp {
        raw: raw.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::connect;

    /// A fresh connection against a brand-new on-disk file applies all migrations cleanly.
    #[tokio::test]
    async fn connect_creates_and_migrates_a_fresh_database() {
        let dir = tempfile::tempdir().expect("creating a temp dir cannot fail in CI");
        let path = dir.path().join("life-dashboard.sqlite3");

        let pool = connect(&path).await.expect("fresh database should open");

        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM preferences")
            .fetch_one(&pool)
            .await
            .expect("preferences table should exist after migrating");
        assert_eq!(row.0, 1);
    }
}
