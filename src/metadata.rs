use crate::models::{ComicInfoField, ParsedMetadata, PresetField};

pub fn build_comic_info_xml(meta: &ParsedMetadata, preset: &[PresetField]) -> String {
    let mut body = String::new();

    // Title and Writer come from the per-folder parsing, always first.
    push_element(&mut body, "Title", &meta.title);
    push_element(&mut body, "Writer", &meta.author.join(", "));

    // Tags: merge folder-name tags with any preset Tags rows, de-duplicated.
    let merged_tags = merge_tags(meta, preset);
    push_element(&mut body, "Tags", &merged_tags.join(", "));

    // Remaining preset fields, emitted in canonical ComicInfo order. Tags is
    // skipped here since it was already merged above.
    for field in ComicInfoField::ALL {
        if *field == ComicInfoField::Tags {
            continue;
        }
        for pf in preset.iter().filter(|pf| pf.field == *field) {
            push_element(&mut body, field.xml_tag(), &pf.value);
        }
    }

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<ComicInfo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
           xmlns:xsd="http://www.w3.org/2001/XMLSchema">
{body}</ComicInfo>
"#
    )
}

/// Folder-name tags first, then preset `Tags` values (comma-separated),
/// preserving order and removing exact-string duplicates and empties.
fn merge_tags(meta: &ParsedMetadata, preset: &[PresetField]) -> Vec<String> {
    let preset_tags = preset
        .iter()
        .filter(|pf| pf.field == ComicInfoField::Tags)
        .flat_map(|pf| pf.value.split(','))
        .map(|t| t.trim().to_string());

    let mut out: Vec<String> = Vec::new();
    for tag in meta.tags.iter().cloned().chain(preset_tags) {
        if tag.is_empty() {
            continue;
        }
        if !out.contains(&tag) {
            out.push(tag);
        }
    }
    out
}

/// Append `  <Tag>escaped</Tag>\n`, skipping empty values.
fn push_element(buf: &mut String, tag: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    buf.push_str("  <");
    buf.push_str(tag);
    buf.push('>');
    buf.push_str(&xml_escape(value));
    buf.push_str("</");
    buf.push_str(tag);
    buf.push_str(">\n");
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ComicInfoField;

    fn meta() -> ParsedMetadata {
        ParsedMetadata {
            author: vec!["Author".into()],
            title: "Title".into(),
            tags: vec!["SF".into(), "Fantasy".into()],
        }
    }

    fn pf(field: ComicInfoField, value: &str) -> PresetField {
        PresetField {
            field,
            value: value.into(),
        }
    }

    #[test]
    fn empty_preset_emits_core_fields() {
        let xml = build_comic_info_xml(&meta(), &[]);
        assert!(xml.contains("<Title>Title</Title>"));
        assert!(xml.contains("<Writer>Author</Writer>"));
        assert!(xml.contains("<Tags>SF, Fantasy</Tags>"));
        // No Manga line unless preset provides it.
        assert!(!xml.contains("<Manga>"));
    }

    #[test]
    fn preset_field_emitted() {
        let preset = vec![
            pf(ComicInfoField::Publisher, "Munhak"),
            pf(ComicInfoField::Manga, "YesAndRightToLeft"),
        ];
        let xml = build_comic_info_xml(&meta(), &preset);
        assert!(xml.contains("<Publisher>Munhak</Publisher>"));
        assert!(xml.contains("<Manga>YesAndRightToLeft</Manga>"));
    }

    #[test]
    fn tags_merge_dedup() {
        // Folder has SF, Fantasy; preset adds Fantasy (dup) and Webtoon.
        let preset = vec![pf(ComicInfoField::Tags, "Fantasy, Webtoon")];
        let xml = build_comic_info_xml(&meta(), &preset);
        assert!(xml.contains("<Tags>SF, Fantasy, Webtoon</Tags>"));
    }

    #[test]
    fn empty_value_skipped() {
        let preset = vec![pf(ComicInfoField::Publisher, "")];
        let xml = build_comic_info_xml(&meta(), &preset);
        assert!(!xml.contains("<Publisher>"));
    }

    #[test]
    fn xml_escaped() {
        let m = ParsedMetadata {
            author: vec![],
            title: "Tom & <Jerry>".into(),
            tags: vec![],
        };
        let xml = build_comic_info_xml(&m, &[]);
        assert!(xml.contains("<Title>Tom &amp; &lt;Jerry&gt;</Title>"));
    }

    #[test]
    fn canonical_order() {
        // Publisher comes before Manga in canonical order.
        let preset = vec![
            pf(ComicInfoField::Manga, "Yes"),
            pf(ComicInfoField::Publisher, "P"),
        ];
        let xml = build_comic_info_xml(&meta(), &preset);
        let pub_pos = xml.find("<Publisher>").unwrap();
        let manga_pos = xml.find("<Manga>").unwrap();
        assert!(pub_pos < manga_pos);
    }
}
