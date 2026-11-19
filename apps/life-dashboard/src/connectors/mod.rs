//! The connector contract: what it takes to fetch an institution's current accounts,
//! positions and recent transactions, and have them normalized into a shape [`crate::sync`]
//! can persist. [`interactive_brokers`] is the first (and so far only) implementation; the
//! aggregator-tier and CSV connectors planned next follow the same shape.

pub mod interactive_brokers;

use jiff::Timestamp;

use crate::db::connections::Connection;

/// Which connector a connection's `connector_type` string names. Parsed once at the top of
/// [`crate::sync::refresh_connection`] so the match that picks a connector is exhaustive over
/// every connector that exists, instead of comparing strings ad hoc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectorType {
    /// `interactive_brokers::InteractiveBrokersConnector`.
    InteractiveBrokers,
    /// A `connector_type` value that doesn't match any known connector.
    Unknown(String),
}

impl ConnectorType {
    /// Every connector this app implements, for populating a selection list. Does not include
    /// [`Self::Unknown`], which only arises from parsing a string that doesn't match one of
    /// these.
    pub const KNOWN: &'static [ConnectorType] = &[Self::InteractiveBrokers];

    /// Parses a connection's `connector_type` field.
    pub fn parse(raw: &str) -> Self {
        match raw {
            "interactive-brokers" => Self::InteractiveBrokers,
            other => Self::Unknown(other.to_owned()),
        }
    }

    /// The raw `connector_type` string this variant parses back from, i.e. the inverse of
    /// [`Self::parse`].
    pub fn as_str(&self) -> &str {
        match self {
            Self::InteractiveBrokers => "interactive-brokers",
            Self::Unknown(raw) => raw,
        }
    }

    /// A human-readable label for this connector, for display in the GUI.
    pub fn label(&self) -> &str {
        match self {
            Self::InteractiveBrokers => "Interactive Brokers",
            Self::Unknown(raw) => raw,
        }
    }

    /// Whether this connector's `credentials` field holds an actual secret, as opposed to
    /// non-sensitive configuration (e.g. a local gateway URL). The Settings screen uses this
    /// to decide whether to mask the field: IBKR's own gateway handles authentication via the
    /// user's browser session, so its `credentials` is just a URL, not something to hide.
    pub fn credentials_are_secret(&self) -> bool {
        match self {
            Self::InteractiveBrokers => false,
            // Unknown to this app (either not implemented yet, or a typo) — assume the
            // worst and mask it, rather than risk showing a real secret in the clear.
            Self::Unknown(_) => true,
        }
    }

    /// A short explanation of what setting up this connector requires, for display as a
    /// tooltip next to the type selector on the Settings screen.
    pub fn setup_help(&self) -> &str {
        match self {
            Self::InteractiveBrokers => {
                "Requires IBKR's Client Portal Gateway running locally and logged in via \
                 your browser. Enter its base URL as the credentials, e.g. \
                 https://localhost:5000/v1/api"
            }
            Self::Unknown(_) => "Not a connector this app implements yet.",
        }
    }

    /// The credentials value to prefill when this connector type is picked and the field is
    /// still empty, for connectors whose "credentials" is really just a well-known default
    /// (e.g. IBKR's local gateway URL) rather than a secret the user must supply themselves.
    pub fn default_credentials(&self) -> Option<&str> {
        match self {
            Self::InteractiveBrokers => Some("https://localhost:5000/v1/api"),
            Self::Unknown(_) => None,
        }
    }
}

/// One account as reported by a connector, with its positions and recent transactions,
/// before it's matched against (or inserted into) the local `accounts` table.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchedAccount {
    /// The connector's own identifier for this account (e.g. an IBKR account id), matched
    /// against [`crate::db::accounts::Account::external_id`] so repeated refreshes line up
    /// with the same local row.
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

#[cfg(test)]
mod tests {
    use super::ConnectorType;

    #[test]
    fn parses_the_known_connector_type() {
        assert_eq!(
            ConnectorType::parse("interactive-brokers"),
            ConnectorType::InteractiveBrokers
        );
    }

    #[test]
    fn an_unrecognized_connector_type_is_unknown() {
        assert_eq!(
            ConnectorType::parse("mystery-broker"),
            ConnectorType::Unknown("mystery-broker".to_owned())
        );
    }

    #[test]
    fn interactive_brokers_credentials_are_not_a_secret() {
        assert!(!ConnectorType::InteractiveBrokers.credentials_are_secret());
    }

    #[test]
    fn an_unknown_connector_type_is_treated_as_having_secret_credentials() {
        assert!(ConnectorType::Unknown("mystery-broker".to_owned()).credentials_are_secret());
    }
}
