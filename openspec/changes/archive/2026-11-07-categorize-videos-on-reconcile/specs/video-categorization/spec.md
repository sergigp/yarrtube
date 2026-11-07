# Spec Delta

## Purpose

Decides, with the AI, which categories with an AI prompt each stored video fits, asking the model at most once per video and category, so reconcile passes don't repeat hours of evaluation.

## ADDED Requirements

### Requirement: Verdicts
Each stored video copy SHALL have at most one verdict per category: `yes`, `no` or `unavailable`, recorded with who decided it (`ai` for now), when, and for an AI verdict the model and its one-sentence reason. A pair with a verdict SHALL NOT be evaluated again unless that verdict is removed.

#### Scenario: Answer stored
- **WHEN** the model answers that a video of "@GothamChess" doesn't fit "Sicilian"
- **THEN** the video has a `no` verdict for "Sicilian" decided by `ai`, with the model and the reason

#### Scenario: No second evaluation
- **WHEN** a video has a `no` verdict for "Sicilian" and its channel's reconcile pass runs again
- **THEN** the model is not asked about that video and "Sicilian"

### Requirement: Pending Pairs
A category SHALL be pending for a stored video when the category has an AI prompt, the video's playlist or channel is not a member of it, and the video has no verdict for it.

#### Scenario: Category given by the source
- **WHEN** "@GothamChess" is a member of "Chess", which has an AI prompt, and a video of it has no verdict for "Chess"
- **THEN** "Chess" is not pending for that video

#### Scenario: Category without a prompt
- **WHEN** the category "Funny" has no AI prompt
- **THEN** "Funny" is pending for no video

#### Scenario: Source leaves a category
- **WHEN** "@GothamChess" is removed from "Chess", which has an AI prompt, and a video of it has no verdict for "Chess"
- **THEN** "Chess" is pending for that video

### Requirement: When A Video Is Categorized
A video's sync SHALL schedule its categorization only when AI, categories and categorization are on in the preferences, the video's playlist or channel has `categorization_enabled` `true`, the video's summary state is Generated and at least one category is pending for it. A categorization already pending or running for the video SHALL NOT be scheduled again. Otherwise the sync SHALL schedule nothing for categorization. Generating a summary SHALL NOT itself sync the video: a newly summarized video is categorized from its next sync, at the latest its source's next reconcile pass.

#### Scenario: Video with a summary and pending categories
- **WHEN** a video of a channel with categorization enabled has a Generated summary and "Sicilian" is pending for it, and its sync runs
- **THEN** a categorization of that video is scheduled

#### Scenario: Video without a summary
- **WHEN** a video of a channel with categorization enabled has summary state Missing and its sync runs
- **THEN** no categorization is scheduled

#### Scenario: Nothing pending
- **WHEN** a video has a verdict for every category with an AI prompt and its sync runs
- **THEN** no categorization is scheduled and the model is not asked anything

#### Scenario: Source with categorization off
- **WHEN** a video of a playlist with `categorization_enabled` `false` has a Generated summary and pending categories, and its sync runs
- **THEN** no categorization is scheduled

#### Scenario: Summary generated after the video's sync
- **WHEN** a new video's summary is generated after its sync ran
- **THEN** no categorization is scheduled until its next sync

### Requirement: Categorization Run
A categorization SHALL re-check, when it runs, the conditions for scheduling it and do nothing if they no longer hold. It SHALL then ask the model about each category pending at that moment, one call per category, and store each verdict as soon as its answer is read. A verdict SHALL NOT be stored for a category deleted, or a video removed, while it was being evaluated or while its verdict was being stored.

#### Scenario: Category deleted before the run
- **WHEN** a categorization is scheduled while "Sicilian" is pending, and "Sicilian" is deleted before it runs
- **THEN** the model is not asked about "Sicilian"

#### Scenario: Category deleted during its call
- **WHEN** "Sicilian" is deleted while the model is being asked about it for a video
- **THEN** no verdict for "Sicilian" is stored and the video doesn't belong to it

#### Scenario: Video removed during its call
- **WHEN** a video drops out of its channel's recent uploads while the model is being asked about "Sicilian" for it
- **THEN** no verdict for that video is stored

#### Scenario: Video removed while its verdict is stored
- **WHEN** a video is removed while its verdict for "Sicilian" is being stored
- **THEN** no verdict and no category membership of that video remain

#### Scenario: Prompt added before the run
- **WHEN** a categorization is scheduled and "Tech" gets an AI prompt before it runs
- **THEN** the model is asked about "Tech" too

### Requirement: What The Model Is Asked
Each call SHALL send the saved categorization prompt (or the shipped one) as the system message. The user message SHALL hold the video's title, channel or playlist name, length in whole minutes, chapters (each as `<start> <title>`, or `(none)`), summary, and the category's name and AI prompt, and SHALL ask for a JSON answer with a one-sentence `reason` then a boolean `belongs`.

