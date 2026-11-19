//! Personal finance/stats dashboard

mod app;
mod db;
mod settings;

use std::path::PathBuf;

use anyhow::{Context, Result};
use etcetera::{AppStrategy, AppStrategyArgs, choose_app_strategy};

use app::App;

/// Runs the application: opens the main window and blocks until it closes.
pub fn run() -> Result<()> {
    tracing::info!("life-dashboard started");
    let db_path = resolve_db_path()?;
    let app = App::new(db_path)?;
    let options = eframe::NativeOptions::default();
    eframe::run_native("life-dashboard", options, Box::new(|_cc| Ok(Box::new(app))))
        .map_err(|err| anyhow::anyhow!(err.to_string()))
        .context("failed to run life-dashboard window")
}

/// Resolves (and creates) the per-OS data directory, then returns the path to the SQLite
/// file inside it.
fn resolve_db_path() -> Result<PathBuf> {
    let strategy = choose_app_strategy(AppStrategyArgs {
        top_level_domain: "dev".to_owned(),
        author: "jackcat13".to_owned(),
        app_name: "life-dashboard".to_owned(),
    })
    .context("could not resolve the OS data directory")?;
    let dir = strategy.data_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    Ok(dir.join("life-dashboard.sqlite3"))
}
