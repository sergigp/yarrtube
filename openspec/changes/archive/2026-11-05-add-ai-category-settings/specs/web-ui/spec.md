## RENAMED Requirements

- FROM: `### Requirement: Manage Sources Dialog`
- TO: `### Requirement: Category Settings Dialog`

## MODIFIED Requirements

### Requirement: Sidebar Category Row Actions
Each category row SHALL provide a menu control, shown under the same conditions as the channel and playlist rows' menu control in "Sidebar Row Menu", offering "Edit settings", which navigates to that category's view with the category settings dialog open, and "Delete", which deletes the category after the user confirms. The confirmation SHALL state how many channels and playlists the category holds and that none of them, nor any video, will be deleted. Deleting the category whose view is open SHALL return the user to the home view.

#### Scenario: Managing sources from the row menu
- **WHEN** a user chooses "Edit settings" from the "Chess" row menu
- **THEN** the "Chess" category view opens with the category settings dialog open

#### Scenario: Deleting from the row menu
- **WHEN** a user chooses "Delete" from the menu of "Chess", which holds 4 channels and playlists
- **THEN** a confirmation states that "Chess" holds 4 channels and playlists and that none of them, nor any video, will be deleted
- **AND** confirming deletes the category and removes its row

#### Scenario: Deleting the open category
- **WHEN** a user deletes the category whose view is open
- **THEN** the application navigates to the home view

#### Scenario: Menu control on hover
- **WHEN** a user with a mouse hovers the "Chess" row
- **THEN** its menu control becomes visible

### Requirement: Add Category Dialog
The "Add category" control SHALL open a dialog asking for a name and a color through the category color picker, with the palette color used by the fewest categories preselected (the earliest in palette order on a tie). Creating SHALL be possible once the name is not blank, and SHALL create the category with the chosen color. On success, the dialog SHALL close and the application SHALL navigate to the new category's view with the category settings dialog open. When creation fails (including a name already taken), the dialog SHALL stay open and show the error. On viewports narrower than the desktop breakpoint, activating "Add category" SHALL close the sidebar before the dialog is shown.

#### Scenario: Creating a category
- **WHEN** a user activates "Add category", enters "Chess" and creates it
- **THEN** the dialog closes and the "Chess" category view opens with the category settings dialog open and its search field focused

#### Scenario: Next free color preselected
- **WHEN** a user opens "Add category" and the existing categories are colored Amber and Red
- **THEN** "Orange" is preselected

#### Scenario: Creating with a chosen color
- **WHEN** a user enters "Chess", chooses "Violet" and creates it
- **THEN** "Chess" is created with the color "#8b5cf6"

#### Scenario: Blank name
- **WHEN** the name field is empty or blank
- **THEN** the create action is disabled

#### Scenario: Name already taken
- **WHEN** a user creates a category with a name another category already has
- **THEN** the dialog stays open and says the name is already taken

### Requirement: Category View
Each category SHALL have a view at an address of its own with a header and its videos as a grid of the same cards as the home view, each YouTube video once, newest sync first. Activating a card SHALL open the video within the category, as described in "Category Playback", instead of in its channel's or playlist's view. The header SHALL be tinted with the soft tint of the category's color and SHALL show the category's name, a summary of its number of sources, its number of videos and, when above 0, its number of unwatched videos, and the same actions layout as the channel and playlist views' page header, offering "Change color", which opens the category color picker and saves the chosen color immediately, "Edit settings", which opens the category settings dialog, "Mark all watched" (only when the category has sources), which marks every downloaded video of the category watched after the confirmation described in "Confirm Bulk Mark Watched", and "Delete", which deletes the category after the same confirmation as the sidebar row's "Delete" and then navigates to the home view; at or above the desktop breakpoint "Delete" SHALL be in the "More actions" menu as on the channel and playlist views. When the category is known from the categories list, the header SHALL show its name and tint at once, before its videos load, with a placeholder in place of the summary; the videos SHALL show as placeholders as described in "Loading Placeholders" until they load. A category with no sources SHALL say so and offer a control that opens the category settings dialog. A category with sources but without downloaded videos SHALL say it has no videos yet. When the category does not exist, the view SHALL say it was not found.

