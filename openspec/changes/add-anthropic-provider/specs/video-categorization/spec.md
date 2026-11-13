## MODIFIED Requirements

### Requirement: Categorization Request Settings
With the Ollama provider, each request SHALL use the saved categorization model, temperature 0, the categorization context length (see `preferences`), the answer's JSON schema with every category asked about required, and an output cap of 1,365 tokens per category asked, at least 3,072 and at most the context length. Thinking SHALL be requested when Ollama lists it among the model's capabilities. Every request SHALL tell Ollama neither to truncate the prompt nor to shift the context.

#### Scenario: Thinking model
- **WHEN** the categorization model is `qwen3:4b`, Ollama lists `thinking` among its capabilities, the context length is Auto with 9 categories with an AI prompt, and 9 are asked about
- **THEN** the request asks for thinking, temperature 0, context 16384, at most 12285 output tokens and the JSON schema with the 9 categories

#### Scenario: One category
- **WHEN** one category is asked about
- **THEN** the request allows at most 3072 output tokens

#### Scenario: Cap above the context length
- **WHEN** the context length is overridden to `8192` and 9 categories are asked about
- **THEN** the request allows at most 8192 output tokens

### Requirement: Ollama Unavailable For Categorization
Before asking, the categorization SHALL check the provider and its models. When the provider is unavailable, on that check or on any call of the run (the fallback included), the categorization SHALL stop with a logged warning, storing nothing more, leave its status and failed attempts unchanged, write no `failed` verdict, and re-queue it 6 hours later (see "Waiting For Ollama" in `video-retries`), without failing the task. Verdicts stored earlier in the run SHALL be kept. With Ollama, unavailable is when it can't be connected to or the saved categorization model isn't installed on it; with Anthropic, when no key is set, the saved Anthropic categorization model isn't listed, or a call finds Anthropic unavailable (see `llm-connection`).

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

#### Scenario: Anthropic key revoked
- **WHEN** the provider is Anthropic and the categorization call is answered 401
- **THEN** no verdict is stored, the categorization keeps its status and failed attempts, and it is re-queued 6 hours later

#### Scenario: Anthropic model retired
- **WHEN** the provider is Anthropic and the saved Anthropic categorization model isn't among those Anthropic lists
- **THEN** Anthropic is not asked, the categorization keeps its status and failed attempts, and it is re-queued 6 hours later

### Requirement: Failed Verdicts Reset
Saving a change to the AI provider, the Ollama URL or to any categorization setting that changes how the model is called (the Ollama model, context length, temperature and thinking, the Anthropic categorization model and effort, and the prompt) SHALL delete every `failed` verdict; turning categorization on or off is not such a change. Saving a different AI prompt for a category SHALL delete that category's `failed` verdicts. The categorization of every video that had a deleted `failed` verdict SHALL be Pending; other videos' categorization SHALL be unchanged. Saving the same values SHALL change nothing. A categorization that started before such a change SHALL NOT store `failed` verdicts after it; when that leaves a category pending, its categorization SHALL be Pending with no failed attempts instead of Failed.

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

#### Scenario: Provider switched
- **WHEN** a video has a `failed` verdict for "Tech" and the provider is changed from Ollama to Anthropic
- **THEN** "Tech" is pending for the video again and its categorization is Pending

#### Scenario: Anthropic effort changed
- **WHEN** a video has a `failed` verdict for "Tech" and `anthropic_categorization_effort` is changed
- **THEN** "Tech" is pending for the video again

## ADDED Requirements

### Requirement: Categorization Request Settings For Anthropic
With the Anthropic provider, each request SHALL use the saved `anthropic_categorization_model` and `anthropic_categorization_effort`, the answer's JSON schema with every category asked about required, and an output cap of 1,365 tokens per category asked, at least 3,072 (see "Anthropic Request Shape" in `llm-connection`). The Ollama categorization settings, Auto context length included, SHALL NOT be used.

#### Scenario: Nine categories
- **WHEN** the provider is Anthropic with `claude-haiku-5-5` and effort `medium`, and 9 categories are asked about
- **THEN** one request asks `claude-haiku-5-5` with effort `medium`, at most 12285 output tokens and the JSON schema with the 9 categories, and no temperature

### Requirement: No Retry Without Thinking On Anthropic
With the Anthropic provider, no call SHALL be asked again without thinking: a call that stops at the output limit SHALL be handled as a call without thinking in "Rejected Category Calls", so "Retry Without Thinking" never applies.

#### Scenario: Batch hits the output limit
- **WHEN** the provider is Anthropic and the call about "AI" and "Tech" stops at the output limit
- **THEN** "AI" and "Tech" are each asked in their own call, in the same run, and no call is made without thinking

#### Scenario: Own call hits the output limit
- **WHEN** the provider is Anthropic and the call about "Tech" alone stops at the output limit on the categorization's first attempt
- **THEN** "Tech" stays pending with no `failed` verdict, and the categorization is Retrying with one failed attempt
