use crate::infrastructure::client::ytdlp_updater::{RealYtdlpUpdater, YtdlpUpdater};
use crate::infrastructure::repositories::filesystem_channel_avatar_repository::{
    ChannelAvatarRepository, FilesystemChannelAvatarRepository,
};
use crate::infrastructure::repositories::filesystem_directory_repository::{
    DirectoryRepository, FilesystemDirectoryRepository,
};
use crate::infrastructure::repositories::filesystem_video_file_repository::{
    FilesystemVideoFileRepository, VideoFileRepository,
};
use crate::infrastructure::repositories::sqlite_channel_repository::{
    ChannelRepository, SqliteChannelRepository,
};
use crate::infrastructure::repositories::sqlite_channel_video_repository::{
    ChannelVideoRepository, SqliteChannelVideoRepository,
};
use crate::infrastructure::repositories::sqlite_playlist_repository::{
    PlaylistRepository, SqlitePlaylistRepository,
};
use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
    PlaylistVideoRepository, SqlitePlaylistVideoRepository,
};
use crate::infrastructure::repositories::sqlite_task_repository::{
    SqliteTaskRepository, TaskRepository,
};
use crate::infrastructure::repositories::sqlite_video_metadata_repository::{
    SqliteVideoMetadataRepository, VideoMetadataRepository,
};
use crate::infrastructure::repositories::sqlite_video_repository::{
    SqliteVideoRepository, VideoRepository,
};
use crate::infrastructure::repositories::youtube_channel_repository::{
    YoutubeApiChannelRepository, YoutubeChannelRepository,
};
use crate::infrastructure::repositories::youtube_channel_videos_repository::{
    ChannelVideosRepository, YtDlpChannelVideosRepository,
};
use crate::infrastructure::repositories::youtube_metadata_repository::{
    YoutubeApiMetadataRepository, YoutubeMetadataRepository,
};
use crate::infrastructure::repositories::youtube_playlist_items_repository::{
    YoutubeApiPlaylistItemsRepository, YoutubePlaylistItemsRepository,
};
use crate::infrastructure::repositories::youtube_playlist_repository::{
    YoutubeApiPlaylistRepository, YoutubePlaylistRepository,
};
use crate::infrastructure::repositories::youtube_video_downloader_repository::{
    VideoDownloaderRepository, YtDlpVideoDownloaderRepository,
};
use crate::infrastructure::shared::domain_events::event_publisher::{
    EventPublisher, SqliteEventPublisher,
};
use crate::infrastructure::shared::domain_events::event_repository::{
    EventRepository, SqliteEventRepository,
};
use crate::infrastructure::shared::sqlite_connection;
use crate::infrastructure::shared::system_clock::{Clock, SystemClock};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Where the adapters that touch the outside world point at.
pub struct InfrastructureSettings {
    pub db_path: PathBuf,
    pub youtube_api_key: String,
    pub videos_path: PathBuf,
    pub avatars_path: PathBuf,
    pub ytdlp_path: PathBuf,
}

/// One production instance of every port implementation, shared by every
/// domain service built from it. Each SQLite-backed repository opens its own
/// connection, so the schema must already be migrated.
pub struct InfrastructureContainer {
    pub clock: Arc<dyn Clock>,
    pub playlist_repository: Arc<dyn PlaylistRepository>,
    pub channel_repository: Arc<dyn ChannelRepository>,
    pub video_repository: Arc<dyn VideoRepository>,
    pub playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    pub channel_video_repository: Arc<dyn ChannelVideoRepository>,
    pub video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    pub task_repository: Arc<dyn TaskRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub event_repository: Arc<dyn EventRepository>,
    pub youtube_playlist_repository: Arc<dyn YoutubePlaylistRepository>,
    pub youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
    pub youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
    pub youtube_channel_repository: Arc<dyn YoutubeChannelRepository>,
    pub channel_videos_repository: Arc<dyn ChannelVideosRepository>,
    pub video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    pub video_file_repository: Arc<dyn VideoFileRepository>,
    pub channel_avatar_repository: Arc<dyn ChannelAvatarRepository>,
    pub directory_repository: Arc<dyn DirectoryRepository>,
    pub ytdlp_updater: Arc<dyn YtdlpUpdater>,
}

impl InfrastructureContainer {
    pub fn new(settings: InfrastructureSettings) -> Result<Self> {
        let db_path = settings.db_path.as_path();
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);

        Ok(Self {
            playlist_repository: Arc::new(SqlitePlaylistRepository::new(sqlite_connection::open(
                db_path,
            )?)),
            channel_repository: Arc::new(SqliteChannelRepository::new(sqlite_connection::open(
                db_path,
            )?)),
            video_repository: Arc::new(SqliteVideoRepository::new(sqlite_connection::open(
                db_path,
            )?)),
            playlist_video_repository: Arc::new(SqlitePlaylistVideoRepository::new(
                sqlite_connection::open(db_path)?,
            )),
            channel_video_repository: Arc::new(SqliteChannelVideoRepository::new(
                sqlite_connection::open(db_path)?,
            )),
            video_metadata_repository: Arc::new(SqliteVideoMetadataRepository::new(
                sqlite_connection::open(db_path)?,
            )),
            task_repository: Arc::new(SqliteTaskRepository::new(
                open_shared(db_path)?,
                clock.clone(),
            )),
            event_publisher: Arc::new(SqliteEventPublisher::new(
                open_shared(db_path)?,
                clock.clone(),
            )),
            event_repository: Arc::new(SqliteEventRepository::new(open_shared(db_path)?)),
            youtube_playlist_repository: Arc::new(YoutubeApiPlaylistRepository::new(
                settings.youtube_api_key.clone(),
            )),
            youtube_playlist_items_repository: Arc::new(YoutubeApiPlaylistItemsRepository::new(
                settings.youtube_api_key.clone(),
            )),
            youtube_metadata_repository: Arc::new(YoutubeApiMetadataRepository::new(
                settings.youtube_api_key.clone(),
            )),
            youtube_channel_repository: Arc::new(YoutubeApiChannelRepository::new(
                settings.youtube_api_key,
            )),
            channel_videos_repository: Arc::new(YtDlpChannelVideosRepository::new(
                settings.ytdlp_path.clone(),
            )),
            video_downloader_repository: Arc::new(YtDlpVideoDownloaderRepository::new(
                settings.ytdlp_path,
            )),
            video_file_repository: Arc::new(FilesystemVideoFileRepository),
            channel_avatar_repository: Arc::new(FilesystemChannelAvatarRepository::new(
                settings.avatars_path,
            )),
            directory_repository: Arc::new(FilesystemDirectoryRepository::new(
                settings.videos_path,
            )),
            ytdlp_updater: Arc::new(RealYtdlpUpdater),
            clock,
        })
    }
}

fn open_shared(db_path: &Path) -> Result<Arc<Mutex<rusqlite::Connection>>> {
    Ok(Arc::new(Mutex::new(sqlite_connection::open(db_path)?)))
}
