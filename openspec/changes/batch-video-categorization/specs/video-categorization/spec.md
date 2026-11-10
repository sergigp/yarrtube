# Spec Delta

## MODIFIED Requirements

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

### Requirement: Rejected Calls
A call SHALL be rejected when Ollama stopped at the output limit, when Ollama refuses the prompt, or when it runs past its timeout: its output cap at 5 tokens a second, at least 30 minutes; a category SHALL be rejected when its call is rejected or the answer isn't usable for it. When a call about several categories leaves any rejected, the categorization SHALL ask about each of them again in its own call, in the same run. A category rejected by its own call SHALL fail the categorization once every pending category has been tried, so the task queue retries it with its usual backoff; categories already answered keep their verdicts. On the last attempt such a category SHALL get an `unavailable` verdict.

#### Scenario: Batch hits the output limit
- **WHEN** the call about 9 categories for a video stops at the output limit
- **THEN** the model is asked about each of the 9 in its own call, in the same run, and each usable answer is stored

#### Scenario: Batch partly usable
- **WHEN** the call about "Sicilian" and "Tech" answers usably for "Sicilian" only
- **THEN** "Sicilian"'s verdict is stored and "Tech" is asked about in its own call

#### Scenario: Output limit on the last attempt
- **WHEN** the model stops at the output limit in the call about "Food for thought" alone, on the categorization's last attempt
- **THEN** the video has an `unavailable` verdict for "Food for thought" and doesn't belong to it

#### Scenario: Retry resumes
- **WHEN** a categorization answers "Sicilian" then its own call about "Tech" is rejected, and the task is retried
- **THEN** the retry asks only about "Tech"

#### Scenario: Timeout scales with the cap
- **WHEN** a call about 9 categories allows 12285 output tokens
- **THEN** it is rejected only after 2457 seconds without an answer, and a call about one category after 30 minutes

#### Scenario: Ollama down during the fallback
- **WHEN** the call about 9 categories is rejected and Ollama can't be reached for the first of their own calls
- **THEN** the categorization stops as when Ollama is unavailable, and the 9 stay pending

### Requirement: Categorization Logged
Each call SHALL be logged at info level with the video, the categories asked, whether it is a fallback call, the input and output tokens and the duration. Each stored verdict SHALL be logged at info level with the video, the category, the verdict and the reason. A rejected call about several categories SHALL be logged at warning level with the reason and that it falls back. Skips SHALL be logged with their reason at info level, and rejected categories of a call about one category at error level with the reason.

#### Scenario: Verdict logged
- **WHEN** the model answers for a video about "Sicilian" and "Tech"
- **THEN** an info line names the video, both categories, the tokens and the duration, and an info line per category names the video, the category, the verdict and the reason

#### Scenario: Fallback logged
- **WHEN** the call about 9 categories for a video stops at the output limit
- **THEN** a warning names the video, the output limit and that each category is asked on its own
