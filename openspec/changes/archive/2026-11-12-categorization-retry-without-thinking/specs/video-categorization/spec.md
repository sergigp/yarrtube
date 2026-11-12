# Spec Delta

## ADDED Requirements

### Requirement: Retry Without Thinking
When a call asked with thinking stops at the output limit, its categories SHALL be asked once more, in the same run, in one call without thinking, before any fallback. When that call is about several categories, each it leaves rejected SHALL be asked alone, without thinking. A category rejected by a call without thinking made this way SHALL get a `failed` verdict in that run, instead of waiting for a later attempt.

#### Scenario: Single category answered without thinking
- **WHEN** the call about "Tech" alone, with thinking, stops at the output limit, and the call about "Tech" without thinking answers `no`
- **THEN** the model was asked twice, the second time without thinking, "Tech" has a `no` verdict and the categorization is Done

#### Scenario: Batch answered without thinking
- **WHEN** the call about "AI" and "Tech", with thinking, stops at the output limit, and the call about both without thinking answers both
- **THEN** the model was asked twice, both about "AI" and "Tech", the second time without thinking, and both verdicts are stored

#### Scenario: Batch partly answered without thinking
- **WHEN** the call about "Chess", "Sicilian" and "Tech", with thinking, stops at the output limit, and the call about all three without thinking answers usably for "Chess" and "Sicilian" only
- **THEN** "Tech" is asked alone without thinking, and is never asked alone with thinking

#### Scenario: Fallback call hits the limit
- **WHEN** the call about "Sicilian" and "Tech" with thinking answers usably for "Sicilian" only, and the call about "Tech" alone with thinking stops at the output limit
- **THEN** "Tech" is asked alone once more without thinking

#### Scenario: Rejected again without thinking
- **WHEN** the call about "Tech" alone with thinking stops at the output limit, the call without thinking stops at the output limit too, and the categorization has no failed attempt
- **THEN** "Tech" has a `failed` verdict, the categorization is Failed and nothing is scheduled to ask about it again

#### Scenario: Given up beside a retryable rejection
- **WHEN** in one run "Tech" is rejected by its call without thinking after a limit stop, and "Food" is rejected by its own call with thinking for an unusable answer
- **THEN** "Tech" has a `failed` verdict, "Food" stays pending, and the categorization is Retrying with one more failed attempt

#### Scenario: Settings changed during the call without thinking
- **WHEN** the categorization model is changed while the call about "Tech" without thinking runs, and it is rejected
- **THEN** "Tech" has no verdict and stays pending

#### Scenario: Limit without thinking
- **WHEN** thinking is off in the categorization settings, or the model can't think, and the call about "Tech" alone stops at the output limit
- **THEN** the model is asked once, "Tech" stays pending and the categorization is Retrying with one more failed attempt

### Requirement: Output Limit Logged
A call that stops at the output limit SHALL be logged at warning level with the video, the categories asked, whether it thought, the length of its thinking and of its answer in characters, and the last 300 characters of its thinking. A category given a `failed` verdict after a call without thinking SHALL be logged at error level with the video, the category and why.

#### Scenario: Thinking ran out
- **WHEN** the call about "Tech" with thinking stops at the output limit after 9,000 characters of thinking and no answer
- **THEN** a warning names the video, "Tech", thinking, 9000 thinking characters, 0 answer characters and the thinking's last 300 characters

#### Scenario: Given up logged
- **WHEN** "Tech" gets a `failed` verdict after its call without thinking stops at the output limit
- **THEN** an error line names the video, "Tech" and that it hit the output limit with and without thinking

## MODIFIED Requirements

