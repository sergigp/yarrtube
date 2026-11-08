# Spec Delta

## ADDED Requirements

### Requirement: Edit A Video's Categories
While categories are on, every video's actions menu SHALL offer "Edit categories…", opening a dialog titled with the video's title that shows every category as a chip in its color, filled when the video belongs to it. A chip from the video's channel or playlist SHALL be locked, with the hint "From <source name>". A chip the AI decided SHALL carry an "AI" mark. Clicking any other chip SHALL set that category by hand to the opposite state and show the result; a failure SHALL show the error and leave the chip as it was.

#### Scenario: Opening the dialog
- **WHEN** the user opens "Edit categories…" on a video of "@GothamChess", in "Chess", with an AI `yes` for "Sicilian"
- **THEN** "Chess" is filled and locked with "From @GothamChess", "Sicilian" is filled with an "AI" mark, and the other categories are outlined

#### Scenario: Removing an AI category
- **WHEN** the user clicks the filled "Sicilian" chip
- **THEN** the daemon is asked to set "Sicilian" to `belongs` false and the chip shows outlined without the "AI" mark

#### Scenario: Categories off
- **WHEN** categories are off
- **THEN** the video actions menu has no "Edit categories…"

### Requirement: Re-run AI From The Video Dialog
While AI categorization is available, the Edit categories dialog SHALL show a button that deletes the video's AI verdicts and then shows the categories as they are, without a confirmation. It SHALL read "Re-run AI" when the video's channel or playlist has categorization on, and "Clear AI categories" when it is off, since then nothing is evaluated again.

#### Scenario: Re-running one video
- **WHEN** the user clicks "Re-run AI" in the dialog of a video with an AI `yes` for "Politics"
- **THEN** the daemon is asked to delete the video's AI verdicts and "Politics" shows outlined

#### Scenario: Source with categorization off
- **WHEN** the user opens the dialog of a video whose channel has categorization off
- **THEN** the button reads "Clear AI categories", and clicking it asks the daemon to delete the video's AI verdicts

### Requirement: Re-run Or Clear From A Source's Menu
While categories are on, the actions menu of a channel or playlist SHALL offer "Re-run AI categorization" when the source has categorization on, and "Clear AI categories" when it is off. Either SHALL ask for confirmation, naming the source, before deleting the source's AI verdicts.

#### Scenario: Source with categorization off
- **WHEN** the user opens the menu of "@GothamChess", whose categorization is off, and chooses "Clear AI categories", then confirms
- **THEN** the daemon is asked to delete the channel's AI verdicts

#### Scenario: Cancelled
- **WHEN** the user chooses "Re-run AI categorization" and cancels the confirmation
- **THEN** nothing is sent to the daemon

### Requirement: Re-evaluate A Category From Its Settings
The category settings dialog SHALL show "Re-evaluate <N> videos" below the AI prompt when the category's `ai_verdict_count` N is above 0. It SHALL ask for confirmation saying the N videos will be evaluated again by the AI, then delete the category's AI verdicts. While the AI prompt has unsaved edits the button SHALL be disabled, with the hint "Save the prompt first to re-evaluate with it.", since the AI asks with the saved prompt.

#### Scenario: After editing a prompt
- **WHEN** "Food for thought" has `ai_verdict_count` 40 and the user clicks "Re-evaluate 40 videos" and confirms
- **THEN** the daemon is asked to delete the category's AI verdicts

#### Scenario: Nothing evaluated yet
- **WHEN** a category has `ai_verdict_count` 0
- **THEN** no re-evaluate button is shown

#### Scenario: Unsaved prompt
- **WHEN** "Food for thought" has `ai_verdict_count` 40 and the user edits its prompt without saving
- **THEN** "Re-evaluate 40 videos" is disabled and "Save the prompt first to re-evaluate with it." is shown
