use std::path::PathBuf;

use crate::app::AppSettings;

/// Name of the portable settings file kept next to the executable so it
/// survives reinstalls / rebuilds (independent of eframe's app-storage).
const CONFIG_FILE: &str = "cbz-converter-config.json";

/// Resolve the settings file path: alongside the executable, falling back to
/// the current working directory when the exe path is unavailable.
fn config_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join(CONFIG_FILE);
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(CONFIG_FILE)
}

/// Load settings from the portable file, falling back to defaults when the
/// file is missing or cannot be parsed.
pub fn load() -> AppSettings {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(settings) => settings,
            Err(e) => {
                eprintln!("[config] failed to parse {}: {e}", path.display());
                AppSettings::default()
            }
        },
        Err(_) => AppSettings::default(),
    }
}

/// Persist settings to the portable file (best-effort).
pub fn save(settings: &AppSettings) {
    let path = config_path();
    match serde_json::to_string_pretty(settings) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                eprintln!("[config] failed to write {}: {e}", path.display());
            }
        }
        Err(e) => eprintln!("[config] failed to serialize settings: {e}"),
    }
}
