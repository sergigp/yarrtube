use crate::domain::services::{PlexCollectionDeleter, PlexCollectionDeleterApi};
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ChannelDeletedPayload {
    name: String,
}

/// Reacts to `ChannelDeleted` by deleting the Plex collection named after
/// the deleted channel. The channel row is already gone, so the name
/// travels on the event payload.
pub struct DeletePlexCollectionOnChannelDeleted {
    deleter: PlexCollectionDeleter,
}

impl DeletePlexCollectionOnChannelDeleted {
    pub fn new(deleter: PlexCollectionDeleter) -> Self {
        Self { deleter }
    }
}

impl EventSubscriber for DeletePlexCollectionOnChannelDeleted {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: ChannelDeletedPayload = serde_json::from_str(payload)?;
        self.deleter.delete(&payload.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::plex_collection_repository::{
        FakePlexCollection, FakePlexCollectionRepository,
    };
    use std::sync::Arc;

    #[test]
    fn it_should_delete_the_collection_if_a_channel_is_deleted() {
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items_and_collections(
            vec![],
            vec![FakePlexCollection {
                rating_key: "c1".to_string(),
                title: "Some Channel".to_string(),
                member_rating_keys: vec![],
            }],
        ));
        let subscriber = DeletePlexCollectionOnChannelDeleted::new(PlexCollectionDeleter::new(
            plex_repository.clone(),
        ));

        let result = handle(
            &subscriber,
            r#"{"channel_id": "@somechannel", "name": "Some Channel", "path": "creators/somechannel"}"#,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(plex_repository.collections(), vec![]);
    }

    fn handle(
        subscriber: &DeletePlexCollectionOnChannelDeleted,
        payload: &str,
    ) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
