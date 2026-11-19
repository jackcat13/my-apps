//! Interactive Brokers connector: talks to the user's locally running IBKR Client Portal
//! Gateway over its REST API. The gateway itself handles authentication through the user's
//! browser session, so this connector only calls already-authenticated endpoints — the
//! [`Connection::credentials`] field holds the gateway's base URL (e.g.
//! `https://localhost:5000/v1/api`), not a secret.

use jiff::Timestamp;
use serde::Deserialize;

use super::{Connector, ConnectorError, FetchedAccount, FetchedPosition, FetchedTransaction};
use crate::db::connections::Connection;

#[derive(Debug, Deserialize)]
struct AccountSummary {
    #[serde(rename = "accountId")]
    account_id: String,
    currency: String,
}

#[derive(Debug, Deserialize)]
struct PositionEntry {
    #[serde(rename = "contractDesc")]
    contract_desc: String,
    position: f64,
    #[serde(rename = "avgCost")]
    avg_cost: f64,
    #[serde(rename = "mktValue")]
    mkt_value: f64,
    currency: String,
}

#[derive(Debug, Deserialize)]
struct TradeEntry {
    account: String,
    symbol: String,
    side: String,
    #[serde(rename = "netAmount")]
    net_amount: f64,
    /// Execution time, as milliseconds since the Unix epoch.
    #[serde(rename = "tradeTimeR")]
    trade_time_r: i64,
}

/// Fetches accounts, positions and recent trades from an IBKR Client Portal Gateway.
#[derive(Debug, Clone, Copy, Default)]
pub struct InteractiveBrokersConnector;

impl Connector for InteractiveBrokersConnector {
    async fn fetch(
        &self,
        client: &reqwest::Client,
        connection: &Connection,
    ) -> Result<Vec<FetchedAccount>, ConnectorError> {
        let base_url = connection.credentials.trim().trim_end_matches('/');
        if base_url.is_empty() {
            return Err(ConnectorError::MissingCredential { field: "base_url" });
        }

        let accounts: Vec<AccountSummary> =
            get_json(client, &format!("{base_url}/portfolio/accounts")).await?;
        let trades: Vec<TradeEntry> =
            get_json(client, &format!("{base_url}/iserver/account/trades")).await?;

        let mut fetched = Vec::with_capacity(accounts.len());
        for account in accounts {
            let positions: Vec<PositionEntry> = get_json(
                client,
                &format!("{base_url}/portfolio/{}/positions/0", account.account_id),
            )
            .await?;

            let positions = positions
                .into_iter()
                .map(|position| FetchedPosition {
                    symbol: position.contract_desc,
                    quantity: position.position,
                    cost_basis: position.avg_cost,
                    current_value: position.mkt_value,
                    native_currency: position.currency,
                })
                .collect();

            let transactions = trades
                .iter()
                .filter(|trade| trade.account == account.account_id)
                .filter_map(|trade| {
                    let occurred_at = Timestamp::from_millisecond(trade.trade_time_r).ok()?;
                    Some(FetchedTransaction {
                        occurred_at,
                        kind: trade.side.clone(),
                        amount: trade.net_amount,
                        asset: Some(trade.symbol.clone()),
                    })
                })
                .collect();

            fetched.push(FetchedAccount {
                external_id: account.account_id,
                currency: account.currency,
                positions,
                transactions,
            });
        }

        Ok(fetched)
    }
}

async fn get_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
) -> Result<T, ConnectorError> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|source| ConnectorError::Request {
            url: url.to_owned(),
            source,
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(ConnectorError::Status {
            url: url.to_owned(),
            status,
        });
    }

    response
        .json::<T>()
        .await
        .map_err(|source| ConnectorError::Decode {
            url: url.to_owned(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::InteractiveBrokersConnector;
    use crate::connectors::Connector;
    use crate::db::connections::Connection;

    fn connection(base_url: String) -> Connection {
        Connection {
            id: 1,
            name: "Interactive Brokers".to_owned(),
            connector_type: "interactive-brokers".to_owned(),
            credentials: base_url,
            last_sync: None,
        }
    }

    #[tokio::test]
    async fn an_empty_base_url_fails_without_a_request() {
        let result = InteractiveBrokersConnector
            .fetch(&reqwest::Client::new(), &connection(String::new()))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn a_missing_endpoint_surfaces_as_a_status_error() {
        let server = MockServer::start().await;

        let result = InteractiveBrokersConnector
            .fetch(&reqwest::Client::new(), &connection(server.uri()))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn fetches_accounts_with_their_positions_and_matching_trades() {
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
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "account": "U1234567",
                    "symbol": "AAPL",
                    "side": "BUY",
                    "netAmount": 1500.0,
                    "tradeTimeR": 1_700_000_000_000_i64
                },
                {
                    "account": "U9999999",
                    "symbol": "MSFT",
                    "side": "SELL",
                    "netAmount": 500.0,
                    "tradeTimeR": 1_700_000_000_000_i64
                }
            ])))
            .mount(&server)
            .await;

        let fetched = InteractiveBrokersConnector
            .fetch(&reqwest::Client::new(), &connection(server.uri()))
            .await
            .expect("fetch");

        assert_eq!(fetched.len(), 1);
        let account = &fetched[0];
        assert_eq!(account.external_id, "U1234567");
        assert_eq!(account.positions.len(), 1);
        assert_eq!(account.positions[0].symbol, "AAPL");
        assert_eq!(account.transactions.len(), 1);
        assert_eq!(account.transactions[0].asset, Some("AAPL".to_owned()));
    }
}
