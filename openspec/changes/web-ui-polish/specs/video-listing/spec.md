## ADDED Requirements

### Requirement: Home Videos Include Their Publish Date
Every video the home endpoint returns, in every section, SHALL include its YouTube publish date as `published_at`, an RFC 3339 timestamp, or `null` when the publish date is unknown.

#### Scenario: Video with a publish date
- **WHEN** a video under "Latest videos" was published on 2026-10-28
- **THEN** its entry has a `published_at` on 2026-10-28

#### Scenario: Unknown publish date
- **WHEN** a home video's publish date is unknown
- **THEN** its entry has `published_at` set to `null`
