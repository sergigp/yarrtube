# Design

## Files

Rust
- `migrations/0023_video_summaries.sql` — new: `video_summaries` table; `summary_state` (default `MISSING`) on `videos`.
- `src/infrastructure/shared/sqlite_migrations.rs` — register `0023`.
- `src/domain/video_summary/mod.rs` — new aggregate.
- `src/domain/video_summary/video_summary.rs` — new: the stored entity.
- `src/domain/video_summary/summary_text.rs` — new: a cleaned model answer (drops `</think>` prefix, rejects empty).
- `src/domain/video_summary/summary_input.rs` — new: pure rendering of the single, map and reduce messages.
- `src/domain/video/summary_state.rs` — new: `Missing`/`Generated`/`Unavailable`.
- `src/domain/video/video.rs` — `summary_state`; `is_summarizable`, `mark_summary_generated`, `mark_summary_unavailable`.
- `src/domain/video/late_data_checks.rs` — `failures_exhausted()`, shared by `settles`.
- `src/domain/video/video_owner.rs` — `summaries_enabled`.
- `src/domain/video/video_sync_plan.rs` — `for_video` takes `summaries_wanted`; `Summarize` step.
- `src/domain/video/video_sync_step.rs` — `Summarize` → `Task::SummarizeVideo`.
- `src/domain/video_transcript/timed_line.rs` — new: one timed line of a transcript.
- `src/domain/video_summary/summary_messages.rs` — new: a call's system and user messages.
- `src/domain/video_summary/part_answer.rs` — new: an answer with the span it covers, so a reduce group keeps its `{span}`.
- `src/domain/video_transcript/timed_transcript.rs` — new: SRT cues as `(start, text)` lines, same cleaning as `TranscriptText::from_srt`; `parts`.
- `src/domain/video_transcript/transcript_part.rs` — new: one consecutive part of a timed transcript.
- `src/domain/video_transcript/transcript_text.rs` — share cue cleaning with `TimedTranscript`.
- `src/domain/video_transcript/caption_language.rs` — `english_name()`.
- `src/domain/llm_server/llm_chat.rs` — new: request to the chat port.
- `src/domain/llm_server/llm_reply.rs` — new: raw answer, `input_tokens`, `stopped_at_limit`, `refusal`.
- `src/domain/llm_server/llm_server.rs` — `model(name)`.
- `src/domain/preferences/summary_prompt.rs` — `validated`, `render`.
- `src/domain/preferences/context_length.rs` — `input_budget_tokens`, `output_budget_tokens`.
- `src/domain/services/video_summarizer.rs` — new: orchestrates one video's generation.
- `src/domain/services/video_reconciler.rs` — inject `PreferencesRepository`, compute `summaries_wanted`.
- `src/domain/services/video_owner_finder.rs` — fill `summaries_enabled`.
- `src/domain/services/{channel,playlist}_deleter.rs`, `{channel,playlist}_video_reconciler.rs` — delete the summary beside the transcript.
- `src/domain/services/task_view_searcher.rs` — resolve `summarize_video` context.
- `src/domain/task/task.rs` — `SummarizeVideo`, lane, exclusivity key, payload decode.
- `src/domain/task/task_lane.rs` — `Summaries`.
- `src/application/tasks/summarize_video_task.rs` — new: fails with the generation; passes `is_last_attempt`.
- `src/domain/services/preferences_updater.rs` — reset Unavailable summaries when summary settings or the URL change.
- `src/application/tasks/mod.rs` — register the handler.
- `src/application/subscribers/sync_video_on_video_subtitles_fetched.rs` — new, mirrors `SyncVideoOnVideoDownloaded`.
- `src/application/subscribers/mod.rs` — second subscriber on `VideoSubtitlesFetched`.
- `src/infrastructure/repositories/ollama_llm_server_repository.rs` — `chat` port method (`POST /api/chat`); fake records chats.
- `src/infrastructure/repositories/sqlite_video_summary_repository.rs` — new: port + SQLite adapter.
- `src/infrastructure/repositories/sqlite_video_repository.rs` — read/insert `summary_*`; `update_summary`.
- `src/infrastructure/repositories/sqlite_video_transcript_repository.rs` — drop `allow(dead_code)` on `find`.
- `src/infrastructure/repositories/mod.rs`, `src/infrastructure/infrastructure_container.rs`, `src/domain/mod.rs`, `src/domain/services/mod.rs`, `src/serve.rs` — wiring.

