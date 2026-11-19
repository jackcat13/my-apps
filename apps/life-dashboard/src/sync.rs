//! Orchestrates a connection's refresh: pick the connector for its `connector_type`, fetch,
//! and persist the result as accounts/positions/transactions/a new snapshot, then stamp the
//! connection's `last_sync`. The fetch runs fully before any write, and every write for the
//! refresh (every account, plus `last_sync`) happens inside one database transaction, so a
//! failed refresh — whether the fetch itself fails or a later account's write does — leaves
//! previously persisted data completely untouched: either the whole refresh commits, or none
//! of it does.

use jiff::Timestamp;
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::connectors::interactive_brokers::InteractiveBrokersConnector;
use crate::connectors::{Connector, ConnectorError, ConnectorType, FetchedAccount};
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
    let fetched = match ConnectorType::parse(&connection.connector_type) {
        ConnectorType::InteractiveBrokers => {
            InteractiveBrokersConnector
                .fetch(client, connection)
                .await?
        }
        ConnectorType::Unknown(connector_type) => {
            return Err(SyncError::UnknownConnector { connector_type });
        }
    };

    let taken_at = Timestamp::now();
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    for account in fetched {
        persist_account(&mut tx, connection.id, taken_at, account).await?;
    }
    connections::mark_synced(&mut tx, connection.id, taken_at).await?;
    tx.commit().await.map_err(DbError::Query)?;
    Ok(())
}

async fn persist_account(
    tx: &mut Transaction<'_, Sqlite>,
    connection_id: i64,
    taken_at: Timestamp,
    fetched: FetchedAccount,
) -> Result<(), DbError> {
    let account = accounts::upsert(
        &mut *tx,
        connection_id,
        &fetched.external_id,
        &fetched.currency,
    )
    .await?;

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
    positions::replace_for_account(&mut *tx, account.id, &position_drafts).await?;

    // Positions in a currency other than the account's own aren't converted yet (planned:
    // FX conversion for multi-currency net worth), so summing them into the account's single
    // `total_value`/`currency` would silently add mismatched-currency numbers together.
    // Until FX conversion lands, the total only covers same-currency positions; anything else
    // is excluded (and logged) rather than folded into a wrong number.
    let total_value: f64 = fetched
        .positions
        .iter()
        .filter(|position| {
            let matches = position.native_currency == account.currency;
            if !matches {
                tracing::warn!(
                    account = %account.name,
                    symbol = %position.symbol,
                    position_currency = %position.native_currency,
                    account_currency = %account.currency,
                    "excluding a position from the account total: currency doesn't match \
                     the account's (FX conversion isn't implemented yet)",
                );
            }
            matches
        })
        .map(|position| position.current_value)
        .sum();
    snapshots::insert(
        &mut *tx,
        account.id,
        taken_at,
        total_value,
        &account.currency,
    )
    .await?;

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
    transactions::insert_new_for_account(&mut *tx, account.id, &transaction_drafts).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::refresh_connection;
    use crate::db::connections::{self, ConnectionDraft};
    use crate::db::{accounts, snapshots, test_support};

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
                },
                {
                    "contractDesc": "ASML",
                    "position": 2.0,
                    "avgCost": 600.0,
                    "mktValue": 1300.0,
                    "currency": "EUR"
                }
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/iserver/account/trades"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;
        let (_dir, pool) = test_support::pool().await;
        let connection = test_support::connection_with_credentials(&pool, &server.uri()).await;

        refresh_connection(&pool, &reqwest::Client::new(), &connection)
            .await
            .expect("refresh");

        let stored_accounts = accounts::list(&pool).await.expect("list accounts");
        assert_eq!(stored_accounts.len(), 1);
        assert_eq!(stored_accounts[0].external_id, "U1234567");
        assert_eq!(stored_accounts[0].name, "U1234567");

        let snapshot = snapshots::latest_for_account(&pool, stored_accounts[0].id)
            .await
            .expect("latest snapshot")
            .expect("a snapshot should have been written");
        // Only the USD position (1800.0) counts: the EUR position is excluded because FX
        // conversion isn't implemented yet (see the comment in persist_account).
        assert_eq!(snapshot.total_value, 1800.0);
        assert_eq!(snapshot.currency, "USD");

        let refreshed = connections::list(&pool).await.expect("list connections");
        assert!(refreshed[0].last_sync.is_some());
    }

    #[tokio::test]
    async fn a_failed_fetch_leaves_previously_persisted_data_untouched() {
        let server = MockServer::start().await;
        // No mocks mounted: every request gets wiremock's default 404.
        let (_dir, pool) = test_support::pool().await;
        let connection = test_support::connection_with_credentials(&pool, &server.uri()).await;

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
    async fn a_failure_on_the_second_account_rolls_back_the_first_accounts_writes_too() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/portfolio/accounts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "accountId": "U1111111", "currency": "USD" },
                { "accountId": "U2222222", "currency": "USD" }
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/portfolio/U1111111/positions/0"))
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
        // No mock for U2222222's positions: that request 404s, failing the whole refresh.
        Mock::given(method("GET"))
            .and(path("/iserver/account/trades"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;
        let (_dir, pool) = test_support::pool().await;
        let connection = test_support::connection_with_credentials(&pool, &server.uri()).await;

        let result = refresh_connection(&pool, &reqwest::Client::new(), &connection).await;

        assert!(result.is_err());
        // The first account's positions/snapshot must not have been committed either: the
        // whole refresh is one transaction.
        assert_eq!(
            accounts::list(&pool).await.expect("list accounts"),
            Vec::new()
        );
        let refreshed = connections::list(&pool).await.expect("list connections");
        assert_eq!(refreshed[0].last_sync, None);
    }

    #[tokio::test]
    async fn an_unknown_connector_type_fails_without_a_request() {
        let (_dir, pool) = test_support::pool().await;
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
