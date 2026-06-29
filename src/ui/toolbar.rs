use crate::app::AppState;
use crate::models::{ComicInfoField, ConversionStatus, PageRule, PageType, PresetField};

/// Split a comma-separated preset value into trimmed, non-empty items.
fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

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

                let tag_lib_label = if app.show_tag_library {
                    "Tag Library ▾"
                } else {
                    "Tag Library ▸"
                };
                if ui.button(tag_lib_label).clicked() {
                    app.show_tag_library = !app.show_tag_library;
                }

                let genre_lib_label = if app.show_genre_library {
                    "Genre Library ▾"
                } else {
                    "Genre Library ▸"
                };
                if ui.button(genre_lib_label).clicked() {
                    app.show_genre_library = !app.show_genre_library;
                }
            });

            if app.show_format_help {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(
                        "{author} = author name    {title} = series & episode title    {tags} = title boundary only (not stored)\nExample: [{author}] {title} ({tags})  →  [Author] Title (trailing part ignored)",
                    )
                    .small()
                    .color(egui::Color32::GRAY),
                );
            }

            if app.show_preset {
                render_preset(app, ui);
            }
            if app.show_tag_library {
                render_tag_library(app, ui);
            }
            if app.show_genre_library {
                render_genre_library(app, ui);
            }
            ui.add_space(4.0);
        });
}

fn render_preset(app: &mut AppState, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.separator();
    ui.label(
        egui::RichText::new(
            "Default ComicInfo fields applied to every CBZ  (use {author}, {title} for per-folder values)",
        )
        .small()
        .color(egui::Color32::GRAY),
    );

    // Global Series/Title overrides (blank = use the per-folder value).
    ui.horizontal(|ui| {
        ui.label("Series");
        ui.add(
            egui::TextEdit::singleline(&mut app.settings.preset_series)
                .desired_width(200.0)
                .hint_text("override all folders (blank = per-folder)"),
        );
    });
    ui.horizontal(|ui| {
        ui.label("Title");
        ui.add(
            egui::TextEdit::singleline(&mut app.settings.preset_title)
                .desired_width(200.0)
                .hint_text("override all folders (blank = per-folder)"),
        );
    });

    // Snapshot the libraries so the chip selectors can read them while the
    // preset rows hold a mutable borrow of `app.settings.preset`.
    let tag_library = app.settings.tag_library.clone();
    let genre_library = app.settings.genre_library.clone();

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
                let hint = match pf.field {
                    ComicInfoField::Tags => "pick from Tag Library or type",
                    ComicInfoField::Genre => "pick from Genre Library or type",
                    _ => "value",
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

        // Library chip selectors, laid out below the row for Tags and Genre.
        let library = match pf.field {
            ComicInfoField::Tags => Some(&tag_library),
            ComicInfoField::Genre => Some(&genre_library),
            _ => None,
        };
        if let Some(library) = library {
            if !library.is_empty() {
                render_library_chips(ui, library, &mut pf.value);
            }
        }
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

/// Renders library values as wrap-around toggle chips that select into a
/// comma-separated preset `value`. Values already in `value` but missing from
/// the library are preserved untouched.
fn render_library_chips(ui: &mut egui::Ui, library: &[String], value: &mut String) {
    let mut selected = split_csv(value);
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        for item in library {
            let is_selected = selected.iter().any(|s| s == item);
            if ui.selectable_label(is_selected, item).clicked() {
                if is_selected {
                    selected.retain(|s| s != item);
                } else {
                    selected.push(item.clone());
                }
                changed = true;
            }
        }
    });
    if changed {
        *value = selected.join(", ");
    }
}

/// Editable library list (add / inline-edit / remove). Returns `(old, new)`
/// rename pairs detected when an entry's text field loses focus, so the caller
/// can propagate the rename to already-selected values.
fn render_string_library(
    ui: &mut egui::Ui,
    description: &str,
    library: &mut Vec<String>,
    input: &mut String,
    edit_snapshot: &mut Option<String>,
) -> Vec<(String, String)> {
    ui.add_space(4.0);
    ui.separator();
    ui.label(
        egui::RichText::new(description)
            .small()
            .color(egui::Color32::GRAY),
    );

    let mut renames: Vec<(String, String)> = Vec::new();
    let mut to_remove: Vec<usize> = Vec::new();
    for (i, item) in library.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let resp = ui.add(egui::TextEdit::singleline(item).desired_width(200.0));
            if resp.gained_focus() {
                *edit_snapshot = Some(item.clone());
            }
            if resp.lost_focus() {
                if let Some(old) = edit_snapshot.take() {
                    let new = item.trim().to_string();
                    if new.is_empty() {
                        *item = old; // reject blank rename
                    } else if new != old {
                        *item = new.clone();
                        renames.push((old, new));
                    }
                }
            }
            if ui.small_button("✕").clicked() {
                to_remove.push(i);
            }
        });
    }
    for i in to_remove.into_iter().rev() {
        library.remove(i);
    }

    ui.horizontal(|ui| {
        let resp = ui.add(
            egui::TextEdit::singleline(input)
                .desired_width(200.0)
                .hint_text("new value"),
        );
        let submitted = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.button("+ Add").clicked() || submitted {
            let value = input.trim().to_string();
            if !value.is_empty() && !library.contains(&value) {
                library.push(value);
            }
            input.clear();
        }
    });

    renames
}

/// Apply `(old, new)` renames to every preset row of `field`, de-duplicating.
fn rename_in_preset(
    preset: &mut [PresetField],
    field: ComicInfoField,
    renames: &[(String, String)],
) {
    if renames.is_empty() {
        return;
    }
    for pf in preset.iter_mut().filter(|pf| pf.field == field) {
        let mut items = split_csv(&pf.value);
        for (old, new) in renames {
            for it in items.iter_mut() {
                if it == old {
                    *it = new.clone();
                }
            }
        }
        let mut deduped: Vec<String> = Vec::new();
        for it in items {
            if !deduped.contains(&it) {
                deduped.push(it);
            }
        }
        pf.value = deduped.join(", ");
    }
}

fn render_tag_library(app: &mut AppState, ui: &mut egui::Ui) {
    let renames = render_string_library(
        ui,
        "Saved tags — pick these in the preset Tags field",
        &mut app.settings.tag_library,
        &mut app.tag_library_input,
        &mut app.library_edit_snapshot,
    );
    rename_in_preset(&mut app.settings.preset, ComicInfoField::Tags, &renames);
}

fn render_genre_library(app: &mut AppState, ui: &mut egui::Ui) {
    let renames = render_string_library(
        ui,
        "Saved genres — pick these in the preset Genre field",
        &mut app.settings.genre_library,
        &mut app.genre_library_input,
        &mut app.library_edit_snapshot,
    );
    rename_in_preset(&mut app.settings.preset, ComicInfoField::Genre, &renames);
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
