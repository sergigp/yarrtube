# Spec Delta

## MODIFIED Requirements

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
