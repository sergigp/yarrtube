## MODIFIED Requirements

### Requirement: Summary Settings Sent To Ollama
With the Ollama provider, every request SHALL use the saved `summaries_model`, `summaries_context_length` and `summaries_temperature`, and SHALL let the answer use all the context the prompt leaves, with no lower output cap. Thinking SHALL be requested only when `summaries_thinking` is on and Ollama lists `thinking` among the model's capabilities. Every request SHALL tell Ollama neither to truncate the prompt nor to shift the context, because by default it silently cuts an oversized prompt and reports only what it kept.

#### Scenario: Never truncated
- **WHEN** any request is sent
- **THEN** it asks Ollama not to truncate and not to shift

#### Scenario: All the free context available
- **WHEN** a request is sent with a context length of 8192
- **THEN** its output limit is 8192, leaving Ollama to stop where the context ends

#### Scenario: Model able to think
- **WHEN** thinking is on and the saved model supports it
- **THEN** requests ask for thinking

#### Scenario: Model unable to think
- **WHEN** thinking is on and Ollama doesn't list `thinking` for the saved model
- **THEN** requests don't ask for thinking and the summary is generated

### Requirement: Ollama Unavailable
When the AI provider is unavailable the generation SHALL stop with a logged warning, store nothing, leave the summary's status and failed attempts unchanged and re-queue it 6 hours later (see "Waiting For Ollama" in `video-retries`), without failing the task. With Ollama, that is when it can't be connected to before or during generation, or the saved model isn't installed on it; with Anthropic, when no key is set, the saved Anthropic model isn't listed before generation, or a call finds Anthropic unavailable (see `llm-connection`).

#### Scenario: Ollama down
- **WHEN** a summary task runs while nothing answers at the Ollama URL
- **THEN** Ollama is not asked to summarize, the summary keeps its status and failed attempts, and it is re-queued 6 hours later

#### Scenario: Model not installed
- **WHEN** the saved model isn't among those Ollama lists
- **THEN** Ollama is not asked to summarize, the summary keeps its status and failed attempts, and it is re-queued 6 hours later

#### Scenario: Ollama goes down between parts
- **WHEN** a transcript in 3 parts is summarized and Ollama can't be connected to for the second part's call
- **THEN** no summary is stored, the summary keeps its status and failed attempts, and it is re-queued 6 hours later

#### Scenario: Anthropic model retired
- **WHEN** the provider is Anthropic and the saved Anthropic summaries model isn't among those Anthropic lists
- **THEN** Anthropic is not asked to summarize, the summary keeps its status and failed attempts, and it is re-queued 6 hours later

#### Scenario: Out of credit
- **WHEN** the provider is Anthropic and the call answers that the credit balance is too low
- **THEN** no summary is stored, the summary keeps its status and failed attempts, and it is re-queued 6 hours later

#### Scenario: No key
- **WHEN** the saved provider is Anthropic and the daemon has no key
- **THEN** nothing is called, the summary keeps its status and failed attempts, and it is re-queued 6 hours later

### Requirement: Failed Summaries Reset
Saving a change to the AI provider, the Ollama URL or any summary setting (the Ollama model, context length, temperature and thinking, the Anthropic summaries model and effort, and the prompt) SHALL put every video whose summary is Failed back to Pending with no failed attempts, because the new settings may succeed. Saving the same values SHALL change nothing.

#### Scenario: Context length raised
- **WHEN** a video's summary is Failed and the context length is changed
- **THEN** its summary is Pending and a later sync schedules its summary

#### Scenario: Unrelated change
- **WHEN** a video's summary is Failed and a preference unrelated to summaries is changed
- **THEN** its summary stays Failed

#### Scenario: Provider switched
- **WHEN** a video's summary is Failed and the provider is changed from Ollama to Anthropic
- **THEN** its summary is Pending with no failed attempts

#### Scenario: Anthropic effort changed
- **WHEN** a video's summary is Failed and `anthropic_summaries_effort` is changed
- **THEN** its summary is Pending with no failed attempts

## ADDED Requirements

### Requirement: Summary Settings Sent To Anthropic
With the Anthropic provider, every request SHALL use the saved `anthropic_summaries_model` and `anthropic_summaries_effort` and an output cap of 16000 tokens (see "Anthropic Request Shape" in `llm-connection`). The context length used for the input budget SHALL be the model's context window as Anthropic lists it, at most 131072. The Ollama summary settings SHALL NOT be used.

#### Scenario: Haiku request
- **WHEN** the provider is Anthropic, the saved model is `claude-haiku-5-5` with effort `medium`, and Anthropic lists it with a 1000000-token context window
- **THEN** each request asks `claude-haiku-5-5` with effort `medium`, at most 16000 output tokens and no temperature, and the input budget is computed from a context length of 131072

#### Scenario: Long transcript on Anthropic
- **WHEN** the provider is Anthropic and a Spanish transcript has 400000 characters
- **THEN** the transcript is summarized in parts, each within the budget of the 131072 context length

#### Scenario: Summary stored with the model id
- **WHEN** Anthropic answers with a summary
- **THEN** it is stored with the model name `claude-haiku-5-5`
