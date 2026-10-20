use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use std::fmt;

/// A source grouped into a category, one variant per kind of member.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CategoryMember {
    Channel(ChannelHandle),
    Playlist(PlaylistId),
}

impl fmt::Display for CategoryMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Channel(handle) => write!(f, "channel {handle}"),
            Self::Playlist(id) => write!(f, "playlist {id}"),
        }
    }
}