#### Scenario: Category videos
- **WHEN** a user opens the "Chess" view, which holds 2 sources with 14 downloaded videos, 3 of them unwatched
- **THEN** the view shows "Chess", the summary "2 sources · 14 videos · 3 unwatched", and the videos as cards, newest sync first

#### Scenario: Card opens the video in the category
- **WHEN** a user activates a video card on the "Chess" view
- **THEN** the application stays on the "Chess" view and plays that video, as described in "Category Playback"

#### Scenario: Header tinted
- **WHEN** a user opens the "Chess" view and "Chess" is colored "#0ea5e9"
- **THEN** the header's background is the soft tint of that color's hue

#### Scenario: Header while loading
- **WHEN** a user opens the "Chess" view from the sidebar and its videos are still loading
- **THEN** the header already shows "Chess" with its tint and actions, a placeholder in place of the summary, and placeholder cards below

#### Scenario: Changing the color
- **WHEN** a user activates "Change color" on the "Chess" view and chooses "Violet"
- **THEN** "Chess" is saved with the color "#8b5cf6", and the header, the sidebar dot and the home chips of its videos show it

#### Scenario: Mark all watched
- **WHEN** a user chooses "Mark all watched" from a category view's header and confirms
- **THEN** every card shows as watched, and the category's sidebar count disappears promptly

#### Scenario: Deleting from the category view
- **WHEN** a user chooses "Delete" from the "Chess" view's header and confirms
- **THEN** the category is deleted and the application navigates to the home view

#### Scenario: Empty category
- **WHEN** a user opens a category with no sources
- **THEN** the view says it holds no channels or playlists yet and offers a control that opens the category settings dialog

#### Scenario: Unknown category
- **WHEN** a user opens the view of a category that does not exist
- **THEN** the view says the category was not found

### Requirement: Category Settings Dialog
The category settings dialog SHALL be titled "Edit <name> settings" and SHALL show every tracked channel and playlist as a chip, channels under a "Channels" group and playlists under a "Playlists" group, each group ordered by name ignoring case. Channel chips SHALL show the channel's avatar; playlist chips a playlist icon. Each chip SHALL be a toggle whose pressed state means the source is in the category, shown with a check mark and a tinted fill, while a source not in it is shown outlined with a plus mark. Toggling a chip SHALL NOT change the order of the chips. A search field, focused when the dialog opens, SHALL filter the chips by name ignoring case, and pressing Enter in it while exactly one chip matches SHALL toggle that chip. Filter controls "All", "In <name>" and "Not in <name>", the last two showing how many sources they hold, SHALL narrow the chips by state. A group with no chip left to show SHALL be hidden. While AI, categories and categorization are all on, the dialog SHALL show, above the search field, an "AI prompt" multi-line text field prefilled with the category's AI prompt and described by the hint "Describe the videos that belong in this category."; otherwise it SHALL NOT show, and the saved prompt SHALL be kept. Toggling a chip or editing the prompt SHALL only stage changes: the dialog SHALL summarise the sources staged (such as "+2 added, −1 removed"), its "Save" control SHALL be disabled until something is staged, and saving SHALL send only what changed, making the category's AI prompt the text in the field (blank removes it) and its sources exactly the pressed chips, then close the dialog. The AI prompt SHALL be sent only while its field shows and differs from what it started from, so a prompt changed elsewhere while the dialog is open is neither staged nor overwritten. The dialog SHALL set no length limit of its own: a prompt the daemon rejects SHALL show the daemon's message and keep the text. When saving fails, the dialog SHALL stay open, keep the staged changes and say why. When the prompt is saved but the sources fail, the error SHALL read "The AI prompt was saved, but the sources weren't: <reason>", saving again SHALL send only the sources, and discarding SHALL say "The AI prompt is already saved. Your changes to the sources in <name> won't be saved." Cancelling or dismissing the dialog with staged changes SHALL ask to discard them first. With no channel or playlist tracked, the dialog SHALL say so. Chips SHALL be reachable with the arrow keys within a group. On viewports narrower than the desktop breakpoint the dialog SHALL fill the screen; the search, the filter controls and the save controls SHALL stay visible while the AI prompt and the chips scroll together. At every viewport size the save controls SHALL stay visible, and the AI prompt field SHALL NOT be resizable.

