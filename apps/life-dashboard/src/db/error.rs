//! Typed error for the persistence layer (module boundary callers match on).

use thiserror::Error;

/// Everything that can go wrong opening the database or running a query against it.
#[derive(Debug, Error)]
pub enum DbError {
    /// Failed to open the SQLite file or establish a pool connection.
    #[error("failed to connect to the database")]
    Connect(#[source] sqlx::Error),
    /// A migration failed to apply.
    #[error("failed to run database migrations")]
    Migrate(#[source] sqlx::migrate::MigrateError),
    /// A query, insert, update or delete failed.
    #[error("database query failed")]
    Query(#[source] sqlx::Error),
    /// A timestamp stored in the database could not be parsed back.
    #[error("stored timestamp {raw:?} is not valid RFC 3339")]
    InvalidTimestamp {
        /// The raw value read from the database.
        raw: String,
        /// Why `jiff` rejected it.
        #[source]
        source: jiff::Error,
    },
}
