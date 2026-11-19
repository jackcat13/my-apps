//! Settings screen: Connections (CRUD against the local DB) and Preferences (persisted
//! display currency + theme). Every mutation is fired on the app's tokio runtime and the
//! screen is refreshed from a [`poll_promise::Promise`] once it resolves, so `ui()` never
//! blocks on the database (see `docs/stacks/gui.md`).

mod connections_tab;
mod preferences_tab;

use eframe::egui;
use poll_promise::Promise;
use sqlx::SqlitePool;
use tokio::runtime::Handle;

use crate::db::preferences::Theme;
use crate::db::{DbError, connections, preferences};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Connections,
    Preferences,
}

#[derive(Default)]
struct ConnectionForm {
    editing_id: Option<i64>,
    name: String,
    connector_type: String,
    credentials: String,
}

impl ConnectionForm {
    fn clear(&mut self) {
        *self = Self::default();
    }

    fn load(&mut self, connection: &connections::Connection) {
        self.editing_id = Some(connection.id);
        self.name.clone_from(&connection.name);
        self.connector_type.clone_from(&connection.connector_type);
        self.credentials.clone_from(&connection.credentials);
    }

    fn as_draft(&self) -> connections::ConnectionDraft {
        connections::ConnectionDraft {
            name: self.name.clone(),
            connector_type: self.connector_type.clone(),
            credentials: self.credentials.clone(),
        }
    }
}

/// The Settings screen's state: a connection pool plus whatever is in flight.
pub struct SettingsScreen {
    pool: SqlitePool,
    tab: Tab,
    connections: Vec<connections::Connection>,
    preferences: preferences::Preferences,
    form: ConnectionForm,
    busy: bool,
    last_error: Option<String>,
    reload: Option<Promise<Result<Vec<connections::Connection>, DbError>>>,
    save_preferences: Option<Promise<Result<preferences::Preferences, DbError>>>,
}

impl SettingsScreen {
    pub(crate) fn new(
        pool: SqlitePool,
        connections: Vec<connections::Connection>,
        preferences: preferences::Preferences,
    ) -> Self {
        Self {
            pool,
            tab: Tab::Connections,
            connections,
            preferences,
            form: ConnectionForm::default(),
            busy: false,
            last_error: None,
            reload: None,
            save_preferences: None,
        }
    }

    /// The currently persisted theme, so the caller can apply it to `egui::Context`.
    pub(crate) fn theme(&self) -> Theme {
        self.preferences.theme
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, handle: &Handle) {
        self.poll();

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Connections, "Connections");
            ui.selectable_value(&mut self.tab, Tab::Preferences, "Preferences");
        });
        ui.separator();

        if let Some(error) = self.last_error.clone() {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::RED, error);
                if ui.button("Dismiss").clicked() {
                    self.last_error = None;
                }
            });
            ui.separator();
        }

        match self.tab {
            Tab::Connections => connections_tab::show(self, ui, handle),
            Tab::Preferences => preferences_tab::show(self, ui, handle),
        }
    }

    fn poll(&mut self) {
        if let Some(promise) = self.reload.take() {
            match promise.try_take() {
                Ok(Ok(connections)) => {
                    self.connections = connections;
                    self.form.clear();
                    self.busy = false;
                }
                Ok(Err(err)) => self.fail(&err),
                Err(promise) => self.reload = Some(promise),
            }
        }
        if let Some(promise) = self.save_preferences.take() {
            match promise.try_take() {
                Ok(Ok(preferences)) => {
                    self.preferences = preferences;
                    self.busy = false;
                }
                Ok(Err(err)) => self.fail(&err),
                Err(promise) => self.save_preferences = Some(promise),
            }
        }
    }

    fn fail(&mut self, err: &DbError) {
        tracing::error!(%err, "settings operation failed");
        self.last_error = Some(err.to_string());
        self.busy = false;
    }

    fn spawn_create(&mut self, handle: &Handle) {
        let pool = self.pool.clone();
        let draft = self.form.as_draft();
        self.busy = true;
        let _guard = handle.enter();
        self.reload = Some(Promise::spawn_async(async move {
            connections::create(&pool, &draft).await?;
            connections::list(&pool).await
        }));
    }

    fn spawn_update(&mut self, handle: &Handle, id: i64) {
        let pool = self.pool.clone();
        let draft = self.form.as_draft();
        self.busy = true;
        let _guard = handle.enter();
        self.reload = Some(Promise::spawn_async(async move {
            connections::update(&pool, id, &draft).await?;
            connections::list(&pool).await
        }));
    }

    fn spawn_delete(&mut self, handle: &Handle, id: i64) {
        let pool = self.pool.clone();
        self.busy = true;
        let _guard = handle.enter();
        self.reload = Some(Promise::spawn_async(async move {
            connections::delete(&pool, id).await?;
            connections::list(&pool).await
        }));
    }

    fn spawn_save_preferences(&mut self, handle: &Handle, preferences: preferences::Preferences) {
        let pool = self.pool.clone();
        self.busy = true;
        let _guard = handle.enter();
        self.save_preferences = Some(Promise::spawn_async(async move {
            preferences::save(&pool, &preferences).await?;
            Ok(preferences)
        }));
    }
}
