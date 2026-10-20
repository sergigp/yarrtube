use crate::domain::category::CategoryMember;
use crate::domain::channel::ChannelHandle;
use crate::domain::services::{CategoryMemberRemover, CategoryMemberRemoverApi};
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ChannelDeletedPayload {
    channel_id: String,
}

/// Reacts to `ChannelDeleted` by removing the channel from every category
/// holding it. The categories themselves stay.
pub struct RemoveChannelFromCategoriesOnChannelDeleted {
    remover: CategoryMemberRemover,
}

impl RemoveChannelFromCategoriesOnChannelDeleted {
    pub fn new(remover: CategoryMemberRemover) -> Self {
        Self { remover }
    }
}

impl EventSubscriber for RemoveChannelFromCategoriesOnChannelDeleted {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: ChannelDeletedPayload = serde_json::from_str(payload)?;
        let Ok(handle) = ChannelHandle::new(payload.channel_id) else {
            return Ok(());
        };

        self.remover
            .remove_from_all(&CategoryMember::Channel(handle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::category::{Category, CategoryName};
    use crate::domain::playlist::PlaylistId;
    use crate::infrastructure::repositories::sqlite_category_repository::{
        CategoryRepository, SqliteCategoryRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::{Database, TestDatabase};
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::sync::Arc;

    #[test]
    fn it_should_remove_the_channel_from_every_category() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let chess = category(
            "Chess",
            &[channel_member("@Hikaru"), channel_member("@GothamChess")],
        );
        let cooking = category("Cooking", &[playlist_member("PLrecipes")]);
        let streams = category("Streams", &[channel_member("@Hikaru")]);
        [&chess, &cooking, &streams]
            .into_iter()
            .for_each(|category| repository.insert(category).unwrap());
        let subscriber = RemoveChannelFromCategoriesOnChannelDeleted::new(
            CategoryMemberRemover::new(repository.clone()),
        );

        let result = handle(
            &subscriber,
            r#"{"channel_id": "@Hikaru", "name": "Hikaru", "path": "channels/hikaru"}"#,
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
                    ..streams
                },
            ]
        );
    }

    #[test]
    fn it_should_ignore_an_invalid_channel_id() {
        let result = handle(
            &any_subscriber(),
            r#"{"channel_id": "", "name": "Hikaru", "path": "channels/hikaru"}"#,
        );

        assert_eq!(result, Ok(()));
    }

    /// A subscriber for tests whose payload is rejected before removing
    /// anything. Its repository sits on an unmigrated in-memory database, so
    /// a payload that wrongly got through would fail loudly instead of
    /// passing.
    fn any_subscriber() -> RemoveChannelFromCategoriesOnChannelDeleted {
        RemoveChannelFromCategoriesOnChannelDeleted::new(CategoryMemberRemover::new(Arc::new(
            SqliteCategoryRepository::new(Database::single(Connection::open_in_memory().unwrap())),
        )))
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
        subscriber: &RemoveChannelFromCategoriesOnChannelDeleted,
        payload: &str,
    ) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
