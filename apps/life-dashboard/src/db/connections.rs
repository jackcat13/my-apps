//! CRUD for `connections`: one row per institution the Settings screen lets the user manage.

use std::future::Future;

use jiff::Timestamp;
use sqlx::{Acquire, Sqlite, SqlitePool};

use super::{DbError, decode_timestamp};

/// An institution connection: credentials are stored as entered, plaintext (see
/// `apps/life-dashboard/AGENTS.md` for why this is an accepted risk for v1).
#[derive(Debug, Clone, PartialEq)]
pub struct Connection {
    /// Row id.
    pub id: i64,
    /// Display name, e.g. "Interactive Brokers".
    pub name: String,
    /// Which connector this credential set is for, e.g. "interactive-brokers".
    pub connector_type: String,
    /// Opaque credential blob, as entered by the user.
    pub credentials: String,
    /// When this connection last synced successfully, if ever.
    pub last_sync: Option<Timestamp>,
}

/// Fields the Settings screen collects to create or update a [`Connection`].
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionDraft {
    /// Display name.
    pub name: String,
    /// Connector type.
    pub connector_type: String,
    /// Opaque credential blob.
    pub credentials: String,
}

#[derive(sqlx::FromRow)]
struct ConnectionRow {
    id: i64,
    name: String,
    connector_type: String,
    credentials: String,
    last_sync: Option<String>,
}

impl ConnectionRow {
    fn into_connection(self) -> Result<Connection, DbError> {
        let last_sync = self
            .last_sync
            .as_deref()
            .map(decode_timestamp)
            .transpose()?;
        Ok(Connection {
            id: self.id,
            name: self.name,
            connector_type: self.connector_type,
            credentials: self.credentials,
            last_sync,
        })
    }
}

/// Lists every connection, ordered by name.
pub async fn list(pool: &SqlitePool) -> Result<Vec<Connection>, DbError> {
    let rows: Vec<ConnectionRow> = sqlx::query_as(
        "SELECT id, name, connector_type, credentials, last_sync \
         FROM connections ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)?;
    rows.into_iter()
        .map(ConnectionRow::into_connection)
        .collect()
}

/// Inserts a new connection and returns it with its assigned id.
pub async fn create(pool: &SqlitePool, draft: &ConnectionDraft) -> Result<Connection, DbError> {
    let id =
        sqlx::query("INSERT INTO connections (name, connector_type, credentials) VALUES (?, ?, ?)")
            .bind(&draft.name)
            .bind(&draft.connector_type)
            .bind(&draft.credentials)
            .execute(pool)
            .await
            .map_err(DbError::Query)?
            .last_insert_rowid();

    Ok(Connection {
        id,
        name: draft.name.clone(),
        connector_type: draft.connector_type.clone(),
        credentials: draft.credentials.clone(),
        last_sync: None,
    })
}

/// Overwrites the editable fields of an existing connection.
pub async fn update(pool: &SqlitePool, id: i64, draft: &ConnectionDraft) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE connections SET name = ?, connector_type = ?, credentials = ? WHERE id = ?",
    )
    .bind(&draft.name)
    .bind(&draft.connector_type)
    .bind(&draft.credentials)
    .bind(id)
    .execute(pool)
    .await
    .map_err(DbError::Query)?;
    Ok(())
}

/// Stamps a connection as having just synced successfully.
///
/// Accepts anything [`sqlx::Acquire`] can produce a connection from: a `&SqlitePool` acquires
/// its own connection; a `&mut sqlx::Transaction` (used by
/// [`crate::sync::refresh_connection`]) writes as part of the caller's transaction instead,
/// so the whole refresh stays atomic.
#[expect(
    clippy::manual_async_fn,
    reason = "async fn sugar here triggers sqlx's known 'implementation of Acquire is not \
              general enough' HRTB error when called from Promise::spawn_async; the explicit \
              impl Future + Send + 'a signature is sqlx's own documented workaround"
)]
pub fn mark_synced<'a, A>(
    conn: A,
    id: i64,
    at: Timestamp,
) -> impl Future<Output = Result<(), DbError>> + Send + 'a
where
    A: Acquire<'a, Database = Sqlite> + Send + 'a,
{
    async move {
        let mut conn = conn.acquire().await.map_err(DbError::Query)?;
        sqlx::query("UPDATE connections SET last_sync = ? WHERE id = ?")
            .bind(at.to_string())
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(DbError::Query)?;
        Ok(())
    }
}

/// Deletes a connection. Accounts that referenced it keep their history (`connection_id`
/// is set to `NULL` by the schema's `ON DELETE SET NULL`).
pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), DbError> {
    sqlx::query("DELETE FROM connections WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(DbError::Query)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ConnectionDraft, create, delete, list, update};
    use crate::db::connect;

    fn draft(name: &str) -> ConnectionDraft {
        ConnectionDraft {
            name: name.to_owned(),
            connector_type: "interactive-brokers".to_owned(),
            credentials: "token=abc".to_owned(),
        }
    }

    #[tokio::test]
    async fn created_connection_appears_in_the_list() {
        let dir = tempfile::tempdir().expect("creating a temp dir cannot fail in CI");
        let pool = connect(&dir.path().join("db.sqlite3"))
            .await
            .expect("connect");

        create(&pool, &draft("Interactive Brokers"))
            .await
            .expect("create");

        let connections = list(&pool).await.expect("list");
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].name, "Interactive Brokers");
        assert_eq!(connections[0].last_sync, None);
    }

    #[tokio::test]
    async fn updated_connection_keeps_its_id() {
        let dir = tempfile::tempdir().expect("creating a temp dir cannot fail in CI");
        let pool = connect(&dir.path().join("db.sqlite3"))
            .await
            .expect("connect");
        let created = create(&pool, &draft("Old name")).await.expect("create");

        update(&pool, created.id, &draft("New name"))
            .await
            .expect("update");

        let connections = list(&pool).await.expect("list");
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].id, created.id);
        assert_eq!(connections[0].name, "New name");
    }

    #[tokio::test]
    async fn deleted_connection_no_longer_appears_in_the_list() {
        let dir = tempfile::tempdir().expect("creating a temp dir cannot fail in CI");
        let pool = connect(&dir.path().join("db.sqlite3"))
            .await
            .expect("connect");
        let created = create(&pool, &draft("Interactive Brokers"))
            .await
            .expect("create");

        delete(&pool, created.id).await.expect("delete");

        assert_eq!(list(&pool).await.expect("list"), Vec::new());
    }
}
