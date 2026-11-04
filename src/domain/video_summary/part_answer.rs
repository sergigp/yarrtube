use super::SummaryText;

/// What the model answered about a span of the video: one part, or several
/// reduced together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartAnswer {
    pub start_seconds: i64,
    pub end_seconds: i64,
    pub summary: SummaryText,
}

impl PartAnswer {
    /// `summary` of everything `group` covers, in order.
    pub fn merged(group: &[Self], summary: SummaryText) -> Self {
        Self {
            start_seconds: group.first().map_or(0, |answer| answer.start_seconds),
            end_seconds: group.last().map_or(0, |answer| answer.end_seconds),
            summary,
        }
    }
}
