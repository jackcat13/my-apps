//! Preferences tab: display currency and theme, saved as soon as they change.

use eframe::egui;
use tokio::runtime::Handle;

use super::SettingsScreen;
use crate::db::preferences::Theme;

const COMMON_CURRENCIES: [&str; 6] = ["EUR", "USD", "GBP", "CHF", "JPY", "CAD"];

pub(super) fn show(screen: &mut SettingsScreen, ui: &mut egui::Ui, handle: &Handle) {
    let mut next = screen.preferences.clone();

    ui.horizontal(|ui| {
        ui.label("Display currency");
        egui::ComboBox::from_id_salt("display_currency")
            .selected_text(&next.display_currency)
            .show_ui(ui, |ui| {
                for currency in COMMON_CURRENCIES {
                    ui.selectable_value(&mut next.display_currency, currency.to_owned(), currency);
                }
            });
    });

    ui.horizontal(|ui| {
        ui.label("Theme");
        ui.selectable_value(&mut next.theme, Theme::System, "System");
        ui.selectable_value(&mut next.theme, Theme::Light, "Light");
        ui.selectable_value(&mut next.theme, Theme::Dark, "Dark");
    });

    if next != screen.preferences && !screen.busy {
        screen.spawn_save_preferences(handle, next);
    }
}
