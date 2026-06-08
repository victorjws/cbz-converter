use crate::app::AppState;
use crate::models::ConversionStatus;

pub fn render_top(app: &mut AppState, ui: &mut egui::Ui) {
    egui::Panel::top("toolbar_top")
        .min_size(32.0)
        .show_inside(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Format:");
                let mut template = app.settings.format_template.clone();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut template)
                        .desired_width(340.0)
                        .hint_text("[{author}] {title} ({tags})"),
                );
                if response.changed() {
                    app.update_format_template(&template);
                }
                if ui.small_button("?").clicked() {
                    app.show_format_help = !app.show_format_help;
                }
            });

            if app.show_format_help {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(
                        "{author} = author name    {title} = title    {tags} = tags (comma-separated)\nExample: [{author}] {title} ({tags})  →  [Author] Title (tag1,tag2)",
                    )
                    .small()
                    .color(egui::Color32::GRAY),
                );
            }
            ui.add_space(4.0);
        });
}

pub fn render_bottom(app: &mut AppState, ui: &mut egui::Ui) {
    egui::Panel::bottom("toolbar_bottom")
        .min_size(40.0)
        .show_inside(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let n_pending = app
                    .entries
                    .iter()
                    .filter(|e| e.status == ConversionStatus::Pending)
                    .count();

                if ui.button("Add Folders…").clicked() {
                    app.open_folder_picker(ui.ctx().clone());
                }

                ui.add_enabled_ui(!app.is_converting && !app.entries.is_empty(), |ui| {
                    if ui.button("Clear All").clicked() {
                        app.entries.clear();
                    }
                });

                ui.separator();

                let can_convert = !app.is_converting && n_pending > 0;
                if ui
                    .add_enabled(
                        can_convert,
                        egui::Button::new(format!("Convert ({n_pending})")),
                    )
                    .clicked()
                {
                    app.start_conversion(ui.ctx().clone());
                }

                if app.is_converting {
                    let done = app
                        .entries
                        .iter()
                        .filter(|e| {
                            matches!(
                                e.status,
                                ConversionStatus::Done | ConversionStatus::Error(_)
                            )
                        })
                        .count();
                    let total = app.entries.len();
                    ui.label(format!("Converting… {done}/{total}"));
                    ui.spinner();
                } else {
                    let done = app
                        .entries
                        .iter()
                        .filter(|e| e.status == ConversionStatus::Done)
                        .count();
                    if done > 0 {
                        ui.colored_label(egui::Color32::GREEN, format!("{done} done"));
                    }
                }
            });
            ui.add_space(6.0);
        });
}
