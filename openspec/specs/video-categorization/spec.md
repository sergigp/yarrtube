# video-categorization Specification

## Purpose

Decides, with the AI, which categories with an AI prompt each stored video fits, asking the model at most once per video and category, so reconcile passes don't repeat hours of evaluation.

## Requirements

### Requirement: Verdicts
Each stored video copy SHALL have at most one verdict per category: `yes`, `no` or `failed` (formerly `unavailable`: the AI gave up on the pair), recorded with who decided it (`ai` for now), when, and for an AI verdict the model and its one-sentence reason. A pair with a verdict SHALL NOT be evaluated again unless that verdict is removed.

#### Scenario: Answer stored
- **WHEN** the model answers that a video of "@GothamChess" doesn't fit "Sicilian"
- **THEN** the video has a `no` verdict for "Sicilian" decided by `ai`, with the model and the reason

#### Scenario: No second evaluation
- **WHEN** a video has a `no` verdict for "Sicilian" and its channel's reconcile pass runs again
- **THEN** the model is not asked about that video and "Sicilian"

#### Scenario: Stored verdicts upgraded
- **WHEN** the system starts on a database with `unavailable` verdicts
- **THEN** they are `failed` verdicts

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
A video's sync SHALL schedule its categorization only when AI, categories and categorization are on in the preferences, the video's playlist or channel has `categorization_enabled` `true`, the video's summary is Done, at least one category is pending for it and its categorization is Pending, Done, or Retrying and due (see `video-retries`). A categorization already pending or running for the video SHALL NOT be scheduled again. Otherwise the sync SHALL schedule nothing for categorization. Generating a summary SHALL NOT itself sync the video: a newly summarized video is categorized from its next sync, at the latest its source's next reconcile pass.

#### Scenario: Video with a summary and pending categories
- **WHEN** a video of a channel with categorization enabled has a Done summary and "Sicilian" is pending for it, and its sync runs
- **THEN** a categorization of that video is scheduled

#### Scenario: Video without a summary
- **WHEN** a video of a channel with categorization enabled has a Pending summary and its sync runs
- **THEN** no categorization is scheduled

#### Scenario: Nothing pending
- **WHEN** a video has a verdict for every category with an AI prompt and its sync runs
- **THEN** no categorization is scheduled and the model is not asked anything

#### Scenario: Source with categorization off
- **WHEN** a video of a playlist with `categorization_enabled` `false` has a Done summary and pending categories, and its sync runs
- **THEN** no categorization is scheduled

#### Scenario: Summary generated after the video's sync
- **WHEN** a new video's summary is generated after its sync ran
- **THEN** no categorization is scheduled until its next sync

#### Scenario: Categorization failed recently
- **WHEN** a video with pending categories whose categorization is Retrying, having failed once 3 minutes ago, is synced with no categorization pending
- **THEN** no categorization is scheduled

### Requirement: Categorization Run
A categorization SHALL re-check, when it runs, the conditions for scheduling it and do nothing if they no longer hold. It SHALL then ask the model about every category pending at that moment in one call, and store each verdict of that call's answer as soon as the answer is read. A verdict SHALL NOT be stored for a category deleted, or a video removed, while it was being evaluated or while its verdict was being stored.

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

#### Scenario: All pending categories in one call
- **WHEN** "Sicilian", "Tech" and "AI news" are pending for a video and its categorization runs
- **THEN** the model is called once, about the three, and a verdict is stored for each

#### Scenario: One category's prompt edited during the call
- **WHEN** "Tech"'s AI prompt is edited while the model is being asked about "Sicilian" and "Tech" for a video
- **THEN** the verdict for "Sicilian" is stored, none for "Tech", and "Tech" stays pending

### Requirement: What The Model Is Asked
Each call SHALL send the saved categorization prompt (or the shipped one) as the system message. The user message SHALL hold the video's title, channel or playlist name, length in whole minutes, chapters (each as `<start> <title>`, or `(none)`) and summary, then every category asked about, each as its name and AI prompt, and SHALL ask for a JSON answer with one entry per category, keyed by its name, each a one-sentence `reason` then a boolean `belongs`.

#### Scenario: Request sent
- **WHEN** a video titled "Crushing the Sicilian" with chapter "0:00 Intro" is evaluated for "Sicilian" and "Tech"
- **THEN** the user message holds the title, the channel name, the length, `0:00 Intro`, the summary, "Sicilian" and "Tech" with their AI prompts, and asks for `reason` and `belongs` for each, keyed by name

#### Scenario: One category
- **WHEN** a video is evaluated for "Sicilian" only
- **THEN** the user message lists "Sicilian" alone, in the same shape

