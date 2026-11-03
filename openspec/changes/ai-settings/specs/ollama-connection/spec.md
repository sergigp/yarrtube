# Spec Delta

## Purpose

Lets the daemon check that an Ollama server answers at a given URL and list the models installed on it, so the user can configure AI features against a real server.

## ADDED Requirements

### Requirement: Check An Ollama Server
The daemon SHALL provide `GET /api/ollama?url=<url>`, which contacts the Ollama server at `url` (saved or not) and responds with HTTP status 200, the server's `version` and its installed `models`, each with its `name`, `size_bytes` and `thinking` (whether Ollama lists `thinking` among the model's capabilities), sorted by name. The check SHALL change no preference.

#### Scenario: Server reachable
- **WHEN** a client requests `GET /api/ollama?url=http://mac.local:11434` and an Ollama 0.40.13 server there has `qwen3:4b` (capable of thinking) and `gemma3:4b` (not) installed
- **THEN** the daemon responds with HTTP status 200, `version` `"0.40.13"` and models `gemma3:4b` with `thinking` `false` then `qwen3:4b` with `thinking` `true`, each with its size in bytes

#### Scenario: Server with no models
- **WHEN** the Ollama server at the URL has no models installed
- **THEN** the daemon responds with HTTP status 200, its version and an empty `models` list

#### Scenario: Trailing slash
- **WHEN** a client requests the check with `url` `http://mac.local:11434/`
- **THEN** the daemon contacts the same server as for `http://mac.local:11434`

### Requirement: Ollama Check Errors
A missing `url` SHALL be rejected with HTTP status 400 and the message `Ollama URL must be stated (missing)`, and a `url` that is not an absolute `http` or `https` URL with HTTP status 400 and the message `Ollama URL must be an http(s) URL (got "<value>")`.

#### Scenario: Missing URL
- **WHEN** a client requests `GET /api/ollama` without `url`
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
