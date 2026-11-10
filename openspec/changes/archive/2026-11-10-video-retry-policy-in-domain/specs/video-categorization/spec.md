## RENAMED Requirements

- FROM: `### Requirement: Unavailable Verdicts Reset`
- TO: `### Requirement: Failed Verdicts Reset`

## MODIFIED Requirements

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

### Requirement: Ollama Unavailable For Categorization
When Ollama can't be reached or the saved categorization model isn't installed on it, the categorization SHALL stop with a logged warning, storing nothing more, make the categorization Retrying with one more failed attempt and re-queue it under the `video-retries` policy, without failing the task. The 11th consecutive failed attempt SHALL make it Failed and give every category still pending a `failed` verdict, as a rejected call does.

#### Scenario: Ollama down
- **WHEN** Ollama can't be reached when a categorization runs
- **THEN** no verdict is stored, the categorization is Retrying with one more failed attempt, it is re-queued for its next attempt, and the categories stay pending

#### Scenario: Ollama down on the eleventh attempt
- **WHEN** Ollama can't be reached when a categorization with 10 consecutive failed attempts runs and "Tech" is pending
- **THEN** the categorization is Failed and the video has a `failed` verdict for "Tech"

## ADDED Requirements

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

## REMOVED Requirements

### Requirement: Rejected Calls
**Reason**: Replaced by "Rejected Category Calls": rejections back off and give up after 11 attempts on the video, instead of on the task queue's 5th attempt.
**Migration**: Existing `unavailable` verdicts become `failed`, kept until the categorization settings change or the user retries the video; a video with any becomes Failed (11 failed attempts), so the user can retry it.
