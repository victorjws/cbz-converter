use crate::app::AppState;
use crate::models::{ComicInfoField, ConversionStatus, PageRule, PageType, PresetField};

pub fn render_top(app: &mut AppState, ui: &mut egui::Ui) {
    egui::Panel::top("toolbar_top")
        .min_size(32.0)
        .show(ui, |ui| {
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

                let preset_label = if app.show_preset {
                    "ComicInfo Preset ▾"
                } else {
                    "ComicInfo Preset ▸"
                };
                if ui.button(preset_label).clicked() {
                    app.show_preset = !app.show_preset;
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

            if app.show_preset {
                render_preset(app, ui);
            }
            ui.add_space(4.0);
        });
}

fn render_preset(app: &mut AppState, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.separator();
    ui.label(
        egui::RichText::new("Default ComicInfo fields applied to every CBZ")
            .small()
            .color(egui::Color32::GRAY),
    );

    let mut to_remove: Vec<usize> = Vec::new();
    for (i, pf) in app.settings.preset.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            // Field selector.
            egui::ComboBox::from_id_salt(("preset_field", i))
                .selected_text(pf.field.label())
                .width(150.0)
                .show_ui(ui, |ui| {
                    for field in ComicInfoField::ALL {
                        if ui
                            .selectable_label(pf.field == *field, field.label())
                            .clicked()
                            && pf.field != *field
                        {
                            pf.field = *field;
                            // Reset value when switching to an enum field whose
                            // allowed set does not contain the current value.
                            if let Some(allowed) = field.allowed_values() {
                                if !allowed.contains(&pf.value.as_str()) {
                                    pf.value = field.default_value();
                                }
                            }
                        }
                    }
                });

            // Value input: dropdown for enum fields, free text otherwise.
            if let Some(allowed) = pf.field.allowed_values() {
                egui::ComboBox::from_id_salt(("preset_value", i))
                    .selected_text(pf.value.clone())
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for v in allowed {
                            ui.selectable_value(&mut pf.value, v.to_string(), *v);
                        }
                    });
            } else {
                let hint = if pf.field == ComicInfoField::Tags {
                    "merged with folder-name tags"
                } else {
                    "value"
                };
                ui.add(
                    egui::TextEdit::singleline(&mut pf.value)
                        .desired_width(200.0)
                        .hint_text(hint),
                );
            }

            if ui.small_button("✕").clicked() {
                to_remove.push(i);
            }
        });
    }

    for i in to_remove.into_iter().rev() {
        app.settings.preset.remove(i);
    }

    if ui.button("+ Add field").clicked() {
        let field = ComicInfoField::Publisher;
        app.settings.preset.push(PresetField {
            value: field.default_value(),
            field,
        });
    }

    render_page_rules(app, ui);
}

fn render_page_rules(app: &mut AppState, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new("Page types  (position: 1 = first page, -1 = last page)")
            .small()
            .color(egui::Color32::GRAY),
    );

    let mut to_remove: Vec<usize> = Vec::new();
    for (i, rule) in app.settings.page_rules.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut rule.position)
                    .speed(1)
                    .prefix("page "),
            );
            egui::ComboBox::from_id_salt(("page_type", i))
                .selected_text(rule.page_type.label())
                .width(150.0)
                .show_ui(ui, |ui| {
                    for t in PageType::ALL {
                        ui.selectable_value(&mut rule.page_type, *t, t.label());
                    }
                });
            ui.checkbox(&mut rule.double_page, "Double");
            if ui.small_button("✕").clicked() {
                to_remove.push(i);
            }
        });
    }

    for i in to_remove.into_iter().rev() {
        app.settings.page_rules.remove(i);
    }

    if ui.button("+ Add page type").clicked() {
        app.settings.page_rules.push(PageRule {
            position: 1,
            page_type: PageType::FrontCover,
            double_page: false,
        });
    }
}

pub fn render_bottom(app: &mut AppState, ui: &mut egui::Ui) {
    egui::Panel::bottom("toolbar_bottom")
        .min_size(40.0)
        .show(ui, |ui| {
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
