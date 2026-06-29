use std::io::Read;
use std::path::Path;

use crate::models::{ComicInfoField, PresetField};

/// Preset data extracted from a ComicInfo.xml document.
pub struct Imported {
    /// Recognized fields, ready to replace the current preset rows.
    pub preset: Vec<PresetField>,
    /// `<Series>` value, if present — maps to the global Series override rather
    /// than a preset row.
    pub series: Option<String>,
}

/// Read the ComicInfo.xml text from either a standalone `.xml` file or the
/// `ComicInfo.xml` entry inside a `.cbz`/`.zip` archive.
pub fn read_xml(path: &Path) -> Result<String, String> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "cbz" || ext == "zip" {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        let mut entry = archive
            .by_name("ComicInfo.xml")
            .map_err(|_| "No ComicInfo.xml inside the archive.".to_string())?;
        let mut buf = String::new();
        entry.read_to_string(&mut buf).map_err(|e| e.to_string())?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    }
}

/// Parse ComicInfo.xml into preset rows. Elements matching a `ComicInfoField`
/// become preset rows; `<Series>` becomes the override; `Writer`, `PageCount`
/// and `Pages` are skipped (per-folder author / derived page data). Empty
/// values are dropped.
pub fn parse(xml: &str) -> Result<Imported, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    let root = doc.root_element();

    let mut preset = Vec::new();
    let mut series = None;

    for node in root.children().filter(|n| n.is_element()) {
        let tag = node.tag_name().name();
        let value = node.text().unwrap_or("").trim().to_string();
        if value.is_empty() {
            continue;
        }
        match tag {
            "Series" => series = Some(value),
            "Writer" | "PageCount" | "Pages" => {}
            _ => {
                if let Some(field) = ComicInfoField::from_xml_tag(tag) {
                    preset.push(PresetField { field, value });
                }
            }
        }
    }

    Ok(Imported { preset, series })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fields_and_series() {
        let xml = r#"<?xml version="1.0"?>
<ComicInfo>
  <Title>Ep 1</Title>
  <Series>My Work</Series>
  <Writer>Kim</Writer>
  <Publisher>Munhak</Publisher>
  <Manga>YesAndRightToLeft</Manga>
  <Tags>SF, Fantasy</Tags>
  <PageCount>20</PageCount>
  <Pages><Page Image="0" /></Pages>
</ComicInfo>"#;
        let imported = parse(xml).unwrap();
        assert_eq!(imported.series.as_deref(), Some("My Work"));

        let got: Vec<(ComicInfoField, &str)> = imported
            .preset
            .iter()
            .map(|pf| (pf.field, pf.value.as_str()))
            .collect();
        assert!(got.contains(&(ComicInfoField::Title, "Ep 1")));
        assert!(got.contains(&(ComicInfoField::Publisher, "Munhak")));
        assert!(got.contains(&(ComicInfoField::Manga, "YesAndRightToLeft")));
        assert!(got.contains(&(ComicInfoField::Tags, "SF, Fantasy")));
        // Writer / PageCount / Pages are skipped (not mappable to a field).
        assert_eq!(got.len(), 4);
    }

    #[test]
    fn entities_unescaped() {
        let xml = r#"<ComicInfo><Publisher>Tom &amp; Jerry</Publisher></ComicInfo>"#;
        let imported = parse(xml).unwrap();
        assert_eq!(imported.preset[0].value, "Tom & Jerry");
    }
}
