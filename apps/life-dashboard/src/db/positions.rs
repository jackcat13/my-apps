//! CRUD for `positions`: the holdings under one account, as of the last refresh. Positions
//! are a point-in-time snapshot of what the connector reported, not an append-only history
//! (see [`crate::db::snapshots`] for the historical total), so a refresh replaces the set.

use sqlx::SqlitePool;

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
pub async fn replace_for_account(
    pool: &SqlitePool,
    account_id: i64,
    positions: &[PositionDraft],
) -> Result<(), DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;

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

#[cfg(test)]
mod tests {
    use super::{PositionDraft, list_for_account, replace_for_account};
    use crate::db::accounts::upsert;
    use crate::db::connect;
    use crate::db::connections::{self, ConnectionDraft};

    // The returned `TempDir` must be kept alive for as long as `pool` is used: dropping it
    // deletes the underlying SQLite file out from under the open pool.
    async fn pool_with_account() -> (tempfile::TempDir, sqlx::SqlitePool, i64) {
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
        let account = upsert(&pool, connection.id, "U1234567", "USD")
            .await
            .expect("upsert account");
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
