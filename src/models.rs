use std::path::PathBuf;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ParsedMetadata {
    pub author: Vec<String>,
    pub series: String,
    pub tags: Vec<String>,
}

/// ComicInfo.xml fields that can be configured via the global preset.
/// `Series` and `Writer` are intentionally excluded: they come from the
/// per-folder name parsing / manual edits. `Title` is a regular preset field.
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ComicInfoField {
    Title,
    Number,
    Count,
    Volume,
    AlternateSeries,
    AlternateNumber,
    AlternateCount,
    Summary,
    Notes,
    Year,
    Month,
    Day,
    Penciller,
    Inker,
    Colorist,
    Letterer,
    CoverArtist,
    Editor,
    Translator,
    Publisher,
    Imprint,
    Genre,
    Tags,
    Web,
    LanguageISO,
    Format,
    BlackAndWhite,
    Manga,
    Characters,
    Teams,
    Locations,
    ScanInformation,
    StoryArc,
    StoryArcNumber,
    SeriesGroup,
    AgeRating,
    CommunityRating,
    MainCharacterOrTeam,
    Review,
    GTIN,
}

impl ComicInfoField {
    /// Full list in canonical ComicInfo.xml element order. This MUST stay in
    /// spec order: it drives both the preset selector order and the order
    /// elements are written to ComicInfo.xml.
    pub const ALL: &'static [ComicInfoField] = &[
        ComicInfoField::Title,
        ComicInfoField::Number,
        ComicInfoField::Count,
        ComicInfoField::Volume,
        ComicInfoField::AlternateSeries,
        ComicInfoField::AlternateNumber,
        ComicInfoField::AlternateCount,
        ComicInfoField::Summary,
        ComicInfoField::Notes,
        ComicInfoField::Year,
        ComicInfoField::Month,
        ComicInfoField::Day,
        ComicInfoField::Penciller,
        ComicInfoField::Inker,
        ComicInfoField::Colorist,
        ComicInfoField::Letterer,
        ComicInfoField::CoverArtist,
        ComicInfoField::Editor,
        ComicInfoField::Translator,
        ComicInfoField::Publisher,
        ComicInfoField::Imprint,
        ComicInfoField::Genre,
        ComicInfoField::Tags,
        ComicInfoField::Web,
        ComicInfoField::LanguageISO,
        ComicInfoField::Format,
        ComicInfoField::BlackAndWhite,
        ComicInfoField::Manga,
        ComicInfoField::Characters,
        ComicInfoField::Teams,
        ComicInfoField::Locations,
        ComicInfoField::ScanInformation,
        ComicInfoField::StoryArc,
        ComicInfoField::StoryArcNumber,
        ComicInfoField::SeriesGroup,
        ComicInfoField::AgeRating,
        ComicInfoField::CommunityRating,
        ComicInfoField::MainCharacterOrTeam,
        ComicInfoField::Review,
        ComicInfoField::GTIN,
    ];

    /// XML element name (also used as the dropdown label).
    pub fn xml_tag(self) -> &'static str {
        match self {
            ComicInfoField::Title => "Title",
            ComicInfoField::Number => "Number",
            ComicInfoField::Count => "Count",
            ComicInfoField::Volume => "Volume",
            ComicInfoField::AlternateSeries => "AlternateSeries",
            ComicInfoField::AlternateNumber => "AlternateNumber",
            ComicInfoField::AlternateCount => "AlternateCount",
            ComicInfoField::Summary => "Summary",
            ComicInfoField::Notes => "Notes",
            ComicInfoField::Year => "Year",
            ComicInfoField::Month => "Month",
            ComicInfoField::Day => "Day",
            ComicInfoField::Penciller => "Penciller",
            ComicInfoField::Inker => "Inker",
            ComicInfoField::Colorist => "Colorist",
            ComicInfoField::Letterer => "Letterer",
            ComicInfoField::CoverArtist => "CoverArtist",
            ComicInfoField::Editor => "Editor",
            ComicInfoField::Translator => "Translator",
            ComicInfoField::Publisher => "Publisher",
            ComicInfoField::Imprint => "Imprint",
            ComicInfoField::Genre => "Genre",
            ComicInfoField::Tags => "Tags",
            ComicInfoField::Web => "Web",
            ComicInfoField::LanguageISO => "LanguageISO",
            ComicInfoField::Format => "Format",
            ComicInfoField::BlackAndWhite => "BlackAndWhite",
            ComicInfoField::Manga => "Manga",
            ComicInfoField::Characters => "Characters",
            ComicInfoField::Teams => "Teams",
            ComicInfoField::Locations => "Locations",
            ComicInfoField::ScanInformation => "ScanInformation",
            ComicInfoField::StoryArc => "StoryArc",
            ComicInfoField::StoryArcNumber => "StoryArcNumber",
            ComicInfoField::SeriesGroup => "SeriesGroup",
            ComicInfoField::AgeRating => "AgeRating",
            ComicInfoField::CommunityRating => "CommunityRating",
            ComicInfoField::MainCharacterOrTeam => "MainCharacterOrTeam",
            ComicInfoField::Review => "Review",
            ComicInfoField::GTIN => "GTIN",
        }
    }

    pub fn label(self) -> &'static str {
        self.xml_tag()
    }

    /// For fields whose value is an enumeration in the ComicInfo spec, the
    /// allowed values. `None` means free text.
    pub fn allowed_values(self) -> Option<&'static [&'static str]> {
        match self {
            ComicInfoField::Manga => Some(&["Unknown", "No", "Yes", "YesAndRightToLeft"]),
            ComicInfoField::BlackAndWhite => Some(&["Unknown", "No", "Yes"]),
            ComicInfoField::AgeRating => Some(&[
                "Unknown",
                "Everyone",
                "Everyone 10+",
                "Teen",
                "Mature 17+",
                "Adults Only 18+",
                "G",
                "PG",
                "M",
                "MA15+",
                "R18+",
                "X18+",
                "Rating Pending",
                "Kids to Adults",
                "Early Childhood",
            ]),
            _ => None,
        }
    }

    /// Whether the free-text value input should be a multi-line, wrapping box
    /// (long values like URLs, tag lists, summaries) rather than a single line.
    pub fn multiline(self) -> bool {
        matches!(
            self,
            ComicInfoField::Summary
                | ComicInfoField::Notes
                | ComicInfoField::Review
                | ComicInfoField::Web
                | ComicInfoField::Tags
                | ComicInfoField::Genre
                | ComicInfoField::Characters
                | ComicInfoField::Teams
                | ComicInfoField::Locations
                | ComicInfoField::StoryArc
                | ComicInfoField::AlternateSeries
                | ComicInfoField::SeriesGroup
                | ComicInfoField::ScanInformation
        )
    }

    /// Canonical ComicInfo position of this field, used to keep preset rows
    /// sorted in spec order (matching the XML output order).
    pub fn order(self) -> usize {
        Self::ALL
            .iter()
            .position(|f| *f == self)
            .unwrap_or(usize::MAX)
    }

    /// Default value for a newly-added row of this field: first allowed value
    /// for enum fields, empty otherwise.
    pub fn default_value(self) -> String {
        self.allowed_values()
            .and_then(|v| v.first())
            .map(|s| s.to_string())
            .unwrap_or_default()
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PresetField {
    pub field: ComicInfoField,
    pub value: String,
}

/// ComicInfo `<Page Type="...">` values. `Story` is the spec default and is
/// emitted by leaving the attribute off.
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PageType {
    FrontCover,
    InnerCover,
    Roundup,
    Story,
    Advertisement,
    Editorial,
    Letters,
    Preview,
    BackCover,
    Other,
    Deleted,
}

impl PageType {
    pub const ALL: &'static [PageType] = &[
        PageType::FrontCover,
        PageType::InnerCover,
        PageType::Roundup,
        PageType::Story,
        PageType::Advertisement,
        PageType::Editorial,
        PageType::Letters,
        PageType::Preview,
        PageType::BackCover,
        PageType::Other,
        PageType::Deleted,
    ];

    pub fn xml_value(self) -> &'static str {
        match self {
            PageType::FrontCover => "FrontCover",
            PageType::InnerCover => "InnerCover",
            PageType::Roundup => "Roundup",
            PageType::Story => "Story",
            PageType::Advertisement => "Advertisement",
            PageType::Editorial => "Editorial",
            PageType::Letters => "Letters",
            PageType::Preview => "Preview",
            PageType::BackCover => "BackCover",
            PageType::Other => "Other",
            PageType::Deleted => "Deleted",
        }
    }

    pub fn label(self) -> &'static str {
        self.xml_value()
    }
}

