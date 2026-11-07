# video-categories Specification

## Purpose

Records which categories each stored video copy belongs to, so categories can be read, listed and exported per video rather than only per playlist or channel.

## Requirements

### Requirement: Categories Of A Stored Video
The system SHALL record, for each stored video copy (one per playlist or channel tracking a YouTube video), the set of categories it belongs to. Copies of the same YouTube video tracked by different sources SHALL have independent sets.

#### Scenario: Same YouTube video in two sources
- **WHEN** a YouTube video is tracked by the channel "@GothamChess", a member of "Chess", and by the playlist "PLendgames", a member of no category
- **THEN** the channel's copy belongs to "Chess" and the playlist's copy belongs to no category

### Requirement: Source Categories And Yes Verdicts
A stored video SHALL belong to every category its playlist or channel is a member of, plus every category for which it has a `yes` verdict (see `video-categorization`), and to no other.

#### Scenario: Source in two categories
- **WHEN** the channel "@GothamChess" is a member of "Chess" and "Funny" and a video of it has no `yes` verdict
- **THEN** the video belongs to exactly "Chess" and "Funny"

#### Scenario: Source category plus an AI category
- **WHEN** the channel "@GothamChess" is a member of "Chess" and a video of it has a `yes` verdict for "Sicilian"
- **THEN** the video belongs to exactly "Chess" and "Sicilian"

#### Scenario: Source in no category
- **WHEN** the playlist "PLendgames" is a member of no category and a video of it has a `yes` verdict for "Chess" and a `no` verdict for "Funny"
- **THEN** the video belongs to exactly "Chess"

### Requirement: Membership Changes Apply To Videos
When a category's members are set, every stored video of every source that was or becomes a member SHALL have its categories recomputed by the time the request completes.

#### Scenario: Source added to a category
- **WHEN** the channel "@GothamChess", in no category, is added to "Chess"
- **THEN** all its stored videos belong to "Chess" once the request completes

#### Scenario: Source removed from one of its categories
- **WHEN** the channel "@GothamChess" is a member of "Chess" and "Funny" and is removed from "Funny"
- **THEN** its stored videos belong to "Chess" and to their `yes` verdicts only

#### Scenario: Source removed from its last category
- **WHEN** the channel "@GothamChess" is a member of "Chess" only and is removed from it
- **THEN** its stored videos belong only to the categories of their `yes` verdicts

#### Scenario: Removed source with a yes verdict for the category
- **WHEN** a video of "@GothamChess" has a `yes` verdict for "Chess" and the channel is removed from "Chess"
- **THEN** the video still belongs to "Chess"

### Requirement: Category Deletion Applies To Videos
When a category is deleted, its verdicts SHALL be deleted and no stored video SHALL belong to it anymore by the time the request completes. Renaming or recoloring a category SHALL NOT change which videos belong to it.

#### Scenario: Category deleted
- **WHEN** the category "Funny", holding the channel "@GothamChess" also in "Chess", is deleted
- **THEN** the channel's stored videos belong to "Chess" and to their `yes` verdicts only

#### Scenario: Category deleted from videos of an uncategorized source
- **WHEN** a video of the channel "@GothamChess", a member of no category, has `yes` verdicts for "Chess" and "Funny", and "Funny" is deleted
- **THEN** the video belongs to "Chess" only and has no verdict for "Funny"

#### Scenario: Category renamed
- **WHEN** the category "Funny" is renamed "Humor"
- **THEN** every video that belonged to it belongs to "Humor"

### Requirement: Concurrent Changes Do Not Revert Categories
A reconcile pass or video sync running while a category's members are set or the category is deleted SHALL NOT write back a category that change removed.

#### Scenario: Pass during a member removal
- **WHEN** the reconcile pass of the channel "@GothamChess" runs while "@GothamChess" is removed from "Chess", its last category
- **THEN** once both complete, the channel's stored videos belong to no category

### Requirement: New Videos Get Their Source's Categories
A video newly stored for a playlist or channel that is a member of at least one category SHALL be given its source's categories by its first video sync.

#### Scenario: New video in a categorized channel
- **WHEN** a reconcile pass adds a new video to the channel "@GothamChess", a member of "Chess", and the video's sync runs
- **THEN** the new video belongs to "Chess", whether or not it has been downloaded yet

### Requirement: Reconcile Passes Repair Video Categories
Each video sync SHALL set the categories of its video to its source's categories plus its `yes` verdicts. Since every playlist or channel reconcile pass runs the video sync of every stored video of that source (see `video-sync`), each pass repairs the categories of all of them.

#### Scenario: Drifted video repaired
- **WHEN** a stored video of the channel "@GothamChess", a member of "Chess", belongs to no category, and the channel's reconcile pass runs
- **THEN** the video belongs to "Chess"

#### Scenario: Uncategorized source untouched
- **WHEN** the reconcile pass of a playlist that is a member of no category runs, and one of its videos belongs to "Chess" through a `yes` verdict
- **THEN** that video still belongs to exactly "Chess"

### Requirement: Deleted Videos Leave Their Categories
When a stored video is deleted, its category memberships and its verdicts SHALL be deleted with it.

#### Scenario: Video removed from its source
- **WHEN** a video of the channel "@GothamChess", a member of "Chess", with a `yes` verdict for "Sicilian", is removed from the channel and its record deleted
- **THEN** no category lists it anymore and it has no verdicts

### Requirement: Existing Memberships Backfilled On Upgrade
On upgrade, every video stored before this change SHALL be given the categories its playlist or channel is a member of.

#### Scenario: Upgrade with existing categories
- **WHEN** the system upgrades with the channel "@GothamChess" in "Chess" and 40 stored videos
- **THEN** after the upgrade all 40 stored videos belong to "Chess" and category lists, counts and feeds are unchanged

### Requirement: Video Category Change Notification
Whenever the categories of stored videos change, or a category they belong to is renamed or deleted, the system SHALL publish one domain event per operation naming every affected video. A video sync is one operation for its video, and so is storing a verdict. An operation that changes no video SHALL publish nothing.

#### Scenario: Members set
- **WHEN** the channel "@GothamChess" with 3 stored videos is added to "Chess"
- **THEN** one video-categories-changed event naming those 3 videos is published

#### Scenario: Category renamed
- **WHEN** the category "Chess", to which 5 stored videos belong, is renamed
- **THEN** one video-categories-changed event naming those 5 videos is published

#### Scenario: Reconcile pass repairing several videos
- **WHEN** a reconcile pass runs for "@GothamChess", a member of "Chess", and 2 of its stored videos belong to no category
- **THEN** one video-categories-changed event per repaired video is published, each naming that video

#### Scenario: Reconcile pass with nothing to repair
- **WHEN** a reconcile pass runs for a source whose videos already have its categories and their `yes` verdicts
- **THEN** no video-categories-changed event is published

#### Scenario: Yes verdict stored
- **WHEN** a `yes` verdict for "Sicilian" is stored for a video that didn't belong to it
- **THEN** one video-categories-changed event naming that video is published

#### Scenario: Verdict that changes nothing
- **WHEN** a `no` verdict is stored, or a `yes` verdict for a category the video already belongs to through its source
- **THEN** no video-categories-changed event is published
