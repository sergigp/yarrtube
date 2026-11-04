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
    use crate::domain::video::{SummaryState, Video, VideoChapter, VideoId};
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
