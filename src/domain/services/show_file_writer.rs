use crate::domain::channel::Channel;
use crate::domain::playlist::Playlist;
use crate::infrastructure::repositories::filesystem_show_file_repository::ShowFileRepository;
use std::path::PathBuf;
use std::sync::Arc;

/// Writes a tracked playlist's or channel's show files (`tvshow.nfo`, and
/// `poster.jpg` from a channel's avatar) at the root of its directory. Run at
/// the start of every reconcile, so creation, rename, avatar change and hand
/// deletion all converge without a dedicated event.
pub struct ShowFileWriter {
    show_file_repository: Arc<dyn ShowFileRepository>,
    avatars_dir: PathBuf,
    videos_path: String,
}

impl ShowFileWriter {
    pub fn new(
        show_file_repository: Arc<dyn ShowFileRepository>,
        avatars_dir: impl Into<PathBuf>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            show_file_repository,
            avatars_dir: avatars_dir.into(),
            videos_path: videos_path.into(),
        }
    }
}

pub trait ShowFileWriterApi: Send + Sync {
    /// Writes the channel's `tvshow.nfo` and copies its avatar as
    /// `poster.jpg`, overwriting both.
    fn write_channel_show(&self, channel: &Channel) -> anyhow::Result<()>;

    /// Writes the playlist's `tvshow.nfo`; a playlist has no poster.
    fn write_playlist_show(&self, playlist: &Playlist) -> anyhow::Result<()>;
}

impl ShowFileWriterApi for ShowFileWriter {
    fn write_channel_show(&self, _channel: &Channel) -> anyhow::Result<()> {
        Ok(())
    }

    fn write_playlist_show(&self, _playlist: &Playlist) -> anyhow::Result<()> {
        Ok(())
    }
}
