## Purpose
Lets the daemon check the LLM provider AI features run on (an Ollama server at a URL, or Anthropic's API with the key from the environment), list its models, and decide how each provider's replies and failures count for the AI steps.

## ADDED Requirements

### Requirement: Check An Ollama Server
The daemon SHALL provide `GET /api/llm/ollama?url=<url>`, which contacts the Ollama server at `url` (saved or not) and responds with HTTP status 200, the server's `version` and its installed `models`, each with its `name`, `size_bytes` and `thinking` (whether Ollama lists `thinking` among the model's capabilities), sorted by name. The check SHALL change no preference.

#### Scenario: Server reachable
- **WHEN** a client requests `GET /api/llm/ollama?url=http://mac.local:11434` and an Ollama 0.40.13 server there has `qwen3:4b` (capable of thinking) and `gemma3:4b` (not) installed
- **THEN** the daemon responds with HTTP status 200, `version` `"0.40.13"` and models `gemma3:4b` with `thinking` `false` then `qwen3:4b` with `thinking` `true`, each with its size in bytes

#### Scenario: Server with no models
- **WHEN** the Ollama server at the URL has no models installed
- **THEN** the daemon responds with HTTP status 200, its version and an empty `models` list

#### Scenario: Trailing slash
- **WHEN** a client requests the check with `url` `http://mac.local:11434/`
- **THEN** the daemon contacts the same server as for `http://mac.local:11434`

#### Scenario: Old path gone
- **WHEN** a client requests `GET /api/ollama?url=http://mac.local:11434`
- **THEN** the daemon doesn't serve it as an API endpoint

### Requirement: Ollama Check Errors
A missing `url` SHALL be rejected with HTTP status 400 and the message `Ollama URL must be stated (missing)`, and a `url` that is not an absolute `http` or `https` URL with HTTP status 400 and the message `Ollama URL must be an http(s) URL (got "<value>")`.

#### Scenario: Missing URL
- **WHEN** a client requests `GET /api/llm/ollama` without `url`
- **THEN** the daemon responds with HTTP status 400 and the message `Ollama URL must be stated (missing)`

#### Scenario: Not an http URL
- **WHEN** a client requests the check with `url` `mac.local:11434`
- **THEN** the daemon responds with HTTP status 400 and the message `Ollama URL must be an http(s) URL (got "mac.local:11434")`

### Requirement: Unreachable Ollama Server
When the server does not answer any request of the check within 5 seconds, refuses the connection, or answers with something that is not an Ollama response, the daemon SHALL respond with HTTP status 502 and the message `Could not reach Ollama at <url>: <reason>`.

#### Scenario: Nothing listening
- **WHEN** a client requests the check for a URL where nothing listens
- **THEN** the daemon responds with HTTP status 502 and a message starting `Could not reach Ollama at <url>: `

#### Scenario: Not an Ollama server
- **WHEN** a client requests the check for a URL whose server answers `/api/version` with an HTML page
- **THEN** the daemon responds with HTTP status 502 and a message starting `Could not reach Ollama at <url>: `

### Requirement: Anthropic API Key From The Environment
The daemon SHALL read the Anthropic API key from `YARRTUBE_ANTHROPIC_API_KEY` at startup; unset or empty means no key. The key SHALL NOT be stored, accepted over HTTP, returned by any endpoint or written to the logs.

#### Scenario: Key set
- **WHEN** the daemon starts with `YARRTUBE_ANTHROPIC_API_KEY` set to a key
- **THEN** Anthropic is available and no response or log line contains the key

#### Scenario: Empty variable
- **WHEN** the daemon starts with `YARRTUBE_ANTHROPIC_API_KEY` set to an empty string
- **THEN** Anthropic is not available

