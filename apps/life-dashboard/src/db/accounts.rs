//! CRUD for `accounts`: one row per account, optionally linked to the [`super::connections`]
//! row that feeds it.

use sqlx::SqlitePool;

use super::DbError;

/// An account under a connection (or standalone, if `connection_id` is `None`).
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct Account {
    /// Row id.
    pub id: i64,
    /// The connection this account was fetched from, if any.
    pub connection_id: Option<i64>,
    /// The connector's own identifier for this account (e.g. an IBKR account id), used so a
    /// later refresh can find the same row again.
    pub name: String,
    /// ISO 4217 currency code the account's positions and snapshots are denominated in.
    pub currency: String,
}

/// Lists every account, ordered by name.
pub async fn list(pool: &SqlitePool) -> Result<Vec<Account>, DbError> {
    sqlx::query_as("SELECT id, connection_id, name, currency FROM accounts ORDER BY name")
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)
}

/// Finds the account for `connection_id` named `name`, creating it if it doesn't exist yet.
/// Updates its currency if the connector reports a different one than last time.
pub async fn upsert(
    pool: &SqlitePool,
    connection_id: i64,
    name: &str,
    currency: &str,
) -> Result<Account, DbError> {
    let existing: Option<Account> = sqlx::query_as(
        "SELECT id, connection_id, name, currency FROM accounts \
         WHERE connection_id = ? AND name = ?",
    )
    .bind(connection_id)
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Query)?;

    if let Some(account) = existing {
        if account.currency == currency {
            return Ok(account);
        }
        sqlx::query("UPDATE accounts SET currency = ? WHERE id = ?")
            .bind(currency)
            .bind(account.id)
            .execute(pool)
            .await
            .map_err(DbError::Query)?;
        return Ok(Account {
            currency: currency.to_owned(),
            ..account
        });
    }

    let id = sqlx::query("INSERT INTO accounts (connection_id, name, currency) VALUES (?, ?, ?)")
        .bind(connection_id)
        .bind(name)
        .bind(currency)
        .execute(pool)
        .await
        .map_err(DbError::Query)?
        .last_insert_rowid();

    Ok(Account {
        id,
        connection_id: Some(connection_id),
        name: name.to_owned(),
        currency: currency.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::{list, upsert};
    use crate::db::connect;
    use crate::db::connections::{self, ConnectionDraft};

    // The returned `TempDir` must be kept alive for as long as `pool` is used: dropping it
    // deletes the underlying SQLite file out from under the open pool.
    async fn pool_with_connection() -> (tempfile::TempDir, sqlx::SqlitePool, i64) {
        let dir = tempfile::tempdir().expect("creating a temp dir cannot fail in CI");
        let pool = connect(&dir.path().join("db.sqlite3"))
            .await
            .expect("connect");
        let connection = connections::create(
            &pool,
            &ConnectionDraft {
                name: "Interactive Brokers".to_owned(),
                connector_type: "interactive-brokers".to_owned(),
                credentials: "https://localhost:5000".to_owned(),
            },
        )
        .await
        .expect("create connection");
        (dir, pool, connection.id)
    }

    #[tokio::test]
    async fn upserting_a_new_account_inserts_it() {
        let (_dir, pool, connection_id) = pool_with_connection().await;

        let account = upsert(&pool, connection_id, "U1234567", "USD")
            .await
            .expect("upsert");

        let accounts = list(&pool).await.expect("list");
        assert_eq!(accounts, vec![account]);
    }

    #[tokio::test]
    async fn upserting_the_same_account_twice_keeps_one_row() {
        let (_dir, pool, connection_id) = pool_with_connection().await;
        let first = upsert(&pool, connection_id, "U1234567", "USD")
            .await
            .expect("first upsert");

        let second = upsert(&pool, connection_id, "U1234567", "EUR")
            .await
            .expect("second upsert");

        assert_eq!(first.id, second.id);
        assert_eq!(second.currency, "EUR");
        let accounts = list(&pool).await.expect("list");
        assert_eq!(accounts.len(), 1);
    }
}
