use crate::domain::services::{VideoCategorizer, VideoCategorizerApi};
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::sync::Arc;

/// Asks the AI which of its pending categories one video fits, scheduled by
/// the video sync (`SyncVideoTask` via `VideoReconciler`) while any is
/// pending (see the `video-categorization` capability). Fails when a call
/// was rejected, so the queue retries the categories still pending.
pub struct CategorizeVideoTask {
    video_repository: Arc<dyn VideoRepository>,
    video_categorizer: Arc<VideoCategorizer>,
}

impl CategorizeVideoTask {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_categorizer: Arc<VideoCategorizer>,
    ) -> Self {
        Self {
            video_repository,
            video_categorizer,
        }
    }
}

impl TaskHandler for CategorizeVideoTask {
    fn handle(&self, payload: &str, is_last_attempt: bool) -> anyhow::Result<()> {
        let video_id = VideoRecordId::new(Task::decode_categorize_video_payload(payload)?)?;
        let Some(video) = self.video_repository.find(&video_id)? else {
            return Ok(());
        };
        self.video_categorizer.categorize(&video, is_last_attempt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::category::{
        Category, CategoryColor, CategoryMember, CategoryName, CategoryPrompt,
    };
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::llm_server::{LlmModel, LlmReply, LlmServer, OllamaUrl};
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::preferences::SummaryModel;
    use crate::domain::preferences::{CategorizationSettings, Preferences};
    use crate::domain::services::{VideoCategoryAssigner, VideoOwnerFinder};
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::video::{Video, VideoId};
    use crate::domain::video_category::{
        Verdict, VerdictAuthor, VideoCategories, VideoCategoriesChanged, VideoCategoryVerdict,
    };
    use crate::domain::video_summary::{SummaryText, VideoSummary};
    use crate::infrastructure::repositories::ollama_llm_server_repository::FakeLlmServerRepository;
    use crate::infrastructure::repositories::sqlite_category_repository::{
        CategoryRepository, SqliteCategoryRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, SqliteChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_preferences_repository::{
        PreferencesRepository, SqlitePreferencesRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_categories_repository::{
        SqliteVideoCategoriesRepository, VideoCategoriesRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::{
        SqliteVideoCategoryVerdictRepository, VideoCategoryVerdictRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_summary_repository::{
        SqliteVideoSummaryRepository, VideoSummaryRepository,
    };
    use crate::infrastructure::shared::category_lock::CategoryLock;
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    const YES: &str = r#"{"reason": "A Sicilian game.", "belongs": true}"#;
    const NO: &str = r#"{"reason": "A chess game.", "belongs": false}"#;

    #[test]
    fn it_should_store_a_yes_verdict_and_add_the_category_to_the_video() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video_categories_repository = SqliteVideoCategoriesRepository::new(db.database());
        let event_repository = SqliteEventRepository::new(db.database());
        let video = summarized_video();
        let sicilian = prompted_category("Sicilian");
        seed(&db, &video, &[&sicilian]);
        let task = categorize_video_task(&db, llm_replying(vec![reply(YES)]));

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![ai_verdict(
                &video,
                &sicilian,
                Verdict::Yes,
                "A Sicilian game."
            )]
        );
        assert_eq!(
            video_categories_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![VideoCategories::new(
                video.id.clone(),
                vec![sicilian.id.clone()]
            )]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoCategoriesChanged(VideoCategoriesChanged {
                    video_ids: vec![video.id.as_str().to_string()],
                })
            )]
        );
    }

    #[test]
    fn it_should_store_a_no_verdict_without_changing_the_video_categories() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video_categories_repository = SqliteVideoCategoriesRepository::new(db.database());
        let event_repository = SqliteEventRepository::new(db.database());
        let video = summarized_video();
        let tech = prompted_category("Tech");
        seed(&db, &video, &[&tech]);
        let task = categorize_video_task(&db, llm_replying(vec![reply(NO)]));

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![ai_verdict(&video, &tech, Verdict::No, "A chess game.")]
        );
        assert_eq!(
            video_categories_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![VideoCategories::none(video.id.clone())]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    /// The task over `db`, asking `llm`.
    fn categorize_video_task(
        db: &TestDatabase,
        llm: Arc<FakeLlmServerRepository>,
    ) -> CategorizeVideoTask {
        CategorizeVideoTask::new(
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(VideoCategorizer::new(
                Arc::new(SqlitePreferencesRepository::new(db.database())),
                Arc::new(VideoOwnerFinder::new(
                    Arc::new(SqliteChannelVideoRepository::new(db.database())),
                    Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                    Arc::new(SqliteChannelRepository::new(db.database())),
                    Arc::new(SqlitePlaylistRepository::new(db.database())),
                    "/videos",
                )),
                Arc::new(SqliteCategoryRepository::new(db.database())),
                Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
                Arc::new(SqliteVideoSummaryRepository::new(db.database())),
                llm,
                VideoCategoryAssigner::new(
                    Arc::new(SqliteVideoRepository::new(db.database())),
                    Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
                    Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
                    Arc::new(SqliteCategoryRepository::new(db.database())),
                    Arc::new(SqliteChannelVideoRepository::new(db.database())),
                    Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                    Arc::new(SqliteEventPublisher::new(
                        db.database(),
                        Arc::new(FixedClock(fixed_timestamp())),
                    )),
                    Arc::new(CategoryLock::default()),
                ),
                Arc::new(FixedClock(later())),
            )),
        )
    }

    fn run(task: &CategorizeVideoTask, video: &Video) -> Result<(), String> {
        task.handle(&payload_for(video), false)
            .map_err(|error| error.to_string())
    }

    fn payload_for(video: &Video) -> String {
        Task::CategorizeVideo {
            video_id: video.id.as_str().to_string(),
        }
        .payload()
        .to_string()
    }

    /// Stores `video` with its summary, `my_channel()` with categorization on
    /// as its owner, `categories`, and AI categorization on.
    fn seed(db: &TestDatabase, video: &Video, categories: &[&Category]) {
        SqliteVideoRepository::new(db.database())
            .save(video)
            .unwrap();
        seed_channel(db, video, true);
        SqliteVideoSummaryRepository::new(db.database())
            .save(&VideoSummary::create(
                video.id.clone(),
                SummaryText::new(SUMMARY),
                &SummaryModel::default(),
                fixed_timestamp(),
            ))
            .unwrap();
        let category_repository = SqliteCategoryRepository::new(db.database());
        categories
            .iter()
            .for_each(|category| category_repository.insert(category).unwrap());
        SqlitePreferencesRepository::new(db.database())
            .save(&categorization_on())
            .unwrap();
    }

    /// Stores `my_channel()`, with categorization on or off, as the owner of `video`.
    fn seed_channel(db: &TestDatabase, video: &Video, categorization_enabled: bool) {
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel().with_categorization_enabled(categorization_enabled))
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    fn categorization_on() -> Preferences {
        Preferences {
            ai_enabled: true,
            ollama_url: Some(OllamaUrl::new("http://mac.local:11434").unwrap()),
            categorization: CategorizationSettings {
                enabled: true,
                ..CategorizationSettings::default()
            },
            ..Preferences::default()
        }
    }

    const SUMMARY: &str = "Three online games in the Najdorf Sicilian.";

    /// A downloaded, subtitled, summarized 30-minute video.
    fn summarized_video() -> Video {
        Video::create(
            VideoId::new("yt1").unwrap(),
            "Crushing the Sicilian",
            fixed_timestamp(),
        )
        .mark_downloaded(
            Quality::High,
            "Season 2026/S2026E010101 - Crushing the Sicilian.mp4",
            None,
            Some(1800),
            fixed_timestamp(),
        )
        .mark_subtitles_fetched(
            "Season 2026/S2026E010101 - Crushing the Sicilian.en.srt",
            fixed_timestamp(),
        )
        .mark_summary_generated(fixed_timestamp())
    }

    fn prompted_category(name: &str) -> Category {
        Category::create(
            CategoryName::new(name).unwrap(),
            CategoryColor::new(CategoryColor::PALETTE[0]).unwrap(),
            fixed_timestamp(),
        )
        .with_ai_prompt(Some(
            CategoryPrompt::new(format!("The video is about {name}."))
                .unwrap()
                .unwrap(),
        ))
    }

    fn ai_verdict(
        video: &Video,
        category: &Category,
        verdict: Verdict,
        reason: &str,
    ) -> VideoCategoryVerdict {
        VideoCategoryVerdict {
            video_id: video.id.clone(),
            category_id: category.id.clone(),
            verdict,
            decided_by: VerdictAuthor::Ai,
            model: Some("qwen3:4b".to_string()),
            reason: Some(reason.to_string()),
            decided_at: later(),
        }
    }

    fn pending_event(id: i64, event: DomainEvent) -> ScheduledEvent {
        ScheduledEvent {
            id,
            event_type: event.event_type().to_string(),
            payload: event.payload().to_string(),
            retries: 0,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    fn my_channel_id() -> ChannelHandle {
        ChannelHandle::new("@somechannel").unwrap()
    }

    fn my_channel() -> Channel {
        Channel::create(
            my_channel_id(),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            ReconcileInterval::default_interval(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    /// An installed thinking `qwen3:4b` answering `chat` with `replies`, in order.
    fn llm_replying(replies: Vec<anyhow::Result<LlmReply>>) -> Arc<FakeLlmServerRepository> {
        Arc::new(FakeLlmServerRepository::new(server_with(thinking_model())).with_replies(replies))
    }

    fn thinking_model() -> LlmModel {
        LlmModel {
            name: "qwen3:4b".to_string(),
            size_bytes: 1,
            thinking: true,
        }
    }

    fn server_with(model: LlmModel) -> LlmServer {
        LlmServer {
            version: "0.40.13".to_string(),
            models: vec![model],
        }
    }

    fn reply(content: &str) -> anyhow::Result<LlmReply> {
        Ok(LlmReply {
            content: content.to_string(),
            input_tokens: 100,
            stopped_at_limit: false,
            refusal: None,
        })
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_000_000, 0).unwrap()
    }

    fn later() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(2_000_000, 0).unwrap()
    }
}
