use std::io::Read;
use std::path::Path;

use crate::models::{ComicInfoField, PageRule, PageType, PresetField};

/// Preset data extracted from a ComicInfo.xml document.
pub struct Imported {
    /// Recognized fields, ready to replace the current preset rows.
    pub preset: Vec<PresetField>,
    /// `<Series>` value, if present — maps to the global Series override rather
    /// than a preset row.
    pub series: Option<String>,
    /// Page rules reconstructed from the `<Pages>` element. Empty when the
    /// document has no typed/double pages.
    pub page_rules: Vec<PageRule>,
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
/// become preset rows; `<Series>` becomes the override; `<Pages>` is turned
/// into page rules; `Writer` and `PageCount` are skipped (per-folder author /
/// derived count). Empty values are dropped.
pub fn parse(xml: &str) -> Result<Imported, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    let root = doc.root_element();

    let mut preset = Vec::new();
    let mut series = None;
    let mut page_rules = Vec::new();

    for node in root.children().filter(|n| n.is_element()) {
        let tag = node.tag_name().name();
        if tag == "Pages" {
            page_rules = parse_pages(node);
            continue;
        }
        let value = node.text().unwrap_or("").trim().to_string();
        if value.is_empty() {
            continue;
        }
        match tag {
            "Series" => series = Some(value),
            "Writer" | "PageCount" => {}
            _ => {
                if let Some(field) = ComicInfoField::from_xml_tag(tag) {
                    preset.push(PresetField { field, value });
                }
            }
        }
    }

    Ok(Imported {
        preset,
        series,
        page_rules,
    })
}

/// Reconstruct page rules from a `<Pages>` element. Only pages carrying a
/// `Type` and/or `DoublePage="true"` produce rules (plain `Story` pages need
/// none). Consecutive pages sharing the same attributes collapse into a single
/// ranged rule. Positions are 1-based, matching `PageRule`.
fn parse_pages(pages_node: roxmltree::Node) -> Vec<PageRule> {
    let mut pages: Vec<(usize, Option<PageType>, bool)> = Vec::new();
    for page in pages_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "Page")
    {
        let Some(image) = page
            .attribute("Image")
            .and_then(|s| s.parse::<usize>().ok())
        else {
            continue;
        };
        let page_type = page.attribute("Type").and_then(PageType::from_xml_value);
        let double_page = page
            .attribute("DoublePage")
            .is_some_and(|v| v.eq_ignore_ascii_case("true"));
        if page_type.is_some() || double_page {
            pages.push((image, page_type, double_page));
        }
    }
    pages.sort_by_key(|p| p.0);

    let mut rules = Vec::new();
    let mut i = 0;
    while i < pages.len() {
        let (start, page_type, double_page) = pages[i];
        let mut end = start;
        let mut j = i + 1;
        // Extend the run over contiguous pages with identical attributes.
        while j < pages.len()
            && pages[j].0 == end + 1
            && pages[j].1 == page_type
            && pages[j].2 == double_page
        {
            end = pages[j].0;
            j += 1;
        }
        rules.push(PageRule {
            position: (start + 1) as i32,
            end: (end > start).then_some((end + 1) as i32),
            // A DoublePage-only page has no Type; Story is the neutral default.
            page_type: page_type.unwrap_or(PageType::Story),
            double_page,
        });
        i = j;
    }
    rules
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
    fn plain_pages_produce_no_rules() {
        // Pages without Type / DoublePage are plain Story pages: no rules.
        let xml = r#"<ComicInfo><Pages>
            <Page Image="0" />
            <Page Image="1" />
        </Pages></ComicInfo>"#;
        let imported = parse(xml).unwrap();
        assert!(imported.page_rules.is_empty());
    }

    #[test]
    fn typed_pages_become_rules() {
        let xml = r#"<ComicInfo><Pages>
            <Page Image="0" Type="FrontCover" />
            <Page Image="1" />
            <Page Image="4" Type="BackCover" DoublePage="true" />
        </Pages></ComicInfo>"#;
        let rules = parse(xml).unwrap().page_rules;
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].position, 1);
        assert_eq!(rules[0].end, None);
        assert_eq!(rules[0].page_type, PageType::FrontCover);
        assert!(!rules[0].double_page);
        assert_eq!(rules[1].position, 5);
        assert_eq!(rules[1].page_type, PageType::BackCover);
        assert!(rules[1].double_page);
    }

    #[test]
    fn contiguous_pages_collapse_into_range() {
        let xml = r#"<ComicInfo><Pages>
            <Page Image="1" Type="Story" />
            <Page Image="2" Type="Story" />
            <Page Image="3" Type="Story" />
        </Pages></ComicInfo>"#;
        let rules = parse(xml).unwrap().page_rules;
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].position, 2);
        assert_eq!(rules[0].end, Some(4));
        assert_eq!(rules[0].page_type, PageType::Story);
    }

    #[test]
    fn import_roundtrips_with_builder() {
        use crate::metadata::build_comic_info_xml;
        use crate::models::ParsedMetadata;

        let rules = parse(
            r#"<ComicInfo><Pages>
                <Page Image="0" Type="FrontCover" />
                <Page Image="1" Type="Editorial" />
                <Page Image="2" Type="Editorial" />
                <Page Image="4" Type="BackCover" />
            </Pages></ComicInfo>"#,
        )
        .unwrap()
        .page_rules;

        let meta = ParsedMetadata::default();
        let xml = build_comic_info_xml(&meta, &[], 5, &rules);
        assert!(xml.contains("<Page Image=\"0\" Type=\"FrontCover\" />"));
        assert!(xml.contains("<Page Image=\"1\" Type=\"Editorial\" />"));
        assert!(xml.contains("<Page Image=\"2\" Type=\"Editorial\" />"));
        assert!(xml.contains("<Page Image=\"3\" />"));
        assert!(xml.contains("<Page Image=\"4\" Type=\"BackCover\" />"));
    }

    #[test]
    fn entities_unescaped() {
        let xml = r#"<ComicInfo><Publisher>Tom &amp; Jerry</Publisher></ComicInfo>"#;
        let imported = parse(xml).unwrap();
        assert_eq!(imported.preset[0].value, "Tom & Jerry");
    }
}
