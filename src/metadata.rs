use crate::models::ParsedMetadata;

pub fn build_comic_info_xml(meta: &ParsedMetadata) -> String {
    let title = xml_escape(&meta.title);
    let writer = xml_escape(&meta.author);
    let tags = xml_escape(&meta.tags.join(", "));

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<ComicInfo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
           xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <Title>{title}</Title>
  <Writer>{writer}</Writer>
  <Tags>{tags}</Tags>
  <Manga>YesAndRightToLeft</Manga>
</ComicInfo>
"#
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
