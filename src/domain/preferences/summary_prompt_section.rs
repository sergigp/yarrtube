/// A section of the summary prompt document (`## <heading>`): a message
/// template, or a snippet inserted into one, with the variables it can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryPromptSection {
    System,
    User,
    Chapters,
    MapSystem,
    MapUser,
    MapChapters,
    ReduceSystem,
    ReduceUser,
    ReducePart,
}

impl SummaryPromptSection {
    pub const ALL: [Self; 9] = [
        Self::System,
        Self::User,
        Self::Chapters,
        Self::MapSystem,
        Self::MapUser,
        Self::MapChapters,
        Self::ReduceSystem,
        Self::ReduceUser,
        Self::ReducePart,
    ];

    pub fn from_heading(heading: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|section| section.heading() == heading)
    }

    pub fn heading(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Chapters => "chapters",
            Self::MapSystem => "map system",
            Self::MapUser => "map user",
            Self::MapChapters => "map chapters",
            Self::ReduceSystem => "reduce system",
            Self::ReduceUser => "reduce user",
            Self::ReducePart => "reduce part",
        }
    }

    pub fn variables(self) -> &'static [&'static str] {
        match self {
            Self::System | Self::MapSystem | Self::ReduceSystem => &["lang"],
            Self::User => &["title", "minutes", "chapters", "transcript", "lang"],
            Self::Chapters | Self::MapChapters => &["chapter_list"],
            Self::MapUser => &[
                "title",
                "minutes",
                "part",
                "parts",
                "span",
                "chapters",
                "transcript",
                "lang",
            ],
            Self::ReduceUser => &["title", "minutes", "chapters", "part_summaries", "lang"],
            Self::ReducePart => &["part", "span", "summary"],
        }
    }
}