### Requirement: Rejected Category Calls
A call SHALL be rejected when Ollama stopped at the output limit, when Ollama refuses the prompt, or when it runs past its timeout: its output cap at 5 tokens a second, at least 30 minutes; a category SHALL be rejected when its call is rejected or the answer isn't usable for it. A call with thinking that stops at the output limit SHALL be handled as in "Retry Without Thinking". When any other call about several categories leaves any rejected, the categorization SHALL ask about each of them again in its own call, in the same run. A category rejected by its own call SHALL make the categorization Retrying with one more failed attempt once every pending category has been tried, and the categorization SHALL be re-queued under the `video-retries` policy without failing the task; categories already answered keep their verdicts. After the 11th consecutive failed attempt the categorization SHALL be Failed and every category still pending SHALL get a `failed` verdict. A run that leaves nothing rejected SHALL make it Done and clear the failed attempts, unless the video holds a `failed` verdict (see "Categorization Status").

#### Scenario: Batch hits the output limit
- **WHEN** the call about 9 categories for a video, without thinking, stops at the output limit
- **THEN** the model is asked about each of the 9 in its own call, in the same run, and each usable answer is stored

#### Scenario: Batch partly usable
- **WHEN** the call about "Sicilian" and "Tech" answers usably for "Sicilian" only
- **THEN** "Sicilian"'s verdict is stored and "Tech" is asked about in its own call

#### Scenario: Output limit in a category's own call
- **WHEN** the model, without thinking, stops at the output limit in the call about "Food for thought" alone, on the categorization's first attempt
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

### Requirement: Categorization Status
Every video SHALL record a categorization status from the `video-retries` vocabulary: Pending (a run is wanted and none succeeded since the categories last changed), Retrying, Done (the last run left nothing rejected and the video holds no `failed` verdict) or Failed, with its consecutive failed attempts. A run that leaves nothing to retry while the video holds a `failed` verdict SHALL make it Failed. Adding or re-running a category for the video, giving a category its first AI prompt, deleting its AI verdicts, or a `failed` verdict being reset SHALL make it Pending again, whether or not categorization is on for its playlist or channel. Making it Pending SHALL delete its remaining `failed` verdicts, so the next run asks those categories again.

#### Scenario: Run leaves nothing rejected
- **WHEN** a categorization stores a verdict for every pending category
- **THEN** the video's categorization is Done with no failed attempts

#### Scenario: Run after a category was given up
- **WHEN** a video's categorization is Retrying with a `failed` verdict for "Tech", and its next run answers "Food", the only category pending
- **THEN** its categorization is Failed

#### Scenario: Nothing left but a given-up category
- **WHEN** a video's categorization is Retrying with a `failed` verdict for "Tech", the user sets "Food", the only other category, by hand, and its queued categorization runs
- **THEN** the model is not asked and its categorization is Failed

#### Scenario: New category after a run
- **WHEN** a video's categorization is Done and a category with an AI prompt is added
- **THEN** its categorization is Pending

#### Scenario: Failed categories asked again
- **WHEN** a video's categorization is Failed with a `failed` verdict for "Chess" and a category "Food" with an AI prompt is added
- **THEN** its categorization is Pending, it has no verdict for "Chess", and its next run asks about "Chess" and "Food"

#### Scenario: Category given a prompt
- **WHEN** a video's categorization is Failed and a category without an AI prompt is given one
- **THEN** its categorization is Pending

### Requirement: Categorization Logged
Each call SHALL be logged at info level with the video, the categories asked, whether it is a fallback call, whether it thought, whether it is the call without thinking after a limit stop, the input and output tokens and the duration. Each stored verdict SHALL be logged at info level with the video, the category, the verdict and the reason. A rejected call about several categories SHALL be logged at warning level with the reason and that it falls back. Skips SHALL be logged with their reason at info level, and rejected categories of a call about one category at error level with the reason.

#### Scenario: Verdict logged
- **WHEN** the model answers for a video about "Sicilian" and "Tech"
- **THEN** an info line names the video, both categories, the tokens and the duration, and an info line per category names the video, the category, the verdict and the reason

#### Scenario: Call without thinking logged
- **WHEN** the call about "Tech" with thinking stops at the output limit and "Tech" is asked again without thinking
- **THEN** the first call's info line says it thought and is not a retry without thinking, and the second's says it didn't think and is one

#### Scenario: Fallback logged
- **WHEN** the call about 9 categories for a video, without thinking, stops at the output limit
- **THEN** a warning names the video, the output limit and that each category is asked on its own
