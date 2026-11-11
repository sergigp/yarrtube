use crate::domain::services::{VideoCategorizer, VideoCategorizerApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
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
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::CategorizeVideo { video_id } = task else {
            return Err(unexpected_task(task));
        };
        let video_id = VideoId::new(video_id.clone())?;
        let Some(video) = self.video_repository.find(&video_id)? else {
            return Ok(TaskOutcome::Done);
        };
        self.video_categorizer
            .categorize(&video)
            .map(TaskOutcome::from)
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
    use crate::domain::llm_server::{
        LlmChat, LlmModel, LlmReply, LlmServer, MIN_CHAT_TIMEOUT, OllamaUrl,
    };
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::preferences::SummaryModel;
    use crate::domain::preferences::{
        CategorizationModel, CategorizationPrompt, CategorizationSettings, ContextLength,
        Preferences, Temperature,
    };
    use crate::domain::services::{VideoCategoryAssigner, VideoOwnerFinder};
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::video::Chapters;
    use crate::domain::video::{
        RetryAttempts, StepState, StepStatus, Video, VideoChapter, YoutubeVideoId,
    };
    use crate::domain::video_category::{
        Verdict, VerdictAuthor, VideoCategories, VideoCategoriesChanged, VideoCategoryVerdict,
    };
    use crate::domain::video_summary::{SummaryText, VideoSummary};
    use crate::infrastructure::repositories::ollama_llm_server_repository::{
        FakeLlmServerRepository, unreachable_error,
    };
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
    use std::time::Duration;

    const YES: &str = r#"{"reason": "A Sicilian game.", "belongs": true}"#;
    const NO: &str = r#"{"reason": "A chess game.", "belongs": false}"#;

    #[test]
    fn it_should_ask_about_every_pending_category_in_one_call() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video = summarized_video();
        let (sicilian, tech, ai_news) = (
            prompted_category("Sicilian"),
            prompted_category("Tech"),
            prompted_category("AI news"),
        );
        seed(&db, &video, &[&sicilian, &tech, &ai_news]);
        let llm = llm_replying(vec![reply(&answers(&[
            ("Sicilian", YES),
            ("Tech", NO),
            ("AI news", NO),
        ]))]);
        let task = categorize_video_task(&db, llm.clone());

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            asked_categories(&llm),
            vec![vec!["AI news", "Sicilian", "Tech"]]
        );
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            by_category([
                ai_verdict(&video, &sicilian, Verdict::Yes, "A Sicilian game."),
                ai_verdict(&video, &tech, Verdict::No, "A chess game."),
                ai_verdict(&video, &ai_news, Verdict::No, "A chess game."),
            ])
        );
    }

    #[test]
    fn it_should_store_a_yes_verdict_and_add_the_category_to_the_video() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video_categories_repository = SqliteVideoCategoriesRepository::new(db.database());
        let event_repository = SqliteEventRepository::new(db.database());
        let video = summarized_video();
        let sicilian = prompted_category("Sicilian");
        seed(&db, &video, &[&sicilian]);
        let task = categorize_video_task(
            &db,
            llm_replying(vec![reply(&answers(&[("Sicilian", YES)]))]),
        );

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
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
        let llm = llm_replying(vec![reply(&answers(&[("Tech", NO)]))]);
        let task = categorize_video_task(&db, llm);

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
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
        let llm = llm_replying(vec![reply(&answers(&[("Sicilian", YES)]))]);
        let video = summarized_video();
        seed(&db, &video, &[&prompted_category("Sicilian")]);
        SqlitePreferencesRepository::new(db.database())
            .save(&Preferences {
                categorization: CategorizationSettings {
                    context_length: Some(ContextLength::new(2048).unwrap()),
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
    fn it_should_send_the_video_details_and_category_prompts_with_the_categorization_settings() {
        let db = TestDatabase::new();
        let llm = llm_replying(vec![reply(&answers(&[
            ("Chess", YES),
            ("Sicilian", YES),
            ("Tech", NO),
        ]))]);
        let video = Video {
            chapters: Chapters {
                found: Some(vec![VideoChapter {
                    start_seconds: 0,
                    end_seconds: 60,
                    title: "Intro".to_string(),
                }]),
                ..summarized_video().chapters
            },
            ..summarized_video()
        };
        seed(
            &db,
            &video,
            &[
                &prompted_category("Chess"),
                &prompted_category("Sicilian"),
                &prompted_category("Tech"),
            ],
        );
        let task = categorize_video_task(&db, llm.clone());

        run(&task, &video).unwrap();

        let entry = serde_json::json!({
            "type": "object",
            "properties": {
                "reason": {"type": "string"},
                "belongs": {"type": "boolean"},
            },
            "required": ["reason", "belongs"],
        });
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
                       Categories:\n\
                       \n\
                       ### Chess\n\
                       The video is about Chess.\n\
                       \n\
                       ### Sicilian\n\
                       The video is about Sicilian.\n\
                       \n\
                       ### Tech\n\
                       The video is about Tech.\n\
                       \n\
                       Answer in JSON with one entry per category, keyed by its name: \"reason\", one sentence on what the video covers and how it fits that category, then \"belongs\", true or false."
                    .to_string(),
                context_length: 6144,
                temperature: 0.0,
                thinking: true,
                max_output_tokens: 4095,
                timeout: MIN_CHAT_TIMEOUT,
                format: Some(serde_json::json!({
                    "type": "object",
                    "properties": {"Chess": entry, "Sicilian": entry, "Tech": entry},
                    "required": ["Chess", "Sicilian", "Tech"],
                })),
            }]
        );
    }

    #[test]
    fn it_should_cap_the_output_at_3072_for_one_category() {
        let db = TestDatabase::new();
        let llm = llm_replying(vec![reply(&answers(&[("Sicilian", YES)]))]);
        let video = summarized_video();
        seed(&db, &video, &[&prompted_category("Sicilian")]);
        let task = categorize_video_task(&db, llm.clone());

        run(&task, &video).unwrap();

        assert_eq!(output_caps(&llm), vec![3072]);
    }

    #[test]
    fn it_should_cap_the_output_at_the_context_length() {
        let db = TestDatabase::new();
        let names = NINE_CATEGORIES;
        let llm = llm_replying(vec![reply(&answers(&names.map(|name| (name, NO))))]);
        let video = summarized_video();
        let categories = names.map(prompted_category);
        seed(&db, &video, &categories.each_ref());
        SqlitePreferencesRepository::new(db.database())
            .save(&Preferences {
                categorization: CategorizationSettings {
                    context_length: Some(ContextLength::new(8192).unwrap()),
                    ..categorization_on().categorization
                },
                ..categorization_on()
            })
            .unwrap();
        let task = categorize_video_task(&db, llm.clone());

        run(&task, &video).unwrap();

        assert_eq!(output_caps(&llm), vec![8192]);
    }

    #[test]
    fn it_should_wait_longer_for_a_call_about_more_categories() {
        let timeouts = [&NINE_CATEGORIES[..], &["Sicilian"]].map(|names| {
            let db = TestDatabase::new();
            let entries = names.iter().map(|name| (*name, NO)).collect::<Vec<_>>();
            let llm = llm_replying(vec![reply(&answers(&entries))]);
            let video = summarized_video();
            let categories = names
                .iter()
                .map(|name| prompted_category(name))
                .collect::<Vec<_>>();
            seed(&db, &video, &categories.iter().collect::<Vec<_>>());
            SqlitePreferencesRepository::new(db.database())
                .save(&Preferences {
                    categorization: CategorizationSettings {
                        context_length: Some(ContextLength::new(16384).unwrap()),
                        ..categorization_on().categorization
                    },
                    ..categorization_on()
                })
                .unwrap();
            let task = categorize_video_task(&db, llm.clone());

            run(&task, &video).unwrap();

            llm.chats()
                .iter()
                .map(|chat| chat.timeout)
                .collect::<Vec<_>>()
        });

        assert_eq!(
            timeouts,
            [
                vec![Duration::from_secs(2457)],
                vec![Duration::from_secs(30 * 60)]
            ]
        );
    }

    #[test]
    fn it_should_ask_with_the_auto_context_length() {
        let db = TestDatabase::new();
        let names = NINE_CATEGORIES;
        let llm = llm_replying(vec![reply(&answers(&names.map(|name| (name, NO))))]);
        let video = summarized_video();
        let categories = names.map(prompted_category);
        seed(&db, &video, &categories.each_ref());
        let task = categorize_video_task(&db, llm.clone());

        run(&task, &video).unwrap();

        assert_eq!(
            llm.chats()
                .iter()
                .map(|chat| (chat.context_length, chat.max_output_tokens))
                .collect::<Vec<_>>(),
            vec![(16384, 12285)]
        );
    }

    #[test]
    fn it_should_store_the_usable_answers_of_a_partly_usable_call() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video = summarized_video();
        let (sicilian, tech) = (prompted_category("Sicilian"), prompted_category("Tech"));
        seed(&db, &video, &[&sicilian, &tech]);
        let llm = llm_replying(vec![
            reply(&answers(&[
                ("Sicilian", YES),
                ("Tech", r#"{"reason": "A chess game."}"#),
            ])),
            reply(&answers(&[("Tech", NO)])),
        ]);
        let task = categorize_video_task(&db, llm.clone());

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            asked_categories(&llm),
            vec![vec!["Sicilian", "Tech"], vec!["Tech"]]
        );
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            by_category([
                ai_verdict(&video, &sicilian, Verdict::Yes, "A Sicilian game."),
                ai_verdict(&video, &tech, Verdict::No, "A chess game."),
            ])
        );
    }

    #[test]
    fn it_should_ask_each_category_alone_when_the_call_hits_the_output_limit() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video = summarized_video();
        let (chess, sicilian, tech) = (
            prompted_category("Chess"),
            prompted_category("Sicilian"),
            prompted_category("Tech"),
        );
        seed(&db, &video, &[&chess, &sicilian, &tech]);
        let llm = llm_replying(vec![
            Ok(LlmReply {
                stopped_at_limit: true,
                ..reply("").unwrap()
            }),
            reply(&answers(&[("Chess", YES)])),
            reply(&answers(&[("Sicilian", YES)])),
            reply(&answers(&[("Tech", NO)])),
        ]);
        let task = categorize_video_task(&db, llm.clone());

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            asked_categories(&llm),
            vec![
                vec!["Chess", "Sicilian", "Tech"],
                vec!["Chess"],
                vec!["Sicilian"],
                vec!["Tech"]
            ]
        );
        assert_eq!(output_caps(&llm), vec![4095, 3072, 3072, 3072]);
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            by_category([
                ai_verdict(&video, &chess, Verdict::Yes, "A Sicilian game."),
                ai_verdict(&video, &sicilian, Verdict::Yes, "A Sicilian game."),
                ai_verdict(&video, &tech, Verdict::No, "A chess game."),
            ])
        );
    }

    #[test]
    fn it_should_ask_each_category_alone_when_the_call_is_refused() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video = summarized_video();
        let (sicilian, tech) = (prompted_category("Sicilian"), prompted_category("Tech"));
        seed(&db, &video, &[&sicilian, &tech]);
        let llm = llm_replying(vec![
            Ok(LlmReply::refused("the prompt is too long")),
            reply(&answers(&[("Sicilian", YES)])),
            reply(&answers(&[("Tech", NO)])),
        ]);
        let task = categorize_video_task(&db, llm.clone());

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            asked_categories(&llm),
            vec![vec!["Sicilian", "Tech"], vec!["Sicilian"], vec!["Tech"]]
        );
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            by_category([
                ai_verdict(&video, &sicilian, Verdict::Yes, "A Sicilian game."),
                ai_verdict(&video, &tech, Verdict::No, "A chess game."),
            ])
        );
    }

    #[test]
    fn it_should_not_fall_back_for_a_single_category() {
        let db = TestDatabase::new();
        let video = summarized_video();
        seed(&db, &video, &[&prompted_category("Sicilian")]);
        let llm = llm_replying(vec![Ok(LlmReply {
            stopped_at_limit: true,
            ..reply("").unwrap()
        })]);
        let task = categorize_video_task(&db, llm.clone());

        let result = run(&task, &video);

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: later() + chrono::Duration::minutes(10),
                reason:
                    "the model couldn't answer for Sicilian (it hit the 3072-token output limit)"
                        .to_string(),
            })
        );
        assert_eq!(asked_categories(&llm), vec![vec!["Sicilian"]]);
    }

    #[test]
    fn it_should_count_a_connection_dropped_during_the_fallback_as_a_failed_attempt() {
        let db = TestDatabase::new();
        let video = summarized_video();
        seed(
            &db,
            &video,
            &[&prompted_category("Sicilian"), &prompted_category("Tech")],
        );
        let llm = llm_replying(vec![
            Ok(LlmReply::refused("the prompt is too long")),
            Err(anyhow::anyhow!("connection reset")),
        ]);
        let task = categorize_video_task(&db, llm.clone());

        let result = run(&task, &video);

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: later() + chrono::Duration::minutes(10),
                reason: "connection reset".to_string(),
            })
        );
        assert_eq!(
            asked_categories(&llm),
            vec![vec!["Sicilian", "Tech"], vec!["Sicilian"]]
        );
        assert_eq!(
            SqliteVideoCategoryVerdictRepository::new(db.database())
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![]
        );
        assert_eq!(
            SqliteVideoRepository::new(db.database())
                .find(&video.id)
                .unwrap()
                .map(|video| video.categorization),
            Some(StepState {
                status: StepStatus::Retrying,
                attempts: RetryAttempts {
                    failed: 1,
                    last_failed_at: Some(later()),
                },
            })
        );
    }

    #[test]
    fn it_should_keep_the_other_verdicts_of_a_call_when_one_prompt_changed_during_it() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video = summarized_video();
        let (sicilian, tech) = (prompted_category("Sicilian"), prompted_category("Tech"));
        seed(&db, &video, &[&sicilian, &tech]);
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
                .with_replies(vec![reply(&answers(&[("Sicilian", YES), ("Tech", YES)]))])
                .with_on_chat(move || {
                    SqliteCategoryRepository::new(database.clone())
                        .update(&edited)
                        .unwrap()
                }),
        );
        let task = categorize_video_task(&db, llm);

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
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
    fn it_should_read_the_answer_after_leaked_thinking() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video = summarized_video();
        let sicilian = prompted_category("Sicilian");
        seed(&db, &video, &[&sicilian]);
        let answer = r#"<think>Is {"Sicilian": {"belongs": false}} right?</think>
