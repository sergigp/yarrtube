use crate::domain::services::{PlexCollectionDeleter, PlexCollectionDeleterApi};
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PlaylistDeletedPayload {
    name: String,
}

/// Reacts to `PlaylistDeleted` by deleting the Plex collection named after
/// the deleted playlist. The playlist row is already gone, so the name
/// travels on the event payload.
pub struct DeletePlexCollectionOnPlaylistDeleted {
    deleter: PlexCollectionDeleter,
}

impl DeletePlexCollectionOnPlaylistDeleted {
    pub fn new(deleter: PlexCollectionDeleter) -> Self {
        Self { deleter }
    }
}

impl EventSubscriber for DeletePlexCollectionOnPlaylistDeleted {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: PlaylistDeletedPayload = serde_json::from_str(payload)?;
        self.deleter.delete(&payload.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plex::PlexItem;
    use crate::infrastructure::repositories::plex_collection_repository::{
        FakePlexCollection, FakePlexCollectionRepository,
    };
    use std::sync::Arc;

    #[test]
    fn it_should_delete_the_collection_if_a_playlist_is_deleted() {
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items_and_collections(
            vec![PlexItem {
                rating_key: "101".to_string(),
                youtube_video_id: "yt1".to_string(),
            }],
            vec![
                FakePlexCollection {
                    rating_key: "c1".to_string(),
                    title: "Lofi beats".to_string(),
                    member_rating_keys: vec!["101".to_string()],
                },
                FakePlexCollection {
                    rating_key: "c2".to_string(),
                    title: "Other".to_string(),
                    member_rating_keys: vec![],
                },
            ],
        ));
        let subscriber = DeletePlexCollectionOnPlaylistDeleted::new(PlexCollectionDeleter::new(
            plex_repository.clone(),
        ));

        let result = handle(
            &subscriber,
            r#"{"playlist_id": "PL1", "name": "Lofi beats", "path": "music/chill"}"#,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "c2".to_string(),
                title: "Other".to_string(),
                member_rating_keys: vec![],
            }]
        );
    }

    #[test]
    fn it_should_skip_if_no_collection_matches_the_playlist_name() {
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items_and_collections(
            vec![],
            vec![FakePlexCollection {
                rating_key: "c2".to_string(),
                title: "Other".to_string(),
                member_rating_keys: vec![],
            }],
        ));
        let subscriber = DeletePlexCollectionOnPlaylistDeleted::new(PlexCollectionDeleter::new(
            plex_repository.clone(),
        ));

        let result = handle(
            &subscriber,
            r#"{"playlist_id": "PL1", "name": "Ghost", "path": "music/chill"}"#,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(plex_repository.mutations(), Vec::<String>::new());
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "c2".to_string(),
                title: "Other".to_string(),
                member_rating_keys: vec![],
            }]
        );
    }

    fn handle(
        subscriber: &DeletePlexCollectionOnPlaylistDeleted,
        payload: &str,
    ) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