Web
- `web/src/lib/tasks.ts` — `Summarizing <video>`, family `other`, kind `maintenance` (default).

Docs
- `doc/ARCHITECTURE.md` — summary step in the walkthrough.
- `doc/AI.md` — generation behaviour; native `/api/chat`, not the OpenAI-compatible API (it can't set `num_ctx`).

## Types & Signatures

```rust
// domain/video/summary_state.rs
pub enum SummaryState { Missing, Generated, Unavailable }

// domain/video/video.rs
pub struct Video { /* ... */ pub summary_state: SummaryState }
impl Video {
    /// Downloaded, subtitles Fetched, summary Missing.
    pub fn is_summarizable(&self) -> bool;
    pub fn mark_summary_generated(self, now: DateTime<Utc>) -> Self;
    /// Records a Failed check; Unavailable once failures are exhausted. Ignores
    /// video age, unlike `LateDataChecks::settles`: a summary doesn't get better with time.
    pub fn record_summary_failure(self, now: DateTime<Utc>) -> Self;
}

// domain/video/late_data_checks.rs
impl LateDataChecks { pub fn failures_exhausted(&self) -> bool; }

// domain/video/video_owner.rs
pub struct VideoOwner { /* ... */ pub summaries_enabled: bool }

// domain/video/video_sync_plan.rs
impl VideoSyncPlan {
    pub fn for_video(video: &Video, summaries_wanted: bool, now: DateTime<Utc>) -> Self;
}

// domain/task/task.rs
Task::SummarizeVideo { video_id: String } // "summarize_video", lane Summaries, key video:<id>
impl Task { pub fn decode_summarize_video_payload(payload: &str) -> Result<String, TaskError>; }

// domain/video_transcript/timed_transcript.rs
// domain/video_transcript/timed_line.rs
pub struct TimedLine { pub start_seconds: i64, pub text: String }

// domain/video_transcript/timed_transcript.rs
pub struct TimedTranscript(Vec<TimedLine>);
impl TimedTranscript {
    pub fn from_srt(srt: &str) -> Self;
    pub fn text(&self) -> String;
    /// `self` as one part when it fits, else cut per eval.py's `chunks`.
    pub fn parts(&self, chapters: &[VideoChapter], max_chars: usize) -> Vec<TranscriptPart>;
}

// domain/video_transcript/caption_language.rs
impl CaptionLanguage {
    /// `Spanish` for `es-US`; the code itself when unknown.
    pub fn english_name(&self) -> String;
    /// 4.3 for `en`, 3.4 otherwise (measured on the eval golden set).
    pub fn chars_per_token(&self) -> f64;  // 4.3 en, 1.5 ja/zh/ko, 3.4 otherwise
}

// domain/video_transcript/transcript_part.rs
pub struct TranscriptPart { pub start_seconds: i64, pub end_seconds: i64, pub text: String }

// domain/preferences/context_length.rs
impl ContextLength {
    pub fn input_budget_tokens(&self) -> u32;  // value * 9000 / 12288
    pub fn output_budget_tokens(&self) -> u32; // value - input budget
}

// domain/preferences/summary_prompt.rs
impl SummaryPrompt {
    /// Re-checks a `from_stored` prompt.
    pub fn validated(&self) -> Result<Self, ValidationError>;
    /// `section`'s body with `{name}`s replaced in one pass.
    pub fn render(&self, section: SummaryPromptSection, values: &[(&str, String)]) -> String;
}

// domain/llm_server/llm_chat.rs
pub struct LlmChat {
    pub model: String,
    pub system: String,
    pub user: String,
    pub context_length: u32,
    pub temperature: f64,
    pub thinking: bool,
    pub max_output_tokens: u32,
}

// domain/llm_server/llm_reply.rs
pub struct LlmReply {
    pub content: String,
    pub input_tokens: u32,
    pub stopped_at_limit: bool,
    /// Why Ollama wouldn't answer this request: too long, or past the timeout.
    pub refusal: Option<String>,
}

// domain/llm_server/llm_server.rs
impl LlmServer { pub fn model(&self, name: &str) -> Option<&LlmModel>; }

// domain/video_summary/summary_text.rs
pub struct SummaryText(String);
impl SummaryText {
    pub fn new(value: impl Into<String>) -> Self; // as stored
    /// `answer` after its last `</think>`, trimmed; rejects an empty result.
    pub fn from_answer(answer: &str) -> Result<Self, ValidationError>;
    pub fn as_str(&self) -> &str;
}

// domain/video_summary/video_summary.rs
pub struct VideoSummary {
    pub video_id: VideoRecordId,
    pub text: SummaryText,
    pub model: String,
    pub created_at: DateTime<Utc>,
}
impl VideoSummary {
    pub fn create(video_id: VideoRecordId, text: SummaryText, model: &SummaryModel, now: DateTime<Utc>) -> Self;
}

// domain/video_summary/summary_input.rs
// domain/video_summary/summary_messages.rs
pub struct SummaryMessages { pub system: String, pub user: String }

// domain/video_summary/summary_input.rs
pub struct SummaryInput { /* title, minutes, lang, chapters, parts */ }
impl SummaryInput {
    pub fn new(video: &Video, language: &CaptionLanguage, transcript: &TimedTranscript, budget: &ContextLength) -> Self;
    pub fn parts(&self) -> usize;
    /// `system` + `user`; only when `parts() == 1`.
    pub fn whole(&self, prompt: &SummaryPrompt) -> SummaryMessages;
    /// `map system` + `map user` for the 0-based `index`.
    pub fn part(&self, prompt: &SummaryPrompt, index: usize) -> SummaryMessages;
    /// `reduce system` + `reduce user`, one `reduce part` per answer.
    pub fn answers(&self, summaries: Vec<SummaryText>) -> Vec<PartAnswer>;
    pub fn reduce(&self, prompt: &SummaryPrompt, answers: &[PartAnswer]) -> SummaryMessages;
    /// A reduce of `answers` without the chapters.
    pub fn reduce_group(&self, prompt: &SummaryPrompt, answers: &[PartAnswer]) -> SummaryMessages;
    /// Whether `answers` and the chapters fit the budget together.
    pub fn reduce_fits(&self, answers: &[PartAnswer]) -> bool;
    /// Consecutive groups within the budget, at least two answers each
    /// (a lone last answer stays alone), so each round shrinks.
    pub fn reduce_groups(&self, answers: &[PartAnswer]) -> Vec<Vec<PartAnswer>>;
}

// domain/services/video_summarizer.rs
pub struct VideoSummarizer {
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    preferences_repository: Arc<dyn PreferencesRepository>,
    video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    llm_server_repository: Arc<dyn LlmServerRepository>,
    video_summary_repository: Arc<dyn VideoSummaryRepository>,
    clock: Arc<dyn Clock>,
}
pub trait VideoSummarizerApi: Send + Sync {
    /// See the `video-summaries` capability. An error fails the task, for the
    /// queue to retry. Holds no `VideoLock`: generation takes minutes.
    fn summarize(&self, video: &Video, is_last_attempt: bool) -> anyhow::Result<()>;
}

// infrastructure/repositories/ollama_llm_server_repository.rs
pub trait LlmServerRepository: Send + Sync {
    fn find(&self, url: &OllamaUrl) -> anyhow::Result<LlmServer>;
    /// `POST /api/chat`, `stream: false`, `truncate: false`, `shift: false`,
    /// 30-minute timeout. A 400 or the timeout is a reply with a `refusal`;
    /// any other failure is an `Err`, the server's fault.
    fn chat(&self, url: &OllamaUrl, chat: &LlmChat) -> anyhow::Result<LlmReply>;
}

// infrastructure/repositories/sqlite_video_summary_repository.rs
pub trait VideoSummaryRepository: Send + Sync {
    fn save(&self, summary: &VideoSummary) -> anyhow::Result<()>;
    /// No production reader yet: nothing displays summaries.
    #[allow(dead_code)]
    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoSummary>>;
    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
}

// infrastructure/repositories/sqlite_video_repository.rs
pub trait VideoRepository { /* ... */
    /// Writes only `summary_state` and `updated_at`.
    fn update_summary(&self, video: &Video) -> anyhow::Result<()>;
    /// Puts every Unavailable summary back to Missing.
    fn reset_unavailable_summaries(&self, now: DateTime<Utc>) -> anyhow::Result<()>;
}
```

```sql
CREATE TABLE video_summaries (
    video_id TEXT PRIMARY KEY, text TEXT NOT NULL, model TEXT NOT NULL, created_at TEXT NOT NULL
);
```

Notes:
- Timed lines come from the SRT on disk: `video_transcripts.text` has no timings, and chapter cuts and `{span}` need them. The transcript row supplies the language.
- The existing `find` (5 s timeout) doubles as the reachability and model check, so "Ollama unavailable" is a skip, not a failure.
- `truncate: false` is the truncation check: Ollama 0.40.13 otherwise cuts an oversized prompt to about half the context and reports only what it kept, so no token count can reveal it. Refused, it answers 400 at once with the exact count. `shift: false` keeps the output from pushing the prompt out of the context.
- Failures use the task queue's retries (`is_last_attempt`, as `DownloadVideoTask` does), so summaries need no attempt counter. The only state of their own is the final Unavailable.
- Thinking uses `LlmModel::thinking`: Ollama rejects `think: true` for models without the capability, and the UI keeps the saved value for them.
- Logs live in `VideoSummarizer` (domain), not the Ollama adapter; `ask` times each call with `Instant`, and full prompts stay at info as the user chose.
- The task must keep its exclusivity key: without one, the queue doesn't dedupe and each pass would add a duplicate.

## Call Stack

Scheduling (reconcile pass or `SyncVideoTask`):
```
VideoReconciler::reconcile(video_id)
  ... existing steps ...
  video.is_summarizable() && summaries_wanted(&owner)  // read preferences only when needed
  schedule_steps(&video, &owner.target, summaries_wanted)
    VideoSyncPlan::for_video(video, summaries_wanted, now)
    TaskRepository::schedule(Task::SummarizeVideo { video_id }, now)
```

Trigger after subtitles:
```
VideoSubtitlesFetched
  SyncVideoOnVideoSubtitlesFetched::handle(payload)
    TaskRepository::schedule(Task::SyncVideo { video_id }, now)
```

Generation:
```
SummarizeVideoTask::handle(payload, _)
  Task::decode_summarize_video_payload(payload)
  VideoRepository::find(id)                // gone -> Ok
  VideoSummarizer::summarize(&video)
    PreferencesRepository::find()          // !summaries_available -> return
    VideoOwnerFinder::find(video)          // none / !summaries_enabled -> return
    video.is_summarizable()                // false -> return
    effective_prompt().validated()         // Err -> warn, return
    LlmServerRepository::find(url)         // Err -> warn, return
    LlmServer::model(name)                 // None -> warn, return
    --- failures below: error!(reason); on the last attempt, unless chat returned Err: mark_summary_unavailable + update_summary; return Err ---
    VideoTranscriptRepository::find(id)    // None -> failure
    VideoFileRepository::read_to_string(output_dir, subtitle_filename) // None -> failure
    TimedTranscript::from_srt(srt)         // no lines -> failure
    SummaryInput::new(video, language, transcript, context_length)
    info!(video_id, title, video_seconds, parts, model)
    parts == 1:
      ask(input.whole(prompt))
    parts > 1:
      ask(input.part(prompt, i))           // each, stop at the first failure
      reduced(answers)
    ask(messages)
      info!(part, system, user)            // full messages
      LlmServerRepository::chat(url, LlmChat { .. })
      info!(answer, input_tokens, duration_seconds)
      reply.refusal or reply.stopped_at_limit -> failure
      SummaryText::from_answer(content)
    reduced(answers)
      input.reduce_fits(answers) or one left: ask(input.reduce(prompt, answers))
      else: ask(input.reduce_group(prompt, group)) for each input.reduce_groups(answers), then reduced(results)
    VideoRepository::find(id)              // gone -> return
    VideoSummaryRepository::save(VideoSummary::create(..))
    VideoRepository::update_summary(video.mark_summary_generated(now))
    info!(video_id, summary, duration_seconds, video_seconds, model)
```

Reset:
```
PreferencesUpdater::update(change)
  PreferencesRepository::save(updated)
  summary settings or ollama_url changed: VideoRepository::reset_unavailable_summaries(now)
```

Deletion:
```
{Channel,Playlist}{Deleter,VideoReconciler}
  VideoTranscriptRepository::delete(id)
  VideoSummaryRepository::delete(id)
```

## Test Plan

Added by the audit: see tasks section 5.

Behaviour (application layer):

`src/application/tasks/summarize_video_task.rs`
1. `it_should_store_the_summary_of_a_video_that_fits_one_call` — fake reply stored with model; state Generated; one chat.
2. `it_should_send_the_shipped_prompt_filled_with_the_videos_data` — exact system and user messages, `lang` from the language name.
3. `it_should_send_the_saved_prompt_when_there_is_one` — messages come from the saved text.
4. `it_should_send_the_saved_model_settings` — model, context, temperature, output cap in the chat.
5. `it_should_list_the_chapters_in_the_prompt` — `chapters` section, `0:00 Intro`.
6. `it_should_leave_chapters_out_of_the_prompt_when_the_video_has_none` — no block.
7. `it_should_keep_braces_in_values_unfilled` — title `{transcript} explained` unchanged.
8. `it_should_use_the_language_code_when_its_name_is_unknown` — `xh`.
9. `it_should_drop_the_reasoning_before_the_answer` — `</think>` stripped from the stored text.
10. `it_should_ask_for_thinking_only_when_the_model_supports_it` — table of setting × capability.
11. `it_should_summarize_a_long_transcript_in_parts_then_reduce` — N map chats + reduce holding answers in order.
12. `it_should_cut_parts_at_chapter_starts` — each part starts at a chapter; only overlapping chapters listed; `{span}`.
13. `it_should_cut_by_size_without_chapters` — parts under the budget, no text lost.
14. `it_should_cut_a_chapter_too_long_on_its_own_by_size` — others whole.
15. `it_should_scale_the_budget_with_the_context_length` — same text single at 12288, parts at 6144.
16. `it_should_skip_if_summaries_are_unavailable` — table AI/summaries off; no chat.
17. `it_should_skip_if_the_source_opted_out` — no chat, Missing.
18. `it_should_skip_a_video_already_summarized` — no chat, summary unchanged.
19. `it_should_ignore_a_missing_video` — `Ok`.
20. `it_should_skip_when_the_saved_prompt_is_invalid` — no chat, Missing, `Ok`.
21. `it_should_skip_when_ollama_is_unreachable` — task `Ok`, Missing.
22. `it_should_skip_when_the_model_is_not_installed` — same.
23. `it_should_fail_the_task_and_stay_missing_on_a_server_fault` — `chat` errs, even on the last attempt; nothing stored.
24. `it_should_fail_the_task_when_the_answer_stopped_at_the_limit` — Missing before the last attempt; nothing stored.
25. `it_should_fail_the_task_when_ollama_refuses_the_prompt` — reply with a `refusal`; nothing stored.
26. `it_should_fail_the_task_when_the_answer_is_only_reasoning` — nothing stored.
27. `it_should_stop_at_the_first_failed_part` — no later chats, task fails.
28. `it_should_fail_the_task_when_the_subtitle_file_is_gone` — no chat.
29. `it_should_become_unavailable_when_rejected_on_the_last_attempt` — state Unavailable, task fails.
30. `it_should_ignore_a_video_deleted_while_generating` — fake chat hook deletes it.

`src/application/tasks/sync_video_task.rs`
31. `it_should_schedule_a_summary_for_a_subtitled_video_in_an_opted_in_source` — one `summarize_video`.
32. `it_should_not_schedule_a_summary_when_the_source_opted_out` — none.
33. `it_should_not_schedule_a_summary_while_ai_summaries_are_off` — table.
34. `it_should_not_schedule_a_summary_without_fetched_subtitles` — Missing, Unavailable.
36. `it_should_not_schedule_a_settled_summary` — Generated, Unavailable.
37. `it_should_not_queue_a_second_summary` — dedupe, in `sqlite_task_repository`.

`src/application/tasks/reconcile_channel_task.rs`
38. `it_should_schedule_summaries_for_the_stored_videos_of_an_opted_in_channel` — one per video.

`src/application/subscribers/sync_video_on_video_subtitles_fetched.rs`
39. `it_should_schedule_a_video_sync` — `sync_video` now.

`src/application/http/tasks/mod.rs`
40. `it_should_list_a_summarize_video_task_with_video_and_channel_names`.
41. `it_should_list_a_summarize_video_task_whose_video_is_gone` — no title.

`src/application/http/playlists/mod.rs`, `channels/mod.rs`
42. `it_should_delete_the_summaries_of_a_deleted_playlist`.
43. `it_should_delete_the_summaries_of_a_deleted_channel`.

`src/application/tasks/reconcile_channel_task.rs`, `reconcile_playlist_task.rs`
44. `it_should_delete_the_summary_of_a_video_removed_from_the_channel`.
45. `it_should_delete_the_summary_of_a_video_removed_from_the_playlist`.

`src/domain/task/task.rs`
46. `it_should_run_summarize_video_in_the_summaries_lane_exclusive_per_video` — lane, key.

`src/infrastructure/repositories/task_executor.rs`
47. `it_should_run_one_summary_at_a_time` — lane capacity 1.
48. `it_should_not_hold_other_lanes_back_while_summarizing`.

`web/src/lib/tasks.test.ts`
49. `describes a summarize task` — `Summarizing <video>`; fallback without a title.

`src/application/tasks/summarize_video_task.rs` (logging)
50. `it_should_log_each_calls_prompt_answer_and_duration` — captured info logs hold both messages, the answer and `duration_seconds`.
51. `it_should_log_the_summary_total_time_and_video_length` — final entry fields.
52. `it_should_log_why_a_generation_was_skipped_or_failed` — info reason on skip, error reason on failure.

Infrastructure:

`ollama_llm_server_repository`
53. `it_should_post_the_chat_with_its_options` — mockito body: model, messages, `stream: false`, `truncate: false`, `shift: false`, `think`, `num_ctx`, `temperature`, `num_predict`.
54. `it_should_read_the_answer_and_input_tokens`.
55. `it_should_report_a_reply_stopped_at_the_limit` — `done_reason: "length"`.
56. `it_should_fail_on_an_error_status` — 500 and 404; 400 is `it_should_report_a_refusal_on_a_bad_request`.
57. `it_should_fail_on_an_answer_that_is_not_ollamas`.

`sqlite_video_summary_repository`
58. `it_should_save_and_find_a_summary`.
59. `it_should_replace_a_videos_summary`.
60. `it_should_delete_a_summary`.

`sqlite_video_repository`
61. `it_should_read_missing_summary_state_for_a_new_video`.
62. `it_should_update_only_the_summary_fields`.

`sqlite_migrations`
63. `it_should_start_stored_videos_with_a_missing_summary`.
