## MODIFIED Requirements

### Requirement: Page Header Actions Coverage
The suite SHALL verify on the desktop browser that a playlist's page header lays out Sync and the YouTube link as icon buttons named after them, with every other action behind its "⋮" control.

#### Scenario: Desktop page header
- **WHEN** a user opens a playlist's page on a desktop viewport
- **THEN** a "Sync" button and an "Open playlist on YouTube" link are visible, no button is named "Mark all watched", "Exclude from home", "Hide unwatched count", "Edit settings" or "Delete", and the "More actions for" menu offers "Mark all watched", "Exclude from home", "Hide unwatched count", "Edit settings", "Clear AI categories" and "Delete"

### Requirement: Category Detail Coverage
The suite SHALL verify that a category's view lists the downloaded videos of its sources as cards and that a card plays its video within the category.

#### Scenario: Play from a category
- **WHEN** a category holds a source with a downloaded video and the user opens the category and activates that video's card
- **THEN** the address identifies the category and the video, the info shows that video's title, and it plays from its source
