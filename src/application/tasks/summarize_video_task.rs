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
    fn handle(&self, payload: &str, is_last_attempt: bool) -> anyhow::Result<()> {
        let video_id = VideoRecordId::new(Task::decode_summarize_video_payload(payload)?)?;
        let Some(video) = self.video_repository.find(&video_id)? else {
            return Ok(());
        };
        self.video_summarizer.summarize(&video, is_last_attempt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::log_capture::captured_log_lines;
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::llm_server::{LlmChat, LlmModel, LlmReply, LlmServer, OllamaUrl};
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::preferences::{
        ContextLength, Preferences, SummaryPrompt, SummarySettings, Temperature,
    };
    use crate::domain::services::VideoOwnerFinder;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::video::{SummaryState, Video, VideoChapter, VideoId};
    use crate::domain::video_summary::{SummaryMessages, SummaryText, VideoSummary};
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
    use std::ops::Range;

    const OUTPUT_DIR: &str = "/videos/creators/somechannel";
    const SUBTITLES: &str = "Season 2026/S2026E010101 - Sicilian.es.srt";
    const SHORT_SRT: &str = "1\n00:00:00,000 --> 00:00:02,000\nHola a todos\n";

    #[test]
    fn it_should_store_the_summary_of_a_video_that_fits_one_call() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository.clone(),
            preferences_repository,
            summary_repository.clone(),
            llm.clone(),
            files_with(SHORT_SRT),
        );

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(
            summary_repository.find(&video.id).unwrap(),
            Some(stored_summary(&video, "Three games."))
        );
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(video.clone().mark_summary_generated(later()))
        );
        assert_eq!(llm.chats(), vec![one_call_chat()]);
    }

    #[test]
    fn it_should_send_the_shipped_prompt_filled_with_the_videos_data() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(SHORT_SRT),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            sent_messages(&llm),
            vec![SummaryMessages {
                system: SYSTEM.to_string(),
                user: "Title: Sicilian\nDuration: 30 minutes\n\nTranscript:\nHola a todos\n\nNow write the summary in Spanish, 3 to 5 sentences.".to_string(),
            }]
        );
    }

    #[test]
    fn it_should_send_the_saved_prompt_when_there_is_one() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let saved = SummaryPrompt::shipped()
            .as_str()
            .replace("You summarize YouTube videos", "You digest YouTube videos");
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository
            .save(&with_summaries(|settings| SummarySettings {
                prompt: Some(SummaryPrompt::from_stored(saved)),
                ..settings
            }))
            .unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(SHORT_SRT),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            llm.chats(),
            vec![LlmChat {
                system: SYSTEM.replace("You summarize YouTube videos", "You digest YouTube videos"),
                ..one_call_chat()
            }]
        );
    }

    #[test]
    fn it_should_send_the_saved_model_settings() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository
            .save(&with_summaries(|settings| SummarySettings {
                context_length: ContextLength::new(8192).unwrap(),
                temperature: Temperature::new(0.5).unwrap(),
                ..settings
            }))
            .unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(SHORT_SRT),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            llm.chats(),
            vec![LlmChat {
                context_length: 8192,
                temperature: 0.5,
                max_output_tokens: 2192,
                ..one_call_chat()
            }]
        );
    }

    #[test]
    fn it_should_list_the_chapters_in_the_prompt() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let video = with_chapters(vec![chapter(0, 125, "Intro"), chapter(125, 1800, "Game 1")]);
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(SHORT_SRT),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            llm.chats(),
            vec![LlmChat {
                user: "Title: Sicilian\nDuration: 30 minutes\n\nChapters (set by the uploader; use them to see the video's structure, but summarize what is said, not the chapter titles):\n0:00 Intro\n2:05 Game 1\n\nTranscript:\nHola a todos\n\nNow write the summary in Spanish, 3 to 5 sentences.".to_string(),
                ..one_call_chat()
            }]
        );
    }

    #[test]
    fn it_should_leave_chapters_out_of_the_prompt_when_the_video_has_none() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let video = with_chapters(vec![]);
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(SHORT_SRT),
        );

        run(&task, &video).unwrap();

        assert_eq!(llm.chats(), vec![one_call_chat()]);
    }

    #[test]
    fn it_should_keep_braces_in_values_unfilled() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let video = Video {
            title: "{transcript} explained".to_string(),
            ..subtitled_video()
        };
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(SHORT_SRT),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            llm.chats(),
            vec![LlmChat {
                user: "Title: {transcript} explained\nDuration: 30 minutes\n\nTranscript:\nHola a todos\n\nNow write the summary in Spanish, 3 to 5 sentences.".to_string(),
                ..one_call_chat()
            }]
        );
    }

    #[test]
    fn it_should_use_the_language_code_when_its_name_is_unknown() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "xh");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(SHORT_SRT),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            sent_messages(&llm),
            vec![in_language("xh", whole_messages("Hola a todos"))]
        );
    }

    #[test]
    fn it_should_drop_the_reasoning_before_the_answer() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("<think>a </think> b</think>\n\nThree games.\n")]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository.clone(),
            llm,
            files_with(SHORT_SRT),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            summary_repository.find(&video.id).unwrap(),
            Some(stored_summary(&video, "Three games."))
        );
    }

    #[test]
    fn it_should_ask_for_thinking_only_when_the_model_supports_it() {
        let chats = [(true, true), (true, false), (false, true), (false, false)].map(
            |(setting, capable)| {
                let db = TestDatabase::new();
                let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
                let preferences_repository =
                    Arc::new(SqlitePreferencesRepository::new(db.database()));
                let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
                let llm = Arc::new(
                    FakeLlmServerRepository::new(server_with(LlmModel {
                        thinking: capable,
                        ..thinking_model()
                    }))
                    .with_replies(vec![reply("Three games.")]),
                );
                let video = subtitled_video();
                video_repository.save(&video).unwrap();
                preferences_repository
                    .save(&with_summaries(|settings| SummarySettings {
                        thinking: setting,
                        ..settings
                    }))
                    .unwrap();
                seed_channel(&db, &video, true);
                seed_transcript(&db, &video, "es");
                let task = summarize_video_task(
                    &db,
                    video_repository,
                    preferences_repository,
                    summary_repository,
                    llm.clone(),
                    files_with(SHORT_SRT),
                );

                run(&task, &video).unwrap();

                llm.chats()
            },
        );

        let chat_thinking = |thinking| {
            vec![LlmChat {
                thinking,
                ..one_call_chat()
            }]
        };
        assert_eq!(
            chats,
            [
                chat_thinking(true),
                chat_thinking(false),
                chat_thinking(false),
                chat_thinking(false)
            ]
        );
    }

    #[test]
    fn it_should_summarize_a_long_transcript_in_parts_then_reduce() {
        // Each line is 1000 characters, so 30 lines cut into parts of ~30600.
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("First."), reply("Second."), reply("All of it.")]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository.clone(),
            llm.clone(),
            files_with(&srt_of(45, 60)),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            sent_messages(&llm),
            vec![
                part_messages(1, 2, "0:00–29:00", &[], 0..30),
                part_messages(2, 2, "30:00–44:00", &[], 30..45),
                SummaryMessages {
                    system: REDUCE_SYSTEM.to_string(),
                    user: "Title: Sicilian\nDuration: 30 minutes\n\nPart summaries, in order:\n\nPart 1 (0:00–29:00):\nFirst.\n\nPart 2 (30:00–44:00):\nSecond.\n\nNow write the final summary of the whole video in Spanish, 3 to 5 sentences.".to_string(),
                },
            ]
        );
        assert_eq!(
            summary_repository.find(&video.id).unwrap(),
            Some(stored_summary(&video, "All of it."))
        );
    }

    #[test]
    fn it_should_reduce_answers_in_groups_when_they_do_not_fit() {
        // At 8192 the budget is ~20400 characters: two 8000-character answers
        // fit together, a third doesn't.
        let long_answer = "x".repeat(8000);
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![
            reply(&long_answer),
            reply(&long_answer),
            reply(&long_answer),
            reply("Both."),
            reply("All of it."),
        ]);
        let video = with_chapters(vec![chapter(0, 2700, "Game 1")]);
        video_repository.save(&video).unwrap();
        preferences_repository
            .save(&with_summaries(|settings| SummarySettings {
                context_length: ContextLength::new(8192).unwrap(),
                ..settings
            }))
            .unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository.clone(),
            llm.clone(),
            files_with(&srt_of(45, 60)),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            sent_messages(&llm),
            vec![
                part_messages(1, 3, "0:00–19:00", &["0:00 Game 1"], 0..20),
                part_messages(2, 3, "20:00–39:00", &["0:00 Game 1"], 20..40),
                part_messages(3, 3, "40:00–44:00", &["0:00 Game 1"], 40..45),
                reduce_messages(
                    &[],
                    &[("0:00–19:00", &long_answer), ("20:00–39:00", &long_answer)]
                ),
                reduce_messages(
                    &["0:00 Game 1"],
                    &[("0:00–39:00", "Both."), ("40:00–44:00", &long_answer)]
                ),
            ]
        );
        assert_eq!(
            summary_repository.find(&video.id).unwrap(),
            Some(stored_summary(&video, "All of it."))
        );
    }

    #[test]
    fn it_should_keep_reducing_until_the_answers_fit() {
        // Every answer alone is over the ~20400-character budget at 8192.
        let long_answer = "x".repeat(25000);
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![
            reply(&long_answer),
            reply(&long_answer),
            reply(&long_answer),
            reply(&long_answer),
            reply("Both."),
            reply("All of it."),
        ]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository
            .save(&with_summaries(|settings| SummarySettings {
                context_length: ContextLength::new(8192).unwrap(),
                ..settings
            }))
            .unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository.clone(),
            llm.clone(),
            files_with(&srt_of(45, 60)),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            sent_messages(&llm),
            vec![
                part_messages(1, 3, "0:00–19:00", &[], 0..20),
                part_messages(2, 3, "20:00–39:00", &[], 20..40),
                part_messages(3, 3, "40:00–44:00", &[], 40..45),
                reduce_messages(
                    &[],
                    &[("0:00–19:00", &long_answer), ("20:00–39:00", &long_answer)]
                ),
                reduce_messages(
                    &[],
                    &[("0:00–39:00", &long_answer), ("40:00–44:00", &long_answer)]
                ),
                reduce_messages(&[], &[("0:00–44:00", "Both.")]),
            ]
        );
        assert_eq!(
            summary_repository.find(&video.id).unwrap(),
            Some(stored_summary(&video, "All of it."))
        );
    }

    #[test]
    fn it_should_list_a_chapter_starting_on_the_parts_last_line() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("First."), reply("Second."), reply("All of it.")]);
        let video = with_chapters(vec![
            chapter(0, 1740, "Game 1"),
            chapter(1740, 1800, "Short"),
            chapter(1800, 2700, "Game 2"),
        ]);
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(&srt_of(45, 60)),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            sent_messages(&llm),
            vec![
                part_messages(1, 2, "0:00–29:00", &["0:00 Game 1", "29:00 Short"], 0..30),
                part_messages(2, 2, "30:00–44:00", &["30:00 Game 2"], 30..45),
                reduce_messages(
                    &["0:00 Game 1", "29:00 Short", "30:00 Game 2"],
                    &[("0:00–29:00", "First."), ("30:00–44:00", "Second.")]
                ),
            ]
        );
    }

    #[test]
    fn it_should_cut_parts_at_chapter_starts() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("First."), reply("Second."), reply("All of it.")]);
        let video = with_chapters(vec![
            chapter(0, 600, "Game 1"),
            chapter(600, 1500, "Game 2"),
            chapter(1500, 2100, "Game 3"),
            chapter(2100, 2700, "Game 4"),
        ]);
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(&srt_of(45, 60)),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            sent_messages(&llm),
            vec![
                part_messages(1, 2, "0:00–24:00", &["0:00 Game 1", "10:00 Game 2"], 0..25),
                part_messages(
                    2,
                    2,
                    "25:00–44:00",
                    &["25:00 Game 3", "35:00 Game 4"],
                    25..45
                ),
                reduce_messages(
                    &[
                        "0:00 Game 1",
                        "10:00 Game 2",
                        "25:00 Game 3",
                        "35:00 Game 4"
                    ],
                    &[("0:00–24:00", "First."), ("25:00–44:00", "Second.")]
                ),
            ]
        );
    }

    #[test]
    fn it_should_cut_by_size_without_chapters() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![
            reply("A."),
            reply("B."),
            reply("C."),
            reply("D."),
            reply("All."),
        ]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(&srt_of(100, 60)),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            sent_messages(&llm),
            vec![
                part_messages(1, 4, "0:00–29:00", &[], 0..30),
                part_messages(2, 4, "30:00–59:00", &[], 30..60),
                part_messages(3, 4, "1:00:00–1:29:00", &[], 60..90),
                part_messages(4, 4, "1:30:00–1:39:00", &[], 90..100),
                reduce_messages(
                    &[],
                    &[
                        ("0:00–29:00", "A."),
                        ("30:00–59:00", "B."),
                        ("1:00:00–1:29:00", "C."),
                        ("1:30:00–1:39:00", "D.")
                    ]
                ),
            ]
        );
    }

    #[test]
    fn it_should_cut_a_chapter_too_long_on_its_own_by_size() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("A."), reply("B."), reply("C."), reply("All.")]);
        let video = with_chapters(vec![chapter(0, 300, "Short"), chapter(300, 2700, "Long")]);
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(&srt_of(45, 60)),
        );

        run(&task, &video).unwrap();

        assert_eq!(
            sent_messages(&llm),
            vec![
                part_messages(1, 3, "0:00–4:00", &["0:00 Short"], 0..5),
                part_messages(2, 3, "5:00–34:00", &["5:00 Long"], 5..35),
                part_messages(3, 3, "35:00–44:00", &["5:00 Long"], 35..45),
                reduce_messages(
                    &["0:00 Short", "5:00 Long"],
                    &[
                        ("0:00–4:00", "A."),
                        ("5:00–34:00", "B."),
                        ("35:00–44:00", "C.")
                    ]
                ),
            ]
        );
    }

    #[test]
    fn it_should_scale_the_budget_with_the_context_length() {
        let messages_with = |context_length: i64, replies| {
            let db = TestDatabase::new();
            let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
            let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
            let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
            let llm = llm_replying(replies);
            let video = subtitled_video();
            video_repository.save(&video).unwrap();
            preferences_repository
                .save(&with_summaries(|settings| SummarySettings {
                    context_length: ContextLength::new(context_length).unwrap(),
                    ..settings
                }))
                .unwrap();
            seed_channel(&db, &video, true);
            seed_transcript(&db, &video, "es");
            let task = summarize_video_task(
                &db,
                video_repository,
                preferences_repository,
                summary_repository,
                llm.clone(),
                files_with(&srt_of(20, 60)),
            );

            run(&task, &video).unwrap();

            sent_messages(&llm)
        };

        assert_eq!(
            messages_with(12288, vec![reply("All.")]),
            vec![whole_messages(&transcript_lines(0..20))]
        );
        assert_eq!(
            messages_with(6144, vec![reply("A."), reply("B."), reply("All.")]),
            vec![
                part_messages(1, 2, "0:00–14:00", &[], 0..15),
                part_messages(2, 2, "15:00–19:00", &[], 15..20),
                reduce_messages(&[], &[("0:00–14:00", "A."), ("15:00–19:00", "B.")]),
            ]
        );
    }

    #[test]
    fn it_should_estimate_english_at_more_characters_per_token() {
        // 34 lines of 1000 characters: over 30600, the budget for most
        // languages, but under 38700, the budget for English.
        let messages_with = |language: &str| {
            let db = TestDatabase::new();
            let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
            let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
            let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
            let llm = llm_replying(vec![reply("A."), reply("B."), reply("All.")]);
            let video = subtitled_video();
            video_repository.save(&video).unwrap();
            preferences_repository.save(&summaries_on()).unwrap();
            seed_channel(&db, &video, true);
            seed_transcript(&db, &video, language);
            let task = summarize_video_task(
                &db,
                video_repository,
                preferences_repository,
                summary_repository,
                llm.clone(),
                files_with(&srt_of(34, 60)),
            );

            run(&task, &video).unwrap();

            sent_messages(&llm)
        };

        assert_eq!(
            messages_with("en-US"),
            vec![in_language(
                "English",
                whole_messages(&transcript_lines(0..34))
            )]
        );
        assert_eq!(
            messages_with("es"),
            vec![
                part_messages(1, 2, "0:00–29:00", &[], 0..30),
                part_messages(2, 2, "30:00–33:00", &[], 30..34),
                reduce_messages(&[], &[("0:00–29:00", "A."), ("30:00–33:00", "B.")]),
            ]
        );
    }

    #[test]
    fn it_should_skip_if_summaries_are_unavailable() {
        let video = subtitled_video();
        let changed: Vec<(Vec<LlmChat>, Option<Video>)> = [
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
            let db = TestDatabase::new();
            let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
            let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
            let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
            let llm = llm_replying(vec![reply("Three games.")]);
            video_repository.save(&video).unwrap();
            preferences_repository.save(&preferences).unwrap();
            seed_channel(&db, &video, true);
            seed_transcript(&db, &video, "es");
            let task = summarize_video_task(
                &db,
                video_repository.clone(),
                preferences_repository,
                summary_repository,
                llm.clone(),
                files_with(SHORT_SRT),
            );

            run(&task, &video).unwrap();

            (llm.chats(), video_repository.find(&video.id).unwrap())
        })
        .collect();

        assert_eq!(
            changed,
            vec![(vec![], Some(video.clone())), (vec![], Some(video.clone()))]
        );
    }

    #[test]
    fn it_should_skip_if_the_source_opted_out() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, false);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository.clone(),
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(SHORT_SRT),
        );

        run(&task, &video).unwrap();

        assert_eq!(llm.chats(), vec![]);
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(video.clone())
        );
    }

    #[test]
    fn it_should_skip_a_video_already_summarized() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let video = Video {
            summary_state: SummaryState::Generated,
            ..subtitled_video()
        };
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository.clone(),
            llm.clone(),
            files_with(SHORT_SRT),
        );

        run(&task, &video).unwrap();

        assert_eq!(llm.chats(), vec![]);
        assert_eq!(summary_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_ignore_a_missing_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        video_repository.delete(&video.id).unwrap();
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(SHORT_SRT),
        );

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(llm.chats(), vec![]);
    }

    #[test]
    fn it_should_skip_when_the_saved_prompt_is_invalid() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository
            .save(&with_summaries(|settings| SummarySettings {
                prompt: Some(SummaryPrompt::from_stored("## system\n\nBe brief.\n")),
                ..settings
            }))
            .unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository.clone(),
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(SHORT_SRT),
        );

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(llm.chats(), vec![]);
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(video.clone())
        );
    }

    #[test]
    fn it_should_skip_when_ollama_is_unreachable() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = Arc::new(FakeLlmServerRepository::failing());
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository.clone(),
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(SHORT_SRT),
        );

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(llm.chats(), vec![]);
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(video.clone())
        );
    }

    #[test]
    fn it_should_skip_when_the_model_is_not_installed() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = Arc::new(
            FakeLlmServerRepository::new(server_with(LlmModel {
                name: "gemma3:4b".to_string(),
                ..thinking_model()
            }))
            .with_replies(vec![reply("Three games.")]),
        );
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository.clone(),
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(SHORT_SRT),
        );

        let result = run(&task, &video);

        assert_eq!(result, Ok(()));
        assert_eq!(llm.chats(), vec![]);
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(video.clone())
        );
    }

    #[test]
    fn it_should_fail_the_task_and_stay_missing_on_a_server_fault() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![Err(anyhow::anyhow!("model crashed"))]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository.clone(),
            preferences_repository,
            summary_repository.clone(),
            llm,
            files_with(SHORT_SRT),
        );

        let result = run_last_attempt(&task, &video);

        assert_eq!(result, Err("model crashed".to_string()));
        assert_eq!(summary_repository.find(&video.id).unwrap(), None);
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(video.clone())
        );
    }

    #[test]
    fn it_should_fail_the_task_when_the_answer_stopped_at_the_limit() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![Ok(LlmReply {
            stopped_at_limit: true,
            ..reply("Three games and").unwrap()
        })]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository.clone(),
            preferences_repository,
            summary_repository.clone(),
            llm,
            files_with(SHORT_SRT),
        );

        let result = run(&task, &video);

        assert_eq!(
            result,
            Err(
                "the model stopped at the 3288-token output cap (context 12288, 100 input tokens)"
                    .to_string()
            )
        );
        assert_eq!(summary_repository.find(&video.id).unwrap(), None);
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(video.clone())
        );
    }

    #[test]
    fn it_should_fail_the_task_when_ollama_refuses_the_prompt() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![Ok(LlmReply::refused("request too long"))]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository.clone(),
            preferences_repository,
            summary_repository.clone(),
            llm,
            files_with(SHORT_SRT),
        );

        let result = run(&task, &video);

        assert_eq!(
            result,
            Err("Ollama refused the call: request too long".to_string())
        );
        assert_eq!(summary_repository.find(&video.id).unwrap(), None);
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(video.clone())
        );
    }

    #[test]
    fn it_should_fail_the_task_when_the_answer_is_only_reasoning() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("<think>hmm</think>\n")]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository.clone(),
            preferences_repository,
            summary_repository.clone(),
            llm,
            files_with(SHORT_SRT),
        );

        let result = run(&task, &video);

        assert_eq!(result, Err("The model's answer is empty".to_string()));
        assert_eq!(summary_repository.find(&video.id).unwrap(), None);
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(video.clone())
        );
    }

    #[test]
    fn it_should_stop_at_the_first_failed_part() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![
            reply("A."),
            Err(anyhow::anyhow!("model crashed")),
            reply("C."),
        ]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository.clone(),
            llm.clone(),
            files_with(&srt_of(100, 60)),
        );

        let result = run(&task, &video);

        assert_eq!(result, Err("model crashed".to_string()));
        assert_eq!(
            sent_messages(&llm),
            vec![
                part_messages(1, 4, "0:00–29:00", &[], 0..30),
                part_messages(2, 4, "30:00–59:00", &[], 30..60),
            ]
        );
        assert_eq!(summary_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_fail_the_task_when_the_subtitle_file_is_gone() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository.clone(),
            preferences_repository,
            summary_repository,
            llm.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        );

        let result = run_last_attempt(&task, &video);

        assert_eq!(result, Err("video's subtitle file is gone".to_string()));
        assert_eq!(llm.chats(), vec![]);
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(video.clone().mark_summary_unavailable(later()))
        );
    }

    #[test]
    fn it_should_become_unavailable_when_rejected_on_the_last_attempt() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![Ok(LlmReply::refused("request too long"))]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository.clone(),
            preferences_repository,
            summary_repository,
            llm,
            files_with(SHORT_SRT),
        );

        let result = run_last_attempt(&task, &video);

        assert_eq!(
            result,
            Err("Ollama refused the call: request too long".to_string())
        );
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(video.clone().mark_summary_unavailable(later()))
        );
    }

    #[test]
    fn it_should_ignore_a_video_deleted_while_generating() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let video = subtitled_video();
        let llm = {
            let video_repository = video_repository.clone();
            let video_id = video.id.clone();
            Arc::new(
                FakeLlmServerRepository::new(server_with(thinking_model()))
                    .with_replies(vec![reply("Three games.")])
                    .with_on_chat(move || video_repository.delete(&video_id).unwrap()),
            )
        };
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository.clone(),
            llm.clone(),
            files_with(SHORT_SRT),
        );

        run(&task, &video).unwrap();

        assert_eq!(llm.chats(), vec![one_call_chat()]);
        assert_eq!(summary_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_fail_the_task_when_the_subtitle_file_is_empty() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository.clone(),
            preferences_repository,
            summary_repository,
            llm.clone(),
            files_with(""),
        );

        let result = run(&task, &video);

        assert_eq!(result, Err("video's subtitle file has no text".to_string()));
        assert_eq!(llm.chats(), vec![]);
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(video.clone())
        );
    }

    #[test]
    fn it_should_log_each_call_without_its_prompt_and_its_answer_and_duration() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm,
            files_with(SHORT_SRT),
        );

        let lines = captured_log_lines(|| run(&task, &video).unwrap());

        let sent = lines
            .iter()
            .find(|line| line.contains("sending prompt"))
            .unwrap();
        let answered = lines
            .iter()
            .find(|line| line.contains("model answered"))
            .unwrap();
        assert!(sent.starts_with("INFO "));
        assert!(sent.contains(&format!("video_id={}", video.id)));
        assert!(sent.contains("title=Sicilian"));
        assert!(!sent.contains("Hola a todos"));
        assert!(answered.starts_with("INFO "));
        assert!(answered.contains("answer=Three games."));
        assert!(answered.contains("input_tokens=100"));
        assert!(answered.contains("duration_seconds="));
    }

    #[test]
    fn it_should_log_the_summary_total_time_and_video_length() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        let llm = llm_replying(vec![reply("Three games.")]);
        let video = subtitled_video();
        video_repository.save(&video).unwrap();
        preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&db, &video, true);
        seed_transcript(&db, &video, "es");
        let task = summarize_video_task(
            &db,
            video_repository,
            preferences_repository,
            summary_repository,
            llm,
            files_with(SHORT_SRT),
        );

        let lines = captured_log_lines(|| run(&task, &video).unwrap());

        let started = lines
            .iter()
            .find(|line| line.contains("generating video summary"))
            .unwrap();
        let generated = lines
            .iter()
            .find(|line| line.contains("video summary generated"))
            .unwrap();
        assert!(started.starts_with("INFO "));
        assert!(started.contains("title=Sicilian"));
        assert!(started.contains("video_seconds=1800"));
        assert!(started.contains("parts=1"));
        assert!(generated.starts_with("INFO "));
        assert!(generated.contains("summary=Three games."));
        assert!(generated.contains("video_seconds=1800"));
        assert!(generated.contains("model=qwen3:4b"));
        assert!(generated.contains("duration_seconds="));
    }

    #[test]
    fn it_should_log_why_a_generation_was_skipped_or_failed() {
        let skipped_db = TestDatabase::new();
        let skipped_video_repository = Arc::new(SqliteVideoRepository::new(skipped_db.database()));
        let skipped_preferences_repository =
            Arc::new(SqlitePreferencesRepository::new(skipped_db.database()));
        let video = subtitled_video();
        skipped_video_repository.save(&video).unwrap();
        skipped_preferences_repository
            .save(&summaries_on())
            .unwrap();
        seed_channel(&skipped_db, &video, false);
        seed_transcript(&skipped_db, &video, "es");
        let opted_out = summarize_video_task(
            &skipped_db,
            skipped_video_repository,
            skipped_preferences_repository,
            Arc::new(SqliteVideoSummaryRepository::new(skipped_db.database())),
            llm_replying(vec![]),
            files_with(SHORT_SRT),
        );
        let failed_db = TestDatabase::new();
        let failed_video_repository = Arc::new(SqliteVideoRepository::new(failed_db.database()));
        let failed_preferences_repository =
            Arc::new(SqlitePreferencesRepository::new(failed_db.database()));
        failed_video_repository.save(&video).unwrap();
        failed_preferences_repository.save(&summaries_on()).unwrap();
        seed_channel(&failed_db, &video, true);
        seed_transcript(&failed_db, &video, "es");
        let failing = summarize_video_task(
            &failed_db,
            failed_video_repository,
            failed_preferences_repository,
            Arc::new(SqliteVideoSummaryRepository::new(failed_db.database())),
            llm_replying(vec![Err(anyhow::anyhow!("model crashed"))]),
            files_with(SHORT_SRT),
        );

        let skipped = captured_log_lines(|| run(&opted_out, &video).unwrap());
        let failed = captured_log_lines(|| {
            run(&failing, &video).unwrap_err();
        });

        assert!(
            skipped
                .iter()
                .any(|line| line.starts_with("INFO skipped video summary")
                    && line.contains("its channel or playlist has summaries off"))
        );
        assert!(
            failed
                .iter()
                .any(|line| line.starts_with("ERROR failed to summarize video")
                    && line.contains("model crashed"))
        );
    }

    fn summarize_video_task(
        db: &TestDatabase,
        video_repository: Arc<SqliteVideoRepository>,
        preferences_repository: Arc<SqlitePreferencesRepository>,
        summary_repository: Arc<SqliteVideoSummaryRepository>,
        llm: Arc<FakeLlmServerRepository>,
        files: Arc<FakeVideoFileRepository>,
    ) -> SummarizeVideoTask {
        SummarizeVideoTask::new(
            video_repository.clone(),
            Arc::new(VideoSummarizer::new(
                video_repository,
                Arc::new(VideoOwnerFinder::new(
                    Arc::new(SqliteChannelVideoRepository::new(db.database())),
                    Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                    Arc::new(SqliteChannelRepository::new(db.database())),
                    Arc::new(SqlitePlaylistRepository::new(db.database())),
                    "/videos",
                )),
                preferences_repository,
                Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
                files,
                llm,
                summary_repository,
                Arc::new(FixedClock(later())),
            )),
        )
    }

    fn run(task: &SummarizeVideoTask, video: &Video) -> Result<(), String> {
        task.handle(&payload_for(video), false)
            .map_err(|error| error.to_string())
    }

    fn run_last_attempt(task: &SummarizeVideoTask, video: &Video) -> Result<(), String> {
        task.handle(&payload_for(video), true)
            .map_err(|error| error.to_string())
    }

    fn payload_for(video: &Video) -> String {
        Task::SummarizeVideo {
            video_id: video.id.as_str().to_string(),
        }
        .payload()
        .to_string()
    }

    /// Stores `my_channel()`, with summaries on or off, as the owner of `video`.
    fn seed_channel(db: &TestDatabase, video: &Video, summaries_enabled: bool) {
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel().with_summaries_enabled(summaries_enabled))
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

    fn seed_transcript(db: &TestDatabase, video: &Video, language: &str) {
        SqliteVideoTranscriptRepository::new(db.database())
            .save(&VideoTranscript {
                video_id: video.id.clone(),
                language: CaptionLanguage::new(language).unwrap(),
                source: CaptionSource::Automatic,
                text: TranscriptText::new("unused: timed lines come from the SRT"),
                created_at: fixed_timestamp(),
            })
            .unwrap();
    }

    /// An installed `qwen3:4b` answering `chat` with `replies`, in order.
    fn llm_replying(replies: Vec<anyhow::Result<LlmReply>>) -> Arc<FakeLlmServerRepository> {
        Arc::new(FakeLlmServerRepository::new(server_with(thinking_model())).with_replies(replies))
    }

    /// `srt` as the subtitle file of `subtitled_video()`.
    fn files_with(srt: &str) -> Arc<FakeVideoFileRepository> {
        Arc::new(
            FakeVideoFileRepository::default().with_file(format!("{OUTPUT_DIR}/{SUBTITLES}"), srt),
        )
    }

    fn sent_messages(llm: &FakeLlmServerRepository) -> Vec<SummaryMessages> {
        llm.chats()
            .into_iter()
            .map(|chat| SummaryMessages {
                system: chat.system,
                user: chat.user,
            })
            .collect()
    }

    /// The call for a video that fits one, under `summaries_on()`.
    fn one_call_chat() -> LlmChat {
        let messages = whole_messages("Hola a todos");
        LlmChat {
            model: "qwen3:4b".to_string(),
            system: messages.system,
            user: messages.user,
            context_length: 12288,
            temperature: 0.3,
            thinking: true,
            max_output_tokens: 3288,
        }
    }

    fn whole_messages(transcript: &str) -> SummaryMessages {
        SummaryMessages {
            system: SYSTEM.to_string(),
            user: format!(
                "Title: Sicilian\nDuration: 30 minutes\n\nTranscript:\n{transcript}\n\nNow write the summary in Spanish, 3 to 5 sentences."
            ),
        }
    }

    /// The call for the transcript `lines` of `srt_of`, as one part.
    fn part_messages(
        part: usize,
        parts: usize,
        span: &str,
        chapters: &[&str],
        lines: Range<i64>,
    ) -> SummaryMessages {
        let chapters = match chapters.is_empty() {
            true => String::new(),
            false => format!("\nChapters in this part:\n{}\n", chapters.join("\n")),
        };
        SummaryMessages {
            system: MAP_SYSTEM.to_string(),
            user: format!(
                "Title: Sicilian\nDuration: 30 minutes\nPart {part} of {parts} ({span})\n{chapters}\nTranscript part:\n{}\n\nNow summarize this part in Spanish, 4 to 6 sentences.",
                transcript_lines(lines)
            ),
        }
    }

    /// The call over `answers`, each a span and what the model said about it.
    fn reduce_messages(chapters: &[&str], answers: &[(&str, &str)]) -> SummaryMessages {
        let chapters = match chapters.is_empty() {
            true => String::new(),
            false => format!(
                "\nChapters (set by the uploader; use them to see the video's structure, but summarize what is said, not the chapter titles):\n{}\n",
                chapters.join("\n")
            ),
        };
        let part_summaries = answers
            .iter()
            .enumerate()
            .map(|(index, (span, summary))| format!("Part {} ({span}):\n{summary}", index + 1))
            .collect::<Vec<_>>()
            .join("\n\n");
        SummaryMessages {
            system: REDUCE_SYSTEM.to_string(),
            user: format!(
                "Title: Sicilian\nDuration: 30 minutes\n{chapters}\nPart summaries, in order:\n\n{part_summaries}\n\nNow write the final summary of the whole video in Spanish, 3 to 5 sentences."
            ),
        }
    }

    /// `messages` for a video in `language` rather than Spanish.
    fn in_language(language: &str, messages: SummaryMessages) -> SummaryMessages {
        SummaryMessages {
            system: messages.system.replace("Spanish", language),
            user: messages.user.replace("Spanish", language),
        }
    }

    fn stored_summary(video: &Video, text: &str) -> VideoSummary {
        VideoSummary {
            video_id: video.id.clone(),
            text: SummaryText::new(text),
            model: "qwen3:4b".to_string(),
            created_at: later(),
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

    /// A downloaded, subtitled, 30-minute video.
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
                let text = line_text(line);
                format!("{}\n{start} --> {start}\n{text}\n", line + 1)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The text of `lines` of `srt_of`, as the model receives it.
    fn transcript_lines(lines: Range<i64>) -> String {
        lines.map(line_text).collect::<Vec<_>>().join("\n")
    }

    fn line_text(line: i64) -> String {
        format!("{line:04}").repeat(250)
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

    /// The shipped `system` section, for a Spanish video.
    const SYSTEM: &str = r#"You summarize YouTube videos from their transcript.
Rules:
- Write in Spanish, the language of the video.
- Plain prose, no lists or headings. Start directly with the subject, e.g. "Three online games where…" or "La edad de oro del anarquismo…". Do not repeat the title, and do not start with "The video", "In this video" or "El vídeo".
- Say what the video is about and its main points. For an argument, give the reasoning behind the conclusion, not just the conclusion.
- Say how it ends: the result or verdict as the speaker states it (who won, what was decided). If the video draws no conclusion, do not invent one.
- Keep the speaker's certainty: "I'm not sure it will" is not "it won't".
- If the video has several parts (several games, places, stories), say how many and cover each with its own result.
- Keep the video's own view apart from views it presents or criticizes.
- Keep the main subject apart from things the speaker only mentions or promotes (other projects, courses, websites, other products).
- Use only what the transcript says. Include a number only if the transcript states it, and give it exactly. Do not add names, numbers or comparisons of your own.
- The transcript is auto-generated and may misspell names; use the title to correct them.
- Ignore sponsors, ads and requests to like or subscribe."#;

    /// The shipped `map system` section, for a Spanish video.
    const MAP_SYSTEM: &str = r#"You summarize one part of a long YouTube video transcript.
Write in Spanish, 4 to 6 sentences of plain prose. Keep this part's key facts, names, numbers and conclusions exactly as stated. Use only what this part says. Ignore sponsors and requests to subscribe."#;

    /// The shipped `reduce system` section, for a Spanish video.
    const REDUCE_SYSTEM: &str = r#"You write the final summary of a YouTube video from summaries of its consecutive parts.
Rules:
- Write in Spanish, the language of the video.
- Plain prose, no lists or headings. Start directly with the subject, e.g. "Three online games where…" or "La edad de oro del anarquismo…". Do not repeat the title, and do not start with "The video", "In this video" or "El vídeo".
- Say what the video is about and its main points. For an argument, give the reasoning behind the conclusion, not just the conclusion.
- Say how it ends: the result or verdict as the speaker states it (who won, what was decided). If the video draws no conclusion, do not invent one.
- Keep the speaker's certainty: "I'm not sure it will" is not "it won't".
- If the video has several parts (several games, places, stories), say how many and cover each with its own result.
- Keep the video's own view apart from views it presents or criticizes.
- Keep the main subject apart from things the speaker only mentions or promotes (other projects, courses, websites, other products).
- Use only what the part summaries says. Include a number only if the part summaries states it, and give it exactly. Do not add names, numbers or comparisons of your own.
- The part summaries is auto-generated and may misspell names; use the title to correct them.
- Ignore sponsors, ads and requests to like or subscribe."#;
}
