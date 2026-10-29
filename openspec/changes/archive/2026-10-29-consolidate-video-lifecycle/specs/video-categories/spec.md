# Spec Delta

## MODIFIED Requirements

### Requirement: Reconcile Passes Repair Video Categories
Each video sync SHALL set the categories of its video to its source's categories when the source is a member of at least one category, and SHALL leave them unchanged otherwise. Since every playlist or channel reconcile pass runs the video sync of every stored video of that source (see `video-sync`), each pass repairs the categories of all of them.

#### Scenario: Drifted video repaired
- **WHEN** a stored video of the channel "@GothamChess", a member of "Chess", belongs to no category, and the channel's reconcile pass runs
- **THEN** the video belongs to "Chess"

#### Scenario: Uncategorized source untouched
- **WHEN** the reconcile pass of a playlist that is a member of no category runs
- **THEN** the categories of its stored videos are unchanged

### Requirement: Video Category Change Notification
Whenever the categories of stored videos change, or a category they belong to is renamed or deleted, the system SHALL publish one domain event per operation naming every affected video. A video sync is one operation for its video. An operation that changes no video SHALL publish nothing.

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
- **WHEN** a reconcile pass runs for a source whose videos already have its categories
- **THEN** no video-categories-changed event is published