### Requirement: Check Anthropic
The daemon SHALL provide `GET /api/llm/anthropic`, which lists the models the key can use and responds with HTTP status 200 and `models`, each with its `name` (the model id), `display_name` and `context_window` (its maximum input tokens), in the order Anthropic lists them, newest first. The check SHALL change no preference.

#### Scenario: Models listed
- **WHEN** a key is set and Anthropic lists `claude-sonnet-5-5` then `claude-haiku-5-5`
- **THEN** the daemon responds with HTTP status 200 and models `claude-sonnet-5-5` then `claude-haiku-5-5`, each with its display name and context window

#### Scenario: More models than one page
- **WHEN** Anthropic lists the key's models over two pages
- **THEN** the response holds the models of both pages

### Requirement: Anthropic Check Errors
Without a key, the check SHALL respond with HTTP status 404 and the message `Anthropic isn't configured: set YARRTUBE_ANTHROPIC_API_KEY`. When Anthropic can't be connected to within 5 seconds, rejects the key, or answers with an error or something that isn't its API's response, the check SHALL respond with HTTP status 502 and the message `Could not reach Anthropic: <reason>`.

#### Scenario: No key
- **WHEN** no key is set and a client requests `GET /api/llm/anthropic`
- **THEN** the daemon responds with HTTP status 404 and the message `Anthropic isn't configured: set YARRTUBE_ANTHROPIC_API_KEY`

#### Scenario: Key rejected
- **WHEN** Anthropic answers 401 to the model list
- **THEN** the daemon responds with HTTP status 502 and a message starting `Could not reach Anthropic: ` that says the key was rejected, without the key

#### Scenario: Can't connect
- **WHEN** nothing answers at Anthropic's API address
- **THEN** the daemon responds with HTTP status 502 and a message starting `Could not reach Anthropic: `

### Requirement: Anthropic Call Outcomes
An Anthropic call SHALL count as: an answer, its text blocks joined, with `usage` tokens, when it ends with `end_turn`; stopped at the limit on `stop_reason` `max_tokens`; refused on `stop_reason` `refusal` or any other 400, and refused as too long when the 400 says the prompt is too long; Anthropic unavailable on a connection failure, 401, 403, 404, or a 400 saying the credit balance is too low; and a server fault on 429, 5xx or 529.

#### Scenario: Answer
- **WHEN** Anthropic answers with `stop_reason` `end_turn`, a text block `Three games.` and 900 input and 120 output tokens
- **THEN** the call's answer is `Three games.` with 900 input and 120 output tokens

#### Scenario: Stopped at the limit
- **WHEN** Anthropic answers with `stop_reason` `max_tokens`
- **THEN** the call stopped at the limit

#### Scenario: Declined
- **WHEN** Anthropic answers with `stop_reason` `refusal`
- **THEN** the call is refused

#### Scenario: Prompt too long
- **WHEN** Anthropic answers 400 with the message `prompt is too long: 140000 tokens > 131072 maximum`
- **THEN** the call is refused as too long

#### Scenario: Out of credit
- **WHEN** Anthropic answers 400 saying the credit balance is too low
- **THEN** Anthropic is unavailable

#### Scenario: Key revoked
- **WHEN** Anthropic answers 401
- **THEN** Anthropic is unavailable

#### Scenario: Overloaded
- **WHEN** Anthropic answers 529
- **THEN** the call is a server fault

### Requirement: Anthropic Request Shape
Every Anthropic call SHALL send the model, `max_tokens`, the system message, one user message and the effort, and the answer's JSON schema as structured output when the call has one, with `additionalProperties: false` on every object (the API rejects a schema without it). It SHALL NOT send a temperature, a context length or a thinking setting, so the model's default adaptive thinking applies.

#### Scenario: Categorization call
- **WHEN** a categorization call with a JSON schema is sent to Anthropic with effort `medium`
- **THEN** the request holds the model, `max_tokens`, system and user messages, effort `medium` and the schema as structured output with `additionalProperties: false` on every object, and no temperature or thinking setting
