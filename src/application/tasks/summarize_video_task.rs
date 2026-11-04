use crate::domain::services::{VideoSummarizer, VideoSummarizerApi};
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::sync::Arc;

/// Writes one video's AI summary, scheduled by the video sync
/// (`SyncVideoTask` via `VideoReconciler`) while the video needs one (see the
/// `video-summaries` capability). Best-effort: always succeeds, so the queue
/// never retries it.
pub struct SummarizeVideoTask {
    video_repository: Arc<dyn VideoRepository>,
    video_summarizer: Arc<VideoSummarizer>,
}

impl SummarizeVideoTask {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_summarizer: Arc<VideoSummarizer>,
    ) -> Self {
        Self {
            video_repository,
            video_summarizer,
        }
    }
}

impl TaskHandler for SummarizeVideoTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let video_id = VideoRecordId::new(Task::decode_summarize_video_payload(payload)?)?;
        let Some(video) = self.video_repository.find(&video_id)? else {
            return Ok(());
        };
        self.video_summarizer.summarize(&video);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::llm_server::{LlmChat, LlmModel, LlmReply, LlmServer, OllamaUrl};
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::preferences::{
        ContextLength, Preferences, SummaryModel, SummaryPrompt, SummarySettings, Temperature,
    };
    use crate::domain::services::VideoOwnerFinder;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::video::{LateDataChecks, SummaryState, Video, VideoChapter, VideoId};
    use crate::domain::video_summary::{SummaryText, VideoSummary};
    use crate::domain::video_transcript::{
        CaptionLanguage, CaptionSource, TranscriptText, VideoTranscript,
    };
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::ollama_llm_server_repository::FakeLlmServerRepository;
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
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_summary_repository::{
        SqliteVideoSummaryRepository, VideoSummaryRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_transcript_repository::{
        SqliteVideoTranscriptRepository, VideoTranscriptRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    const OUTPUT_DIR: &str = "/videos/creators/somechannel";
    const SUBTITLES: &str = "Season 2026/S2026E010101 - Sicilian.es.srt";
    const SHORT_SRT: &str = "1\n00:00:00,000 --> 00:00:02,000\nHola a todos\n";

    #[test]
    fn it_should_store_the_summary_of_a_video_that_fits_one_call() {
        let scene = Scene::default()
            .with_replies(vec![reply("Three games.")])
            .build();

        let result = scene.run();

        assert_eq!(result, Ok(()));
        assert_eq!(
            scene.stored_summary(),
            Some(VideoSummary {
                video_id: scene.video.id.clone(),
                text: SummaryText::new("Three games."),
                model: "qwen3:4b".to_string(),
                created_at: later(),
            })
        );
        assert_eq!(scene.stored_video().summary_state, SummaryState::Generated);
        assert_eq!(scene.chats().len(), 1);
    }

    #[test]
    fn it_should_send_the_shipped_prompt_filled_with_the_videos_data() {
        let scene = Scene::default()
            .with_replies(vec![reply("Three games.")])
            .build();

        scene.run().unwrap();

        let chat = &scene.chats()[0];
        assert!(
            chat.system
                .contains("- Write in Spanish, the language of the video.")
        );
        assert!(!chat.system.contains('{'));
        assert_eq!(
            chat.user,
            "Title: Sicilian\nDuration: 30 minutes\n\nTranscript:\nHola a todos\n\nNow write the summary in Spanish, 3 to 5 sentences."
        );
    }

    #[test]
    fn it_should_send_the_saved_prompt_when_there_is_one() {
        let saved = SummaryPrompt::shipped()
            .as_str()
            .replace("You summarize YouTube videos", "You digest YouTube videos");
        let scene = Scene {
            preferences: with_summaries(|settings| SummarySettings {
                prompt: Some(SummaryPrompt::from_stored(saved.clone())),
                ..settings
            }),
            ..Scene::default()
        }
        .with_replies(vec![reply("Three games.")])
        .build();

        scene.run().unwrap();

        assert!(
            scene.chats()[0]
                .system
                .starts_with("You digest YouTube videos")
        );
    }

    #[test]
    fn it_should_send_the_saved_model_settings() {
        let scene = Scene {
            preferences: with_summaries(|settings| SummarySettings {
                context_length: ContextLength::new(8192).unwrap(),
                temperature: Temperature::new(0.5).unwrap(),
                ..settings
            }),
            ..Scene::default()
        }
        .with_replies(vec![reply("Three games.")])
        .build();

        scene.run().unwrap();

        let chat = &scene.chats()[0];
        assert_eq!(
            (
                chat.model.as_str(),
                chat.context_length,
                chat.temperature,
                chat.max_output_tokens
            ),
            ("qwen3:4b", 8192, 0.5, 2192)
        );
    }

    #[test]
    fn it_should_list_the_chapters_in_the_prompt() {
        let scene = Scene {
            video: with_chapters(vec![chapter(0, 125, "Intro"), chapter(125, 1800, "Game 1")]),
            ..Scene::default()
        }
        .with_replies(vec![reply("Three games.")])
        .build();

        scene.run().unwrap();

        assert_eq!(
            scene.chats()[0].user,
            "Title: Sicilian\nDuration: 30 minutes\n\nChapters (set by the uploader; use them to see the video's structure, but summarize what is said, not the chapter titles):\n0:00 Intro\n2:05 Game 1\n\nTranscript:\nHola a todos\n\nNow write the summary in Spanish, 3 to 5 sentences."
        );
    }

    #[test]
    fn it_should_leave_chapters_out_of_the_prompt_when_the_video_has_none() {
        let scene = Scene {
            video: with_chapters(vec![]),
            ..Scene::default()
        }
        .with_replies(vec![reply("Three games.")])
        .build();

        scene.run().unwrap();

        assert!(
            scene.chats()[0]
                .user
                .starts_with("Title: Sicilian\nDuration: 30 minutes\n\nTranscript:")
        );
    }

    #[test]
    fn it_should_keep_braces_in_values_unfilled() {
        let scene = Scene {
            video: Video {
                title: "{transcript} explained".to_string(),
                ..subtitled_video()
            },
            ..Scene::default()
        }
        .with_replies(vec![reply("Three games.")])
        .build();

        scene.run().unwrap();

        assert!(
            scene.chats()[0]
                .user
                .starts_with("Title: {transcript} explained\n")
        );
    }

    #[test]
    fn it_should_use_the_language_code_when_its_name_is_unknown() {
        let scene = Scene {
            language: "xh".to_string(),
            ..Scene::default()
        }
        .with_replies(vec![reply("Three games.")])
        .build();

        scene.run().unwrap();

        assert!(
            scene.chats()[0]
                .user
                .ends_with("Now write the summary in xh, 3 to 5 sentences.")
        );
    }

    #[test]
    fn it_should_drop_the_reasoning_before_the_answer() {
        let scene = Scene::default()
            .with_replies(vec![reply("<think>a </think> b</think>\n\nThree games.\n")])
            .build();

        scene.run().unwrap();

        assert_eq!(
            scene.stored_summary().map(|summary| summary.text),
            Some(SummaryText::new("Three games."))
        );
    }

    #[test]
    fn it_should_ask_for_thinking_only_when_the_model_supports_it() {
        let thinking_asked = [(true, true), (true, false), (false, true), (false, false)].map(
            |(setting, capable)| {
                let scene = Scene {
                    preferences: with_summaries(|settings| SummarySettings {
                        thinking: setting,
                        ..settings
                    }),
                    llm: FakeLlmServerRepository::new(server_with(LlmModel {
                        thinking: capable,
                        ..thinking_model()
                    })),
                    ..Scene::default()
                }
                .with_replies(vec![reply("Three games.")])
                .build();

                scene.run().unwrap();

                scene.chats()[0].thinking
            },
        );

        assert_eq!(thinking_asked, [true, false, false, false]);
    }

    #[test]
    fn it_should_summarize_a_long_transcript_in_parts_then_reduce() {
        // Each line is 1000 characters, so 30 lines cut into parts of ~30600.
        let scene = Scene {
            srt: Some(srt_of(45, 60)),
            ..Scene::default()
        }
        .with_replies(vec![reply("First."), reply("Second."), reply("All of it.")])
        .build();

        scene.run().unwrap();

        let chats = scene.chats();
        assert_eq!(chats.len(), 3);
        assert!(chats[0].system.starts_with("You summarize one part"));
        assert!(chats[1].user.contains("Part 2 of 2 ("));
        assert!(chats[2].system.starts_with("You write the final summary"));
        assert_eq!(
            chats[2].user,
            "Title: Sicilian\nDuration: 30 minutes\n\nPart summaries, in order:\n\nPart 1 (0:00–29:00):\nFirst.\n\nPart 2 (30:00–44:00):\nSecond.\n\nNow write the final summary of the whole video in Spanish, 3 to 5 sentences."
        );
        assert_eq!(
            scene.stored_summary().map(|summary| summary.text),
            Some(SummaryText::new("All of it."))
        );
    }

    #[test]
    fn it_should_cut_parts_at_chapter_starts() {
        let scene = Scene {
            srt: Some(srt_of(45, 60)),
            video: with_chapters(vec![
                chapter(0, 600, "Game 1"),
                chapter(600, 1500, "Game 2"),
                chapter(1500, 2100, "Game 3"),
                chapter(2100, 2700, "Game 4"),
            ]),
            ..Scene::default()
        }
        .with_replies(vec![reply("First."), reply("Second."), reply("All of it.")])
        .build();

        scene.run().unwrap();

        assert!(scene.chats()[1].user.contains(
            "Part 2 of 2 (25:00–44:00)\n\nChapters in this part:\n25:00 Game 3\n35:00 Game 4\n\nTranscript part:\n"
        ));
    }

    #[test]
    fn it_should_cut_by_size_without_chapters() {
        let scene = Scene {
            srt: Some(srt_of(100, 60)),
            ..Scene::default()
        }
        .with_replies(vec![
            reply("A."),
            reply("B."),
            reply("C."),
            reply("D."),
            reply("All."),
        ])
        .build();

        scene.run().unwrap();

        let chats = scene.chats();
        let parts = &chats[..chats.len() - 1];
        let every_line_sent = (0..100).all(|line| {
            parts
                .iter()
                .any(|chat| chat.user.contains(&format!("{line:04}").repeat(250)))
        });
        assert_eq!((parts.len(), every_line_sent), (4, true));
        assert!(parts.iter().all(|chat| chat.user.len() < 32_000));
    }

    #[test]
    fn it_should_cut_a_chapter_too_long_on_its_own_by_size() {
        let scene = Scene {
            srt: Some(srt_of(45, 60)),
            video: with_chapters(vec![chapter(0, 300, "Short"), chapter(300, 2700, "Long")]),
            ..Scene::default()
        }
        .with_replies(vec![reply("A."), reply("B."), reply("C."), reply("All.")])
        .build();

        scene.run().unwrap();

        let chats = scene.chats();
        assert!(chats[0].user.contains("Part 1 of 3 (0:00–4:00)"));
        assert!(chats[1].user.contains("Part 2 of 3 (5:00–34:00)"));
        assert!(chats[2].user.contains("Part 3 of 3 (35:00–44:00)"));
    }

    #[test]
    fn it_should_scale_the_budget_with_the_context_length() {
        let chats_with = |context_length: i64, replies| {
            let scene = Scene {
                srt: Some(srt_of(20, 60)),
                preferences: with_summaries(|settings| SummarySettings {
                    context_length: ContextLength::new(context_length).unwrap(),
                    ..settings
                }),
                ..Scene::default()
            }
            .with_replies(replies)
            .build();
            scene.run().unwrap();
            scene.chats().len()
        };

        assert_eq!(chats_with(12288, vec![reply("All.")]), 1);
        assert_eq!(
            chats_with(6144, vec![reply("A."), reply("B."), reply("All.")]),
            3
        );
    }

    #[test]
    fn it_should_estimate_english_at_more_characters_per_token() {
        // 34 lines of 1000 characters: over 30600, the budget for most
        // languages, but under 38700, the budget for English.
        let chats_with = |language: &str| {
            let scene = Scene {
                srt: Some(srt_of(34, 60)),
                language: language.to_string(),
                ..Scene::default()
            }
            .with_replies(vec![reply("A."), reply("B."), reply("All.")])
            .build();
            scene.run().unwrap();
            scene.chats().len()
        };

        assert_eq!((chats_with("en-US"), chats_with("es")), (1, 3));
    }

    #[test]
    fn it_should_do_nothing_when_summaries_are_no_longer_available() {
        let changed: Vec<(Vec<LlmChat>, SummaryState)> = [
            Preferences {
                ai_enabled: false,
                ..summaries_on()
            },
            with_summaries(|settings| SummarySettings {
                enabled: false,
                ..settings
            }),
        ]
        .into_iter()
        .map(|preferences| {
            let scene = Scene {
                preferences,
                ..Scene::default()
            }
            .with_replies(vec![reply("Three games.")])
            .build();
            scene.run().unwrap();
            (scene.chats(), scene.stored_video().summary_state)
        })
        .collect();

        assert_eq!(
            changed,
            vec![
                (vec![], SummaryState::Missing),
                (vec![], SummaryState::Missing)
            ]
        );
    }

    #[test]
    fn it_should_do_nothing_when_the_source_opted_out() {
        let scene = Scene {
            summaries_enabled: false,
            ..Scene::default()
        }
        .with_replies(vec![reply("Three games.")])
        .build();

        scene.run().unwrap();

        assert_eq!(scene.chats(), vec![]);
        assert_eq!(scene.stored_video().summary_state, SummaryState::Missing);
    }

    #[test]
    fn it_should_do_nothing_for_a_video_already_summarized() {
        let scene = Scene {
            video: Video {
                summary_state: SummaryState::Generated,
                ..subtitled_video()
            },
            ..Scene::default()
        }
        .with_replies(vec![reply("Three games.")])
        .build();

        scene.run().unwrap();

        assert_eq!(scene.chats(), vec![]);
        assert_eq!(scene.stored_summary(), None);
    }

    #[test]
    fn it_should_do_nothing_for_a_missing_video() {
        let scene = Scene::default().build();
        SqliteVideoRepository::new(scene.db.database())
            .delete(&scene.video.id)
            .unwrap();

        assert_eq!(scene.run(), Ok(()));
        assert_eq!(scene.chats(), vec![]);
    }

    #[test]
    fn it_should_skip_without_counting_when_the_saved_prompt_is_invalid() {
        let scene = Scene {
            preferences: with_summaries(|settings| SummarySettings {
                prompt: Some(SummaryPrompt::from_stored("## system\n\nBe brief.\n")),
                ..settings
            }),
            ..Scene::default()
        }
        .with_replies(vec![reply("Three games.")])
        .build();

        assert_eq!(scene.run(), Ok(()));

        assert_eq!(scene.chats(), vec![]);
        assert_eq!(
            scene.stored_video().summary_checks,
            LateDataChecks::default()
        );
    }

    #[test]
    fn it_should_skip_without_counting_when_ollama_is_unreachable() {
        let scene = Scene {
            llm: FakeLlmServerRepository::failing(),
            ..Scene::default()
        }
        .build();

        assert_eq!(scene.run(), Ok(()));

        assert_eq!(scene.chats(), vec![]);
        assert_eq!(
            scene.stored_video().summary_checks,
            LateDataChecks::default()
        );
    }

    #[test]
    fn it_should_skip_without_counting_when_the_model_is_not_installed() {
        let scene = Scene {
            llm: FakeLlmServerRepository::new(server_with(LlmModel {
                name: "gemma3:4b".to_string(),
                ..thinking_model()
            })),
            ..Scene::default()
        }
        .with_replies(vec![reply("Three games.")])
        .build();

        assert_eq!(scene.run(), Ok(()));

        assert_eq!(scene.chats(), vec![]);
        assert_eq!(
            scene.stored_video().summary_checks,
            LateDataChecks::default()
        );
    }

    #[test]
    fn it_should_count_a_failed_attempt_when_the_chat_fails() {
        let scene = Scene::default()
            .with_replies(vec![Err(anyhow::anyhow!("model crashed"))])
            .build();

        assert_eq!(scene.run(), Ok(()));

        assert_eq!(scene.stored_summary(), None);
        let video = scene.stored_video();
        assert_eq!(
            (video.summary_state, video.summary_checks),
            (
                SummaryState::Missing,
                LateDataChecks {
                    last_checked_at: Some(later()),
                    failed_attempts: 1,
                }
            )
        );
    }

    #[test]
    fn it_should_count_a_failed_attempt_when_the_answer_stopped_at_the_limit() {
        let scene = Scene::default()
            .with_replies(vec![Ok(LlmReply {
                stopped_at_limit: true,
                ..reply("Three games and").unwrap()
            })])
            .build();

        scene.run().unwrap();

        assert_eq!(scene.stored_summary(), None);
        assert_eq!(scene.stored_video().summary_checks.failed_attempts, 1);
    }

    #[test]
    fn it_should_count_a_failed_attempt_when_the_input_was_truncated() {
        let scene = Scene::default()
            .with_replies(vec![Ok(LlmReply {
                input_tokens: 12250,
                ..reply("Three games.").unwrap()
            })])
            .build();

        scene.run().unwrap();

        assert_eq!(scene.stored_summary(), None);
        assert_eq!(scene.stored_video().summary_checks.failed_attempts, 1);
    }

    #[test]
    fn it_should_count_a_failed_attempt_when_the_answer_is_only_reasoning() {
        let scene = Scene::default()
            .with_replies(vec![reply("<think>hmm</think>\n")])
            .build();

        scene.run().unwrap();

        assert_eq!(scene.stored_summary(), None);
        assert_eq!(scene.stored_video().summary_checks.failed_attempts, 1);
    }

    #[test]
    fn it_should_stop_at_the_first_failed_part() {
        let scene = Scene {
            srt: Some(srt_of(100, 60)),
            ..Scene::default()
        }
        .with_replies(vec![
            reply("A."),
            Err(anyhow::anyhow!("model crashed")),
            reply("C."),
        ])
        .build();

        scene.run().unwrap();

        assert_eq!(scene.chats().len(), 2);
        assert_eq!(scene.stored_summary(), None);
        assert_eq!(scene.stored_video().summary_checks.failed_attempts, 1);
    }

    /// What a test changes about the default situation: a downloaded,
    /// subtitled, 30-minute Spanish video in a channel with summaries on,
    /// and AI and summaries on, with `qwen3:4b` installed.
    struct Scene {
        preferences: Preferences,
        summaries_enabled: bool,
        video: Video,
        language: String,
        srt: Option<String>,
        llm: FakeLlmServerRepository,
    }

    /// A scene once stored, ready to run.
    struct BuiltScene {
        db: TestDatabase,
        video: Video,
        llm: Arc<FakeLlmServerRepository>,
        task: SummarizeVideoTask,
    }

    impl Default for Scene {
        fn default() -> Self {
            Self {
                preferences: summaries_on(),
                summaries_enabled: true,
                video: subtitled_video(),
                language: "es".to_string(),
                srt: Some(SHORT_SRT.to_string()),
                llm: FakeLlmServerRepository::new(server_with(thinking_model())),
            }
        }
    }

    impl Scene {
        fn with_replies(self, replies: Vec<anyhow::Result<LlmReply>>) -> Self {
            Self {
                llm: self.llm.with_replies(replies),
                ..self
            }
        }

        fn build(self) -> BuiltScene {
            let db = TestDatabase::new();
            let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
            video_repository.save(&self.video).unwrap();
            let preferences = Arc::new(SqlitePreferencesRepository::new(db.database()));
            preferences.save(&self.preferences).unwrap();
            let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
            channel_repository
                .insert(&my_channel().with_summaries_enabled(self.summaries_enabled))
                .unwrap();
            let channel_video_repository =
                Arc::new(SqliteChannelVideoRepository::new(db.database()));
            channel_video_repository
                .save(&ChannelVideo::create(
                    my_channel_id(),
                    self.video.id.clone(),
                    0,
                    fixed_timestamp(),
                ))
                .unwrap();
            let transcript_repository =
                Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
            transcript_repository
                .save(&VideoTranscript {
                    video_id: self.video.id.clone(),
                    language: CaptionLanguage::new(&self.language).unwrap(),
                    source: CaptionSource::Automatic,
                    text: TranscriptText::new("unused: timed lines come from the SRT"),
                    created_at: fixed_timestamp(),
                })
                .unwrap();
            let files = self
                .srt
                .iter()
                .fold(FakeVideoFileRepository::default(), |files, srt| {
                    files.with_file(format!("{OUTPUT_DIR}/{SUBTITLES}"), srt)
                });
            let llm = Arc::new(self.llm);
            let task = SummarizeVideoTask::new(
                video_repository.clone(),
                Arc::new(VideoSummarizer::new(
                    video_repository,
                    Arc::new(VideoOwnerFinder::new(
                        channel_video_repository,
                        Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                        channel_repository,
                        Arc::new(SqlitePlaylistRepository::new(db.database())),
                        "/videos",
                    )),
                    preferences,
                    transcript_repository,
                    Arc::new(files),
                    llm.clone(),
                    Arc::new(SqliteVideoSummaryRepository::new(db.database())),
                    Arc::new(FixedClock(later())),
                )),
            );
            BuiltScene {
                db,
                video: self.video,
                llm,
                task,
            }
        }
    }

    impl BuiltScene {
        fn run(&self) -> Result<(), String> {
            let payload = serde_json::json!({ "video_id": self.video.id.as_str() }).to_string();
            self.task
                .handle(&payload, false)
                .map_err(|error| error.to_string())
        }

        fn chats(&self) -> Vec<LlmChat> {
            self.llm.chats()
        }

        fn stored_summary(&self) -> Option<VideoSummary> {
            SqliteVideoSummaryRepository::new(self.db.database())
                .find(&self.video.id)
                .unwrap()
        }

        fn stored_video(&self) -> Video {
            SqliteVideoRepository::new(self.db.database())
                .find(&self.video.id)
                .unwrap()
                .unwrap()
        }
    }

    fn summaries_on() -> Preferences {
        Preferences {
            ai_enabled: true,
            ollama_url: Some(OllamaUrl::new("http://mac.local:11434").unwrap()),
            summaries: SummarySettings {
                enabled: true,
                ..SummarySettings::default()
            },
            ..Preferences::default()
        }
    }

    /// The default preferences with their summary settings changed by `change`.
    fn with_summaries(change: impl FnOnce(SummarySettings) -> SummarySettings) -> Preferences {
        let preferences = summaries_on();
        Preferences {
            summaries: change(preferences.summaries.clone()),
            ..preferences
        }
    }

    fn subtitled_video() -> Video {
        let video = Video::create(VideoId::new("yt1").unwrap(), "Sicilian", fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "Season 2026/S2026E010101 - Sicilian.mp4",
                None,
                Some(1800),
                fixed_timestamp(),
            );
        video.mark_subtitles_fetched(SUBTITLES, fixed_timestamp())
    }

    fn with_chapters(chapters: Vec<VideoChapter>) -> Video {
        Video {
            chapters: Some(chapters),
            ..subtitled_video()
        }
    }

    fn chapter(start_seconds: i64, end_seconds: i64, title: &str) -> VideoChapter {
        VideoChapter {
            start_seconds,
            end_seconds,
            title: title.to_string(),
        }
    }

    /// `lines` cues of 1000 distinct characters each, one every `step` seconds.
    fn srt_of(lines: i64, step: i64) -> String {
        (0..lines)
            .map(|line| {
                let (hours, minutes) = (line * step / 3600, line * step % 3600 / 60);
                let start = format!("{hours:02}:{minutes:02}:{:02},000", line * step % 60);
                let text = format!("{line:04}").repeat(250);
                format!("{}\n{start} --> {start}\n{text}\n", line + 1)
            })
            .collect::<Vec<_>>()
            .join("\n")
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

    fn thinking_model() -> LlmModel {
        LlmModel {
            name: SummaryModel::default().as_str().to_string(),
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
        })
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_000_000, 0).unwrap()
    }

    fn later() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(2_000_000, 0).unwrap()
    }
}
