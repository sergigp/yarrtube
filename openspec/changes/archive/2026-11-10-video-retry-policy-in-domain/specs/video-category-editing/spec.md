## MODIFIED Requirements

### Requirement: Re-run A Video's AI Categorization
`DELETE /api/videos/{id}/ai-verdicts` SHALL delete every AI verdict of the video (`yes`, `no` and `failed`), keep its user verdicts, recompute its categories, schedule its video sync, and respond with HTTP status 204. An unknown video SHALL get HTTP status 404.

#### Scenario: Wrong AI category
- **WHEN** a video of a channel with categorization on belongs to "Politics" through an AI `yes` and a client re-runs its AI categorization
- **THEN** the video no longer belongs to "Politics", has no AI verdict, and its sync is scheduled so the AI asks again
