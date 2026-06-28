use std::path::PathBuf;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ParsedMetadata {
    pub author: Vec<String>,
    pub title: String,
    pub tags: Vec<String>,
}

/// ComicInfo.xml fields that can be configured via the global preset.
/// `Title` and `Writer` are intentionally excluded: they come from the
/// per-folder name parsing.
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ComicInfoField {
    Series,
    Number,
    Count,
    Volume,
    AlternateSeries,
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
    SeriesGroup,
    AgeRating,
    Review,
}

impl ComicInfoField {
    /// Full list in canonical ComicInfo.xml element order.
    pub const ALL: &'static [ComicInfoField] = &[
        ComicInfoField::Series,
        ComicInfoField::Number,
        ComicInfoField::Count,
        ComicInfoField::Volume,
        ComicInfoField::AlternateSeries,
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
        ComicInfoField::SeriesGroup,
        ComicInfoField::AgeRating,
        ComicInfoField::Review,
    ];

    /// XML element name (also used as the dropdown label).
    pub fn xml_tag(self) -> &'static str {
        match self {
            ComicInfoField::Series => "Series",
            ComicInfoField::Number => "Number",
            ComicInfoField::Count => "Count",
            ComicInfoField::Volume => "Volume",
            ComicInfoField::AlternateSeries => "AlternateSeries",
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
            ComicInfoField::SeriesGroup => "SeriesGroup",
            ComicInfoField::AgeRating => "AgeRating",
            ComicInfoField::Review => "Review",
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
