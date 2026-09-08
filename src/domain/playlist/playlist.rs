use super::playlist_name::PlaylistName;
use crate::domain::shared::PlaylistId;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: PlaylistName,
    pub created_at: DateTime<Utc>,
}

impl Playlist {
    pub fn create(id: PlaylistId, name: PlaylistName, created_at: DateTime<Utc>) -> Self {
        Self {
            id,
            name,
            created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_build_a_playlist_from_valid_value_objects() {
        let id = PlaylistId::new("PLabc123").unwrap();
        let name = PlaylistName::new("My Playlist").unwrap();
        let created_at = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let playlist = Playlist::create(id.clone(), name.clone(), created_at);

        assert_eq!(playlist.id, id);
        assert_eq!(playlist.name, name);
        assert_eq!(playlist.created_at, created_at);
    }
}
