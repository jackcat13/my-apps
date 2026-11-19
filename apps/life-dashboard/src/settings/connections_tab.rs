//! Connections tab: list, add, edit and delete institution connections.

use eframe::egui;
use tokio::runtime::Handle;

use super::SettingsScreen;
use crate::connectors::ConnectorType;

pub(super) fn show(screen: &mut SettingsScreen, ui: &mut egui::Ui, handle: &Handle) {
    show_list(screen, ui, handle);
    ui.separator();
    show_form(screen, ui, handle);
}

fn show_list(screen: &mut SettingsScreen, ui: &mut egui::Ui, handle: &Handle) {
    if screen.connections.is_empty() {
        ui.label("No connections yet.");
        return;
    }

    let mut edit_clicked = None;
    let mut delete_clicked = None;
    let mut refresh_clicked = None;

    egui::Grid::new("connections_grid")
        .num_columns(4)
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Name");
            ui.strong("Type");
            ui.strong("Last sync");
            ui.strong("");
            ui.end_row();

            for connection in &screen.connections {
                ui.label(&connection.name);
                ui.label(&connection.connector_type);
                let last_sync = connection
                    .last_sync
                    .map_or_else(|| "never".to_owned(), |at| at.to_string());
                ui.label(last_sync);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!screen.busy, egui::Button::new("Refresh"))
                        .clicked()
                    {
                        refresh_clicked = Some(connection.id);
                    }
                    if ui.button("Edit").clicked() {
                        edit_clicked = Some(connection.clone());
                    }
                    if ui.button("Delete").clicked() {
                        delete_clicked = Some(connection.id);
                    }
                });
                ui.end_row();
            }
        });

    if let Some(connection) = edit_clicked {
        screen.form.load(&connection);
    }
    if let Some(id) = delete_clicked {
        screen.spawn_delete(handle, id);
    }
    if let Some(id) = refresh_clicked {
        screen.spawn_refresh(handle, id);
    }
}

fn show_form(screen: &mut SettingsScreen, ui: &mut egui::Ui, handle: &Handle) {
    let heading = if screen.form.editing_id.is_some() {
        "Edit connection"
    } else {
        "Add connection"
    };
    ui.strong(heading);

    ui.horizontal(|ui| {
        ui.label("Name");
        ui.text_edit_singleline(&mut screen.form.name);
    });
    ui.horizontal(|ui| {
        ui.label("Type");
        let selected = ConnectorType::parse(&screen.form.connector_type);
        egui::ComboBox::from_id_salt("connector_type")
            .selected_text(selected.label())
            .show_ui(ui, |ui| {
                for known in ConnectorType::KNOWN {
                    if ui
                        .selectable_label(selected == *known, known.label())
                        .on_hover_text(known.setup_help())
                        .clicked()
                    {
                        known.as_str().clone_into(&mut screen.form.connector_type);
                    }
                }
            })
            .response
            .on_hover_text(selected.setup_help());
    });
    ui.horizontal(|ui| {
        ui.label("Credentials");
        let mask = ConnectorType::parse(&screen.form.connector_type).credentials_are_secret();
        ui.add(egui::TextEdit::singleline(&mut screen.form.credentials).password(mask));
    });

    ui.horizontal(|ui| {
        let can_save =
            !screen.busy && !screen.form.name.is_empty() && !screen.form.connector_type.is_empty();
        if ui
            .add_enabled(can_save, egui::Button::new("Save"))
            .clicked()
        {
            match screen.form.editing_id {
                Some(id) => screen.spawn_update(handle, id),
                None => screen.spawn_create(handle),
            }
        }
        if screen.form.editing_id.is_some() && ui.button("Cancel").clicked() {
            screen.form.clear();
        }
    });
}
