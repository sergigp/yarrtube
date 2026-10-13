use crate::domain::video::EpisodeNumber;
use crate::domain::video::video_filename::VideoFilename;

/// Where an episode's files sit in its show, relative to the show directory:
/// every file shares the stem `Season <s>/<code> - <title>` and differs only
/// by extension — the media's own, `.jpg` for the thumbnail, `.nfo` for the
/// episode NFO. The one place that names episode files, so the downloader,
/// the relocator, the thumbnail fetcher and the orphan sweep agree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodePaths {
    season_folder: String,
    base_name: String,
}

impl EpisodePaths {
    pub fn new(number: &EpisodeNumber, title: &str) -> Self {
        Self {
            season_folder: number.season_folder(),
            base_name: format!("{} - {}", number.code(), VideoFilename::from_title(title)),
        }
    }

    /// `Season <s>/<code> - <title>`, the path every file of the episode
    /// shares before its extension.
    pub fn stem(&self) -> String {
        format!("{}/{}", self.season_folder, self.base_name)
    }

    pub fn media(&self, extension: &str) -> String {
        format!("{}.{extension}", self.stem())
    }

    pub fn thumbnail(&self) -> String {
        self.media("jpg")
    }

    pub fn nfo(&self) -> String {
        self.media("nfo")
    }

    /// The thumbnail's file name as the episode NFO references it: a sibling
    /// in the same season folder.
    pub fn thumb(&self) -> String {
        format!("{}.jpg", self.base_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::video::{ChannelEpisodeNumber, PlaylistEpisodeNumber};

    #[test]
    fn it_should_name_every_file_of_a_channel_episode_after_its_stem() {
        let paths = EpisodePaths::new(
            &ChannelEpisodeNumber::new(2026, 31501).unwrap().into(),
            "My: Video",
        );

        assert_eq!(
            [
                paths.stem(),
                paths.media("mkv"),
                paths.thumbnail(),
                paths.nfo(),
                paths.thumb(),
            ],
            [
                "Season 2026/S2026E031501 - My- Video",
                "Season 2026/S2026E031501 - My- Video.mkv",
                "Season 2026/S2026E031501 - My- Video.jpg",
                "Season 2026/S2026E031501 - My- Video.nfo",
                "S2026E031501 - My- Video.jpg",
            ]
            .map(str::to_string)
        );
    }

    #[test]
    fn it_should_name_a_playlist_episode_in_season_one() {
        let paths = EpisodePaths::new(&PlaylistEpisodeNumber::new(7).unwrap().into(), "Song");

        assert_eq!(paths.media("mp4"), "Season 1/S01E007 - Song.mp4");
    }
}
