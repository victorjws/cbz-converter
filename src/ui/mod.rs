mod drop_zone;
mod entry_list;
mod toolbar;

pub fn render(app: &mut crate::app::AppState, ui: &mut egui::Ui) {
    toolbar::render_top(app, ui);
    toolbar::render_bottom(app, ui);

    egui::CentralPanel::default().show_inside(ui, |ui| {
        if app.entries.is_empty() {
            drop_zone::render(ui);
        } else {
            entry_list::render(app, ui);
        }
    });
}
