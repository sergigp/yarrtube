use crate::domain::channel::Channel;
use crate::domain::playlist::Playlist;
use std::path::PathBuf;

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
    /// `poster_source` is the channel's stored avatar, when it has one.
    pub fn from_channel(channel: &Channel, poster_source: Option<PathBuf>) -> Self {
        Self {
            title: channel.name.clone(),
            uniqueid: channel.youtube_channel_id.clone(),
            poster_source,
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
