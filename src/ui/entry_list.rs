use crate::app::AppState;
use crate::models::ConversionStatus;

pub fn render(app: &mut AppState, ui: &mut egui::Ui) {
    let mut to_remove: Vec<usize> = Vec::new();

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("entries")
            .num_columns(5)
            .spacing([8.0, 4.0])
            .striped(true)
            .min_col_width(60.0)
            .show(ui, |ui| {
                ui.strong("Folder");
                ui.strong("Series");
                ui.strong("Author");
                ui.strong("Status");
                ui.label("");
                ui.end_row();

                for (i, entry) in app.entries.iter_mut().enumerate() {
                    let short_name: String = entry.folder_name.chars().take(24).collect();
                    let short_name = if entry.folder_name.chars().count() > 24 {
                        format!("{}…", short_name)
                    } else {
                        short_name
                    };
                    ui.label(short_name).on_hover_text(&entry.folder_name);

                    if entry.editing {
                        if ui
                            .text_edit_singleline(&mut entry.metadata.series)
                            .changed()
                        {
                            entry.edited.series = true;
                        }

                        let mut author_str = entry.metadata.author.join(",");
                        if ui.text_edit_singleline(&mut author_str).changed() {
                            entry.metadata.author = author_str
                                .split(',')
                                .map(|a| a.trim().to_string())
                                .filter(|a| !a.is_empty())
                                .collect();
                            entry.edited.author = true;
                        }
                    } else {
                        ui.label(&entry.metadata.series);
                        ui.label(entry.metadata.author.join(", "));
                    }

                    match &entry.status {
                        ConversionStatus::Pending => {
                            ui.label("Pending");
                        }
                        ConversionStatus::Running { progress } => {
                            ui.add(
                                egui::ProgressBar::new(*progress)
                                    .desired_width(80.0)
                                    .show_percentage(),
                            );
                        }
                        ConversionStatus::Done => {
                            ui.colored_label(egui::Color32::GREEN, "Done ✓");
                        }
                        ConversionStatus::Error(msg) => {
                            ui.colored_label(egui::Color32::RED, "Error ✗")
                                .on_hover_text(msg.as_str());
                        }
                    }

                    ui.horizontal(|ui| {
                        let is_converting =
                            matches!(entry.status, ConversionStatus::Running { .. });
                        ui.add_enabled_ui(!is_converting, |ui| {
                            let edit_label = if entry.editing { "✓" } else { "✏" };
                            if ui.small_button(edit_label).clicked() {
                                entry.editing = !entry.editing;
                            }
                            if ui.small_button("✕").clicked() {
                                to_remove.push(i);
                            }
                        });
                    });

                    ui.end_row();
                }
            });
    });

    for i in to_remove.into_iter().rev() {
        app.entries.remove(i);
    }
}
