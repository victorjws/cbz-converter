use std::io::Write;
use std::path::Path;
use std::sync::mpsc::Sender;

use crate::models::{ParsedMetadata, ProgressEvent};

pub fn convert_folder(
    path: &Path,
    metadata: &ParsedMetadata,
    tx: Sender<ProgressEvent>,
    index: usize,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let image_exts = ["jpg", "jpeg", "png", "webp", "gif"];

    let mut images: Vec<std::path::PathBuf> = walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| image_exts.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    if images.is_empty() {
        return Err("No image files found.".into());
    }

    images.sort_by(|a, b| natord::compare(a.to_str().unwrap_or(""), b.to_str().unwrap_or("")));

    let folder_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Could not read folder name.")?;

    let output = path
        .parent()
        .ok_or("Could not find parent directory.")?
        .join(format!("{}.cbz", folder_name));

    let file = std::fs::File::create(&output)?;
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let comic_info = crate::metadata::build_comic_info_xml(metadata);
    zip.start_file("ComicInfo.xml", options)?;
    zip.write_all(comic_info.as_bytes())?;

    let total = images.len();
    for (i, img_path) in images.iter().enumerate() {
        let relative = img_path.strip_prefix(path).map_err(|e| e.to_string())?;
        let zip_name = relative.to_str().ok_or("Invalid path.")?.replace('\\', "/");

        zip.start_file(&zip_name, options)?;
        let data = std::fs::read(img_path)?;
        zip.write_all(&data)?;

        tx.send(ProgressEvent::Progress {
            index,
            fraction: (i + 1) as f32 / total as f32,
        })
        .ok();
    }

    zip.finish()?;
    Ok(output)
}
