use crate::domain::category::CategoryMember;
use crate::domain::playlist::PlaylistId;
use crate::domain::services::{CategoryMemberRemover, CategoryMemberRemoverApi};
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PlaylistDeletedPayload {
    playlist_id: String,
}

/// Reacts to `PlaylistDeleted` by removing the playlist from every category
/// holding it. The categories themselves stay.
pub struct RemovePlaylistFromCategoriesOnPlaylistDeleted {
    remover: CategoryMemberRemover,
}

impl RemovePlaylistFromCategoriesOnPlaylistDeleted {
    pub fn new(remover: CategoryMemberRemover) -> Self {
        Self { remover }
    }
}

impl EventSubscriber for RemovePlaylistFromCategoriesOnPlaylistDeleted {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: PlaylistDeletedPayload = serde_json::from_str(payload)?;
        let Ok(id) = PlaylistId::new(payload.playlist_id) else {
            return Ok(());
        };

        self.remover.remove_from_all(&CategoryMember::Playlist(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::category::{Category, CategoryName};
    use crate::domain::channel::ChannelHandle;
    use crate::infrastructure::repositories::sqlite_category_repository::{
        CategoryRepository, SqliteCategoryRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    #[test]
    fn it_should_remove_the_playlist_from_every_category() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let chess = category(
            "Chess",
            &[
                playlist_member("PLendgames"),
                channel_member("@GothamChess"),
            ],
        );
        let cooking = category("Cooking", &[playlist_member("PLrecipes")]);
        let endgames = category("Endgames", &[playlist_member("PLendgames")]);
        [&chess, &cooking, &endgames]
            .into_iter()
            .for_each(|category| repository.insert(category).unwrap());
        let subscriber = RemovePlaylistFromCategoriesOnPlaylistDeleted::new(
            CategoryMemberRemover::new(repository.clone()),
        );

        let result = handle(
            &subscriber,
            r#"{"playlist_id": "PLendgames", "name": "Endgames", "path": "playlists/endgames"}"#,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(
            repository.list().unwrap(),
            vec![
                Category {
                    members: vec![channel_member("@GothamChess")],
                    ..chess
                },
                cooking,
                Category {
                    members: vec![],
                    ..endgames
                },
            ]
        );
    }

    fn category(name: &str, members: &[CategoryMember]) -> Category {
        Category::create(CategoryName::new(name).unwrap(), fixed_timestamp())
            .with_members(members.to_vec())
    }

    fn channel_member(handle: &str) -> CategoryMember {
        CategoryMember::Channel(ChannelHandle::new(handle).unwrap())
    }

    fn playlist_member(id: &str) -> CategoryMember {
        CategoryMember::Playlist(PlaylistId::new(id).unwrap())
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn handle(
        subscriber: &RemovePlaylistFromCategoriesOnPlaylistDeleted,
        payload: &str,
    ) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