#### Scenario: Chips show membership
- **WHEN** a user opens the dialog of "Chess", which holds the channel "Hikaru", and "GothamChess" and the playlist "Endgame course" are also tracked
- **THEN** "Channels" shows "GothamChess" unpressed and "Hikaru" pressed, and "Playlists" shows "Endgame course" unpressed

#### Scenario: Staging and saving
- **WHEN** a user presses "GothamChess" and "Endgame course", unpresses "Hikaru" and saves
- **THEN** the summary read "+2 added, −1 removed" before saving, and afterwards "Chess" holds exactly "GothamChess" and "Endgame course"

#### Scenario: Order stays put
- **WHEN** a user toggles a chip
- **THEN** every chip keeps its place

#### Scenario: Searching
- **WHEN** a user types "ver" in the search field
- **THEN** only chips whose name contains "ver", ignoring case, are shown

#### Scenario: Enter toggles the only match
- **WHEN** the search leaves exactly one chip and the user presses Enter
- **THEN** that chip is toggled and the search field keeps focus

#### Scenario: Filtering by state
- **WHEN** a user chooses "In Chess"
- **THEN** only pressed chips are shown, staged ones included

#### Scenario: Nothing to save
- **WHEN** no chip has been toggled, or every toggle has been undone
- **THEN** "Save" is disabled

#### Scenario: AI prompt shown
- **WHEN** AI, categories and categorization are on and a user opens the dialog of "Chess", whose AI prompt is "Chess games"
- **THEN** "AI prompt" holds "Chess games"

#### Scenario: AI prompt hidden
- **WHEN** AI or categorization is off and a user opens the dialog of "Chess"
- **THEN** there is no "AI prompt" field and saving sources leaves its AI prompt as it was

#### Scenario: Saving only the prompt
- **WHEN** a user changes "AI prompt" without toggling a chip and saves
- **THEN** only the category's `ai_prompt` is sent, its members are not, and the dialog closes

#### Scenario: Saving the prompt and sources
- **WHEN** a user changes "AI prompt", presses a chip and saves
- **THEN** both the prompt and the members are saved and the dialog closes

#### Scenario: Removing the prompt
- **WHEN** a user empties "AI prompt" of a category that has one and saves
- **THEN** the category has no AI prompt

#### Scenario: Prompt changed elsewhere
- **WHEN** the dialog of "Chess" is open, its AI prompt changes elsewhere and the user saves only a chip change
- **THEN** only the members are sent and the prompt saved elsewhere is kept

#### Scenario: Prompt too long
- **WHEN** a user pastes 2500 characters into "AI prompt" and saves
- **THEN** all 2500 are sent, and the dialog stays open with the text and shows the daemon's message

#### Scenario: Prompt saved but sources failed
- **WHEN** a user changes "AI prompt", presses a chip and saves, and only the sources fail
- **THEN** the dialog says "The AI prompt was saved, but the sources weren't: <reason>", and saving again sends only the members

#### Scenario: Discarding after the prompt was saved
- **WHEN** the prompt was saved, the sources failed and the user cancels
- **THEN** the dialog asks to discard, saying "The AI prompt is already saved. Your changes to the sources in Chess won't be saved."

#### Scenario: Discarding staged changes
- **WHEN** a user with staged changes cancels the dialog
- **THEN** the user is asked to discard them, and confirming closes the dialog without saving

#### Scenario: Save fails
- **WHEN** saving fails
- **THEN** the dialog stays open with the staged changes and says why

#### Scenario: Nothing tracked
- **WHEN** a user opens the dialog with no channel or playlist tracked
- **THEN** the dialog says there are no channels or playlists to add yet

### Requirement: Summarize Videos Checkbox
The add and edit dialogs for channels and playlists SHALL show a "Summarize videos" checkbox described by the hint "Generate and show an AI summary of each video.". It SHALL sit in the "AI" section of the dialog. In add dialogs it SHALL be always visible (not in "Advanced options"), unchecked by default, sent as `summaries_enabled`. Edit dialogs SHALL prefill it and send `summaries_enabled` only when changed.

