use super::playlist_name::PlaylistName;
use super::playlist_path::PlaylistPath;
use crate::domain::shared::{PlaylistId, Quality};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: PlaylistName,
    pub path: PlaylistPath,
    pub quality: Quality,
    pub created_at: DateTime<Utc>,
}

impl Playlist {
    pub fn create(
        id: PlaylistId,
        name: PlaylistName,
        path: PlaylistPath,
        quality: Quality,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            path,
            quality,
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
        let path = PlaylistPath::new("my-playlist").unwrap();
        let quality = Quality::High;
        let created_at = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let playlist =
            Playlist::create(id.clone(), name.clone(), path.clone(), quality, created_at);

        assert_eq!(playlist.id, id);
        assert_eq!(playlist.name, name);
        assert_eq!(playlist.path, path);
        assert_eq!(playlist.quality, quality);
        assert_eq!(playlist.created_at, created_at);
    }
}
