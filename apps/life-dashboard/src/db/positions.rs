//! CRUD for `positions`: the holdings under one account, as of the last refresh. Positions
//! are a point-in-time snapshot of what the connector reported, not an append-only history
//! (see [`crate::db::snapshots`] for the historical total), so a refresh replaces the set.

use std::future::Future;

use sqlx::{Acquire, Sqlite, SqlitePool};

use super::DbError;

/// A position: one holding in one account.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct Position {
    /// Row id.
    pub id: i64,
    /// The account this position belongs to.
    pub account_id: i64,
    /// Ticker or other identifier the connector reported.
    pub symbol: String,
    /// Number of units held.
    pub quantity: f64,
    /// Total cost basis for the position, in `native_currency`.
    pub cost_basis: f64,
    /// Current market value, in `native_currency`.
    pub current_value: f64,
    /// ISO 4217 currency code this position is denominated in.
    pub native_currency: String,
}

/// Fields a connector reports for one position, before it has an `id`/`account_id`.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionDraft {
    /// Ticker or other identifier the connector reported.
    pub symbol: String,
    /// Number of units held.
    pub quantity: f64,
    /// Total cost basis for the position, in `native_currency`.
    pub cost_basis: f64,
    /// Current market value, in `native_currency`.
    pub current_value: f64,
    /// ISO 4217 currency code this position is denominated in.
    pub native_currency: String,
}

/// Lists every position for `account_id`, ordered by symbol.
pub async fn list_for_account(
    pool: &SqlitePool,
    account_id: i64,
) -> Result<Vec<Position>, DbError> {
    sqlx::query_as(
        "SELECT id, account_id, symbol, quantity, cost_basis, current_value, native_currency \
         FROM positions WHERE account_id = ? ORDER BY symbol",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)
}

/// Replaces every position for `account_id` with `positions`, in one transaction.
///
/// Accepts anything [`sqlx::Acquire`] can produce a connection from: a `&SqlitePool` opens
/// its own transaction; a `&mut sqlx::Transaction` (used by
/// [`crate::sync::refresh_connection`]) runs as a nested savepoint inside the caller's
/// transaction instead, so the whole refresh stays atomic.
#[expect(
    clippy::manual_async_fn,
    reason = "async fn sugar here triggers sqlx's known 'implementation of Acquire is not \
              general enough' HRTB error when called from Promise::spawn_async; the explicit \
              impl Future + Send + 'a signature is sqlx's own documented workaround"
)]
pub fn replace_for_account<'a, A>(
    conn: A,
    account_id: i64,
    positions: &'a [PositionDraft],
) -> impl Future<Output = Result<(), DbError>> + Send + 'a
where
    A: Acquire<'a, Database = Sqlite> + Send + 'a,
{
    async move {
        let mut tx = conn.begin().await.map_err(DbError::Query)?;

        sqlx::query("DELETE FROM positions WHERE account_id = ?")
            .bind(account_id)
            .execute(&mut *tx)
            .await
            .map_err(DbError::Query)?;

        for position in positions {
            sqlx::query(
                "INSERT INTO positions \
                 (account_id, symbol, quantity, cost_basis, current_value, native_currency) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(account_id)
            .bind(&position.symbol)
            .bind(position.quantity)
            .bind(position.cost_basis)
            .bind(position.current_value)
            .bind(&position.native_currency)
            .execute(&mut *tx)
            .await
            .map_err(DbError::Query)?;
        }

        tx.commit().await.map_err(DbError::Query)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{PositionDraft, list_for_account, replace_for_account};
    use crate::db::test_support;

    async fn pool_with_account() -> (tempfile::TempDir, sqlx::SqlitePool, i64) {
        let (dir, pool) = test_support::pool().await;
        let connection = test_support::connection(&pool).await;
        let account = test_support::account(&pool, connection.id).await;
        (dir, pool, account.id)
    }

    fn draft(symbol: &str) -> PositionDraft {
        PositionDraft {
            symbol: symbol.to_owned(),
            quantity: 10.0,
            cost_basis: 1500.0,
            current_value: 1800.0,
            native_currency: "USD".to_owned(),
        }
    }

    #[tokio::test]
    async fn replacing_positions_stores_the_new_set() {
        let (_dir, pool, account_id) = pool_with_account().await;

        replace_for_account(&pool, account_id, &[draft("AAPL")])
            .await
            .expect("replace");

        let positions = list_for_account(&pool, account_id).await.expect("list");
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].symbol, "AAPL");
    }

    #[tokio::test]
    async fn replacing_positions_drops_ones_no_longer_reported() {
        let (_dir, pool, account_id) = pool_with_account().await;
        replace_for_account(&pool, account_id, &[draft("AAPL"), draft("MSFT")])
            .await
            .expect("first replace");

        replace_for_account(&pool, account_id, &[draft("AAPL")])
            .await
            .expect("second replace");

        let positions = list_for_account(&pool, account_id).await.expect("list");
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].symbol, "AAPL");
    }
}