### Requirement: Categorization Request Settings
Each request SHALL use the saved categorization model, temperature 0, the categorization context length (see `preferences`), the answer's JSON schema with every category asked about required, and an output cap of 1,365 tokens per category asked, at least 3,072 and at most the context length. Thinking SHALL be requested when Ollama lists it among the model's capabilities. Every request SHALL tell Ollama neither to truncate the prompt nor to shift the context.

#### Scenario: Thinking model
- **WHEN** the categorization model is `qwen3:4b`, Ollama lists `thinking` among its capabilities, the context length is Auto with 9 categories with an AI prompt, and 9 are asked about
- **THEN** the request asks for thinking, temperature 0, context 16384, at most 12285 output tokens and the JSON schema with the 9 categories

#### Scenario: One category
- **WHEN** one category is asked about
- **THEN** the request allows at most 3072 output tokens

#### Scenario: Cap above the context length
- **WHEN** the context length is overridden to `8192` and 9 categories are asked about
- **THEN** the request allows at most 8192 output tokens

### Requirement: Answer Read
Everything up to and including the last `</think>` SHALL be dropped from the answer. The answer SHALL be usable for a category when what is left holds a JSON object whose entry under that category's name holds a boolean `belongs`; `belongs` `true` SHALL be stored as `yes` and `false` as `no`, with its `reason`. Categories with a usable entry SHALL be stored even when other categories of the same answer aren't usable.

#### Scenario: Thinking leaked into the answer
- **WHEN** the model answers `<think>…</think>{"Sicilian": {"reason": "A Sicilian game.", "belongs": true}}` about "Sicilian"
- **THEN** the verdict is `yes` with reason "A Sicilian game."

#### Scenario: One entry unusable
- **WHEN** the model answers about "Sicilian" and "Tech" with a usable "Sicilian" entry and a "Tech" entry without `belongs`
- **THEN** the verdict for "Sicilian" is stored and "Tech" has no usable answer

### Requirement: Ollama Unavailable For Categorization
Before asking, the categorization SHALL check the server and its models. When Ollama can't be connected to, on that check or on any call of the run (the fallback included), or the saved categorization model isn't installed on it, the categorization SHALL stop with a logged warning, storing nothing more, leave its status and failed attempts unchanged, write no `failed` verdict, and re-queue it 6 hours later (see "Waiting For Ollama" in `video-retries`), without failing the task. Verdicts stored earlier in the run SHALL be kept.

#### Scenario: Ollama down
- **WHEN** Ollama can't be connected to when a categorization runs
- **THEN** no verdict is stored, the categorization keeps its status and failed attempts, it is re-queued 6 hours later, and the categories stay pending

#### Scenario: Ollama down on the eleventh attempt
- **WHEN** Ollama can't be connected to when a categorization with 10 consecutive failed attempts runs and "Tech" is pending
- **THEN** the categorization is Retrying with 10 failed attempts, "Tech" has no verdict and stays pending, and it is re-queued 6 hours later

#### Scenario: Model not installed
- **WHEN** a categorization runs with thinking off and the saved categorization model isn't among those Ollama lists
- **THEN** Ollama is not asked, the categorization keeps its status and failed attempts, and it is re-queued 6 hours later

#### Scenario: Ollama goes down during the fallback
- **WHEN** the call about "Sicilian" and "Tech" answers usably for "Sicilian" only, and Ollama can't be connected to for the call about "Tech" alone
- **THEN** "Sicilian"'s verdict is kept, "Tech" stays pending with no verdict, the categorization keeps its status and failed attempts, and it is re-queued 6 hours later

#### Scenario: Connection dropped after it was accepted
- **WHEN** Ollama accepts the connection for a categorization call and then drops it without answering
- **THEN** the categorization is Retrying with one more failed attempt, as for a server fault

### Requirement: Failed Verdicts Reset
Saving a change to the Ollama URL, the categorization model or the categorization prompt SHALL delete every `failed` verdict. Saving a different AI prompt for a category SHALL delete that category's `failed` verdicts. The categorization of every video that had a deleted `failed` verdict SHALL be Pending; other videos' categorization SHALL be unchanged. Saving the same values SHALL change nothing. A categorization that started before such a change SHALL NOT store `failed` verdicts after it.

#### Scenario: New model
- **WHEN** a video has an `failed` verdict for "Tech" and the categorization model is changed
- **THEN** "Tech" is pending for the video again

#### Scenario: Another category's failure kept
- **WHEN** a video's categorization is Failed with a `failed` verdict for "Chess" and "Tech"'s AI prompt is changed
- **THEN** its categorization is still Failed and its "Chess" verdict is kept

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
Each call SHALL be logged at info level with the video, the categories asked, whether it is a fallback call, the input and output tokens and the duration. Each stored verdict SHALL be logged at info level with the video, the category, the verdict and the reason. A rejected call about several categories SHALL be logged at warning level with the reason and that it falls back. Skips SHALL be logged with their reason at info level, and rejected categories of a call about one category at error level with the reason.

