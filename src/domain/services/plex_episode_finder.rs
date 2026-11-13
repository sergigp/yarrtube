use crate::domain::plex::{PlexEpisode, PlexSection, plex_folder_path};
use crate::domain::services::{VideoOwnerFinder, VideoOwnerFinderApi};
use crate::domain::video::Video;
use crate::infrastructure::repositories::plex_library_repository::PlexLibraryRepository;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// The Plex library sections holding yarrtube's videos, as one sync reads
/// them.
pub struct PlexLibrary {
    pub sections: Vec<PlexSection>,
    /// Every episode of `sections`, by its server-side file.
    pub episodes: HashMap<String, PlexEpisode>,
}

/// Finds the Plex episodes of yarrtube's videos. Plex sees yarrtube's
/// `videos_path` at `plex_videos_path`.
#[derive(Clone)]
pub struct PlexEpisodeFinder {
    plex_library_repository: Arc<dyn PlexLibraryRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    videos_path: String,
    plex_videos_path: String,
}

impl PlexEpisodeFinder {
    pub fn new(
        plex_library_repository: Arc<dyn PlexLibraryRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        videos_path: impl Into<String>,
        plex_videos_path: impl Into<String>,
    ) -> Self {
        Self {
            plex_library_repository,
            video_owner_finder,
            videos_path: videos_path.into(),
            plex_videos_path: plex_videos_path.into(),
        }
    }
}

pub trait PlexEpisodeFinderApi: Send + Sync {
    /// The sections holding yarrtube's videos and their episodes.
    fn read_library(&self) -> anyhow::Result<PlexLibrary>;
    /// The Plex episode of the video's file; `None` until it is downloaded
    /// and Plex has imported it.
    fn find_episode<'a>(
        &self,
        video: &Video,
        library: &'a PlexLibrary,
    ) -> anyhow::Result<Option<&'a PlexEpisode>>;
}

impl PlexEpisodeFinderApi for PlexEpisodeFinder {
    fn read_library(&self) -> anyhow::Result<PlexLibrary> {
        let sections = self.served_sections()?;
        let episodes = self.episodes_by_file(&sections)?;
        Ok(PlexLibrary { sections, episodes })
    }

    fn find_episode<'a>(
        &self,
        video: &Video,
        library: &'a PlexLibrary,
    ) -> anyhow::Result<Option<&'a PlexEpisode>> {
        let Some(owner) = self.video_owner_finder.find(video)? else {
            return Ok(None);
        };
        Ok(video
            .download
            .filename()
            .map(|filename| owner.target.output_dir.join(filename))
            .and_then(|file| {
                plex_folder_path(Path::new(&self.videos_path), &self.plex_videos_path, &file)
            })
            .and_then(|file| library.episodes.get(&file)))
    }
}

impl PlexEpisodeFinder {
    /// The library sections holding yarrtube's videos.
    fn served_sections(&self) -> anyhow::Result<Vec<PlexSection>> {
        Ok(self
            .plex_library_repository
            .list_sections()?
            .into_iter()
            .filter(|section| section.serves(&self.plex_videos_path))
            .collect())
    }

    /// Every episode of `sections`, by its server-side file.
    fn episodes_by_file(
        &self,
        sections: &[PlexSection],
    ) -> anyhow::Result<HashMap<String, PlexEpisode>> {
        sections
            .iter()
            .map(|section| self.plex_library_repository.list_episodes(&section.id))
            .collect::<anyhow::Result<Vec<_>>>()
            .map(|episodes| {
                episodes
                    .into_iter()
                    .flatten()
                    .map(|episode| (episode.file.clone(), episode))
                    .collect()
            })
    }
}
