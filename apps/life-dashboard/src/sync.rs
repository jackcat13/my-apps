//! Orchestrates a connection's refresh: pick the connector for its `connector_type`, fetch,
//! and persist the result as accounts/positions/transactions/a new snapshot, then stamp the
//! connection's `last_sync`. Nothing is written until the fetch has fully succeeded, so a
//! failed refresh leaves previously persisted data untouched.

use jiff::Timestamp;
use sqlx::SqlitePool;

use crate::connectors::interactive_brokers::InteractiveBrokersConnector;
use crate::connectors::{Connector, ConnectorError, FetchedAccount};
use crate::db::connections::Connection;
use crate::db::{DbError, accounts, connections, positions, snapshots, transactions};

/// Everything that can go wrong refreshing a connection.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// `connector_type` doesn't match any known connector.
    #[error("unknown connector type {connector_type:?}")]
    UnknownConnector {
        /// The connection's `connector_type` value.
        connector_type: String,
    },
    /// The connector failed to fetch data from its upstream API.
    #[error(transparent)]
    Connector(#[from] ConnectorError),
    /// Persisting the fetched data failed.
    #[error(transparent)]
    Db(#[from] DbError),
}

/// Refreshes one connection: fetches its current data from the connector matching its
/// `connector_type`, then persists it.
pub async fn refresh_connection(
    pool: &SqlitePool,
    client: &reqwest::Client,
    connection: &Connection,
) -> Result<(), SyncError> {
    let fetched = match connection.connector_type.as_str() {
        "interactive-brokers" => {
            InteractiveBrokersConnector
                .fetch(client, connection)
                .await?
        }
        other => {
            return Err(SyncError::UnknownConnector {
                connector_type: other.to_owned(),
            });
        }
    };

    let taken_at = Timestamp::now();
    for account in fetched {
        persist_account(pool, connection.id, taken_at, account).await?;
    }
    connections::mark_synced(pool, connection.id, taken_at).await?;
    Ok(())
}

async fn persist_account(
    pool: &SqlitePool,
    connection_id: i64,
    taken_at: Timestamp,
    fetched: FetchedAccount,
) -> Result<(), DbError> {
    let account =
        accounts::upsert(pool, connection_id, &fetched.external_id, &fetched.currency).await?;

    let position_drafts: Vec<_> = fetched
        .positions
        .iter()
        .map(|position| positions::PositionDraft {
            symbol: position.symbol.clone(),
            quantity: position.quantity,
            cost_basis: position.cost_basis,
            current_value: position.current_value,
            native_currency: position.native_currency.clone(),
        })
        .collect();
    positions::replace_for_account(pool, account.id, &position_drafts).await?;

    let total_value: f64 = fetched
        .positions
        .iter()
        .map(|position| position.current_value)
        .sum();
    snapshots::insert(pool, account.id, taken_at, total_value, &account.currency).await?;

    let transaction_drafts: Vec<_> = fetched
        .transactions
        .into_iter()
        .map(|transaction| transactions::TransactionDraft {
            occurred_at: transaction.occurred_at,
            kind: transaction.kind,
            amount: transaction.amount,
            asset: transaction.asset,
        })
        .collect();
    transactions::insert_new_for_account(pool, account.id, &transaction_drafts).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::refresh_connection;
    use crate::db::connect;
    use crate::db::connections::{self, ConnectionDraft};
    use crate::db::{accounts, snapshots};

    // The returned `TempDir` must be kept alive for as long as `pool` is used: dropping it
    // deletes the underlying SQLite file out from under the open pool.
    async fn new_connection(
        credentials: String,
    ) -> (tempfile::TempDir, sqlx::SqlitePool, connections::Connection) {
        let dir = tempfile::tempdir().expect("creating a temp dir cannot fail in CI");
        let pool = connect(&dir.path().join("db.sqlite3"))
            .await
            .expect("connect");
        let connection = connections::create(
            &pool,
            &ConnectionDraft {
                name: "Interactive Brokers".to_owned(),
                connector_type: "interactive-brokers".to_owned(),
                credentials,
            },
        )
        .await
        .expect("create connection");
        (dir, pool, connection)
    }

    #[tokio::test]
    async fn a_successful_refresh_persists_accounts_positions_and_a_snapshot() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/portfolio/accounts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "accountId": "U1234567", "currency": "USD" }
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/portfolio/U1234567/positions/0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "contractDesc": "AAPL",
                    "position": 10.0,
                    "avgCost": 150.0,
                    "mktValue": 1800.0,
                    "currency": "USD"
                }
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/iserver/account/trades"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;
        let (_dir, pool, connection) = new_connection(server.uri()).await;

        refresh_connection(&pool, &reqwest::Client::new(), &connection)
            .await
            .expect("refresh");

        let stored_accounts = accounts::list(&pool).await.expect("list accounts");
        assert_eq!(stored_accounts.len(), 1);
        assert_eq!(stored_accounts[0].name, "U1234567");

        let snapshot = snapshots::latest_for_account(&pool, stored_accounts[0].id)
            .await
            .expect("latest snapshot")
            .expect("a snapshot should have been written");
        assert_eq!(snapshot.total_value, 1800.0);

        let refreshed = connections::list(&pool).await.expect("list connections");
        assert!(refreshed[0].last_sync.is_some());
    }

    #[tokio::test]
    async fn a_failed_fetch_leaves_previously_persisted_data_untouched() {
        let server = MockServer::start().await;
        // No mocks mounted: every request gets wiremock's default 404.
        let (_dir, pool, connection) = new_connection(server.uri()).await;

        let result = refresh_connection(&pool, &reqwest::Client::new(), &connection).await;

        assert!(result.is_err());
        assert_eq!(
            accounts::list(&pool).await.expect("list accounts"),
            Vec::new()
        );
        let refreshed = connections::list(&pool).await.expect("list connections");
        assert_eq!(refreshed[0].last_sync, None);
    }

    #[tokio::test]
    async fn an_unknown_connector_type_fails_without_a_request() {
        let dir = tempfile::tempdir().expect("creating a temp dir cannot fail in CI");
        let pool = connect(&dir.path().join("db.sqlite3"))
            .await
            .expect("connect");
        let connection = connections::create(
            &pool,
            &ConnectionDraft {
                name: "Mystery broker".to_owned(),
                connector_type: "mystery-broker".to_owned(),
                credentials: "https://example.test".to_owned(),
            },
        )
        .await
        .expect("create connection");

        let result = refresh_connection(&pool, &reqwest::Client::new(), &connection).await;

        assert!(result.is_err());
    }
}
