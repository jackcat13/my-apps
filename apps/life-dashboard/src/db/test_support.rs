//! Shared fixtures for `db::*` and `sync` tests: a fresh database, a connection, an account.
//! Kept in one place so a change to `ConnectionDraft`/`accounts::upsert` only needs updating
//! here instead of in every test module that needs a connection or account to hang records
//! off of.

use sqlx::SqlitePool;

use super::accounts::{self, Account};
use super::connect;
use super::connections::{self, Connection, ConnectionDraft};

/// A fresh on-disk SQLite pool. The returned `TempDir` must be kept alive for as long as the
/// pool is used: dropping it deletes the underlying file out from under the open pool.
pub(crate) async fn pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().expect("creating a temp dir cannot fail in CI");
    let pool = connect(&dir.path().join("db.sqlite3"))
        .await
        .expect("connect");
    (dir, pool)
}

/// An `interactive-brokers` connection pointing at `credentials`.
pub(crate) async fn connection_with_credentials(
    pool: &SqlitePool,
    credentials: &str,
) -> Connection {
    connections::create(
        pool,
        &ConnectionDraft {
            name: "Interactive Brokers".to_owned(),
            connector_type: "interactive-brokers".to_owned(),
            credentials: credentials.to_owned(),
        },
    )
    .await
    .expect("create connection")
}

/// An `interactive-brokers` connection with a placeholder gateway URL.
pub(crate) async fn connection(pool: &SqlitePool) -> Connection {
    connection_with_credentials(pool, "https://localhost:5000").await
}

/// An account (external id `"U1234567"`, currency `"USD"`) under `connection_id`.
pub(crate) async fn account(pool: &SqlitePool, connection_id: i64) -> Account {
    accounts::upsert(pool, connection_id, "U1234567", "USD")
        .await
        .expect("upsert account")
}
