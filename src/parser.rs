use crate::models::ParsedMetadata;

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Literal(String),
    Field(FieldKind),
}

#[derive(Clone, Debug, PartialEq)]
enum FieldKind {
    Author,
    Title,
    Tags,
}

#[derive(Clone)]
pub struct FormatPattern {
    template: String,
    tokens: Vec<Token>,
    tags_sep: char,
}

impl Default for FormatPattern {
    fn default() -> Self {
        Self::compile("[{author}] {title} ({tags})")
    }
}

impl FormatPattern {
    pub fn compile(template: &str) -> Self {
        let mut tokens = Vec::new();
        let mut rest = template;

        loop {
            let placeholders: &[(&str, FieldKind)] = &[
                ("{author}", FieldKind::Author),
                ("{title}", FieldKind::Title),
                ("{tags}", FieldKind::Tags),
            ];

            let next = placeholders
                .iter()
                .filter_map(|(p, k)| rest.find(p).map(|i| (i, i + p.len(), k)))
                .min_by_key(|(i, _, _)| *i);

            match next {
                None => {
                    if !rest.is_empty() {
                        tokens.push(Token::Literal(rest.to_string()));
                    }
                    break;
                }
                Some((start, end, kind)) => {
                    if start > 0 {
                        tokens.push(Token::Literal(rest[..start].to_string()));
                    }
                    tokens.push(Token::Field(kind.clone()));
                    rest = &rest[end..];
                }
            }
        }

        Self {
            template: template.to_string(),
            tokens,
            tags_sep: ',',
        }
    }

    #[allow(dead_code)]
    pub fn template(&self) -> &str {
        &self.template
    }

    pub fn parse(&self, name: &str) -> ParsedMetadata {
        let mut meta = ParsedMetadata::default();
        let tokens = &self.tokens;
        let n = tokens.len();
        let mut pos = 0usize;
        let mut i = 0;

        while i < n {
            match &tokens[i] {
                Token::Literal(lit) => {
                    let remaining = &name[pos..];
                    let next_is_optional = matches!(
                        tokens.get(i + 1),
                        Some(Token::Field(FieldKind::Author | FieldKind::Tags))
                    );

                    if let Some(found) = remaining.find(lit.as_str()) {
                        pos += found + lit.len();
                        i += 1;
                    } else if next_is_optional {
                        // Delimiter not found: skip Lit + optional Field + following Lit
                        i += 2;
                        if let Some(Token::Literal(_)) = tokens.get(i) {
                            i += 1;
                        }
                        // pos stays the same; continue without incrementing again
                    } else {
                        i += 1;
                    }
                }
                Token::Field(kind) => {
                    let terminator = tokens[i + 1..].iter().find_map(|t| {
                        if let Token::Literal(s) = t {
                            Some(s.as_str())
                        } else {
                            None
                        }
                    });

                    let remaining = &name[pos..];
                    let value = match terminator {
                        Some(term) => {
                            if let Some(p) = remaining.find(term) {
                                let v = remaining[..p].trim();
                                pos += p;
                                v
                            } else {
                                pos = name.len();
                                remaining.trim()
                            }
                        }
                        None => {
                            pos = name.len();
                            remaining.trim()
                        }
                    };

                    match kind {
                        FieldKind::Author => {
                            meta.author = value
                                .split(self.tags_sep)
                                .map(|a| a.trim().to_string())
                                .filter(|a| !a.is_empty())
                                .collect();
                        }
                        FieldKind::Title => {
                            // Per ComicInfo spec, Series is the work title and
                            // Title the individual episode. The folder name only
                            // carries one title, so seed both; the user can edit
                            // either per-folder afterwards.
                            meta.title = value.to_string();
                            meta.series = value.to_string();
                        }
                        FieldKind::Tags => {
                            // Tags are no longer parsed from the folder name; the
                            // `{tags}` token only acts as a title boundary so the
                            // trailing region is consumed (and discarded) instead
                            // of being absorbed into the title.
                        }
                    }
                    i += 1;
                }
            }
        }

        meta
    }

