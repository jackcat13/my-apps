//! CRUD for `accounts`: one row per account, optionally linked to the [`super::connections`]
//! row that feeds it.

use std::future::Future;

use sqlx::{Acquire, Sqlite, SqlitePool};

use super::DbError;

/// An account under a connection (or standalone, if `connection_id` is `None`).
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct Account {
    /// Row id.
    pub id: i64,
    /// The connection this account was fetched from, if any.
    pub connection_id: Option<i64>,
    /// The connector's own identifier for this account (e.g. an IBKR account id). Immutable
    /// once created: it's the key a later refresh uses to find this row again, not a label.
    pub external_id: String,
    /// Human-friendly label shown on the Dashboard. Defaults to `external_id` when the
    /// account is first created (there's no rename UI yet).
    pub name: String,
    /// ISO 4217 currency code the account's positions and snapshots are denominated in.
    pub currency: String,
}

/// Lists every account, ordered by name.
pub async fn list(pool: &SqlitePool) -> Result<Vec<Account>, DbError> {
    sqlx::query_as(
        "SELECT id, connection_id, external_id, name, currency FROM accounts ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)
}

/// Finds the account for `connection_id` with connector identifier `external_id`, creating
/// it (with `name` defaulted to `external_id`) if it doesn't exist yet. Updates its currency
/// if the connector reports a different one than last time; never touches `name`, so a
/// future rename survives refreshes.
///
/// Accepts anything [`sqlx::Acquire`] can produce a connection from: a `&SqlitePool` (used
/// standalone, e.g. in tests) opens its own transaction for the check-then-write; a
/// `&mut sqlx::Transaction` (used by [`crate::sync::refresh_connection`]) runs as a nested
/// savepoint inside the caller's transaction instead, so the whole refresh stays atomic.
#[expect(
    clippy::manual_async_fn,
    reason = "async fn sugar here triggers sqlx's known 'implementation of Acquire is not \
              general enough' HRTB error when called from Promise::spawn_async; the explicit \
              impl Future + Send + 'a signature is sqlx's own documented workaround"
)]
pub fn upsert<'a, A>(
    conn: A,
    connection_id: i64,
    external_id: &'a str,
    currency: &'a str,
) -> impl Future<Output = Result<Account, DbError>> + Send + 'a
where
    A: Acquire<'a, Database = Sqlite> + Send + 'a,
{
    async move {
        let mut tx = conn.begin().await.map_err(DbError::Query)?;

        let existing: Option<Account> = sqlx::query_as(
            "SELECT id, connection_id, external_id, name, currency FROM accounts \
             WHERE connection_id = ? AND external_id = ?",
        )
        .bind(connection_id)
        .bind(external_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(DbError::Query)?;

        let account = if let Some(account) = existing {
            if account.currency == currency {
                account
            } else {
                sqlx::query("UPDATE accounts SET currency = ? WHERE id = ?")
                    .bind(currency)
                    .bind(account.id)
                    .execute(&mut *tx)
                    .await
                    .map_err(DbError::Query)?;
                Account {
                    currency: currency.to_owned(),
                    ..account
                }
            }
        } else {
            let id = sqlx::query(
                "INSERT INTO accounts (connection_id, external_id, name, currency) \
                 VALUES (?, ?, ?, ?)",
            )
            .bind(connection_id)
            .bind(external_id)
            .bind(external_id)
            .bind(currency)
            .execute(&mut *tx)
            .await
            .map_err(DbError::Query)?
            .last_insert_rowid();

            Account {
                id,
                connection_id: Some(connection_id),
                external_id: external_id.to_owned(),
                name: external_id.to_owned(),
                currency: currency.to_owned(),
            }
        };

        tx.commit().await.map_err(DbError::Query)?;
        Ok(account)
    }
}

#[cfg(test)]
mod tests {
    use super::{list, upsert};
    use crate::db::test_support;

    #[tokio::test]
    async fn upserting_a_new_account_inserts_it() {
        let (_dir, pool) = test_support::pool().await;
        let connection = test_support::connection(&pool).await;

        let account = upsert(&pool, connection.id, "U1234567", "USD")
            .await
            .expect("upsert");

        assert_eq!(account.name, "U1234567");
        let accounts = list(&pool).await.expect("list");
        assert_eq!(accounts, vec![account]);
    }

    #[tokio::test]
    async fn upserting_the_same_account_twice_keeps_one_row() {
        let (_dir, pool) = test_support::pool().await;
        let connection = test_support::connection(&pool).await;
        let first = upsert(&pool, connection.id, "U1234567", "USD")
            .await
            .expect("first upsert");

        let second = upsert(&pool, connection.id, "U1234567", "EUR")
            .await
            .expect("second upsert");

        assert_eq!(first.id, second.id);
        assert_eq!(second.currency, "EUR");
        let accounts = list(&pool).await.expect("list");
        assert_eq!(accounts.len(), 1);
    }

    #[tokio::test]
    async fn upserting_an_existing_account_does_not_overwrite_a_renamed_label() {
        let (_dir, pool) = test_support::pool().await;
        let connection = test_support::connection(&pool).await;
        upsert(&pool, connection.id, "U1234567", "USD")
            .await
            .expect("first upsert");
        sqlx::query("UPDATE accounts SET name = ? WHERE external_id = ?")
            .bind("My IBKR account")
            .bind("U1234567")
            .execute(&pool)
            .await
            .expect("rename");

        let account = upsert(&pool, connection.id, "U1234567", "EUR")
            .await
            .expect("second upsert");

        assert_eq!(account.name, "My IBKR account");
    }
}