/// Assigns page attributes to a page position or range. `position` is 1-based
/// (the range start); negative values count from the end (`-1` = last page).
/// `end` (when set) is the inclusive range end, resolved the same way. `end =
/// None` targets the single `position`. `double_page` maps to the ComicInfo
/// `<Page DoublePage="true">` attribute, independent of the type.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PageRule {
    pub position: i32,
    #[serde(default)]
    pub end: Option<i32>,
    pub page_type: PageType,
    #[serde(default)]
    pub double_page: bool,
}

impl PageRule {
    /// Resolve a 1-based (negative = from end) position to a 0-based index for a
    /// book of `page_count` pages, or `None` if it falls outside the range.
    fn resolve_pos(pos: i32, page_count: usize) -> Option<usize> {
        let n = page_count as i32;
        let idx = if pos > 0 {
            pos - 1
        } else if pos < 0 {
            n + pos
        } else {
            return None;
        };
        if idx >= 0 && idx < n {
            Some(idx as usize)
        } else {
            None
        }
    }

    /// Clamp a 1-based (negative = from end) position into `[0, n-1]`, or `None`
    /// for an empty book / invalid `0`. Used for range endpoints so e.g. "3 to
    /// 999" means "3 to last".
    fn clamp_pos(pos: i32, page_count: usize) -> Option<usize> {
        let n = page_count as i32;
        if n == 0 || pos == 0 {
            return None;
        }
        let idx = if pos > 0 { pos - 1 } else { n + pos };
        Some(idx.clamp(0, n - 1) as usize)
    }

    /// 0-based page indices this rule targets. With `end = None` this is the
    /// single (strictly in-range) `position`; with `end = Some(_)` it is the
    /// inclusive range between the two clamped endpoints.
    pub fn resolve_indices(&self, page_count: usize) -> Vec<usize> {
        match self.end {
            None => Self::resolve_pos(self.position, page_count)
                .into_iter()
                .collect(),
            Some(e) => match (
                Self::clamp_pos(self.position, page_count),
                Self::clamp_pos(e, page_count),
            ) {
                (Some(a), Some(b)) => {
                    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                    (lo..=hi).collect()
                }
                _ => Vec::new(),
            },
        }
    }
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

/// Tracks which per-folder fields the user has manually edited so a re-parse
/// (e.g. on format template change) does not overwrite them. Manual wins.
#[derive(Clone, Default)]
pub struct EditedFields {
    pub author: bool,
    pub series: bool,
}

pub struct FolderEntry {
    pub path: PathBuf,
    pub folder_name: String,
    pub metadata: ParsedMetadata,
    pub edited: EditedFields,
    pub status: ConversionStatus,
    pub editing: bool,
}

pub enum ProgressEvent {
    Progress { index: usize, fraction: f32 },
    Done { index: usize },
    Error { index: usize, message: String },
}
