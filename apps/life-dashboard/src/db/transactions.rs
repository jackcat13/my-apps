//! CRUD for `transactions`: an append-only history of trades/movements under one account.
//! Unlike positions, a refresh only adds transactions that aren't already recorded, so
//! repeated refreshes don't duplicate history.

use std::future::Future;

use jiff::Timestamp;
use sqlx::{Acquire, Sqlite, SqlitePool};

use super::{DbError, decode_timestamp};

/// A transaction: one trade or movement in one account.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    /// Row id.
    pub id: i64,
    /// The account this transaction belongs to.
    pub account_id: i64,
    /// When the transaction occurred.
    pub occurred_at: Timestamp,
    /// Connector-reported kind, e.g. `"BUY"`/`"SELL"`.
    pub kind: String,
    /// Signed amount, in the account's currency.
    pub amount: f64,
    /// The asset this transaction concerns, if any (e.g. a ticker).
    pub asset: Option<String>,
}

/// Fields a connector reports for one transaction, before it has an `id`/`account_id`.
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionDraft {
    /// When the transaction occurred.
    pub occurred_at: Timestamp,
    /// Connector-reported kind, e.g. `"BUY"`/`"SELL"`.
    pub kind: String,
    /// Signed amount, in the account's currency.
    pub amount: f64,
    /// The asset this transaction concerns, if any (e.g. a ticker).
    pub asset: Option<String>,
}

#[derive(sqlx::FromRow)]
struct TransactionRow {
    id: i64,
    account_id: i64,
    occurred_at: String,
    kind: String,
    amount: f64,
    asset: Option<String>,
}

impl TransactionRow {
    fn into_transaction(self) -> Result<Transaction, DbError> {
        Ok(Transaction {
            id: self.id,
            account_id: self.account_id,
            occurred_at: decode_timestamp(&self.occurred_at)?,
            kind: self.kind,
            amount: self.amount,
            asset: self.asset,
        })
    }
}

/// Lists every transaction for `account_id`, most recent first.
pub async fn list_for_account(
    pool: &SqlitePool,
    account_id: i64,
) -> Result<Vec<Transaction>, DbError> {
    let rows: Vec<TransactionRow> = sqlx::query_as(
        "SELECT id, account_id, occurred_at, transaction_type AS kind, amount, asset \
         FROM transactions WHERE account_id = ? ORDER BY occurred_at DESC",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)?;
    rows.into_iter()
        .map(TransactionRow::into_transaction)
        .collect()
}

/// Inserts every draft that isn't already recorded for `account_id` (same timestamp, type,
/// amount and asset), so refreshing a connection repeatedly doesn't duplicate history.
///
/// Accepts anything [`sqlx::Acquire`] can produce a connection from: a `&SqlitePool` opens
/// its own transaction; a `&mut sqlx::Transaction` (used by
/// [`crate::sync::refresh_connection`]) runs as a nested savepoint inside the caller's
/// transaction instead, so the whole refresh stays atomic (and the existence-check-then-
/// insert for each draft can't race a concurrent refresh of the same account).
#[expect(
    clippy::manual_async_fn,
    reason = "async fn sugar here triggers sqlx's known 'implementation of Acquire is not \
              general enough' HRTB error when called from Promise::spawn_async; the explicit \
              impl Future + Send + 'a signature is sqlx's own documented workaround"
)]
pub fn insert_new_for_account<'a, A>(
    conn: A,
    account_id: i64,
    transactions: &'a [TransactionDraft],
) -> impl Future<Output = Result<(), DbError>> + Send + 'a
where
    A: Acquire<'a, Database = Sqlite> + Send + 'a,
{
    async move {
        let mut tx = conn.begin().await.map_err(DbError::Query)?;

        for transaction in transactions {
            let occurred_at = transaction.occurred_at.to_string();
            let exists: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM transactions \
                 WHERE account_id = ? AND occurred_at = ? AND transaction_type = ? \
                 AND amount = ? AND asset IS ?",
            )
            .bind(account_id)
            .bind(&occurred_at)
            .bind(&transaction.kind)
            .bind(transaction.amount)
            .bind(&transaction.asset)
            .fetch_optional(&mut *tx)
            .await
            .map_err(DbError::Query)?;
            if exists.is_some() {
                continue;
            }

            sqlx::query(
                "INSERT INTO transactions \
                 (account_id, occurred_at, transaction_type, amount, asset) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(account_id)
            .bind(&occurred_at)
            .bind(&transaction.kind)
            .bind(transaction.amount)
            .bind(&transaction.asset)
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
    use jiff::Timestamp;

    use super::{TransactionDraft, insert_new_for_account, list_for_account};
    use crate::db::test_support;

    async fn pool_with_account() -> (tempfile::TempDir, sqlx::SqlitePool, i64) {
        let (dir, pool) = test_support::pool().await;
        let connection = test_support::connection(&pool).await;
        let account = test_support::account(&pool, connection.id).await;
        (dir, pool, account.id)
    }

    fn draft(occurred_at: Timestamp) -> TransactionDraft {
        TransactionDraft {
            occurred_at,
            kind: "BUY".to_owned(),
            amount: 1500.0,
            asset: Some("AAPL".to_owned()),
        }
    }

    #[tokio::test]
    async fn inserting_new_transactions_stores_them() {
        let (_dir, pool, account_id) = pool_with_account().await;

        insert_new_for_account(&pool, account_id, &[draft(Timestamp::now())])
            .await
            .expect("insert");

        let transactions = list_for_account(&pool, account_id).await.expect("list");
        assert_eq!(transactions.len(), 1);
    }

    #[tokio::test]
    async fn inserting_the_same_transaction_twice_does_not_duplicate_it() {
        let (_dir, pool, account_id) = pool_with_account().await;
        let at = Timestamp::now();

        insert_new_for_account(&pool, account_id, &[draft(at)])
            .await
            .expect("first insert");
        insert_new_for_account(&pool, account_id, &[draft(at)])
            .await
            .expect("second insert");

        let transactions = list_for_account(&pool, account_id).await.expect("list");
        assert_eq!(transactions.len(), 1);
    }
}
