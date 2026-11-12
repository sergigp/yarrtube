# Spec Delta

## ADDED Requirements

### Requirement: Summary Output Limit Logged
A summary call that stops at the output limit SHALL be logged at warning level with the video, the call, whether it thought, the length of its thinking and of its answer in characters, and the last 300 characters of its thinking, before the failure is logged.

#### Scenario: Thinking filled the room left
- **WHEN** a summary call with thinking stops at the output limit after 12,000 characters of thinking and 800 of answer
- **THEN** a warning names the video, the call, thinking, 12000 thinking characters, 800 answer characters and the thinking's last 300 characters
