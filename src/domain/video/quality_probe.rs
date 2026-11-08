use chrono::{DateTime, Utc};

/// The height a quality tier would download now, and the video's publish
/// time, when known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityProbe {
    pub height: Option<u32>,
    pub published_at: Option<DateTime<Utc>>,
    /// Whether `yt-dlp` reported YouTube's SABR-only streaming experiment:
    /// the formats it saw may be worse than what the video offers.
    pub degraded: bool,
}
