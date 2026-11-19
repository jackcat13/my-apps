//! CRUD for `snapshots`: a historical record of one account's total value, written on every
//! connector refresh. Unlike positions, snapshots are append-only.

use jiff::Timestamp;
use sqlx::SqlitePool;

use super::{DbError, decode_timestamp};

/// A snapshot: one account's total value at a point in time.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// Row id.
    pub id: i64,
    /// The account this snapshot belongs to.
    pub account_id: i64,
    /// When this snapshot was taken.
    pub taken_at: Timestamp,
    /// Total value of the account's positions at `taken_at`.
    pub total_value: f64,
    /// ISO 4217 currency code `total_value` is denominated in.
    pub currency: String,
}

#[derive(sqlx::FromRow)]
struct SnapshotRow {
    id: i64,
    account_id: i64,
    taken_at: String,
    total_value: f64,
    currency: String,
}

impl SnapshotRow {
    fn into_snapshot(self) -> Result<Snapshot, DbError> {
        Ok(Snapshot {
            id: self.id,
            account_id: self.account_id,
            taken_at: decode_timestamp(&self.taken_at)?,
            total_value: self.total_value,
            currency: self.currency,
        })
    }
}

/// Records a new snapshot for `account_id`.
pub async fn insert(
    pool: &SqlitePool,
    account_id: i64,
    taken_at: Timestamp,
    total_value: f64,
    currency: &str,
) -> Result<Snapshot, DbError> {
    let id = sqlx::query(
        "INSERT INTO snapshots (account_id, taken_at, total_value, currency) VALUES (?, ?, ?, ?)",
    )
    .bind(account_id)
    .bind(taken_at.to_string())
    .bind(total_value)
    .bind(currency)
    .execute(pool)
    .await
    .map_err(DbError::Query)?
    .last_insert_rowid();

    Ok(Snapshot {
        id,
        account_id,
        taken_at,
        total_value,
        currency: currency.to_owned(),
    })
}

/// The most recently taken snapshot for `account_id`, if any.
pub async fn latest_for_account(
    pool: &SqlitePool,
    account_id: i64,
) -> Result<Option<Snapshot>, DbError> {
    let row: Option<SnapshotRow> = sqlx::query_as(
        "SELECT id, account_id, taken_at, total_value, currency FROM snapshots \
         WHERE account_id = ? ORDER BY taken_at DESC LIMIT 1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Query)?;
    row.map(SnapshotRow::into_snapshot).transpose()
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;

    use super::{insert, latest_for_account};
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

    #[tokio::test]
    async fn an_account_with_no_snapshots_has_none() {
        let (_dir, pool, account_id) = pool_with_account().await;

        assert_eq!(
            latest_for_account(&pool, account_id).await.expect("latest"),
            None
        );
    }

    #[tokio::test]
    async fn the_latest_snapshot_is_the_most_recently_taken_one() {
        let (_dir, pool, account_id) = pool_with_account().await;
        let earlier = Timestamp::now();
        insert(&pool, account_id, earlier, 1000.0, "USD")
            .await
            .expect("insert earlier");
        let later = earlier
            .checked_add(std::time::Duration::from_secs(60))
            .expect("valid");

        insert(&pool, account_id, later, 1800.0, "USD")
            .await
            .expect("insert later");

        let latest = latest_for_account(&pool, account_id)
            .await
            .expect("latest")
            .expect("a snapshot exists");
        assert_eq!(latest.total_value, 1800.0);
    }
}
