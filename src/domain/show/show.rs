use crate::domain::channel::Channel;
use crate::domain::playlist::Playlist;
use std::path::{Path, PathBuf};

/// A tracked playlist or channel seen as a TV show: what its `tvshow.nfo`
/// says and where its `poster.jpg` is copied from (a channel's avatar; a
/// playlist has none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Show {
    pub title: String,
    pub uniqueid: String,
    pub poster_source: Option<PathBuf>,
}

impl Show {
    pub fn from_channel(channel: &Channel, avatars_dir: &Path) -> Self {
        Self {
            title: channel.name.clone(),
            uniqueid: channel.youtube_channel_id.clone(),
            poster_source: channel
                .avatar_filename
                .as_ref()
                .map(|avatar| avatars_dir.join(avatar)),
        }
    }

    pub fn from_playlist(playlist: &Playlist) -> Self {
        Self {
            title: playlist.name.as_str().to_string(),
            uniqueid: playlist.id.as_str().to_string(),
            poster_source: None,
        }
    }
}
