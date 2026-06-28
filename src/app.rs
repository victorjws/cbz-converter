use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use crate::models::{
    ComicInfoField, ConversionStatus, FolderEntry, PageRule, PageType, PresetField, ProgressEvent,
};
use crate::parser::FormatPattern;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub format_template: String,
    pub preset: Vec<PresetField>,
    pub page_rules: Vec<PageRule>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            format_template: "[{author}] {title} ({tags})".to_string(),
            // Preserves the previously hardcoded ComicInfo behavior.
            preset: vec![PresetField {
                field: ComicInfoField::Manga,
                value: "YesAndRightToLeft".to_string(),
            }],
            // First page is the cover by convention; user can change/remove it.
            page_rules: vec![PageRule {
                position: 1,
                page_type: PageType::FrontCover,
            }],
        }
    }
}

pub struct AppState {
    pub entries: Vec<FolderEntry>,
    pub settings: AppSettings,
    pub format_pattern: FormatPattern,
    pub show_format_help: bool,
    pub show_preset: bool,
    pub is_converting: bool,
    progress_rx: Option<Receiver<ProgressEvent>>,
    folder_picker_rx: Option<Receiver<Vec<std::path::PathBuf>>>,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let settings: AppSettings = cc
            .storage
            .and_then(|s| eframe::get_value(s, "settings"))
            .unwrap_or_default();

        let format_pattern = FormatPattern::compile(&settings.format_template);

        Self {
            entries: Vec::new(),
            format_pattern,
            settings,
            show_format_help: false,
            show_preset: false,
            is_converting: false,
            progress_rx: None,
            folder_picker_rx: None,
        }
    }

    pub fn add_entry(&mut self, path: PathBuf) {
        if self.entries.iter().any(|e| e.path == path) {
            return;
        }
        let folder_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let metadata = self.format_pattern.parse(&folder_name);
        self.entries.push(FolderEntry {
            path,
            folder_name,
            metadata,
            status: ConversionStatus::Pending,
            editing: false,
        });
    }

    pub fn update_format_template(&mut self, template: &str) {
        self.settings.format_template = template.to_string();
        self.format_pattern = FormatPattern::compile(template);
        for entry in &mut self.entries {
            entry.metadata = self.format_pattern.parse(&entry.folder_name);
        }
    }

    pub fn open_folder_picker(&mut self, ctx: egui::Context) {
        let (tx, rx) = mpsc::channel();
        self.folder_picker_rx = Some(rx);
        std::thread::spawn(move || {
            let folders = rfd::FileDialog::new().pick_folders().unwrap_or_default();
            tx.send(folders).ok();
            ctx.request_repaint();
        });
    }

    fn poll_folder_picker(&mut self) {
        if let Some(rx) = &self.folder_picker_rx {
            if let Ok(folders) = rx.try_recv() {
                for path in folders {
                    if path.is_dir() {
                        self.add_entry(path);
                    }
                }
                self.folder_picker_rx = None;
            }
        }
    }

    pub fn start_conversion(&mut self, ctx: egui::Context) {
        if self.is_converting {
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);
        self.is_converting = true;

        for (index, entry) in self.entries.iter_mut().enumerate() {
            if entry.status != ConversionStatus::Pending {
                continue;
            }
            entry.status = ConversionStatus::Running { progress: 0.0 };

            let tx = tx.clone();
            let path = entry.path.clone();
            let metadata = entry.metadata.clone();
            let preset = self.settings.preset.clone();
            let page_rules = self.settings.page_rules.clone();
            let ctx = ctx.clone();

            std::thread::spawn(move || {
                match crate::converter::convert_folder(
                    &path,
                    &metadata,
                    &preset,
                    &page_rules,
                    tx.clone(),
                    index,
                ) {
                    Ok(_) => {
                        tx.send(ProgressEvent::Done { index }).ok();
                    }
                    Err(e) => {
                        tx.send(ProgressEvent::Error {
                            index,
                            message: e.to_string(),
                        })
                        .ok();
                    }
                }
                ctx.request_repaint();
            });
        }
    }

    fn poll_progress(&mut self) {
        if let Some(rx) = &self.progress_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    ProgressEvent::Progress { index, fraction } => {
                        if let Some(e) = self.entries.get_mut(index) {
                            e.status = ConversionStatus::Running { progress: fraction };
                        }
                    }
                    ProgressEvent::Done { index } => {
                        if let Some(e) = self.entries.get_mut(index) {
                            e.status = ConversionStatus::Done;
                        }
                    }
                    ProgressEvent::Error { index, message } => {
                        if let Some(e) = self.entries.get_mut(index) {
                            e.status = ConversionStatus::Error(message);
                        }
                    }
                }
            }
        }

        if self.is_converting {
            let all_done = self.entries.iter().all(|e| {
                matches!(
                    e.status,
                    ConversionStatus::Done | ConversionStatus::Error(_)
                )
            });
            if all_done {
                self.progress_rx = None;
                self.is_converting = false;
            }
        }
    }
}

impl eframe::App for AppState {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_progress();
        self.poll_folder_picker();

        let dropped: Vec<_> = ui.ctx().input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            if let Some(path) = file.path {
                if path.is_dir() {
                    self.add_entry(path);
                }
            }
        }

        if self.is_converting {
            ui.ctx().request_repaint();
        }

        crate::ui::render(self, ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "settings", &self.settings);
    }
}
