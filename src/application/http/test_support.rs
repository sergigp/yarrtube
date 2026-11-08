//! Test helpers shared by the acceptance tests of the HTTP resources.

use crate::domain::category::{Category, CategoryColor, CategoryName};
use crate::domain::services::{VideoCategoryAssigner, VideoCategoryEditor, VideoOwnerFinder};
use crate::domain::video::Video;
use crate::domain::video_category::{Verdict, VerdictAuthor, VideoCategoryVerdict};
use crate::infrastructure::repositories::sqlite_category_repository::SqliteCategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::SqliteChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::SqliteChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
use crate::infrastructure::repositories::sqlite_video_categories_repository::SqliteVideoCategoriesRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::SqliteVideoCategoryVerdictRepository;
use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
use crate::infrastructure::shared::category_lock::CategoryLock;
use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
use crate::infrastructure::shared::sqlite_connection::Database;
use crate::infrastructure::shared::system_clock::FixedClock;
use chrono::{DateTime, Utc};
use std::sync::Arc;

pub fn video_category_editor(database: Database) -> VideoCategoryEditor {
    let clock = Arc::new(FixedClock(fixed_timestamp()));
    VideoCategoryEditor::new(
        Arc::new(SqliteVideoRepository::new(database.clone())),
        Arc::new(VideoOwnerFinder::new(
            Arc::new(SqliteChannelVideoRepository::new(database.clone())),
            Arc::new(SqlitePlaylistVideoRepository::new(database.clone())),
            Arc::new(SqliteChannelRepository::new(database.clone())),
            Arc::new(SqlitePlaylistRepository::new(database.clone())),
            "/videos",
        )),
        Arc::new(SqliteCategoryRepository::new(database.clone())),
        Arc::new(SqliteChannelRepository::new(database.clone())),
        Arc::new(SqlitePlaylistRepository::new(database.clone())),
        Arc::new(SqliteChannelVideoRepository::new(database.clone())),
        Arc::new(SqlitePlaylistVideoRepository::new(database.clone())),
        Arc::new(SqliteVideoCategoryVerdictRepository::new(database.clone())),
        video_category_assigner(database.clone()),
        Arc::new(SqliteTaskRepository::new(database, clock.clone())),
        clock,
    )
}

pub fn named_category(name: &str) -> Category {
    Category::create(
        CategoryName::new(name).unwrap(),
        CategoryColor::new(CategoryColor::PALETTE[0]).unwrap(),
        fixed_timestamp(),
    )
}

pub fn ai_verdict(video: &Video, category: &Category, verdict: Verdict) -> VideoCategoryVerdict {
    VideoCategoryVerdict {
        video_id: video.id.clone(),
        category_id: category.id.clone(),
        verdict,
        decided_by: VerdictAuthor::Ai,
        model: Some("qwen3:4b".to_string()),
        reason: Some("A reason.".to_string()),
        decided_at: fixed_timestamp(),
    }
}

pub fn user_verdict(video: &Video, category: &Category, verdict: Verdict) -> VideoCategoryVerdict {
    VideoCategoryVerdict {
        decided_by: VerdictAuthor::User,
        model: None,
        reason: None,
        ..ai_verdict(video, category, verdict)
    }
}

pub fn video_category_assigner(database: Database) -> VideoCategoryAssigner {
    VideoCategoryAssigner::new(
        Arc::new(SqliteVideoRepository::new(database.clone())),
        Arc::new(SqliteVideoCategoryVerdictRepository::new(database.clone())),
        Arc::new(SqliteVideoCategoriesRepository::new(database.clone())),
        Arc::new(SqliteCategoryRepository::new(database.clone())),
        Arc::new(SqliteChannelVideoRepository::new(database.clone())),
        Arc::new(SqlitePlaylistVideoRepository::new(database.clone())),
        Arc::new(SqliteEventPublisher::new(
            database,
            Arc::new(FixedClock(fixed_timestamp())),
        )),
        Arc::new(CategoryLock::default()),
    )
}

fn fixed_timestamp() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
}