    #[allow(dead_code)]
    pub fn format(&self, meta: &ParsedMetadata) -> String {
        let tokens = &self.tokens;
        let n = tokens.len();
        let mut skip = vec![false; n];

        for i in 0..n {
            let is_empty_optional = match &tokens[i] {
                Token::Field(FieldKind::Author) => meta.author.is_empty(),
                // Tags are no longer stored/emitted, so always drop the region.
                Token::Field(FieldKind::Tags) => true,
                _ => false,
            };

            if is_empty_optional {
                skip[i] = true;
                if i > 0 {
                    if let Token::Literal(_) = &tokens[i - 1] {
                        skip[i - 1] = true;
                    }
                }
                if i + 1 < n {
                    if let Token::Literal(_) = &tokens[i + 1] {
                        skip[i + 1] = true;
                    }
                }
            }
        }

        let mut result = String::new();
        for (i, token) in tokens.iter().enumerate() {
            if skip[i] {
                continue;
            }
            match token {
                Token::Literal(s) => result.push_str(s),
                Token::Field(kind) => match kind {
                    FieldKind::Author => {
                        result.push_str(&meta.author.join(&self.tags_sep.to_string()))
                    }
                    FieldKind::Title => result.push_str(&meta.title),
                    // Tags are no longer stored on the metadata; nothing to emit.
                    FieldKind::Tags => {}
                },
            }
        }

        result.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pat() -> FormatPattern {
        FormatPattern::compile("[{author}] {title} ({tags})")
    }

    #[test]
    fn full() {
        // Tags are no longer parsed; the trailing region is consumed/discarded.
        let m = pat().parse("[Author] Title (SF,Fantasy)");
        assert_eq!(m.author, vec!["Author"]);
        assert_eq!(m.title, "Title");
        assert!(m.tags.is_empty());
    }

    #[test]
    fn series_and_title_filled() {
        // The folder-name title seeds both Series and Title.
        let m = pat().parse("[Author] Title (SF)");
        assert_eq!(m.title, "Title");
        assert_eq!(m.series, "Title");
    }

    #[test]
    fn title_not_polluted_by_tags() {
        // The `({tags})` region must not be absorbed into the title.
        let m = pat().parse("[A] Title (SF, Fantasy)");
        assert_eq!(m.title, "Title");
        assert_eq!(m.series, "Title");
        assert!(m.tags.is_empty());
    }

    #[test]
    fn multi_author() {
        let m = pat().parse("[Author1, Author2] Title (SF)");
        assert_eq!(m.author, vec!["Author1", "Author2"]);
        assert_eq!(m.title, "Title");
        assert!(m.tags.is_empty());
    }

    #[test]
    fn no_author() {
        let m = pat().parse("Title (Complete)");
        assert!(m.author.is_empty());
        assert_eq!(m.title, "Title");
        assert!(m.tags.is_empty());
    }

    #[test]
    fn no_tags() {
        let m = pat().parse("[Author] Title");
        assert_eq!(m.author, vec!["Author"]);
        assert_eq!(m.title, "Title");
        assert!(m.tags.is_empty());
    }

    #[test]
    fn title_only() {
        let m = pat().parse("Title");
        assert_eq!(m.title, "Title");
        assert!(m.author.is_empty());
        assert!(m.tags.is_empty());
    }

    #[test]
    fn format_full() {
        let p = pat();
        let m = ParsedMetadata {
            author: vec!["Author".into()],
            series: "Title".into(),
            title: "Title".into(),
            tags: vec![],
        };
        assert_eq!(p.format(&m), "[Author] Title");
    }

    #[test]
    fn format_multi_author() {
        let p = pat();
        let m = ParsedMetadata {
            author: vec!["Author1".into(), "Author2".into()],
            series: "Title".into(),
            title: "Title".into(),
            tags: vec![],
        };
        assert_eq!(p.format(&m), "[Author1,Author2] Title");
    }

    #[test]
    fn format_no_author() {
        let p = pat();
        let m = ParsedMetadata {
            author: vec![],
            series: "Title".into(),
            title: "Title".into(),
            tags: vec![],
        };
        assert_eq!(p.format(&m), "Title");
    }
}
