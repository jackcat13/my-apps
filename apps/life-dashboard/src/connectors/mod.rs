//! The connector contract: what it takes to fetch an institution's current accounts,
//! positions and recent transactions, and have them normalized into a shape [`crate::sync`]
//! can persist. [`interactive_brokers`] is the first (and so far only) implementation; the
//! aggregator-tier and CSV connectors planned next follow the same shape.

pub mod interactive_brokers;

use jiff::Timestamp;

use crate::db::connections::Connection;

/// One account as reported by a connector, with its positions and recent transactions,
/// before it's matched against (or inserted into) the local `accounts` table.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchedAccount {
    /// The connector's own identifier for this account (e.g. an IBKR account id), used as
    /// the account's `name` so repeated refreshes line up with the same local row.
    pub external_id: String,
    /// ISO 4217 currency code the account is denominated in.
    pub currency: String,
    /// The account's current holdings.
    pub positions: Vec<FetchedPosition>,
    /// Recent transactions for the account.
    pub transactions: Vec<FetchedTransaction>,
}

/// One position as reported by a connector.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchedPosition {
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

/// One transaction as reported by a connector.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchedTransaction {
    /// When the transaction occurred.
    pub occurred_at: Timestamp,
    /// Connector-reported kind, e.g. `"BUY"`/`"SELL"`.
    pub kind: String,
    /// Signed amount, in the account's currency.
    pub amount: f64,
    /// The asset this transaction concerns, if any (e.g. a ticker).
    pub asset: Option<String>,
}

/// Everything that can go wrong fetching from a connector's upstream API.
#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    /// The HTTP request itself failed (DNS, connection refused, timed out, …).
    #[error("request to {url} failed")]
    Request {
        /// The URL that was being requested.
        url: String,
        #[source]
        source: reqwest::Error,
    },
    /// The upstream API responded, but not with success.
    #[error("upstream returned {status} for {url}")]
    Status {
        /// The URL that was being requested.
        url: String,
        /// The HTTP status code the upstream returned.
        status: reqwest::StatusCode,
    },
    /// The response body could not be parsed into the shape this connector expects.
    #[error("could not parse the response from {url}")]
    Decode {
        /// The URL that was being requested.
        url: String,
        #[source]
        source: reqwest::Error,
    },
    /// The connection's `credentials` field is missing something this connector needs.
    #[error("connection is missing {field}")]
    MissingCredential {
        /// The name of the missing piece of configuration.
        field: &'static str,
    },
}

/// Fetches an institution's current accounts, positions and recent transactions. Each
/// connector reads whatever it needs out of [`Connection::credentials`] (the format is
/// connector-specific) and talks to that institution's own API.
pub trait Connector {
    /// Fetches `connection`'s current data from the upstream API.
    fn fetch(
        &self,
        client: &reqwest::Client,
        connection: &Connection,
    ) -> impl Future<Output = Result<Vec<FetchedAccount>, ConnectorError>> + Send;
}
