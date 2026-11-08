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
    use crate::application::tasks::log_capture::captured_log_lines;
    use crate::domain::category::{Category, CategoryColor, CategoryName, CategoryPrompt};
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::llm_server::{LlmChat, LlmModel, LlmReply, LlmServer, OllamaUrl};
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::preferences::SummaryModel;
    use crate::domain::preferences::{
        CategorizationModel, CategorizationPrompt, CategorizationSettings, ContextLength,
        Preferences, Temperature,
    };
    use crate::domain::services::{VideoCategoryAssigner, VideoOwnerFinder};
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::video::{Video, VideoChapter, VideoId};
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
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_summary_repository::{
        SqliteVideoSummaryRepository, VideoSummaryRepository,
    };
    use crate::infrastructure::shared::category_lock::CategoryLock;
    use crate::infrastructure::shared::domain_events::event_publisher::{
        EventPublisher, SqliteEventPublisher,
    };
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

    #[test]
    fn it_should_ask_with_the_saved_context_length_temperature_and_thinking() {
        let db = TestDatabase::new();
        let llm = llm_replying(vec![reply(YES)]);
        let video = summarized_video();
        seed(&db, &video, &[&prompted_category("Sicilian")]);
        SqlitePreferencesRepository::new(db.database())
            .save(&Preferences {
                categorization: CategorizationSettings {
                    context_length: ContextLength::new(2048).unwrap(),
                    temperature: Temperature::new(0.7).unwrap(),
                    thinking: false,
                    ..categorization_on().categorization
                },
                ..categorization_on()
            })
            .unwrap();
        let task = categorize_video_task(&db, llm.clone());

        run(&task, &video).unwrap();

        assert_eq!(
            llm.chats()
                .into_iter()
                .map(|chat| (
                    chat.context_length,
                    chat.temperature,
                    chat.thinking,
                    chat.max_output_tokens
                ))
                .collect::<Vec<_>>(),
            vec![(2048, 0.7, false, 2048)]
        );
    }

    #[test]
    fn it_should_send_the_video_details_and_category_prompt_with_the_categorization_settings() {
        let db = TestDatabase::new();
        let llm = llm_replying(vec![reply(YES)]);
        let video = Video {
            chapters: Some(vec![VideoChapter {
                start_seconds: 0,
                end_seconds: 60,
                title: "Intro".to_string(),
            }]),
            ..summarized_video()
        };
        seed(&db, &video, &[&prompted_category("Sicilian")]);
        let task = categorize_video_task(&db, llm.clone());

        run(&task, &video).unwrap();

        assert_eq!(
            llm.chats(),
            vec![LlmChat {
                model: "qwen3:4b".to_string(),
                system: CategorizationPrompt::shipped().as_str().to_string(),
                user: "Title: Crushing the Sicilian\n\
                       Channel: Some Channel\n\
                       Duration: 30 minutes\n\
                       Chapters:\n\
                       0:00 Intro\n\
                       Summary: Three online games in the Najdorf Sicilian.\n\
                       \n\
                       Category: Sicilian\n\
                       Description:\n\
                       The video is about Sicilian.\n\
                       \n\
                       Answer in JSON: \"reason\", one sentence on what the video covers and how it fits, then \"belongs\", true or false."
                    .to_string(),
                context_length: 4096,
                temperature: 0.0,
                thinking: true,
                max_output_tokens: 3072,
                format: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "reason": {"type": "string"},
                        "belongs": {"type": "boolean"},
                    },
                    "required": ["reason", "belongs"],
                })),
            }]
        );
    }

    #[test]
    fn it_should_read_the_answer_after_leaked_thinking() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video = summarized_video();
        let sicilian = prompted_category("Sicilian");
        seed(&db, &video, &[&sicilian]);
        let answer = r#"<think>Is {"belongs": false} right?</think>
