## MODIFIED Requirements

### Requirement: Read A Video's Categories
The daemon SHALL provide `GET /api/videos/{id}/categories` for a stored video copy, responding with HTTP status 200, `categorization_enabled` (whether the video's channel or playlist has both summaries and categorization on) and every category in name order, each with `id`, `name`, `color`, `belongs` (whether the video belongs to it), `from` (`source`, `ai`, `user`, or `null` when it has no verdict and the source isn't a member) and `verdict` (`yes`, `no`, `unavailable` or `null`). `from` SHALL be `source` whenever the source is a member, whatever the verdict. An unknown video SHALL get HTTP status 404.

#### Scenario: Categories from every origin
- **WHEN** a video of "@GothamChess", a member of "Chess", has an AI `yes` for "Sicilian", an AI `no` for "Tech", a user `yes` for "Funny", and no verdict for "Politics"
- **THEN** the response lists "Chess" (`belongs` true, `from` `source`), "Funny" (true, `user`, `yes`), "Politics" (false, `null`, `null`), "Sicilian" (true, `ai`, `yes`) and "Tech" (false, `ai`, `no`)

#### Scenario: Source with summaries off
- **WHEN** a video's channel has `categorization_enabled` `true` and `summaries_enabled` `false`, and a client reads its categories
- **THEN** the response has `categorization_enabled` `false`

#### Scenario: Unknown video
- **WHEN** a client asks for the categories of a video id that isn't stored
- **THEN** the response has HTTP status 404
