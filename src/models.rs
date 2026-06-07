use std::path::PathBuf;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ParsedMetadata {
    pub author: String,
    pub title: String,
    pub tags: Vec<String>,
}

#[derive(Clone, PartialEq)]
pub enum ConversionStatus {
    Pending,
    Running { progress: f32 },
    Done,
    Error(String),
}

impl Default for ConversionStatus {
    fn default() -> Self {
        Self::Pending
    }
}

pub struct FolderEntry {
    pub path: PathBuf,
    pub folder_name: String,
    pub metadata: ParsedMetadata,
    pub status: ConversionStatus,
    pub editing: bool,
}

pub enum ProgressEvent {
    Progress { index: usize, fraction: f32 },
    Done { index: usize },
    Error { index: usize, message: String },
}
