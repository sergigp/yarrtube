use crate::domain::video::EpisodeNumber;
use crate::domain::video::video_filename::VideoFilename;
use crate::domain::video_transcript::CaptionLanguage;
use std::ffi::OsStr;
use std::path::Path;

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

    /// The paths of the episode whose media file was downloaded as
    /// `media_filename` (`Season <s>/<code> - <title>.<ext>`): its files are
    /// named after the recorded media file, which keeps the title it was
    /// downloaded with, not after the current title.
    pub fn from_media(media_filename: &str) -> Self {
        let path = Path::new(media_filename);
        let lossy = |part: Option<&OsStr>| {
            part.map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        Self {
            season_folder: lossy(path.parent().map(Path::as_os_str)),
            base_name: lossy(path.file_stem()),
        }
    }

    /// `Season <s>/<code> - <title>`, the path every file of the episode
    /// shares before its extension; just the base name for a media file
    /// with no folder.
    pub fn stem(&self) -> String {
        match self.season_folder.as_str() {
            "" => self.base_name.clone(),
            folder => format!("{folder}/{}", self.base_name),
        }
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

    /// `<stem>.<lang>.srt`, the episode's subtitle file in `language`, named
    /// by its base code (`es` for `es-US`): media servers recognise those.
    pub fn subtitle(&self, language: &CaptionLanguage) -> String {
        self.media(&format!("{}.srt", language.base()))
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
    fn it_should_name_an_episodes_subtitle_file_after_its_language() {
        let paths = EpisodePaths::new(&PlaylistEpisodeNumber::new(7).unwrap().into(), "Title");

        assert_eq!(
            paths.subtitle(&CaptionLanguage::new("en").unwrap()),
            "Season 1/S01E007 - Title.en.srt"
        );
    }

    #[test]
    fn it_should_name_an_episodes_files_after_its_downloaded_media_file() {
        let paths = EpisodePaths::from_media("Season 2026/S2026E031501 - Old Title.mp4");

        assert_eq!(
            [
                paths.stem(),
                paths.subtitle(&CaptionLanguage::new("es-US").unwrap()),
            ],
            [
                "Season 2026/S2026E031501 - Old Title",
                "Season 2026/S2026E031501 - Old Title.es.srt",
            ]
            .map(str::to_string)
        );
    }

    #[test]
    fn it_should_name_a_bare_media_files_siblings_relative_to_the_show() {
        let paths = EpisodePaths::from_media("My Video.mp4");

        assert_eq!(
            [
                paths.stem(),
                paths.subtitle(&CaptionLanguage::new("en").unwrap()),
            ],
            ["My Video", "My Video.en.srt"].map(str::to_string)
        );
    }

    #[test]
    fn it_should_name_a_playlist_episode_in_season_one() {
        let paths = EpisodePaths::new(&PlaylistEpisodeNumber::new(7).unwrap().into(), "Song");

        assert_eq!(paths.media("mp4"), "Season 1/S01E007 - Song.mp4");
    }
}