Sure: {"Sicilian": {"reason": "A Sicilian game.", "belongs": true}}"#;
        let task = categorize_video_task(&db, llm_replying(vec![reply(answer)]));

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
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
        let llm = llm_replying(vec![reply(&answers(&[("Tech", YES)]))]);
        let video = summarized_video();
        let sicilian = prompted_category("Sicilian");
        let tech = unprompted_category("Tech");
        seed(&db, &video, &[&sicilian, &tech]);
        let task = categorize_video_task(&db, llm.clone());
        category_repository.delete(&sicilian.id).unwrap();
        let tech = tech.with_ai_prompt(prompted_category("Tech").ai_prompt);
        category_repository.update(&tech).unwrap();

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(asked_categories(&llm), vec![vec!["Tech"]]);
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
        let llm = llm_replying(vec![reply(&answers(&[("Sicilian", YES)]))]);
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

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(asked_categories(&llm), vec![vec!["Sicilian"]]);
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
            let llm = llm_replying(vec![reply(&answers(&[("Sicilian", YES)]))]);
            let video = summarized_video();
            seed(&db, &video, &[&prompted_category("Sicilian")]);
            let task = categorize_video_task(&db, llm.clone());
            turn_off(&db);

            let result = run(&task, &video);

            (result, asked_categories(&llm))
        });

        assert_eq!(
            asked,
            [
                (Ok(TaskOutcome::Done), vec![]),
                (Ok(TaskOutcome::Done), vec![])
            ]
        );
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
                .with_replies(vec![reply(&answers(&[("Sicilian", YES)]))])
                .with_on_chat(move || {
                    SqliteCategoryRepository::new(database.clone())
                        .delete(&deleted)
                        .unwrap()
                }),
        );
        let task = categorize_video_task(&db, llm);

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
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
                .with_replies(vec![reply(&answers(&[("Sicilian", YES)]))])
                .with_on_chat(move || {
                    SqliteVideoRepository::new(database.clone())
                        .delete(&removed)
                        .unwrap()
                }),
        );
        let task = categorize_video_task(&db, llm);

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
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
                .with_replies(vec![reply(&answers(&[("Tech", YES)]))])
                .with_on_chat(move || {
                    SqliteVideoCategoryVerdictRepository::new(database.clone())
                        .save(&decided)
                        .unwrap()
                }),
        );
        let task = categorize_video_task(&db, llm);

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
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
                .with_replies(vec![reply(&answers(&[("Tech", YES)]))])
                .with_on_chat(move || {
                    SqliteCategoryRepository::new(database.clone())
                        .update(&edited)
                        .unwrap()
                }),
        );
        let task = categorize_video_task(&db, llm);

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
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
                llm_replying(vec![reply(&answers(&[("Sicilian", YES)]))]),
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
                Arc::new(SqliteVideoRepository::new(db.database())),
            )),
        );

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
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
    fn it_should_schedule_the_next_attempt_if_a_category_is_rejected() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video_repository = SqliteVideoRepository::new(db.database());
        let video = summarized_video();
        let (sicilian, tech) = (prompted_category("Sicilian"), prompted_category("Tech"));
        seed(&db, &video, &[&sicilian, &tech]);
        let task = categorize_video_task(
            &db,
            llm_replying(vec![
                reply(&answers(&[("Tech", NO)])),
                reply("It might belong."),
            ]),
        );

        let result = run(&task, &video);

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: later() + chrono::Duration::minutes(10),
                reason: "the model couldn't answer for Sicilian (its answer isn't usable)"
                    .to_string(),
            })
        );
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(Video {
                version: 1,
                categorization: StepState {
                    status: StepStatus::Retrying,
                    attempts: RetryAttempts {
                        failed: 1,
                        last_failed_at: Some(later()),
                    },
                },
                updated_at: later(),
                ..video.clone()
            })
        );
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![ai_verdict(&video, &tech, Verdict::No, "A chess game.")]
        );
    }

    #[test]
    fn it_should_fail_pending_categories_after_the_eleventh_failed_attempt() {
        let db = TestDatabase::new();
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video_repository = SqliteVideoRepository::new(db.database());
        let video = Video {
            categorization: StepState {
                status: StepStatus::Retrying,
                attempts: RetryAttempts {
                    failed: 10,
                    last_failed_at: Some(fixed_timestamp()),
                },
            },
            ..summarized_video()
        };
        let (chess, food) = (prompted_category("Chess"), prompted_category("Food"));
        seed(&db, &video, &[&chess, &food]);
        let task = categorize_video_task(
            &db,
            llm_replying(vec![
                reply(&answers(&[("Chess", NO)])),
                Ok(LlmReply::refused("model is too large")),
            ]),
        );

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            by_category([
                ai_verdict(&video, &chess, Verdict::No, "A chess game."),
                failed_verdict(&video, &food),
            ])
        );
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(Video {
                version: 1,
                categorization: StepState {
                    status: StepStatus::Failed,
                    attempts: RetryAttempts {
                        failed: 11,
                        last_failed_at: Some(later()),
                    },
                },
                updated_at: later(),
                ..video
            })
        );
    }

    #[test]
    fn it_should_mark_the_categorization_done_when_nothing_is_rejected() {
        let db = TestDatabase::new();
        let video_repository = SqliteVideoRepository::new(db.database());
        let video = Video {
            categorization: StepState {
                status: StepStatus::Retrying,
                attempts: RetryAttempts {
                    failed: 2,
                    last_failed_at: Some(fixed_timestamp()),
                },
            },
            ..summarized_video()
        };
        let chess = prompted_category("Chess");
        seed(&db, &video, &[&chess]);
        let task =
            categorize_video_task(&db, llm_replying(vec![reply(&answers(&[("Chess", NO)]))]));

        let result = run(&task, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(Video {
                version: 1,
                categorization: StepState {
                    status: StepStatus::Done,
                    attempts: RetryAttempts::default(),
                },
                updated_at: later(),
                ..video
            })
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
            llm_replying(vec![
                reply(&answers(&[("Sicilian", YES)])),
                reply("It might belong."),
            ]),
        );
        run(&first, &video).unwrap();
        let llm = llm_replying(vec![reply(&answers(&[("Tech", NO)]))]);
        let retry = categorize_video_task(&db, llm.clone());

        let result = run(&retry, &video);

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(asked_categories(&llm), vec![vec!["Tech"]]);
    }

    #[test]
    fn it_should_not_mark_a_category_failed_when_the_settings_changed_during_its_attempt() {
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

            run(&task, &video).unwrap();

            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap()
        });

        assert_eq!(verdicts, [vec![], vec![]]);
    }

    #[test]
    fn it_should_wait_for_ollama_if_it_is_unavailable_for_a_categorization() {
        let unreachable = FakeLlmServerRepository::unreachable();
        let model_missing = FakeLlmServerRepository::new(server_with(LlmModel {
            name: "llama3:8b".to_string(),
            ..thinking_model()
        }));
        let down_during_call = FakeLlmServerRepository::new(server_with(thinking_model()))
            .with_replies(vec![Err(unreachable_error())]);

        let outcomes = [unreachable, model_missing, down_during_call].map(|llm| {
            let db = TestDatabase::new();
            let video = summarized_video();
            seed(&db, &video, &[&prompted_category("Sicilian")]);
            let task = categorize_video_task(&db, Arc::new(llm));

            let result = run(&task, &video);

            let verdicts = SqliteVideoCategoryVerdictRepository::new(db.database())
                .find_many(std::slice::from_ref(&video.id))
                .unwrap();
            let categorization = SqliteVideoRepository::new(db.database())
                .find(&video.id)
                .unwrap()
                .map(|video| video.categorization);
            (result, verdicts, categorization)
        });

        let pending = Some(StepState::pending());
        let wait = |reason: &str| {
            Ok(TaskOutcome::RunAgainAt {
                run_at: later() + chrono::Duration::hours(6),
                reason: reason.to_string(),
            })
        };
        assert_eq!(
            outcomes,
            [
                (
                    wait("can't connect to Ollama: connection refused"),
                    vec![],
                    pending
                ),
                (wait("qwen3:4b isn't installed on Ollama"), vec![], pending),
                (
                    wait("can't connect to Ollama: connection refused"),
                    vec![],
                    pending
                ),
            ]
        );
    }

    #[test]
    fn it_should_wait_for_ollama_if_the_model_is_missing_with_thinking_off() {
        let db = TestDatabase::new();
        let llm = Arc::new(
            FakeLlmServerRepository::new(server_with(LlmModel {
                name: "llama3:8b".to_string(),
                ..thinking_model()
            }))
            .with_replies(vec![reply(&answers(&[("Sicilian", YES)]))]),
        );
        let video = summarized_video();
        seed(&db, &video, &[&prompted_category("Sicilian")]);
        SqlitePreferencesRepository::new(db.database())
            .save(&Preferences {
                categorization: CategorizationSettings {
                    thinking: false,
                    ..categorization_on().categorization
                },
                ..categorization_on()
            })
            .unwrap();
        let task = categorize_video_task(&db, llm.clone());

        let result = run(&task, &video);

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: later() + chrono::Duration::hours(6),
                reason: "qwen3:4b isn't installed on Ollama".to_string(),
            })
        );
        assert_eq!(llm.chats(), vec![]);
        assert_eq!(
            SqliteVideoRepository::new(db.database())
                .find(&video.id)
                .unwrap(),
            Some(video.clone())
        );
    }

    #[test]
    fn it_should_not_fail_pending_categories_if_ollama_is_down_on_the_eleventh_attempt() {
        let video = Video {
            categorization: StepState {
                status: StepStatus::Retrying,
                attempts: RetryAttempts {
                    failed: 10,
                    last_failed_at: Some(fixed_timestamp()),
                },
            },
            ..summarized_video()
        };
        let (sicilian, tech) = (prompted_category("Sicilian"), prompted_category("Tech"));
        let unreachable = FakeLlmServerRepository::unreachable();
        let model_missing = FakeLlmServerRepository::new(server_with(LlmModel {
            name: "llama3:8b".to_string(),
            ..thinking_model()
        }));
        let down_during_call = FakeLlmServerRepository::new(server_with(thinking_model()))
            .with_replies(vec![Err(unreachable_error())]);
        let down_during_fallback = FakeLlmServerRepository::new(server_with(thinking_model()))
            .with_replies(vec![
                reply(&answers(&[("Sicilian", NO)])),
                Err(unreachable_error()),
            ]);

        let outcomes = [
            unreachable,
            model_missing,
            down_during_call,
            down_during_fallback,
        ]
        .map(|llm| {
            let db = TestDatabase::new();
            seed(&db, &video, &[&sicilian, &tech]);
            let task = categorize_video_task(&db, Arc::new(llm));

            let result = run(&task, &video);

            let verdicts = SqliteVideoCategoryVerdictRepository::new(db.database())
                .find_many(std::slice::from_ref(&video.id))
                .unwrap();
            let categorization = SqliteVideoRepository::new(db.database())
                .find(&video.id)
                .unwrap()
                .map(|video| video.categorization);
            (result, verdicts, categorization)
        });

        let wait = |reason: &str| {
            Ok(TaskOutcome::RunAgainAt {
                run_at: later() + chrono::Duration::hours(6),
                reason: reason.to_string(),
            })
        };
        let unreachable = wait("can't connect to Ollama: connection refused");
        let unchanged = Some(video.categorization);
        assert_eq!(
            outcomes,
            [
                (unreachable.clone(), vec![], unchanged),
                (
                    wait("qwen3:4b isn't installed on Ollama"),
                    vec![],
                    unchanged
                ),
                (unreachable.clone(), vec![], unchanged),
                (
                    unreachable,
                    vec![ai_verdict(&video, &sicilian, Verdict::No, "A chess game.")],
                    unchanged
                ),
            ]
        );
    }

    #[test]
    fn it_should_log_each_call_and_each_verdict() {
        let db = TestDatabase::new();
        let video = summarized_video();
        seed(
            &db,
            &video,
            &[&prompted_category("Sicilian"), &prompted_category("Tech")],
        );
        let task = categorize_video_task(
            &db,
            llm_replying(vec![
                reply(&answers(&[("Sicilian", YES)])),
                reply("It might belong."),
            ]),
        );

        let lines = captured_log_lines(|| {
            run(&task, &video).unwrap();
        });

        let line = |needle: &str| {
            lines
                .iter()
                .find(|line| line.contains(needle))
                .cloned()
                .unwrap_or_default()
        };
        let first_call = line("categories=Sicilian, Tech");
        let fallback_call = line("categories=Tech ");
        let verdict = line("category=Sicilian");
        let rejected = line("ERROR ");
        assert!(first_call.starts_with("INFO asked the model"));
        assert!(first_call.contains(&format!("video_id={}", video.id)));
        assert!(first_call.contains("fallback=false"));
        assert!(first_call.contains("input_tokens=100"));
        assert!(first_call.contains("output_tokens=200"));
        assert!(first_call.contains("duration_seconds="));
        assert!(fallback_call.starts_with("INFO asked the model"));
        assert!(fallback_call.contains("fallback=true"));
        assert!(verdict.starts_with("INFO "));
        assert!(verdict.contains(&format!("video_id={}", video.id)));
        assert!(verdict.contains("verdict=YES"));
        assert!(verdict.contains("reason=A Sicilian game."));
        assert!(rejected.starts_with("ERROR "));
        assert!(rejected.contains("category=Tech"));
        assert!(rejected.contains("reason=its answer isn't usable"));
    }

    #[test]
    fn it_should_log_the_fallback() {
        let db = TestDatabase::new();
        let video = summarized_video();
        seed(
            &db,
            &video,
            &[&prompted_category("Sicilian"), &prompted_category("Tech")],
        );
        let task = categorize_video_task(
            &db,
            llm_replying(vec![
                Ok(LlmReply {
                    stopped_at_limit: true,
                    ..reply("").unwrap()
                }),
                reply(&answers(&[("Sicilian", YES)])),
                reply(&answers(&[("Tech", NO)])),
            ]),
        );

        let lines = captured_log_lines(|| {
            run(&task, &video).unwrap();
        });

        let fallbacks = lines
            .iter()
            .filter(|line| line.starts_with("WARN ") || line.starts_with("ERROR "))
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            fallbacks,
            ["Sicilian", "Tech"].map(|name| format!(
                "WARN the model couldn't answer for a category, asking it alone \
                 video_id={} category={name} reason=it hit the 3072-token output limit",
                video.id
            ))
        );
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
                Arc::new(SqliteVideoRepository::new(db.database())),
            )),
        )
    }

    fn run(task: &CategorizeVideoTask, video: &Video) -> Result<TaskOutcome, String> {
        task.handle(&task_for(video))
            .map_err(|error| error.to_string())
    }

    fn task_for(video: &Video) -> Task {
        Task::CategorizeVideo {
            video_id: video.id.as_str().to_string(),
        }
    }

    /// Stores `video` with its summary, `my_channel()` with categorization on
    /// as its owner, `categories`, and AI categorization on.
    fn seed(db: &TestDatabase, video: &Video, categories: &[&Category]) {
        SqliteVideoRepository::new(db.database())
            .insert(video)
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
            YoutubeVideoId::new("yt1").unwrap(),
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

    /// The categories each chat asked about, in order.
    fn asked_categories(llm: &FakeLlmServerRepository) -> Vec<Vec<String>> {
        llm.chats()
            .iter()
            .map(|chat| {
                chat.user
                    .lines()
                    .filter_map(|line| line.strip_prefix("### "))
                    .map(str::to_string)
                    .collect()
            })
            .collect()
    }

    /// The output cap of each chat, in order.
    fn output_caps(llm: &FakeLlmServerRepository) -> Vec<u32> {
        llm.chats()
            .iter()
            .map(|chat| chat.max_output_tokens)
            .collect()
    }

    /// The nine categories `evals/categories` measures.
    const NINE_CATEGORIES: [&str; 9] = [
        "AI news",
        "Chess gameplays",
        "Coding tutorials",
        "Food for thought",
        "Jobava London",
        "Politics and Economy News",
        "Science News",
        "Sicilian",
        "Tech",
    ];

    /// An answer about each named category, keyed by its name.
    fn answers(entries: &[(&str, &str)]) -> String {
        let entries = entries
            .iter()
            .map(|(name, answer)| format!("{name:?}: {answer}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{{{entries}}}")
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

    fn failed_verdict(video: &Video, category: &Category) -> VideoCategoryVerdict {
        VideoCategoryVerdict {
            reason: None,
            ..ai_verdict(video, category, Verdict::Failed, "")
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
            output_tokens: 200,
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
        video_id: VideoId,
    }

    impl EventPublisher for RemovingVideoOnPublish {
        fn publish(&self, event: &DomainEvent) -> anyhow::Result<()> {
            self.publisher.publish(event)?;
            self.video_repository.delete(&self.video_id)
        }
    }
}