#### Scenario: Request sent
- **WHEN** a video titled "Crushing the Sicilian" with chapter "0:00 Intro" is evaluated for "Sicilian"
- **THEN** the user message holds the title, the channel name, the length, `0:00 Intro`, the summary, "Sicilian" and its AI prompt, and asks for `reason` and `belongs`

### Requirement: Categorization Request Settings
Each request SHALL use the saved categorization model, temperature 0, a context length of 4096, at most 3072 output tokens and the answer's JSON schema. Thinking SHALL be requested when Ollama lists it among the model's capabilities. Every request SHALL tell Ollama neither to truncate the prompt nor to shift the context.

#### Scenario: Thinking model
- **WHEN** the categorization model is `qwen3:4b` and Ollama lists `thinking` among its capabilities
- **THEN** the request asks for thinking, temperature 0, context 4096, at most 3072 output tokens and the JSON schema

### Requirement: Answer Read
Everything up to and including the last `</think>` SHALL be dropped from the answer. The answer SHALL be usable when what is left holds a JSON object with a boolean `belongs`; `belongs` `true` SHALL be stored as `yes` and `false` as `no`, with its `reason`.

#### Scenario: Thinking leaked into the answer
- **WHEN** the model answers `<think>…</think>{"reason": "A Sicilian game.", "belongs": true}`
- **THEN** the verdict is `yes` with reason "A Sicilian game."

### Requirement: Rejected Calls
A call SHALL be rejected when the answer isn't usable, when Ollama stopped at the output limit, when Ollama refuses the prompt, or when it runs past the 30-minute timeout. A rejected call SHALL fail the categorization once every pending category has been tried, so the task queue retries it with its usual backoff; categories already answered keep their verdicts. On the last attempt a rejected category SHALL get an `unavailable` verdict.

#### Scenario: Output limit on the last attempt
- **WHEN** the model stops at the output limit for "Food for thought" on the categorization's last attempt
- **THEN** the video has an `unavailable` verdict for "Food for thought" and doesn't belong to it

#### Scenario: Retry resumes
- **WHEN** a categorization answers "Sicilian" then fails on "Tech", and the task is retried
- **THEN** the retry asks only about "Tech"

### Requirement: Ollama Unavailable For Categorization
When Ollama can't be reached or the saved categorization model isn't installed on it, the categorization SHALL stop with a logged warning, storing nothing more, and SHALL complete, so a later sync tries again.

#### Scenario: Ollama down
- **WHEN** Ollama can't be reached when a categorization runs
- **THEN** no verdict is stored, the task completes, and the categories stay pending

### Requirement: Unavailable Verdicts Reset
Saving a change to the Ollama URL, the categorization model or the categorization prompt SHALL delete every `unavailable` verdict. Saving a different AI prompt for a category SHALL delete that category's `unavailable` verdicts. Saving the same values SHALL change nothing. A categorization that started before such a change SHALL NOT store `unavailable` verdicts after it.

#### Scenario: New model
- **WHEN** a video has an `unavailable` verdict for "Tech" and the categorization model is changed
- **THEN** "Tech" is pending for the video again

#### Scenario: Model changed during a last attempt
- **WHEN** the categorization model is changed while a video's last attempt is running and "Tech" is rejected
- **THEN** "Tech" is pending for the video again

### Requirement: Settings Changes Keep Verdicts
Editing or clearing a category's AI prompt, turning a source's categorization off, and turning categorization or AI off SHALL keep every `yes` and `no` verdict, so the videos keep their categories and nothing is evaluated again.

#### Scenario: Prompt edited
- **WHEN** "Sicilian"'s AI prompt is edited
- **THEN** no video is evaluated again for "Sicilian" and every video with a `yes` verdict still belongs to it

#### Scenario: Source categorization turned off
- **WHEN** "@GothamChess" has a video with a `yes` verdict for "Sicilian" and its categorization is turned off
- **THEN** the video still belongs to "Sicilian" and no new video of it is evaluated

### Requirement: Categorization Logged
Each call SHALL be logged at info level with the video, the category, the verdict, the reason, the tokens and the duration. Skips SHALL be logged with their reason at info level, and rejected calls at error level with the reason.

#### Scenario: Verdict logged
- **WHEN** the model answers for a video and "Sicilian"
- **THEN** an info line names the video, "Sicilian", the verdict, the reason and the duration

### Requirement: Verdicts Deleted With Their Video
Removing a stored video copy SHALL delete all its verdicts, in the same step that deletes its transcript, summary and categories: when it drops out of its channel's recent uploads or its playlist, and when its channel or playlist is deleted.

#### Scenario: Video rotated out of a channel
- **WHEN** a video of "@GothamChess" with an AI `yes` for "Sicilian" is no longer among the channel's recent uploads and the channel reconciles
- **THEN** the video and every verdict of it are deleted

#### Scenario: Playlist deleted
- **WHEN** a playlist whose videos have verdicts is deleted
- **THEN** none of those verdicts remains