Sure: {"reason": "A Sicilian game.", "belongs": true}"#;
        let task = categorize_video_task(&db, llm_replying(vec![reply(answer)]));

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
    }

    #[test]
    fn it_should_only_ask_about_categories_pending_when_it_runs() {
        let db = TestDatabase::new();
        let category_repository = SqliteCategoryRepository::new(db.database());
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let llm = llm_replying(vec![reply(YES)]);
        let video = summarized_video();
        let sicilian = prompted_category("Sicilian");
        let tech = unprompted_category("Tech");
        seed(&db, &video, &[&sicilian, &tech]);
        let task = categorize_video_task(&db, llm.clone());
        category_repository.delete(&sicilian.id).unwrap();
        let tech = tech.with_ai_prompt(prompted_category("Tech").ai_prompt);
        category_repository.update(&tech).unwrap();

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(asked_categories(&llm), vec!["Tech"]);
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![ai_verdict(&video, &tech, Verdict::Yes, "A Sicilian game.")]
        );
    }

    #[test]
    fn it_should_not_evaluate_a_pair_with_a_user_verdict() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let llm = llm_replying(vec![reply(YES)]);
        let video = summarized_video();
        let sicilian = prompted_category("Sicilian");
        let tech = prompted_category("Tech");
        seed(&db, &video, &[&sicilian, &tech]);
        let user_yes = VideoCategoryVerdict::by_user(
            video.id.clone(),
            tech.id.clone(),
            true,
            fixed_timestamp(),
        );
        verdict_repository.save(&user_yes).unwrap();
        let task = categorize_video_task(&db, llm.clone());

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(asked_categories(&llm), vec!["Sicilian"]);
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            by_category([
                ai_verdict(&video, &sicilian, Verdict::Yes, "A Sicilian game."),
                user_yes,
            ])
        );
    }

    #[test]
    fn it_should_do_nothing_when_the_conditions_no_longer_hold() {
        let source_off = |db: &TestDatabase| {
            SqliteChannelRepository::new(db.database())
                .update(&my_channel().with_categorization_enabled(false))
                .unwrap()
        };
        let categorization_off = |db: &TestDatabase| {
            SqlitePreferencesRepository::new(db.database())
                .save(&Preferences {
                    ai_enabled: false,
                    ..categorization_on()
                })
                .unwrap()
        };

        let asked = [source_off, categorization_off].map(|turn_off| {
            let db = TestDatabase::new();
            let llm = llm_replying(vec![reply(YES)]);
            let video = summarized_video();
            seed(&db, &video, &[&prompted_category("Sicilian")]);
            let task = categorize_video_task(&db, llm.clone());
            turn_off(&db);

            let result = run(&task, &video);

            (result, asked_categories(&llm))
        });

        assert_eq!(asked, [(Ok(()), vec![]), (Ok(()), vec![])]);
    }

    #[test]
    fn it_should_not_store_a_verdict_for_a_category_deleted_during_its_call() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video_categories_repository = SqliteVideoCategoriesRepository::new(db.database());
        let event_repository = SqliteEventRepository::new(db.database());
        let video = summarized_video();
        let sicilian = prompted_category("Sicilian");
        seed(&db, &video, &[&sicilian]);
        let (database, deleted) = (db.database(), sicilian.id.clone());
        let llm = Arc::new(
            FakeLlmServerRepository::new(server_with(thinking_model()))
                .with_replies(vec![reply(YES)])
                .with_on_chat(move || {
                    SqliteCategoryRepository::new(database.clone())
                        .delete(&deleted)
                        .unwrap()
                }),
        );
        let task = categorize_video_task(&db, llm);

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![]
        );
        assert_eq!(
            video_categories_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![VideoCategories::none(video.id.clone())]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_store_a_verdict_for_a_video_removed_during_its_call() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let event_repository = SqliteEventRepository::new(db.database());
        let video = summarized_video();
        seed(&db, &video, &[&prompted_category("Sicilian")]);
        let (database, removed) = (db.database(), video.id.clone());
        let llm = Arc::new(
            FakeLlmServerRepository::new(server_with(thinking_model()))
                .with_replies(vec![reply(YES)])
                .with_on_chat(move || {
                    SqliteVideoRepository::new(database.clone())
                        .delete(&removed)
                        .unwrap()
                }),
        );
        let task = categorize_video_task(&db, llm);

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_override_a_user_verdict_set_during_its_call() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video_categories_repository = SqliteVideoCategoriesRepository::new(db.database());
        let video = summarized_video();
        let tech = prompted_category("Tech");
        seed(&db, &video, &[&tech]);
        let user_no = VideoCategoryVerdict::by_user(
            video.id.clone(),
            tech.id.clone(),
            false,
            fixed_timestamp(),
        );
        let (database, decided) = (db.database(), user_no.clone());
        let llm = Arc::new(
            FakeLlmServerRepository::new(server_with(thinking_model()))
                .with_replies(vec![reply(YES)])
                .with_on_chat(move || {
                    SqliteVideoCategoryVerdictRepository::new(database.clone())
                        .save(&decided)
                        .unwrap()
                }),
        );
        let task = categorize_video_task(&db, llm);

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![user_no]
        );
        assert_eq!(
            video_categories_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![VideoCategories::none(video.id.clone())]
        );
    }

    #[test]
    fn it_should_not_store_an_answer_to_a_prompt_changed_during_its_call() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video = summarized_video();
        let tech = prompted_category("Tech");
        seed(&db, &video, &[&tech]);
        let (database, edited) = (
            db.database(),
            tech.clone().with_ai_prompt(Some(
                CategoryPrompt::new("The video is about cooking.")
                    .unwrap()
                    .unwrap(),
            )),
        );
        let llm = Arc::new(
            FakeLlmServerRepository::new(server_with(thinking_model()))
                .with_replies(vec![reply(YES)])
                .with_on_chat(move || {
                    SqliteCategoryRepository::new(database.clone())
                        .update(&edited)
                        .unwrap()
                }),
        );
        let task = categorize_video_task(&db, llm);

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![]
        );
    }

    #[test]
    fn it_should_not_keep_a_verdict_for_a_video_removed_while_it_was_stored() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video_categories_repository = SqliteVideoCategoriesRepository::new(db.database());
        let video = summarized_video();
        seed(&db, &video, &[&prompted_category("Sicilian")]);
        let task = CategorizeVideoTask::new(
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
                llm_replying(vec![reply(YES)]),
                VideoCategoryAssigner::new(
                    Arc::new(SqliteVideoRepository::new(db.database())),
                    Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
                    Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
                    Arc::new(SqliteCategoryRepository::new(db.database())),
                    Arc::new(SqliteChannelVideoRepository::new(db.database())),
                    Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                    Arc::new(RemovingVideoOnPublish {
                        publisher: SqliteEventPublisher::new(
                            db.database(),
                            Arc::new(FixedClock(fixed_timestamp())),
                        ),
                        video_repository: SqliteVideoRepository::new(db.database()),
                        video_id: video.id.clone(),
                    }),
                    Arc::new(CategoryLock::default()),
                ),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![]
        );
        assert_eq!(
            video_categories_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![VideoCategories::none(video.id.clone())]
        );
    }

    #[test]
    fn it_should_fail_after_trying_every_category_when_a_call_is_rejected() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video = summarized_video();
        let (sicilian, tech) = (prompted_category("Sicilian"), prompted_category("Tech"));
        seed(&db, &video, &[&sicilian, &tech]);
        let task = categorize_video_task(
            &db,
            llm_replying(vec![reply("It might belong."), reply(NO)]),
        );

        let result = run(&task, &video);

        assert_eq!(
            result,
            Err("the model couldn't answer for Sicilian (its answer isn't usable)".to_string())
        );
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![ai_verdict(&video, &tech, Verdict::No, "A chess game.")]
        );
    }

    #[test]
    fn it_should_retry_only_the_categories_still_pending() {
        let db = TestDatabase::new();
        let video = summarized_video();
        seed(
            &db,
            &video,
            &[&prompted_category("Sicilian"), &prompted_category("Tech")],
        );
        let first = categorize_video_task(
            &db,
            llm_replying(vec![reply(YES), reply("It might belong.")]),
        );
        run(&first, &video).unwrap_err();
        let llm = llm_replying(vec![reply(NO)]);
        let retry = categorize_video_task(&db, llm.clone());

        let result = run(&retry, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(asked_categories(&llm), vec!["Tech"]);
    }

    #[test]
    fn it_should_mark_rejected_categories_unavailable_on_the_last_attempt() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video_categories_repository = SqliteVideoCategoriesRepository::new(db.database());
        let video = summarized_video();
        let (food, sicilian, tech) = (
            prompted_category("Food"),
            prompted_category("Sicilian"),
            prompted_category("Tech"),
        );
        seed(&db, &video, &[&food, &sicilian, &tech]);
        let stopped_at_limit = Ok(LlmReply {
            stopped_at_limit: true,
            ..reply(YES).unwrap()
        });
        let task = categorize_video_task(
            &db,
            llm_replying(vec![
                stopped_at_limit,
                Ok(LlmReply::refused("model is too large")),
                reply("It might belong."),
            ]),
        );

        let result = run_last_attempt(&task, &video);

        assert_eq!(
            result,
            Err(
                "the model couldn't answer for Food (it hit the 3072-token output limit), \
                 Sicilian (Ollama refused the call: model is too large), \
                 Tech (its answer isn't usable)"
                    .to_string()
            )
        );
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            by_category(
                [&food, &sicilian, &tech].map(|category| unavailable_verdict(&video, category))
            )
        );
        assert_eq!(
            video_categories_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![VideoCategories::none(video.id.clone())]
        );
    }

    #[test]
    fn it_should_not_mark_a_category_unavailable_when_the_settings_changed_during_its_attempt() {
        let changes: [fn(&TestDatabase, &Category); 2] = [
            |db, _| {
                SqlitePreferencesRepository::new(db.database())
                    .save(&Preferences {
                        categorization: CategorizationSettings {
                            model: CategorizationModel::new("gemma3:4b").unwrap(),
                            ..categorization_on().categorization
                        },
                        ..categorization_on()
                    })
                    .unwrap()
            },
            |db, category| {
                SqliteCategoryRepository::new(db.database())
                    .update(
                        &category.clone().with_ai_prompt(Some(
                            CategoryPrompt::new("The video is about cooking.")
                                .unwrap()
                                .unwrap(),
                        )),
                    )
                    .unwrap()
            },
        ];

        let verdicts = changes.map(|change| {
            let db = Arc::new(TestDatabase::new());
            let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
            let video = summarized_video();
            let tech = prompted_category("Tech");
            seed(&db, &video, &[&tech]);
            let (changed_db, changed) = (db.clone(), tech.clone());
            let llm = Arc::new(
                FakeLlmServerRepository::new(server_with(thinking_model()))
                    .with_replies(vec![reply("It might belong.")])
                    .with_on_chat(move || change(&changed_db, &changed)),
            );
            let task = categorize_video_task(&db, llm);

            run_last_attempt(&task, &video).unwrap_err();

            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap()
        });

        assert_eq!(verdicts, [vec![], vec![]]);
    }

    #[test]
    fn it_should_complete_without_verdicts_when_ollama_is_unreachable() {
        let unreachable = FakeLlmServerRepository::failing();
        let model_missing = FakeLlmServerRepository::new(server_with(LlmModel {
            name: "llama3:8b".to_string(),
            ..thinking_model()
        }));
        let down_during_call = FakeLlmServerRepository::new(server_with(thinking_model()))
            .with_replies(vec![Err(anyhow::anyhow!("connection reset"))]);

        let outcomes = [unreachable, model_missing, down_during_call].map(|llm| {
            let db = TestDatabase::new();
            let video = summarized_video();
            seed(&db, &video, &[&prompted_category("Sicilian")]);
            let task = categorize_video_task(&db, Arc::new(llm));

            let result = run_last_attempt(&task, &video);

            let verdicts = SqliteVideoCategoryVerdictRepository::new(db.database())
                .find_many(std::slice::from_ref(&video.id))
                .unwrap();
            (result, verdicts)
        });

        assert_eq!(
            outcomes,
            [(Ok(()), vec![]), (Ok(()), vec![]), (Ok(()), vec![])]
        );
    }

    #[test]
    fn it_should_log_each_verdict() {
        let db = TestDatabase::new();
        let video = summarized_video();
        seed(
            &db,
            &video,
            &[&prompted_category("Sicilian"), &prompted_category("Tech")],
        );
        let task = categorize_video_task(
            &db,
            llm_replying(vec![reply(YES), reply("It might belong.")]),
        );

        let lines = captured_log_lines(|| {
            run(&task, &video).unwrap_err();
        });

        let verdict = lines
            .iter()
            .find(|line| line.contains("category=Sicilian"))
            .unwrap();
        let rejected = lines
            .iter()
            .find(|line| line.contains("category=Tech"))
            .unwrap();
        assert!(verdict.starts_with("INFO "));
        assert!(verdict.contains(&format!("video_id={}", video.id)));
        assert!(verdict.contains("verdict=YES"));
        assert!(verdict.contains("reason=A Sicilian game."));
        assert!(verdict.contains("input_tokens=100"));
        assert!(verdict.contains("duration_seconds="));
        assert!(rejected.starts_with("ERROR "));
        assert!(rejected.contains("reason=its answer isn't usable"));
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

    fn run_last_attempt(task: &CategorizeVideoTask, video: &Video) -> Result<(), String> {
        task.handle(&payload_for(video), true)
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

    fn unprompted_category(name: &str) -> Category {
        prompted_category(name).with_ai_prompt(None)
    }

    /// The category each chat asked about, in order.
    fn asked_categories(llm: &FakeLlmServerRepository) -> Vec<String> {
        llm.chats()
            .iter()
            .filter_map(|chat| {
                chat.user
                    .lines()
                    .find_map(|line| line.strip_prefix("Category: "))
                    .map(str::to_string)
            })
            .collect()
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

    /// `verdicts` in the order they are found: by category id.
    fn by_category<const N: usize>(
        mut verdicts: [VideoCategoryVerdict; N],
    ) -> Vec<VideoCategoryVerdict> {
        verdicts.sort_by(|a, b| a.category_id.cmp(&b.category_id));
        verdicts.to_vec()
    }

    fn unavailable_verdict(video: &Video, category: &Category) -> VideoCategoryVerdict {
        VideoCategoryVerdict {
            verdict: Verdict::Unavailable,
            reason: None,
            ..ai_verdict(video, category, Verdict::Unavailable, "")
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

    /// Publishes through the real outbox, then removes `video_id`: a video
    /// removed between its verdict being stored and the categories announced.
    struct RemovingVideoOnPublish {
        publisher: SqliteEventPublisher,
        video_repository: SqliteVideoRepository,
        video_id: VideoRecordId,
    }

    impl EventPublisher for RemovingVideoOnPublish {
        fn publish(&self, event: &DomainEvent) -> anyhow::Result<()> {
            self.publisher.publish(event)?;
            self.video_repository.delete(&self.video_id)
        }
    }
}
