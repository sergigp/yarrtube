# Spec Delta

## MODIFIED Requirements

### Requirement: Giving Up After A Week
A step SHALL become Failed after its 11th consecutive failed attempt (about 7 days after the first), and SHALL then never be attempted again until the user retries the video. When the categorization becomes Failed, every category still pending for the video SHALL get a `failed` verdict. A categorization MAY give a category a `failed` verdict before then, when a later attempt would only repeat the same calls (see `video-categorization`'s "Retry Without Thinking"); with nothing else left to retry it SHALL be Failed in that run.

#### Scenario: Download gives up
- **WHEN** a video's download fails for the 11th consecutive time
- **THEN** the video's download is Failed, no download of it is pending, and no video sync schedules one

#### Scenario: Summary gives up while Ollama is down
- **WHEN** a video's summary with 10 consecutive failed attempts runs while Ollama is down
- **THEN** it doesn't give up: its summary stays Retrying with 10 failed attempts and is re-queued 6 hours later (see "Waiting For Ollama")

#### Scenario: Summary gives up
- **WHEN** a video's summary call is rejected for the 11th consecutive time
- **THEN** its summary is Failed and no summary of it is scheduled again

#### Scenario: Categorization gives up
- **WHEN** a video's categorization fails for the 11th consecutive time with "Tech" and "Sicilian" pending
- **THEN** its categorization is Failed and the video has `failed` verdicts for "Tech" and "Sicilian"

#### Scenario: Categorization gives up at once
- **WHEN** on its first attempt a video's only pending category, "Tech", stops at the output limit with thinking and is rejected again without thinking
- **THEN** its categorization is Failed with one failed attempt, the video has a `failed` verdict for "Tech", and no categorization of it is scheduled again
