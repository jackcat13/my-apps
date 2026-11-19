//! Personal finance/stats dashboard

use anyhow::{Context, Result};
use eframe::egui;

/// Empty shell app window: no features yet, just the scaffold.
#[derive(Debug, Default)]
struct App;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("life-dashboard");
    }
}

/// Runs the application: opens the main window and blocks until it closes.
pub fn run() -> Result<()> {
    tracing::info!("life-dashboard started");
    let options = eframe::NativeOptions::default();
    eframe::run_native("life-dashboard", options, Box::new(|_cc| Ok(Box::new(App))))
        .map_err(|err| anyhow::anyhow!(err.to_string()))
        .context("failed to run life-dashboard window")
}
