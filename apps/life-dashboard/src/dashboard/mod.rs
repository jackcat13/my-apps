//! Dashboard screen: a read-only view of each account's latest snapshot and current
//! positions, loaded from the local database (never the network) so the app has something
//! to show even with no connectivity at all.

use eframe::egui;
use poll_promise::Promise;
use sqlx::SqlitePool;
use tokio::runtime::Handle;

use crate::db::{DbError, accounts, positions, snapshots, transactions};

const RECENT_TRANSACTIONS_SHOWN: usize = 5;

struct AccountView {
    account: accounts::Account,
    positions: Vec<positions::Position>,
    snapshot: Option<snapshots::Snapshot>,
    recent_transactions: Vec<transactions::Transaction>,
}

async fn load(pool: SqlitePool) -> Result<Vec<AccountView>, DbError> {
    let accounts = accounts::list(&pool).await?;
    let mut views = Vec::with_capacity(accounts.len());
    for account in accounts {
        let account_positions = positions::list_for_account(&pool, account.id).await?;
        let snapshot = snapshots::latest_for_account(&pool, account.id).await?;
        let mut recent_transactions = transactions::list_for_account(&pool, account.id).await?;
        recent_transactions.truncate(RECENT_TRANSACTIONS_SHOWN);
        views.push(AccountView {
            account,
            positions: account_positions,
            snapshot,
            recent_transactions,
        });
    }
    Ok(views)
}

/// The Dashboard screen's state: the local pool plus whatever load is in flight.
pub struct DashboardScreen {
    pool: SqlitePool,
    accounts: Vec<AccountView>,
    loaded_once: bool,
    last_error: Option<String>,
    load: Option<Promise<Result<Vec<AccountView>, DbError>>>,
}

impl DashboardScreen {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            accounts: Vec::new(),
            loaded_once: false,
            last_error: None,
            load: None,
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, handle: &Handle) {
        self.poll();
        if !self.loaded_once && self.load.is_none() {
            self.spawn_load(handle);
        }

        if let Some(error) = self.last_error.clone() {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::RED, &error);
                if ui.button("Dismiss").clicked() {
                    self.last_error = None;
                }
            });
            ui.separator();
        }

        if ui.button("Reload").clicked() {
            self.spawn_load(handle);
        }
        ui.separator();

        if self.accounts.is_empty() {
            ui.label("No accounts yet. Add a connection in Settings and refresh it.");
            return;
        }

        for view in &self.accounts {
            show_account(ui, view);
        }
    }

    fn poll(&mut self) {
        let Some(promise) = self.load.take() else {
            return;
        };
        match promise.try_take() {
            Ok(Ok(accounts)) => {
                self.accounts = accounts;
                self.loaded_once = true;
            }
            Ok(Err(err)) => {
                tracing::error!(%err, "failed to load dashboard data");
                self.last_error = Some(err.to_string());
                self.loaded_once = true;
            }
            Err(promise) => self.load = Some(promise),
        }
    }

    fn spawn_load(&mut self, handle: &Handle) {
        let pool = self.pool.clone();
        let _guard = handle.enter();
        self.load = Some(Promise::spawn_async(load(pool)));
    }
}

fn show_account(ui: &mut egui::Ui, view: &AccountView) {
    ui.strong(&view.account.name);
    let total = view.snapshot.as_ref().map_or_else(
        || "no snapshot yet".to_owned(),
        |snapshot| format!("{:.2} {}", snapshot.total_value, snapshot.currency),
    );
    ui.label(format!("Total: {total}"));

    egui::Grid::new(("dashboard_positions", view.account.id))
        .num_columns(3)
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Symbol");
            ui.strong("Quantity");
            ui.strong("Value");
            ui.end_row();
            for position in &view.positions {
                ui.label(&position.symbol);
                ui.label(format!("{:.4}", position.quantity));
                ui.label(format!(
                    "{:.2} {}",
                    position.current_value, position.native_currency
                ));
                ui.end_row();
            }
        });

    if !view.recent_transactions.is_empty() {
        ui.label("Recent transactions:");
        egui::Grid::new(("dashboard_transactions", view.account.id))
            .num_columns(3)
            .striped(true)
            .show(ui, |ui| {
                for transaction in &view.recent_transactions {
                    ui.label(transaction.occurred_at.to_string());
                    ui.label(&transaction.kind);
                    ui.label(format!(
                        "{:.2} {}",
                        transaction.amount,
                        transaction.asset.as_deref().unwrap_or("-")
                    ));
                    ui.end_row();
                }
            });
    }
    ui.separator();
}
