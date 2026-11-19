//! Load/save for the singleton `preferences` row.

use sqlx::SqlitePool;

use super::DbError;

/// Visual theme preference. `System` follows the OS; the others pin one mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Follow the OS setting.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

impl Theme {
    const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn parse(raw: &str) -> Self {
        match raw {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }
}

/// App-wide preferences, persisted across restarts.
#[derive(Debug, Clone, PartialEq)]
pub struct Preferences {
    /// ISO 4217 code totals are shown in.
    pub display_currency: String,
    /// Visual theme.
    pub theme: Theme,
}

/// Loads the current preferences. The migration seeds the singleton row, so this never
/// needs to invent a default.
pub async fn load(pool: &SqlitePool) -> Result<Preferences, DbError> {
    let (display_currency, theme): (String, String) =
        sqlx::query_as("SELECT display_currency, theme FROM preferences WHERE id = 0")
            .fetch_one(pool)
            .await
            .map_err(DbError::Query)?;
    Ok(Preferences {
        display_currency,
        theme: Theme::parse(&theme),
    })
}

/// Persists new preferences over the singleton row.
pub async fn save(pool: &SqlitePool, preferences: &Preferences) -> Result<(), DbError> {
    sqlx::query("UPDATE preferences SET display_currency = ?, theme = ? WHERE id = 0")
        .bind(&preferences.display_currency)
        .bind(preferences.theme.as_str())
        .execute(pool)
        .await
        .map_err(DbError::Query)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Preferences, Theme, load, save};
    use crate::db::connect;

    #[tokio::test]
    async fn freshly_migrated_database_has_sensible_defaults() {
        let dir = tempfile::tempdir().expect("creating a temp dir cannot fail in CI");
        let pool = connect(&dir.path().join("db.sqlite3"))
            .await
            .expect("connect");

        let preferences = load(&pool).await.expect("load");

        assert_eq!(preferences.display_currency, "EUR");
        assert_eq!(preferences.theme, Theme::System);
    }

    #[tokio::test]
    async fn saved_preferences_round_trip() {
        let dir = tempfile::tempdir().expect("creating a temp dir cannot fail in CI");
        let pool = connect(&dir.path().join("db.sqlite3"))
            .await
            .expect("connect");
        let updated = Preferences {
            display_currency: "USD".to_owned(),
            theme: Theme::Dark,
        };

        save(&pool, &updated).await.expect("save");

        assert_eq!(load(&pool).await.expect("load"), updated);
    }
}