#### Scenario: Visible without advanced options
- **WHEN** a user opens either add dialog without expanding "Advanced options"
- **THEN** "Summarize videos" is visible

#### Scenario: Adding a channel without touching it
- **WHEN** a user adds a channel or playlist without touching "Summarize videos"
- **THEN** it is created with `summaries_enabled` `false`

#### Scenario: Adding a playlist with summaries
- **WHEN** AI summaries are on and a user checks "Summarize videos" in the add playlist dialog and submits
- **THEN** the playlist is created with `summaries_enabled` `true`

#### Scenario: Edit dialog prefilled
- **WHEN** a user opens "Edit settings" for a channel whose summaries are enabled
- **THEN** "Summarize videos" is checked

#### Scenario: Enabling summaries from the edit dialog
- **WHEN** AI summaries are on and a user checks "Summarize videos" for a playlist and saves
- **THEN** the application sends `summaries_enabled` `true` alone, closes the dialog, and does not trigger a sync

## ADDED Requirements

### Requirement: AI Section In Source Dialogs
The add and edit dialogs for channels and playlists SHALL group their AI options in a section headed "AI": "Summarize videos", then "Categorize videos". In add dialogs the section SHALL sit below "Save to" and categories; in edit dialogs, above categories.

#### Scenario: Both options grouped
- **WHEN** a user opens the add channel dialog with categories on
- **THEN** an "AI" section holds "Summarize videos" and "Categorize videos", in that order

### Requirement: Categorize Videos Checkbox
The add and edit dialogs for channels and playlists SHALL show, while categories are on, a "Categorize videos" checkbox described by the hint "Let AI add each video to the categories it fits.". In add dialogs it SHALL be always visible, unchecked by default and sent as `categorization_enabled`, `false` while the checkbox is not shown. Edit dialogs SHALL prefill it and send `categorization_enabled` only when changed.

#### Scenario: Adding a source without touching it
- **WHEN** a user adds a channel or playlist without touching "Categorize videos"
- **THEN** it is created with `categorization_enabled` `false`

#### Scenario: Adding a playlist with categorization
- **WHEN** AI categorization is on and a user checks "Categorize videos" in the add playlist dialog and submits
- **THEN** the playlist is created with `categorization_enabled` `true`

#### Scenario: Edit dialog prefilled
- **WHEN** a user opens "Edit settings" for a channel whose categorization is enabled
- **THEN** "Categorize videos" is checked

#### Scenario: Enabling categorization from the edit dialog
- **WHEN** AI categorization is on and a user checks "Categorize videos" for a playlist and saves
- **THEN** the application sends `categorization_enabled` `true` alone, closes the dialog, and does not trigger a sync

#### Scenario: Categories off
- **WHEN** categories are off and a user opens the add channel dialog
- **THEN** "Categorize videos" is not shown and "Summarize videos" still is

### Requirement: Categorize Videos Needs AI Categorization
"Categorize videos" SHALL be enabled only while the preferences have AI, categories and categorization all on. Otherwise, including while preferences load or fail to load, it SHALL be disabled unless the source already has categorization on, in which case it SHALL stay enabled so it can be turned off; its hint SHALL read "Turn on AI categorization in AI settings." with "AI settings" a link to `/ai` that closes the dialog.

#### Scenario: AI off
- **WHEN** AI is off and a user opens the add channel dialog
- **THEN** "Categorize videos" is unchecked and disabled, and the hint links to AI settings

#### Scenario: Turning categorization off while AI is off
- **WHEN** AI is off and a user opens "Edit settings" for a channel whose categorization is enabled, unchecks "Categorize videos" and saves
- **THEN** "Categorize videos" was shown checked and enabled, and only `categorization_enabled` `false` is sent

#### Scenario: Following the link
- **WHEN** AI is off and a user activates "AI settings" in the "Categorize videos" hint
- **THEN** the dialog closes and the AI page at `/ai` opens
