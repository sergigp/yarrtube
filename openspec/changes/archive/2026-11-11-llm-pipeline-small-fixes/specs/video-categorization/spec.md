## MODIFIED Requirements

### Requirement: When A Video Is Categorized
A video's sync SHALL schedule its categorization only when AI, categories and categorization are on in the preferences, the video's playlist or channel has `categorization_enabled` `true`, the video's summary is Done, at least one category is pending for it and its categorization is Pending, Done, or Retrying and due (see `video-retries`). A categorization already pending or running for the video SHALL NOT be scheduled again. Otherwise the sync SHALL schedule nothing for categorization. Storing a video's summary SHALL sync the video right away (see "Summary Stored" in `video-summaries`), so a newly summarized video is categorized without waiting for its source's next reconcile pass.

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
- **WHEN** a new video's summary is stored after its sync ran, and "Sicilian" is pending for it
- **THEN** the video is synced again right away and that sync schedules its categorization

#### Scenario: Categorization failed recently
- **WHEN** a video with pending categories whose categorization is Retrying, having failed once 3 minutes ago, is synced with no categorization pending
- **THEN** no categorization is scheduled

### Requirement: Categorization Run
A categorization SHALL re-check, when it runs, the conditions for scheduling it and do nothing if they no longer hold. When no category is pending for the video at that moment, it SHALL neither check the Ollama server nor call the model, and SHALL mark the video's categorization Done with no failed attempts. Otherwise it SHALL ask the model about every category pending at that moment in one call, and store each verdict of that call's answer as soon as the answer is read. A verdict SHALL NOT be stored for a category deleted, or a video removed, while it was being evaluated or while its verdict was being stored.

#### Scenario: Category deleted before the run
- **WHEN** a categorization is scheduled while "Sicilian" is pending, and "Sicilian" is deleted before it runs
- **THEN** the model is not asked about "Sicilian"

#### Scenario: Nothing pending when it runs
- **WHEN** a categorization is scheduled while only "Sicilian" is pending for a video whose categorization is Retrying, and the user sets a verdict for "Sicilian" before it runs
- **THEN** Ollama is neither checked nor asked anything, and the video's categorization is Done with no failed attempts

#### Scenario: Nothing pending while Ollama is down
- **WHEN** a categorization runs with no category pending for the video and nothing answers at the Ollama URL
- **THEN** the video's categorization is Done and the categorization is not re-queued

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

### Requirement: Failed Verdicts Reset
Saving a change to the Ollama URL or to any categorization setting that changes how the model is called (model, context length, temperature, thinking, prompt) SHALL delete every `failed` verdict; turning categorization on or off is not such a change. Saving a different AI prompt for a category SHALL delete that category's `failed` verdicts. The categorization of every video that had a deleted `failed` verdict SHALL be Pending; other videos' categorization SHALL be unchanged. Saving the same values SHALL change nothing. A categorization that started before such a change SHALL NOT store `failed` verdicts after it; when that leaves a category pending, its categorization SHALL be Pending with no failed attempts instead of Failed.

#### Scenario: New model
- **WHEN** a video has an `failed` verdict for "Tech" and the categorization model is changed
- **THEN** "Tech" is pending for the video again

#### Scenario: Generation setting changed
- **WHEN** a video's categorization is Failed with a `failed` verdict for "Tech" and the categorization context length, temperature or thinking is changed
- **THEN** "Tech" is pending for the video again and its categorization is Pending

#### Scenario: Categorization turned off
- **WHEN** a video has a `failed` verdict for "Tech" and categorization is turned off
- **THEN** its `failed` verdict is kept

#### Scenario: Another category's failure kept
- **WHEN** a video's categorization is Failed with a `failed` verdict for "Chess" and "Tech"'s AI prompt is changed
- **THEN** its categorization is still Failed and its "Chess" verdict is kept

#### Scenario: Model changed during a last attempt
- **WHEN** the categorization model is changed while a video's last attempt is running and "Tech" is rejected
- **THEN** "Tech" is pending for the video again and its categorization is Pending with no failed attempts

#### Scenario: Temperature changed during a last attempt
- **WHEN** the categorization temperature is changed while a video's last attempt is running and "Tech" is rejected
- **THEN** "Tech" is pending for the video again and its categorization is Pending with no failed attempts

#### Scenario: Category prompt changed during a last attempt
- **WHEN** "Tech"'s AI prompt is edited while a video's last attempt is running and "Tech" is rejected
- **THEN** "Tech" is pending for the video again and its categorization is Pending with no failed attempts
