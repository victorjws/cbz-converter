use crate::models::{ComicInfoField, PageRule, ParsedMetadata, PresetField};

/// Creator-role fields that default to the author value when the preset
/// provides no explicit value of their own. `Writer` is handled separately
/// (always emitted from the author at the top of the document).
const AUTHOR_DEFAULT: &[ComicInfoField] = &[
    ComicInfoField::Penciller,
    ComicInfoField::Inker,
    ComicInfoField::Colorist,
    ComicInfoField::Letterer,
    ComicInfoField::CoverArtist,
    ComicInfoField::Editor,
];

/// Replace per-folder placeholders in a free-text preset value. Mirrors the
/// `{author}`/`{title}` tokens used by the folder-name format pattern.
fn apply_placeholders(value: &str, meta: &ParsedMetadata) -> String {
    value
        .replace("{author}", &meta.author.join(", "))
        .replace("{title}", &meta.title)
}

pub fn build_comic_info_xml(
    meta: &ParsedMetadata,
    preset: &[PresetField],
    page_count: usize,
    page_rules: &[PageRule],
) -> String {
    let mut body = String::new();
    let author_value = meta.author.join(", ");

    // Title, Series and Writer come from the per-folder parsing / edits, always
    // first. Spec-wise Series is the work title and Title the individual episode.
    push_element(&mut body, "Title", &meta.title);
    push_element(&mut body, "Series", &meta.series);
    push_element(&mut body, "Writer", &author_value);

    // Tags: merge folder-name tags with any preset Tags rows, de-duplicated.
    let merged_tags = merge_tags(meta, preset);
    push_element(&mut body, "Tags", &merged_tags.join(", "));

    // Remaining preset fields, emitted in canonical ComicInfo order. Tags is
    // skipped here since it was already merged above.
    for field in ComicInfoField::ALL {
        if *field == ComicInfoField::Tags {
            continue;
        }
        // Free-text fields support per-folder placeholders; enum values are
        // emitted verbatim.
        let mut emitted = false;
        for pf in preset.iter().filter(|pf| pf.field == *field) {
            let value = if field.allowed_values().is_none() {
                apply_placeholders(&pf.value, meta)
            } else {
                pf.value.clone()
            };
            if !value.is_empty() {
                push_element(&mut body, field.xml_tag(), &value);
                emitted = true;
            }
        }
        // Creator-role fields fall back to the author when unset.
        if !emitted && AUTHOR_DEFAULT.contains(field) && !author_value.is_empty() {
            push_element(&mut body, field.xml_tag(), &author_value);
        }
    }

    // Page info, derived from the image count. Page types come from the
    // global page rules; later rules win when two target the same page.
    if page_count > 0 {
        push_element(&mut body, "PageCount", &page_count.to_string());

        // Per-page attributes: (Type, DoublePage). Later rules win.
        let mut attrs: Vec<(Option<&'static str>, bool)> = vec![(None, false); page_count];
        for rule in page_rules {
            if let Some(idx) = rule.resolve(page_count) {
                attrs[idx] = (Some(rule.page_type.xml_value()), rule.double_page);
            }
        }

        body.push_str("  <Pages>\n");
        for (i, (page_type, double_page)) in attrs.iter().enumerate() {
            body.push_str(&format!("    <Page Image=\"{i}\""));
            if let Some(t) = page_type {
                body.push_str(&format!(" Type=\"{t}\""));
            }
            if *double_page {
                body.push_str(" DoublePage=\"true\"");
            }
            body.push_str(" />\n");
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
        .flat_map(|pf| {
            apply_placeholders(&pf.value, meta)
                .split(',')
                .map(|t| t.trim().to_string())
                .collect::<Vec<_>>()
        });

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
            double_page: false,
        }
    }

    fn meta() -> ParsedMetadata {
        ParsedMetadata {
            author: vec!["Author".into()],
            series: "Title".into(),
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
            series: String::new(),
            title: "Tom & <Jerry>".into(),
            tags: vec![],
        };
        let xml = build_comic_info_xml(&m, &[], 0, &[]);
        assert!(xml.contains("<Title>Tom &amp; &lt;Jerry&gt;</Title>"));
    }

    #[test]
    fn series_emitted() {
        let m = ParsedMetadata {
            author: vec![],
            series: "Work".into(),
            title: "Ep 1".into(),
            tags: vec![],
        };
        let xml = build_comic_info_xml(&m, &[], 0, &[]);
        assert!(xml.contains("<Series>Work</Series>"));
        assert!(xml.contains("<Title>Ep 1</Title>"));
    }

    #[test]
    fn placeholder_substituted() {
        let preset = vec![pf(ComicInfoField::CoverArtist, "{author}")];
        let xml = build_comic_info_xml(&meta(), &preset, 0, &[]);
        assert!(xml.contains("<CoverArtist>Author</CoverArtist>"));
    }

    #[test]
    fn placeholder_multi_author() {
        let m = ParsedMetadata {
            author: vec!["A".into(), "B".into()],
            series: String::new(),
            title: "T".into(),
            tags: vec![],
        };
        let preset = vec![pf(ComicInfoField::Publisher, "by {author}")];
        let xml = build_comic_info_xml(&m, &preset, 0, &[]);
        assert!(xml.contains("<Publisher>by A, B</Publisher>"));
    }

    #[test]
    fn enum_value_not_substituted() {
        // Enum fields keep their literal value (placeholders never apply).
        let preset = vec![pf(ComicInfoField::Manga, "Yes")];
        let xml = build_comic_info_xml(&meta(), &preset, 0, &[]);
        assert!(xml.contains("<Manga>Yes</Manga>"));
    }

    #[test]
    fn mixed_literal_placeholder() {
        let preset = vec![pf(ComicInfoField::Publisher, "art by {author}")];
        let xml = build_comic_info_xml(&meta(), &preset, 0, &[]);
        assert!(xml.contains("<Publisher>art by Author</Publisher>"));
    }

    #[test]
    fn roles_default_to_author() {
        let m = ParsedMetadata {
            author: vec!["Kim".into()],
            series: String::new(),
            title: "T".into(),
            tags: vec![],
        };
        let xml = build_comic_info_xml(&m, &[], 0, &[]);
        for tag in [
            "Penciller",
            "Inker",
            "Colorist",
            "Letterer",
            "CoverArtist",
            "Editor",
        ] {
            assert!(
                xml.contains(&format!("<{tag}>Kim</{tag}>")),
                "missing author default for {tag}"
            );
        }
    }

    #[test]
    fn roles_no_author_not_emitted() {
        let m = ParsedMetadata {
            author: vec![],
            series: String::new(),
            title: "T".into(),
            tags: vec![],
        };
        let xml = build_comic_info_xml(&m, &[], 0, &[]);
        assert!(!xml.contains("<Penciller>"));
        assert!(!xml.contains("<CoverArtist>"));
    }

    #[test]
    fn role_preset_overrides_author() {
        let m = ParsedMetadata {
            author: vec!["Kim".into()],
            series: String::new(),
            title: "T".into(),
            tags: vec![],
        };
        let preset = vec![pf(ComicInfoField::CoverArtist, "Lee")];
        let xml = build_comic_info_xml(&m, &preset, 0, &[]);
        assert!(xml.contains("<CoverArtist>Lee</CoverArtist>"));
        assert!(!xml.contains("<CoverArtist>Kim</CoverArtist>"));
        // Other roles still fall back to the author.
        assert!(xml.contains("<Penciller>Kim</Penciller>"));
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
    fn page_rule_double_page() {
        let rules = vec![PageRule {
            position: 3,
            page_type: PageType::Story,
            double_page: true,
        }];
        let xml = build_comic_info_xml(&meta(), &[], 4, &rules);
        assert!(xml.contains("<Page Image=\"2\" Type=\"Story\" DoublePage=\"true\" />"));
        // Other pages stay plain.
        assert!(xml.contains("<Page Image=\"0\" />"));
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
