use super::VideoMetadata;
use crate::domain::shared::VideoId;
use crate::infrastructure::repositories::youtube_metadata_repository::{
    YoutubeMetadata, map_category_to_genre,
};
use chrono::{DateTime, Datelike, Utc};

const PLOT_LIMIT: usize = 500;

/// Truncates `description` to at most `PLOT_LIMIT` characters, cutting at
/// the last whole word within that limit and appending an ellipsis. Left
/// unchanged when already within the limit.
pub fn truncate_plot(description: &str) -> String {
    let chars: Vec<char> = description.chars().collect();
    if chars.len() <= PLOT_LIMIT {
        return description.to_string();
    }

    let window: String = chars[..PLOT_LIMIT].iter().collect();
    let cut = window
        .rfind(' ')
        .map_or(window.as_str(), |idx| &window[..idx]);
    format!("{cut}...")
}

/// Resolves `sorttitle` as a fixed-width, zero-padded numeric prefix
/// followed by `title`: `playlist_position` (a YouTube-linked playlist's
/// recorded position) when present, otherwise a prefix derived from
/// `published_at` — see design.md's "Sorttitle source" decision.
/// `ChannelVideo.position` is deliberately never passed in here, since it is
/// a recency rank that shifts on every reconcile pass.
pub fn resolve_sorttitle(
    title: &str,
    published_at: DateTime<Utc>,
    playlist_position: Option<i64>,
) -> String {
    let prefix = match playlist_position {
        Some(position) => format!("{position:04}"),
        None => published_at.format("%Y%m%d").to_string(),
    };
    format!("{prefix} {title}")
}

/// Assembles a `VideoMetadata` from a video's fetched `YoutubeMetadata`, its
/// already-resolved `sorttitle`, and its saved thumbnail filename (if any).
pub fn build_video_metadata(
    youtube_id: &VideoId,
    metadata: &YoutubeMetadata,
    sorttitle: impl Into<String>,
    thumbnail_filename: Option<String>,
) -> VideoMetadata {
    VideoMetadata::new(
        metadata.title.clone(),
        truncate_plot(&metadata.description),
        metadata.channel_title.clone(),
        metadata.channel_title.clone(),
        metadata.published_at.format("%Y-%m-%d").to_string(),
        metadata.published_at.year(),
        map_category_to_genre(metadata.category_id.as_deref()),
        metadata.tags.clone(),
        youtube_id.as_str().to_string(),
        thumbnail_filename,
        sorttitle,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn published_at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2024-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn youtube_metadata() -> YoutubeMetadata {
        YoutubeMetadata {
            title: "My Video".to_string(),
            description: "A description".to_string(),
            channel_title: "My Channel".to_string(),
            published_at: published_at(),
            tags: vec!["tag1".to_string()],
            category_id: Some("10".to_string()),
        }
    }

    #[test]
    fn it_should_leave_a_description_within_the_limit_unchanged() {
        let description = "a".repeat(500);

        assert_eq!(truncate_plot(&description), description);
    }

    #[test]
    fn it_should_leave_a_description_exactly_at_the_limit_unchanged() {
        let description = "a ".repeat(250);
        assert_eq!(description.chars().count(), 500);

        assert_eq!(truncate_plot(&description), description);
    }

    #[test]
    fn it_should_truncate_a_description_over_the_limit_at_a_word_boundary() {
        let description = format!("{}overflow", "a ".repeat(300));

        let truncated = truncate_plot(&description);

        assert!(truncated.chars().count() < description.chars().count());
        assert!(truncated.ends_with("..."));
        assert!(!truncated.trim_end_matches("...").ends_with(' '));
        assert!(description.starts_with(truncated.trim_end_matches("...").trim_end()));
    }

    #[test]
    fn it_should_resolve_sorttitle_from_a_playlist_position_when_present() {
        let sorttitle = resolve_sorttitle("My Video", published_at(), Some(3));

        assert_eq!(sorttitle, "0003 My Video");
    }

    #[test]
    fn it_should_resolve_sorttitle_from_the_publish_date_for_a_custom_playlist_video() {
        let sorttitle = resolve_sorttitle("My Video", published_at(), None);

        assert_eq!(sorttitle, "20240102 My Video");
    }

    #[test]
    fn it_should_resolve_sorttitle_from_the_publish_date_for_a_channel_video_ignoring_recency() {
        // A channel-tracked video's recency position is never passed in at
        // all — callers only look up `PlaylistVideo::position`, so a
        // channel video always resolves via the publish-date branch.
        let sorttitle = resolve_sorttitle("My Video", published_at(), None);

        assert_eq!(sorttitle, "20240102 My Video");
    }

    #[test]
    fn it_should_assemble_video_metadata_with_a_thumbnail() {
        let youtube_id = VideoId::new("yt1").unwrap();
        let metadata = youtube_metadata();

        let video_metadata = build_video_metadata(
            &youtube_id,
            &metadata,
            "0001 My Video",
            Some("My Video.jpg".to_string()),
        );

        assert_eq!(video_metadata.title, "My Video");
        assert_eq!(video_metadata.plot, "A description");
        assert_eq!(video_metadata.studio, "My Channel");
        assert_eq!(video_metadata.director, "My Channel");
        assert_eq!(video_metadata.premiered, "2024-01-02");
        assert_eq!(video_metadata.year, 2024);
        assert_eq!(video_metadata.genre, Some("Music".to_string()));
        assert_eq!(video_metadata.tags, vec!["tag1".to_string()]);
        assert_eq!(video_metadata.uniqueid, "yt1");
        assert_eq!(video_metadata.thumb, Some("My Video.jpg".to_string()));
        assert_eq!(video_metadata.sorttitle, "0001 My Video");
    }

    #[test]
    fn it_should_assemble_video_metadata_without_a_thumbnail() {
        let youtube_id = VideoId::new("yt1").unwrap();
        let metadata = youtube_metadata();

        let video_metadata = build_video_metadata(&youtube_id, &metadata, "0001 My Video", None);

        assert_eq!(video_metadata.thumb, None);
    }
}
