## REMOVED Requirements

### Requirement: Check An Ollama Server
**Reason**: Moved to `llm-connection`, which covers every LLM provider; the endpoint moves from `GET /api/ollama` to `GET /api/llm/ollama`.
**Migration**: Request `GET /api/llm/ollama?url=<url>`; the response is unchanged. The web UI is the only client.

### Requirement: Ollama Check Errors
**Reason**: Moved to `llm-connection` with the same messages, under `GET /api/llm/ollama`.
**Migration**: None beyond the new path.

### Requirement: Unreachable Ollama Server
**Reason**: Moved to `llm-connection` with the same status and message, under `GET /api/llm/ollama`.
**Migration**: None beyond the new path.
