//! Powens connector: the aggregator-tier connector, talking to a Powens (formerly Budget
//! Insight/Budgea) open-banking account on behalf of banks that don't offer their own personal
//! API — the Bridge/Powens/TrueLayer-class PSD2 aggregator this app uses for them.
//!
//! One Powens user can have several banks linked (each a "connection" in Powens' own
//! vocabulary, distinct from this app's [`Connection`]); [`Credentials::connection_id`] scopes
//! which linked bank's accounts a given [`PowensConnector`] fetch pulls. The same connector
//! type is reused for La Banque Postale and Revolut by creating one app connection per bank,
//! each with its own `base_url`/`access_token`/`connection_id` — exactly the "one connector
//! type reused for three bank IDs" shape this app's aggregator tier is meant to have.
//!
//! Revolut has no personal API of its own and must go through an AISP aggregator like this
//! one; its AIS consent only grants a 5-minute access window after each re-consent (tightened
//! November 2024), so in practice expect `fetch` to start failing with 401s between
//! re-consents — there's no way to detect or refresh that from this connector alone, short of
//! the user re-authenticating through Powens' webview each time.
//!
//! UBS isn't wired here at all: it has no presence on EU PSD2-class aggregators like Powens.
//! UBS's own multibanking route is SIX bLink, launched 2025-11-25 and Switzerland-only, and as
//! of writing no accessible bLink aggregator has confirmed individual/hobby-scale access. Until
//! one does, UBS falls back to the CSV connector instead.

use jiff::Timestamp;
use serde::Deserialize;

use super::{Connector, ConnectorError, FetchedAccount, FetchedPosition, FetchedTransaction};
use crate::db::connections::Connection;

/// Parsed form of [`Connection::credentials`] for this connector: a JSON object, since (unlike
/// IBKR's bare gateway URL) talking to Powens needs more than one piece of configuration.
#[derive(Debug, Deserialize)]
struct Credentials {
    /// Base URL of the Powens domain, e.g. `https://your-domain.biapi.pro/2.0`.
    base_url: String,
    /// The end user's Powens access token, obtained through Powens' own webview/OAuth connect
    /// flow (outside this app) when the user first links a bank.
    access_token: String,
    /// Which of the Powens user's linked banks ("connections" in Powens' own vocabulary) to
    /// pull accounts for. Omit to pull every linked bank under this Powens user.
    #[serde(default)]
    connection_id: Option<i64>,
}

impl Credentials {
    fn parse(raw: &str) -> Result<Self, ConnectorError> {
        if raw.trim().is_empty() {
            return Err(ConnectorError::MissingCredential {
                field: "credentials",
            });
        }
        serde_json::from_str(raw).map_err(|_| ConnectorError::MissingCredential {
            field: "credentials (expected JSON: \
                    {\"base_url\":...,\"access_token\":...,\"connection_id\":...})",
        })
    }
}

#[derive(Debug, Deserialize)]
struct AccountsResponse {
    accounts: Vec<AccountEntry>,
}

#[derive(Debug, Deserialize)]
struct AccountEntry {
    id: i64,
    #[serde(default)]
    id_connection: Option<i64>,
    balance: f64,
    currency: AccountCurrency,
}

#[derive(Debug, Deserialize)]
struct AccountCurrency {
    id: String,
}

#[derive(Debug, Deserialize)]
struct TransactionsResponse {
    transactions: Vec<TransactionEntry>,
}

#[derive(Debug, Deserialize)]
struct TransactionEntry {
    /// Civil date the transaction was booked on, as `YYYY-MM-DD` (Powens reports no
    /// time-of-day for bank transactions).
    date: String,
    value: f64,
    wording: String,
}

/// Fetches accounts and transactions from a Powens (or Powens-compatible) open-banking
/// aggregator account.
#[derive(Debug, Clone, Copy, Default)]
pub struct PowensConnector;

impl Connector for PowensConnector {
    async fn fetch(
        &self,
        client: &reqwest::Client,
        connection: &Connection,
    ) -> Result<Vec<FetchedAccount>, ConnectorError> {
        let credentials = Credentials::parse(&connection.credentials)?;
        let base_url = credentials.base_url.trim().trim_end_matches('/');

        let accounts: AccountsResponse = get_json(
            client,
            &format!("{base_url}/users/me/accounts"),
            &credentials.access_token,
        )
        .await?;

        let mut fetched = Vec::new();
        for account in accounts.accounts {
            if let Some(wanted) = credentials.connection_id
                && account.id_connection != Some(wanted)
            {
                continue;
            }

            let transactions: TransactionsResponse = get_json(
                client,
                &format!("{base_url}/users/me/accounts/{}/transactions", account.id),
                &credentials.access_token,
            )
            .await?;

            let transactions = transactions
                .transactions
                .into_iter()
                .filter_map(
                    |transaction| match booked_date_to_timestamp(&transaction.date) {
                        Ok(occurred_at) => Some(FetchedTransaction {
                            occurred_at,
                            kind: transaction.wording,
                            amount: transaction.value,
                            asset: None,
                        }),
                        Err(err) => {
                            tracing::warn!(
                                account = account.id,
                                date = %transaction.date,
                                %err,
                                "dropping a transaction with an unparsable date",
                            );
                            None
                        }
                    },
                )
                .collect();

            // A bank account has no "positions" in the brokerage sense; its balance is
            // represented as a single synthetic cash position so the existing
            // positions/snapshot pipeline (built for IBKR's brokerage holdings) still totals
            // it correctly without needing a schema change.
            let positions = vec![FetchedPosition {
                symbol: "Cash".to_owned(),
                quantity: 1.0,
                cost_basis: account.balance,
                current_value: account.balance,
                native_currency: account.currency.id.clone(),
            }];

            fetched.push(FetchedAccount {
                external_id: account.id.to_string(),
                currency: account.currency.id,
                positions,
                transactions,
            });
        }

        Ok(fetched)
    }
}

