# Spec Delta

## Purpose

Lets the user correct the categories of one video by hand, and ask the AI again for a video, a source or a category, so a wrong or outdated AI answer is never permanent.

## ADDED Requirements

### Requirement: Video Copies Listed With Their Record Id
Every video in the channel, playlist, category, home and search listings SHALL include `record_id`, the id of that stored copy, besides its YouTube `id`. Two copies of one YouTube video (e.g. in a channel and a playlist) SHALL have different `record_id`s. Every `/api/videos/{id}/categories…` and `/api/videos/{id}/ai-verdicts` route SHALL take the copy's `record_id` as `{id}`.

#### Scenario: Same video in a channel and a playlist
- **WHEN** a YouTube video is stored as a copy of "@GothamChess" and a copy of a playlist
- **THEN** the channel's and the playlist's listings show the same `id` and different `record_id`s

### Requirement: Read A Video's Categories
The daemon SHALL provide `GET /api/videos/{id}/categories` for a stored video copy, responding with HTTP status 200, `categorization_enabled` (whether the video's channel or playlist has categorization on) and every category in name order, each with `id`, `name`, `color`, `belongs` (whether the video belongs to it), `from` (`source`, `ai`, `user`, or `null` when it has no verdict and the source isn't a member) and `verdict` (`yes`, `no`, `unavailable` or `null`). `from` SHALL be `source` whenever the source is a member, whatever the verdict. An unknown video SHALL get HTTP status 404.

#### Scenario: Categories from every origin
- **WHEN** a video of "@GothamChess", a member of "Chess", has an AI `yes` for "Sicilian", an AI `no` for "Tech", a user `yes` for "Funny", and no verdict for "Politics"
- **THEN** the response lists "Chess" (`belongs` true, `from` `source`), "Funny" (true, `user`, `yes`), "Politics" (false, `null`, `null`), "Sicilian" (true, `ai`, `yes`) and "Tech" (false, `ai`, `no`)

#### Scenario: Unknown video
- **WHEN** a client asks for the categories of a video id that isn't stored
- **THEN** the response has HTTP status 404

### Requirement: Set A Video's Category By Hand
The daemon SHALL provide `PUT /api/videos/{id}/categories/{category_id}` with a body `{"belongs": <boolean>}`. It SHALL store a user verdict for that pair (`yes` or `no`), replacing any verdict, recompute the video's categories, and respond with HTTP status 200 and the body of `GET /api/videos/{id}/categories`.

#### Scenario: Removing an AI category
- **WHEN** a video belongs to "Food for thought" through an AI `yes` and a client sets `belongs` false for it
- **THEN** the video has a user `no` for "Food for thought" and no longer belongs to it

#### Scenario: Adding a category the AI said no to
- **WHEN** a video has an AI `no` for "Sicilian" and a client sets `belongs` true for it
- **THEN** the video has a user `yes` for "Sicilian" and belongs to it

### Requirement: Rejected Category Edits
Setting a category by hand SHALL change nothing when the video's playlist or channel is a member of the category, responding with HTTP status 409 and `This category comes from the video's channel` (or `playlist`). An unknown video or category SHALL get HTTP status 404, and a missing or non-boolean `belongs` HTTP status 400 with `belongs must be true or false`.

#### Scenario: Category from the source
- **WHEN** a video of "@GothamChess", a member of "Chess", gets a request setting `belongs` false for "Chess"
- **THEN** nothing changes and the response is a 409 with `This category comes from the video's channel`

#### Scenario: Missing belongs
- **WHEN** a client sets a video's category with an empty body
- **THEN** nothing changes and the response is a 400 with `belongs must be true or false`

### Requirement: User Verdicts Are Never Overridden By The AI
A pair with a user verdict SHALL NOT be evaluated by the AI, and SHALL keep its verdict through every re-run and clear. An AI answer SHALL NOT be stored for a pair the user decided while the model was being asked.

#### Scenario: User verdict before the AI got to it
- **WHEN** a video has a user `yes` for "Tech" and no other verdict, and its categorization runs
- **THEN** the model is not asked about "Tech"

#### Scenario: User verdict set while the AI is asking
- **WHEN** the model is being asked about "Tech" for a video and the user sets "Tech" for it meanwhile
- **THEN** the model's answer is not stored and the user's verdict stays

#### Scenario: Re-run keeps user verdicts
- **WHEN** a video has a user `no` for "Food for thought" and its AI verdicts are re-run
- **THEN** the user `no` stays and the video still doesn't belong to "Food for thought"

### Requirement: Re-run A Video's AI Categorization
`DELETE /api/videos/{id}/ai-verdicts` SHALL delete every AI verdict of the video (`yes`, `no` and `unavailable`), keep its user verdicts, recompute its categories, schedule its video sync, and respond with HTTP status 204. An unknown video SHALL get HTTP status 404.

#### Scenario: Wrong AI category
- **WHEN** a video of a channel with categorization on belongs to "Politics" through an AI `yes` and a client re-runs its AI categorization
- **THEN** the video no longer belongs to "Politics", has no AI verdict, and its sync is scheduled so the AI asks again

### Requirement: Re-run Or Clear A Source's AI Categorization
`DELETE /api/channels/{handle}/ai-verdicts` and `DELETE /api/playlists/{id}/ai-verdicts` SHALL do the same for every stored video of the source, schedule the source's reconcile pass, and respond with HTTP status 204. When the source has categorization off nothing is evaluated again, so this clears its AI categories. An unknown source SHALL get HTTP status 404.

#### Scenario: Clearing a source with categorization off
- **WHEN** "@GothamChess" has categorization off and 12 videos with AI verdicts, and a client deletes its AI verdicts
- **THEN** none of its videos has an AI verdict or an AI category, and none is evaluated again

#### Scenario: Re-running a source with categorization on
- **WHEN** "@GothamChess" has categorization on and a client deletes its AI verdicts
- **THEN** its reconcile pass is scheduled and its summarized videos get a categorization scheduled

### Requirement: Re-evaluate A Category
`DELETE /api/categories/{id}/ai-verdicts` SHALL delete every AI verdict for the category, keep its user verdicts, recompute the categories of every video that had one, schedule each such video's sync, and respond with HTTP status 204. An unknown category SHALL get HTTP status 404.

#### Scenario: Prompt edited then re-evaluated
- **WHEN** "Food for thought"'s prompt is edited, 40 videos have AI verdicts for it, and a client re-evaluates it
- **THEN** those 40 verdicts are deleted, the videos' AI membership of it goes, and their syncs are scheduled

#### Scenario: Prompt changed while the AI is asking
- **WHEN** the model is being asked about "Food for thought" for a video and its prompt or the categorization settings change before it answers
- **THEN** the answer is not stored and the pair stays pending, to be asked with the new prompt

### Requirement: Verdict Changes Announced
Storing a user verdict and deleting AI verdicts SHALL publish one video-categories-changed event per request naming every video whose categories changed, and nothing when none did.

#### Scenario: Clearing changes three videos
- **WHEN** a source's AI verdicts are deleted and 3 of its videos had an AI `yes`
- **THEN** one video-categories-changed event naming those 3 videos is published
