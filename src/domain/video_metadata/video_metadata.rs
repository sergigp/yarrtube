/// A video's Plex-facing `movie.nfo` content: everything derived from its
/// YouTube metadata (plus a locally-resolved sort order and, when present, a
/// saved thumbnail filename) needed to render that file. Fields are already
/// fully resolved (plot truncated, sorttitle computed, genre mapped) by the
/// time a `VideoMetadata` is constructed — see `mapping.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoMetadata {
    pub title: String,
    pub plot: String,
    pub studio: String,
    pub director: String,
    pub premiered: String,
    pub year: i32,
    pub genre: Option<String>,
    pub tags: Vec<String>,
    pub uniqueid: String,
    pub thumb: Option<String>,
    pub sorttitle: String,
}

impl VideoMetadata {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        title: impl Into<String>,
        plot: impl Into<String>,
        studio: impl Into<String>,
        director: impl Into<String>,
        premiered: impl Into<String>,
        year: i32,
        genre: Option<String>,
        tags: Vec<String>,
        uniqueid: impl Into<String>,
        thumb: Option<String>,
        sorttitle: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            plot: plot.into(),
            studio: studio.into(),
            director: director.into(),
            premiered: premiered.into(),
            year,
            genre,
            tags,
            uniqueid: uniqueid.into(),
            thumb,
            sorttitle: sorttitle.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_build_a_video_metadata_with_every_field() {
        let metadata = VideoMetadata::new(
            "My Video",
            "A description",
            "My Channel",
            "My Channel",
            "2024-01-02",
            2024,
            Some("Music".to_string()),
            vec!["tag1".to_string(), "tag2".to_string()],
            "yt1",
            Some("My Video.jpg".to_string()),
            "0001 - My Video",
        );

        assert_eq!(metadata.title, "My Video");
        assert_eq!(metadata.plot, "A description");
        assert_eq!(metadata.studio, "My Channel");
        assert_eq!(metadata.director, "My Channel");
        assert_eq!(metadata.premiered, "2024-01-02");
        assert_eq!(metadata.year, 2024);
        assert_eq!(metadata.genre, Some("Music".to_string()));
        assert_eq!(metadata.tags, vec!["tag1".to_string(), "tag2".to_string()]);
        assert_eq!(metadata.uniqueid, "yt1");
        assert_eq!(metadata.thumb, Some("My Video.jpg".to_string()));
        assert_eq!(metadata.sorttitle, "0001 - My Video");
    }

    #[test]
    fn it_should_build_a_video_metadata_with_no_genre_no_tags_and_no_thumbnail() {
        let metadata = VideoMetadata::new(
            "My Video",
            "A description",
            "My Channel",
            "My Channel",
            "2024-01-02",
            2024,
            None,
            Vec::new(),
            "yt1",
            None,
            "0001 - My Video",
        );

        assert_eq!(metadata.genre, None);
        assert!(metadata.tags.is_empty());
        assert_eq!(metadata.thumb, None);
    }
}
