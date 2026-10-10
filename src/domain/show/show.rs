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
    pub fn from_channel(_channel: &Channel, _avatars_dir: &Path) -> Self {
        Self {
            title: String::new(),
            uniqueid: String::new(),
            poster_source: None,
        }
    }

    pub fn from_playlist(_playlist: &Playlist) -> Self {
        Self {
            title: String::new(),
            uniqueid: String::new(),
            poster_source: None,
        }
    }
}
