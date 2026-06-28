use crate::models::{ComicInfoField, PageRule, ParsedMetadata, PresetField};

pub fn build_comic_info_xml(
    meta: &ParsedMetadata,
    preset: &[PresetField],
    page_count: usize,
    page_rules: &[PageRule],
) -> String {
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

    // Page info, derived from the image count. Page types come from the
    // global page rules; later rules win when two target the same page.
    if page_count > 0 {
        push_element(&mut body, "PageCount", &page_count.to_string());

        let mut types: Vec<Option<&'static str>> = vec![None; page_count];
        for rule in page_rules {
            if let Some(idx) = rule.resolve(page_count) {
                types[idx] = Some(rule.page_type.xml_value());
            }
        }

        body.push_str("  <Pages>\n");
        for (i, page_type) in types.iter().enumerate() {
            match page_type {
                Some(t) => {
                    body.push_str(&format!("    <Page Image=\"{i}\" Type=\"{t}\" />\n"));
                }
                None => body.push_str(&format!("    <Page Image=\"{i}\" />\n")),
            }
        }
        body.push_str("  </Pages>\n");
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
    use crate::models::{ComicInfoField, PageType};

    fn rule(position: i32, page_type: PageType) -> PageRule {
        PageRule {
            position,
            page_type,
        }
    }

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
        let xml = build_comic_info_xml(&meta(), &[], 0, &[]);
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
        let xml = build_comic_info_xml(&meta(), &preset, 0, &[]);
        assert!(xml.contains("<Publisher>Munhak</Publisher>"));
        assert!(xml.contains("<Manga>YesAndRightToLeft</Manga>"));
    }

    #[test]
    fn tags_merge_dedup() {
        // Folder has SF, Fantasy; preset adds Fantasy (dup) and Webtoon.
        let preset = vec![pf(ComicInfoField::Tags, "Fantasy, Webtoon")];
        let xml = build_comic_info_xml(&meta(), &preset, 0, &[]);
        assert!(xml.contains("<Tags>SF, Fantasy, Webtoon</Tags>"));
    }

    #[test]
    fn empty_value_skipped() {
        let preset = vec![pf(ComicInfoField::Publisher, "")];
        let xml = build_comic_info_xml(&meta(), &preset, 0, &[]);
        assert!(!xml.contains("<Publisher>"));
    }

    #[test]
    fn xml_escaped() {
        let m = ParsedMetadata {
            author: vec![],
            title: "Tom & <Jerry>".into(),
            tags: vec![],
        };
        let xml = build_comic_info_xml(&m, &[], 0, &[]);
        assert!(xml.contains("<Title>Tom &amp; &lt;Jerry&gt;</Title>"));
    }

    #[test]
    fn canonical_order() {
        // Publisher comes before Manga in canonical order.
        let preset = vec![
            pf(ComicInfoField::Manga, "Yes"),
            pf(ComicInfoField::Publisher, "P"),
        ];
        let xml = build_comic_info_xml(&meta(), &preset, 0, &[]);
        let pub_pos = xml.find("<Publisher>").unwrap();
        let manga_pos = xml.find("<Manga>").unwrap();
        assert!(pub_pos < manga_pos);
    }

    #[test]
    fn page_info_no_rules_is_plain() {
        let xml = build_comic_info_xml(&meta(), &[], 3, &[]);
        assert!(xml.contains("<PageCount>3</PageCount>"));
        assert!(xml.contains("<Page Image=\"0\" />"));
        assert!(xml.contains("<Page Image=\"1\" />"));
        assert!(xml.contains("<Page Image=\"2\" />"));
        assert!(xml.contains("</Pages>"));
    }

    #[test]
    fn page_rules_assign_types() {
        // First = FrontCover, second = InnerCover, last = BackCover.
        let rules = vec![
            rule(1, PageType::FrontCover),
            rule(2, PageType::InnerCover),
            rule(-1, PageType::BackCover),
        ];
        let xml = build_comic_info_xml(&meta(), &[], 5, &rules);
        assert!(xml.contains("<Page Image=\"0\" Type=\"FrontCover\" />"));
        assert!(xml.contains("<Page Image=\"1\" Type=\"InnerCover\" />"));
        assert!(xml.contains("<Page Image=\"2\" />"));
        assert!(xml.contains("<Page Image=\"3\" />"));
        assert!(xml.contains("<Page Image=\"4\" Type=\"BackCover\" />"));
    }

    #[test]
    fn page_rule_out_of_range_ignored() {
        // Position 10 in a 3-page book matches nothing.
        let rules = vec![rule(10, PageType::Editorial)];
        let xml = build_comic_info_xml(&meta(), &[], 3, &rules);
        assert!(!xml.contains("Editorial"));
        assert!(xml.contains("<Page Image=\"2\" />"));
    }

    #[test]
    fn no_page_info_when_zero() {
        let xml = build_comic_info_xml(&meta(), &[], 0, &[]);
        assert!(!xml.contains("<PageCount>"));
        assert!(!xml.contains("<Pages>"));
    }
}
