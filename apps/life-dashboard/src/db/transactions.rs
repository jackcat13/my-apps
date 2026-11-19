//! CRUD for `transactions`: an append-only history of trades/movements under one account.
//! Unlike positions, a refresh only adds transactions that aren't already recorded, so
//! repeated refreshes don't duplicate history.

use jiff::Timestamp;
use sqlx::SqlitePool;

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
pub async fn insert_new_for_account(
    pool: &SqlitePool,
    account_id: i64,
    transactions: &[TransactionDraft],
) -> Result<(), DbError> {
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
        .fetch_optional(pool)
        .await
        .map_err(DbError::Query)?;
        if exists.is_some() {
            continue;
        }

        sqlx::query(
            "INSERT INTO transactions (account_id, occurred_at, transaction_type, amount, asset) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(account_id)
        .bind(&occurred_at)
        .bind(&transaction.kind)
        .bind(transaction.amount)
        .bind(&transaction.asset)
        .execute(pool)
        .await
        .map_err(DbError::Query)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;

    use super::{TransactionDraft, insert_new_for_account, list_for_account};
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
