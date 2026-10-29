# Spec Delta

## Purpose

Records which categories each stored video copy belongs to, so categories can be read, listed and exported per video rather than only per playlist or channel.

## ADDED Requirements

### Requirement: Categories Of A Stored Video
The system SHALL record, for each stored video copy (one per playlist or channel tracking a YouTube video), the set of categories it belongs to. Copies of the same YouTube video tracked by different sources SHALL have independent sets.

#### Scenario: Same YouTube video in two sources
- **WHEN** a YouTube video is tracked by the channel "@GothamChess", a member of "Chess", and by the playlist "PLendgames", a member of no category
- **THEN** the channel's copy belongs to "Chess" and the playlist's copy belongs to no category

### Requirement: Source Categories Take Precedence
When a stored video's playlist or channel is a member of at least one category, the video SHALL belong to exactly the categories its source is a member of, and to no other.

#### Scenario: Source in two categories
- **WHEN** the channel "@GothamChess" is a member of "Chess" and "Funny"
- **THEN** every stored video of that channel belongs to exactly "Chess" and "Funny"

#### Scenario: Source in no category
- **WHEN** the playlist "PLendgames" is a member of no category
- **THEN** its stored videos belong to no category

### Requirement: Membership Changes Apply To Videos
When a category's members are set, every stored video of every source that was or becomes a member SHALL have its categories recomputed by the time the request completes. A source left in no category SHALL leave its videos in no category.

#### Scenario: Source added to a category
- **WHEN** the channel "@GothamChess", in no category, is added to "Chess"
- **THEN** all its stored videos belong to "Chess" once the request completes

#### Scenario: Source removed from one of its categories
- **WHEN** the channel "@GothamChess" is a member of "Chess" and "Funny" and is removed from "Funny"
- **THEN** its stored videos belong to "Chess" only

#### Scenario: Source removed from its last category
- **WHEN** the channel "@GothamChess" is a member of "Chess" only and is removed from it
- **THEN** its stored videos belong to no category

### Requirement: Category Deletion Applies To Videos
When a category is deleted, no stored video SHALL belong to it anymore by the time the request completes. Renaming or recoloring a category SHALL NOT change which videos belong to it.

#### Scenario: Category deleted
- **WHEN** the category "Funny", holding the channel "@GothamChess" also in "Chess", is deleted
- **THEN** the channel's stored videos belong to "Chess" only

#### Scenario: Category renamed
- **WHEN** the category "Funny" is renamed "Humor"
- **THEN** every video that belonged to it belongs to "Humor"

### Requirement: New Videos Get Their Source's Categories
A video newly stored for a playlist or channel that is a member of at least one category SHALL be given its source's categories by its first video sync.

#### Scenario: New video in a categorized channel
- **WHEN** a reconcile pass adds a new video to the channel "@GothamChess", a member of "Chess", and the video's sync runs
- **THEN** the new video belongs to "Chess", whether or not it has been downloaded yet

### Requirement: Reconcile Passes Repair Video Categories
Each playlist or channel reconcile pass SHALL set the categories of every stored video of that source to the source's categories when the source is a member of at least one category, and SHALL leave them unchanged otherwise.

#### Scenario: Drifted video repaired
- **WHEN** a stored video of the channel "@GothamChess", a member of "Chess", belongs to no category, and the channel's reconcile pass runs
- **THEN** the video belongs to "Chess"

#### Scenario: Uncategorized source untouched
- **WHEN** the reconcile pass of a playlist that is a member of no category runs
- **THEN** the categories of its stored videos are unchanged

### Requirement: Deleted Videos Leave Their Categories
When a stored video is deleted, its category memberships SHALL be deleted with it.

#### Scenario: Video removed from its source
- **WHEN** a video of the channel "@GothamChess", a member of "Chess", is removed from the channel and its record deleted
- **THEN** no category lists it anymore

### Requirement: Existing Memberships Backfilled On Upgrade
On upgrade, every video stored before this change SHALL be given the categories its playlist or channel is a member of.

#### Scenario: Upgrade with existing categories
- **WHEN** the system upgrades with the channel "@GothamChess" in "Chess" and 40 stored videos
- **THEN** after the upgrade all 40 stored videos belong to "Chess" and category lists, counts and feeds are unchanged

### Requirement: Video Category Change Notification
Whenever the categories of stored videos change, or a category they belong to is renamed or deleted, the system SHALL publish one domain event per operation naming every affected video. An operation that changes no video SHALL publish nothing.

#### Scenario: Members set
- **WHEN** the channel "@GothamChess" with 3 stored videos is added to "Chess"
- **THEN** one video-categories-changed event naming those 3 videos is published

#### Scenario: Category renamed
- **WHEN** the category "Chess", to which 5 stored videos belong, is renamed
- **THEN** one video-categories-changed event naming those 5 videos is published

#### Scenario: Reconcile pass with nothing to repair
- **WHEN** a reconcile pass runs for a source whose videos already have its categories
- **THEN** no video-categories-changed event is published