/// Converts a `YYYY-MM-DD` booking date into a [`Timestamp`], treating it as midnight UTC —
/// consistent with how a date-only value is wholly represented, since Powens reports no
/// time-of-day for bank transactions.
fn booked_date_to_timestamp(date: &str) -> Result<Timestamp, jiff::Error> {
    Ok(date.parse::<jiff::civil::Date>()?.in_tz("UTC")?.timestamp())
}

async fn get_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    access_token: &str,
) -> Result<T, ConnectorError> {
    let response = client
        .get(url)
        .bearer_auth(access_token)
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
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::PowensConnector;
    use crate::connectors::Connector;
    use crate::db::connections::Connection;

    fn connection(credentials: String) -> Connection {
        Connection {
            id: 1,
            name: "La Banque Postale".to_owned(),
            connector_type: "powens".to_owned(),
            credentials,
            last_sync: None,
        }
    }

    #[tokio::test]
    async fn empty_credentials_fail_without_a_request() {
        let result = PowensConnector
            .fetch(&reqwest::Client::new(), &connection(String::new()))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn credentials_that_are_not_json_fail_without_a_request() {
        let result = PowensConnector
            .fetch(&reqwest::Client::new(), &connection("not json".to_owned()))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn a_missing_endpoint_surfaces_as_a_status_error() {
        let server = MockServer::start().await;
        let credentials = format!(
            r#"{{"base_url":"{}","access_token":"test-token"}}"#,
            server.uri()
        );

        let result = PowensConnector
            .fetch(&reqwest::Client::new(), &connection(credentials))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn fetches_every_linked_bank_when_no_connection_id_is_given() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/me/accounts"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "accounts": [
                    { "id": 1, "id_connection": 45, "balance": 1234.56, "currency": { "id": "EUR" } },
                    { "id": 2, "id_connection": 99, "balance": 50.0, "currency": { "id": "EUR" } }
                ]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/users/me/accounts/1/transactions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "transactions": [
                    { "date": "2026-01-15", "value": -45.0, "wording": "SNCF" }
                ]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/users/me/accounts/2/transactions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "transactions": []
            })))
            .mount(&server)
            .await;
        let credentials = format!(
            r#"{{"base_url":"{}","access_token":"test-token"}}"#,
            server.uri()
        );

        let fetched = PowensConnector
            .fetch(&reqwest::Client::new(), &connection(credentials))
            .await
            .expect("fetch");

        assert_eq!(fetched.len(), 2);
        let first = &fetched[0];
        assert_eq!(first.external_id, "1");
        assert_eq!(first.currency, "EUR");
        assert_eq!(first.positions.len(), 1);
        assert_eq!(first.positions[0].current_value, 1234.56);
        assert_eq!(first.transactions.len(), 1);
        assert_eq!(first.transactions[0].amount, -45.0);
    }

    #[tokio::test]
    async fn a_connection_id_scopes_to_only_that_linked_bank() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/me/accounts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "accounts": [
                    { "id": 1, "id_connection": 45, "balance": 1234.56, "currency": { "id": "EUR" } },
                    { "id": 2, "id_connection": 99, "balance": 50.0, "currency": { "id": "EUR" } }
                ]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/users/me/accounts/2/transactions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "transactions": []
            })))
            .mount(&server)
            .await;
        let credentials = format!(
            r#"{{"base_url":"{}","access_token":"test-token","connection_id":99}}"#,
            server.uri()
        );

        let fetched = PowensConnector
            .fetch(&reqwest::Client::new(), &connection(credentials))
            .await
            .expect("fetch");

        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].external_id, "2");
    }

    #[tokio::test]
    async fn a_transaction_with_an_unparsable_date_is_dropped_not_fatal() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/me/accounts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "accounts": [
                    { "id": 1, "id_connection": 45, "balance": 10.0, "currency": { "id": "EUR" } }
                ]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/users/me/accounts/1/transactions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "transactions": [
                    { "date": "not-a-date", "value": -1.0, "wording": "broken" }
                ]
            })))
            .mount(&server)
            .await;
        let credentials = format!(
            r#"{{"base_url":"{}","access_token":"test-token"}}"#,
            server.uri()
        );

        let fetched = PowensConnector
            .fetch(&reqwest::Client::new(), &connection(credentials))
            .await
            .expect("fetch");

        assert_eq!(fetched[0].transactions.len(), 0);
    }
}
