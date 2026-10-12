use crate::domain::channel::Channel;
use crate::domain::playlist::{Playlist, PlaylistPath};
use crate::domain::show::Show;
use crate::domain::video::resolve_output_dir;
use crate::domain::video_metadata::render_show_nfo;
use crate::infrastructure::repositories::filesystem_channel_avatar_repository::ChannelAvatarRepository;
use crate::infrastructure::repositories::filesystem_show_file_repository::ShowFileRepository;
use std::sync::Arc;

/// Writes a tracked playlist's or channel's show files (`tvshow.nfo`, and
/// `poster.jpg` from a channel's avatar) at the root of its directory. Run at
/// the start of every reconcile, so creation, rename, avatar change and hand
/// deletion all converge without a dedicated event.
pub struct ShowFileWriter {
    show_file_repository: Arc<dyn ShowFileRepository>,
    channel_avatar_repository: Arc<dyn ChannelAvatarRepository>,
    videos_path: String,
}

impl ShowFileWriter {
    pub fn new(
        show_file_repository: Arc<dyn ShowFileRepository>,
        channel_avatar_repository: Arc<dyn ChannelAvatarRepository>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            show_file_repository,
            channel_avatar_repository,
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
    fn write_channel_show(&self, channel: &Channel) -> anyhow::Result<()> {
        let poster_source = channel
            .avatar_filename
            .as_deref()
            .map(|avatar| self.channel_avatar_repository.path_of(avatar));
        self.write_show(&Show::from_channel(channel, poster_source), &channel.path)
    }

    fn write_playlist_show(&self, playlist: &Playlist) -> anyhow::Result<()> {
        self.write_show(&Show::from_playlist(playlist), &playlist.path)
    }
}

impl ShowFileWriter {
    fn write_show(&self, show: &Show, path: &PlaylistPath) -> anyhow::Result<()> {
        let show_dir = resolve_output_dir(&self.videos_path, path.as_str());
        self.show_file_repository
            .write_show_nfo(&show_dir, &render_show_nfo(show))?;
        match &show.poster_source {
            Some(source) => self.show_file_repository.copy_poster(&show_dir, source),
            None => Ok(()),
        }
    }
}
