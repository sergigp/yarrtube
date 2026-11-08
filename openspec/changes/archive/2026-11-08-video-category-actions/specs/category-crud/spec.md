# Spec Delta

## ADDED Requirements

### Requirement: AI Verdict Count In Category Responses
Every category in `GET /api/categories` SHALL include `ai_verdict_count`: how many stored videos have an AI verdict (`yes`, `no` or `unavailable`) for it.

#### Scenario: Category evaluated for some videos
- **WHEN** "Sicilian" has AI verdicts for 40 videos and user verdicts for 2 others
- **THEN** its entry has `ai_verdict_count` 40

#### Scenario: Category never evaluated
- **WHEN** "Funny" has no AI prompt and no verdicts
- **THEN** its entry has `ai_verdict_count` 0
