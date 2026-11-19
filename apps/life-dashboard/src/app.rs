//! Top-level `eframe::App`: owns the background tokio runtime, opens the database once at
//! startup, then hands off to the Settings screen.

use std::path::PathBuf;

use anyhow::Context as _;
use eframe::egui;
use poll_promise::Promise;
use sqlx::SqlitePool;
use tokio::runtime::Runtime;

use crate::dashboard::DashboardScreen;
use crate::db::preferences::Theme;
use crate::db::{self, DbError, connections, preferences};
use crate::settings::SettingsScreen;

struct Loaded {
    pool: SqlitePool,
    connections: Vec<connections::Connection>,
    preferences: preferences::Preferences,
}

async fn open_and_load(path: PathBuf) -> Result<Loaded, DbError> {
    let pool = db::connect(&path).await?;
    let connections = connections::list(&pool).await?;
    let preferences = preferences::load(&pool).await?;
    Ok(Loaded {
        pool,
        connections,
        preferences,
    })
}

enum State {
    Connecting(Promise<Result<Loaded, DbError>>),
    Ready(Box<Shell>),
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Dashboard,
    Settings,
}

/// The two top-level screens, once the database is open: Dashboard (read-only, local data)
/// and Settings (Connections + Preferences).
struct Shell {
    screen: Screen,
    dashboard: DashboardScreen,
    settings: SettingsScreen,
}

impl Shell {
    fn new(
        pool: SqlitePool,
        connections: Vec<connections::Connection>,
        preferences: preferences::Preferences,
    ) -> Self {
        Self {
            screen: Screen::Dashboard,
            dashboard: DashboardScreen::new(pool.clone()),
            settings: SettingsScreen::new(pool, connections, preferences),
        }
    }

    fn theme(&self) -> Theme {
        self.settings.theme()
    }

    /// Resolves any in-flight background work, regardless of which screen is currently
    /// showing. Settings' `busy` flag and error banner come from promises spawned by its own
    /// UI (e.g. a connection refresh); those must keep resolving even while the user is
    /// looking at the Dashboard tab, or a refresh's result/error would stay invisible and its
    /// buttons stuck disabled until the user switches back to Settings. A completed refresh
    /// also invalidates the Dashboard's cached data, so switching to it shows the new numbers
    /// instead of what was on disk before the refresh.
    fn poll(&mut self) {
        if self.settings.poll() {
            self.dashboard.invalidate();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, handle: &tokio::runtime::Handle) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.screen, Screen::Dashboard, "Dashboard");
            ui.selectable_value(&mut self.screen, Screen::Settings, "Settings");
        });
        ui.separator();

        match self.screen {
            Screen::Dashboard => self.dashboard.ui(ui, handle),
            Screen::Settings => self.settings.ui(ui, handle),
        }
    }
}

/// The `eframe::App` implementation: a thin shell around [`SettingsScreen`] for now.
pub struct App {
    runtime: Runtime,
    state: State,
}

impl App {
    /// Starts opening the database at `db_path` in the background and returns immediately;
    /// the window shows a loading state until it's ready.
    pub fn new(db_path: PathBuf) -> anyhow::Result<Self> {
        let runtime = Runtime::new().context("failed to start the background tokio runtime")?;
        let promise = {
            let _guard = runtime.enter();
            Promise::spawn_async(open_and_load(db_path))
        };
        Ok(Self {
            runtime,
            state: State::Connecting(promise),
        })
    }
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").finish_non_exhaustive()
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state = match std::mem::replace(&mut self.state, State::Failed(String::new())) {
            State::Connecting(promise) => match promise.try_take() {
                Ok(Ok(loaded)) => State::Ready(Box::new(Shell::new(
                    loaded.pool,
                    loaded.connections,
                    loaded.preferences,
                ))),
                Ok(Err(err)) => {
                    tracing::error!(%err, "failed to open the database");
                    State::Failed(err.to_string())
                }
                Err(promise) => {
                    ctx.request_repaint();
                    State::Connecting(promise)
                }
            },
            other => other,
        };

        if let State::Ready(shell) = &mut self.state {
            shell.poll();
            match shell.theme() {
                Theme::Light => ctx.set_visuals(egui::Visuals::light()),
                Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
                Theme::System => {}
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| match &mut self.state {
            State::Connecting(_) => {
                ui.label("Opening database…");
            }
            State::Failed(message) => {
                ui.colored_label(egui::Color32::RED, message.as_str());
            }
            State::Ready(shell) => shell.ui(ui, self.runtime.handle()),
        });
    }
}