#### Scenario: Verdict logged
- **WHEN** the model answers for a video about "Sicilian" and "Tech"
- **THEN** an info line names the video, both categories, the tokens and the duration, and an info line per category names the video, the category, the verdict and the reason

#### Scenario: Fallback logged
- **WHEN** the call about 9 categories for a video stops at the output limit
- **THEN** a warning names the video, the output limit and that each category is asked on its own

### Requirement: Verdicts Deleted With Their Video
Removing a stored video copy SHALL delete all its verdicts, in the same step that deletes its transcript, summary and categories: when it drops out of its channel's recent uploads or its playlist, and when its channel or playlist is deleted.

#### Scenario: Video rotated out of a channel
- **WHEN** a video of "@GothamChess" with an AI `yes` for "Sicilian" is no longer among the channel's recent uploads and the channel reconciles
- **THEN** the video and every verdict of it are deleted

#### Scenario: Playlist deleted
- **WHEN** a playlist whose videos have verdicts is deleted
- **THEN** none of those verdicts remains

### Requirement: Categorization Status
Every video SHALL record a categorization status from the `video-retries` vocabulary: Pending (a run is wanted and none succeeded since the categories last changed), Retrying, Done (the last run left nothing rejected) or Failed, with its consecutive failed attempts. Adding or re-running a category for the video, giving a category its first AI prompt, deleting its AI verdicts, or a `failed` verdict being reset SHALL make it Pending again, whether or not categorization is on for its playlist or channel. Making it Pending SHALL delete its remaining `failed` verdicts, so the next run asks those categories again.

#### Scenario: Run leaves nothing rejected
- **WHEN** a categorization stores a verdict for every pending category
- **THEN** the video's categorization is Done with no failed attempts

#### Scenario: New category after a run
- **WHEN** a video's categorization is Done and a category with an AI prompt is added
- **THEN** its categorization is Pending

#### Scenario: Failed categories asked again
- **WHEN** a video's categorization is Failed with a `failed` verdict for "Chess" and a category "Food" with an AI prompt is added
- **THEN** its categorization is Pending, it has no verdict for "Chess", and its next run asks about "Chess" and "Food"

#### Scenario: Category given a prompt
- **WHEN** a video's categorization is Failed and a category without an AI prompt is given one
- **THEN** its categorization is Pending

### Requirement: Rejected Category Calls
A call SHALL be rejected when Ollama stopped at the output limit, when Ollama refuses the prompt, or when it runs past its timeout: its output cap at 5 tokens a second, at least 30 minutes; a category SHALL be rejected when its call is rejected or the answer isn't usable for it. When a call about several categories leaves any rejected, the categorization SHALL ask about each of them again in its own call, in the same run. A category rejected by its own call SHALL make the categorization Retrying with one more failed attempt once every pending category has been tried, and the categorization SHALL be re-queued under the `video-retries` policy without failing the task; categories already answered keep their verdicts. After the 11th consecutive failed attempt the categorization SHALL be Failed and every category still pending SHALL get a `failed` verdict. A run that leaves nothing rejected SHALL make it Done and clear the failed attempts.

#### Scenario: Batch hits the output limit
- **WHEN** the call about 9 categories for a video stops at the output limit
- **THEN** the model is asked about each of the 9 in its own call, in the same run, and each usable answer is stored

#### Scenario: Batch partly usable
- **WHEN** the call about "Sicilian" and "Tech" answers usably for "Sicilian" only
- **THEN** "Sicilian"'s verdict is stored and "Tech" is asked about in its own call

#### Scenario: Output limit in a category's own call
- **WHEN** the model stops at the output limit in the call about "Food for thought" alone, on the categorization's first attempt
- **THEN** "Food for thought" stays pending, the categorization is Retrying with one failed attempt, and it is re-queued 10 minutes later

#### Scenario: Eleventh failed categorization
- **WHEN** the call about "Food for thought" alone is rejected for a video whose categorization has 10 consecutive failed attempts
- **THEN** the categorization is Failed, the video has a `failed` verdict for "Food for thought" and doesn't belong to it

#### Scenario: Retry resumes
- **WHEN** a categorization answers "Sicilian" then its own call about "Tech" is rejected, and the categorization runs again
- **THEN** it asks only about "Tech"

#### Scenario: Timeout scales with the cap
- **WHEN** a call about 9 categories allows 12285 output tokens
- **THEN** it is rejected only after 2457 seconds without an answer, and a call about one category after 30 minutes

#### Scenario: Ollama down during the fallback
- **WHEN** the call about 9 categories is rejected and Ollama can't be reached for the first of their own calls
- **THEN** the categorization stops as when Ollama is unavailable, and the 9 stay pending
