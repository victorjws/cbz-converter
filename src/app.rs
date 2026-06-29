use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use crate::models::{
    ComicInfoField, ConversionStatus, EditedFields, FolderEntry, PageRule, PageType, PresetField,
    ProgressEvent,
};
use crate::parser::FormatPattern;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub format_template: String,
    pub preset: Vec<PresetField>,
    pub page_rules: Vec<PageRule>,
    /// Global Series set in the preset. When non-empty it overrides the
    /// per-folder value for every CBZ; blank falls back to the per-folder value.
    #[serde(default)]
    pub preset_series: String,
    /// Saved tag vocabulary the user picks from in the preset Tags field.
    #[serde(default)]
    pub tag_library: Vec<String>,
    /// Saved genre vocabulary the user picks from in the preset Genre field.
    #[serde(default)]
    pub genre_library: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            format_template: "[{author}] {series} ({tags})".to_string(),
            // Preserves the previously hardcoded ComicInfo behavior.
            preset: vec![PresetField {
                field: ComicInfoField::Manga,
                value: "YesAndRightToLeft".to_string(),
            }],
            // First page is the cover by convention; user can change/remove it.
            page_rules: vec![PageRule {
                position: 1,
                end: None,
                page_type: PageType::FrontCover,
                double_page: false,
            }],
            preset_series: String::new(),
            tag_library: Vec::new(),
            genre_library: Vec::new(),
        }
    }
}

/// Best-effort fallback when `DroppedFile::path` is `None`: interpret the name
/// as a `file://` URI or an absolute path. Returns `None` for anything that
/// isn't an existing absolute path.
fn path_from_dropped(file: &egui::DroppedFile) -> Option<PathBuf> {
    let name = file.name.trim();
    if name.is_empty() {
        return None;
    }
    let raw = name.strip_prefix("file://").unwrap_or(name);
    let decoded = percent_decode(raw);
    let path = PathBuf::from(decoded);
    if path.is_absolute() && path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Minimal `%XX` percent-decoding for `file://` URIs.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub struct AppState {
    pub entries: Vec<FolderEntry>,
    pub settings: AppSettings,
    pub format_pattern: FormatPattern,
    pub show_format_help: bool,
    pub show_preset: bool,
    pub show_tag_library: bool,
    pub show_genre_library: bool,
    /// "+ Add" input buffers for the two libraries.
    pub tag_library_input: String,
    pub genre_library_input: String,
    /// Snapshot of a library entry captured when its text field gains focus, so
    /// a rename can be detected (and propagated) when the field loses focus.
    pub library_edit_snapshot: Option<String>,
    pub is_converting: bool,
    prev_hovered: bool,
    progress_rx: Option<Receiver<ProgressEvent>>,
    folder_picker_rx: Option<Receiver<Vec<std::path::PathBuf>>>,
}

impl AppState {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let settings = crate::config::load();

        let format_pattern = FormatPattern::compile(&settings.format_template);

        Self {
            entries: Vec::new(),
            format_pattern,
            settings,
            show_format_help: false,
            show_preset: false,
            show_tag_library: false,
            show_genre_library: false,
            tag_library_input: String::new(),
            genre_library_input: String::new(),
            library_edit_snapshot: None,
            is_converting: false,
            prev_hovered: false,
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
            edited: EditedFields::default(),
            status: ConversionStatus::Pending,
            editing: false,
        });
    }

    pub fn update_format_template(&mut self, template: &str) {
        self.settings.format_template = template.to_string();
        self.format_pattern = FormatPattern::compile(template);
        for entry in &mut self.entries {
            let parsed = self.format_pattern.parse(&entry.folder_name);
            // Re-parse from the folder name, but keep any field the user has
            // manually edited (manual input takes priority).
            if !entry.edited.author {
                entry.metadata.author = parsed.author;
            }
            if !entry.edited.series {
                entry.metadata.series = parsed.series;
            }
            entry.metadata.tags = parsed.tags;
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
            let mut metadata = entry.metadata.clone();
            // Global preset Series overrides the per-folder value when set.
            if !self.settings.preset_series.trim().is_empty() {
                metadata.series = self.settings.preset_series.trim().to_string();
            }
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

        // Log hover transitions so we can tell whether egui receives drag
        // events at all (useful for diagnosing Wayland drag-and-drop).
        let hovered = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
        if hovered != self.prev_hovered {
            eprintln!("[dnd] hovered_files non-empty = {hovered}");
            self.prev_hovered = hovered;
        }

        let dropped: Vec<_> = ui.ctx().input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            eprintln!(
                "[dnd] dropped: path={:?} name={:?} mime={:?}",
                file.path, file.name, file.mime
            );
            // Native winit usually fills `path`; fall back to parsing the name
            // as a `file://` URI / plain path otherwise.
            let path = file.path.clone().or_else(|| path_from_dropped(&file));
            match path {
                Some(p) if p.is_dir() => self.add_entry(p),
                Some(p) => eprintln!("[dnd] ignored (not a directory): {}", p.display()),
                None => eprintln!("[dnd] ignored (no resolvable path)"),
            }
        }

        if self.is_converting {
            ui.ctx().request_repaint();
        }

        crate::ui::render(self, ui);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        crate::config::save(&self.settings);
    }
}
