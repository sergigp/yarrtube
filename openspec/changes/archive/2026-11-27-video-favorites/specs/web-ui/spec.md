# Spec Delta

## ADDED Requirements

### Requirement: Favorite Toggle In The Video Detail
The video detail pane SHALL show a star toggle with the accessible name "Add to favorites" while the video is not a favorite and "Remove from favorites" (filled, pressed) while it is. Toggling it SHALL favorite or unfavorite the video and refresh every video list and the favorites view.

#### Scenario: Favoriting from the detail pane
- **WHEN** a user presses "Add to favorites" on a video that is not a favorite
- **THEN** the application sends `PUT /api/videos/{id}/favorite` and the toggle becomes "Remove from favorites"

#### Scenario: Unfavoriting from the detail pane
- **WHEN** a user presses "Remove from favorites" on a favorite that is not kept
- **THEN** the application sends `DELETE /api/videos/{id}/favorite` without asking, and the toggle becomes "Add to favorites"

#### Scenario: Toggling fails
- **WHEN** the favorite or unfavorite request fails
- **THEN** the application tells the user and the toggle keeps its previous state

### Requirement: Favorite Action In The Video Actions Menu
The video actions menu SHALL offer "Add to favorites" for a video that is not a favorite and "Remove from favorites" for a favorite, behaving like the detail pane's toggle.

#### Scenario: Favoriting from a card's menu
- **WHEN** a user chooses "Add to favorites" from a home card's menu
- **THEN** the video becomes a favorite and its menu now offers "Remove from favorites"

### Requirement: Unfavoriting A Kept Video Asks For Confirmation
In a channel's or playlist's own views (its video list and grid, and their detail pane), unfavoriting a kept video, from the detail pane or the actions menu, SHALL ask for confirmation first, saying that the video is no longer in its channel or playlist and that its files are deleted at the next sync. Cancelling SHALL send nothing. Elsewhere (the Favorites, category and home views), unfavoriting SHALL send at once, as these views don't show whether a video is kept.

#### Scenario: Confirming
- **WHEN** a user unfavorites the kept video "Old talk" of the channel "Talks" and confirms
- **THEN** the dialog said "Old talk" is no longer in "Talks" and its files will be deleted at the next sync, and the application sends `DELETE /api/videos/{id}/favorite`

#### Scenario: Cancelling
- **WHEN** a user unfavorites a kept video and cancels the confirmation
- **THEN** no request is sent and the video stays a favorite

#### Scenario: Kept card's menu asks
- **WHEN** a user chooses "Remove from favorites" from the menu of a playlist card badged "Kept"
- **THEN** the confirmation is shown and nothing is sent until the user confirms

#### Scenario: Unfavoriting from the Favorites view
- **WHEN** a user unfavorites a kept video in the Favorites view
- **THEN** the application sends `DELETE /api/videos/{id}/favorite` without asking, and the video leaves the list

### Requirement: Favorite And Kept Marks On Video Cards
A favorite video's card SHALL show a filled star on its thumbnail. In a channel's or playlist's video list, a kept video's card SHALL also show a "Kept" badge.

#### Scenario: Favorite card
- **WHEN** a channel's list shows a favorite video
- **THEN** its thumbnail shows a filled star

#### Scenario: Kept card
- **WHEN** a channel's list shows a favorite that has aged out of its window
- **THEN** its card shows a filled star and a "Kept" badge

### Requirement: Sidebar Favorites Entry
The sidebar SHALL show a "Favorites" row with a star icon above the Categories section (above Channels while categories are off), whether or not any favorite exists. Selecting it SHALL navigate to the Favorites view at `/favorites`, and while the user is there the row SHALL be marked active.

#### Scenario: Entry above categories
- **WHEN** categories are shown
- **THEN** the sidebar shows the "Favorites" row above the Categories section

#### Scenario: Entry with categories off
- **WHEN** categories are off
- **THEN** the sidebar shows the "Favorites" row above the Channels section

#### Scenario: Opening favorites
- **WHEN** a user selects the "Favorites" row
- **THEN** the application navigates to `/favorites` and marks the row active

### Requirement: Favorites View
The Favorites view SHALL show a "Favorites" page header and the favorite videos as a grid in the order the server returns them (most recently favorited first), each card naming its source. Selecting a video SHALL open it as on a category's view. With no favorites it SHALL say "No favorites yet" and explain that the star on a video adds it here.

#### Scenario: Favorites listed
- **WHEN** a user opens the Favorites view and two videos are favorites
- **THEN** both are shown, the most recently favorited first, each with its channel or playlist name

#### Scenario: No favorites
- **WHEN** a user opens the Favorites view and no video is a favorite
- **THEN** the view says "No favorites yet"

#### Scenario: Unfavoriting from the Favorites view
- **WHEN** a user unfavorites a video shown in the Favorites view
- **THEN** it leaves the view

### Requirement: Source Delete Confirmation Counts Favorites
The confirmation for deleting a channel or playlist, from its sidebar row or its page header, SHALL say how many favorite videos it deletes when it has any (e.g. "Its 2 favorite videos will be deleted too."), and say nothing about favorites otherwise.

#### Scenario: Channel with favorites
- **WHEN** a user chooses to delete a channel whose favorite count is 2
- **THEN** the confirmation says "Its 2 favorite videos will be deleted too."

#### Scenario: Channel with one favorite
- **WHEN** a user chooses to delete a playlist whose favorite count is 1
- **THEN** the confirmation says "Its favorite video will be deleted too."

#### Scenario: No favorites
- **WHEN** a user chooses to delete a channel whose favorite count is 0
- **THEN** the confirmation doesn't mention favorites
