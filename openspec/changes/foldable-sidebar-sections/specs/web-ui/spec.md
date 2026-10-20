## ADDED Requirements

### Requirement: Foldable Sidebar Sections
Each sidebar section heading SHALL be a control that folds and unfolds its section. The heading SHALL show a chevron pointing right while the section is folded and down while it is unfolded, and the whole heading (chevron and title) SHALL be the toggle target. The heading SHALL expose to assistive technology whether its section is unfolded. An unfolded section SHALL list all its rows in the section's order. A folded section SHALL show only its heading: its rows, its add control and its loading, empty and error messages SHALL be hidden. Folding SHALL apply the same way in the mobile sidebar.

#### Scenario: Unfolded section lists every row
- **WHEN** 40 channels are tracked and the Channels section is unfolded
- **THEN** the Channels section lists all 40 channels, with no show-more control

#### Scenario: Folding a section
- **WHEN** a user activates the "Channels" heading while the Channels section is unfolded
- **THEN** the Channels section shows only its heading, with a right-pointing chevron, and no channel rows or "Add channel" control

#### Scenario: Unfolding a section
- **WHEN** a user activates the "Channels" heading while the Channels section is folded
- **THEN** the section shows its add control and all its rows again, with a down-pointing chevron

#### Scenario: Sections fold independently
- **WHEN** a user folds the Channels section
- **THEN** the Playlists section is unchanged

#### Scenario: Folded state is announced
- **WHEN** a screen reader reaches the heading of a folded section
- **THEN** the heading is announced as a collapsed control named after the section

### Requirement: Folded Section Unwatched Total
A folded Channels section SHALL show, beside its heading title and in a muted style distinct from row badges, the sum of its channels' unwatched counts, when that sum is above 0. No total SHALL be shown while the section is unfolded, while it is folded with a sum of 0, or for the Playlists section. The total SHALL update together with the channel badges.

#### Scenario: Folded channels with unwatched videos
- **WHEN** the Channels section is folded and its channels have 3, 0 and 20 unwatched videos
- **THEN** the "Channels" heading shows a muted total of 23

#### Scenario: Folded channels with nothing unwatched
- **WHEN** the Channels section is folded and no channel has unwatched videos
- **THEN** the "Channels" heading shows no total

#### Scenario: Unfolded section shows no total
- **WHEN** the Channels section is unfolded
- **THEN** the "Channels" heading shows no total

#### Scenario: Playlists show no total
- **WHEN** the Playlists section is folded
- **THEN** the "Playlists" heading shows no total

#### Scenario: Total follows watching
- **WHEN** the Channels section is folded and a user finishes watching one of a channel's unwatched videos
- **THEN** the total decreases by one promptly without a page reload

### Requirement: Active Entry In A Folded Section
When the user is on a channel's or playlist's detail view and that entry's section is folded, the section SHALL stay folded and its heading SHALL be marked active, visually and to assistive technology. The heading SHALL NOT be marked active while the section is unfolded or when no entry of the section is being viewed.

#### Scenario: Opening a channel of a folded section
- **WHEN** the Channels section is folded and a user opens a channel's detail view
- **THEN** the Channels section stays folded and its heading is marked active

#### Scenario: Unfolding shows the active entry
- **WHEN** a user unfolds a section whose heading is marked active
- **THEN** the heading is no longer marked active and the viewed entry is shown marked active

#### Scenario: Viewing another section's entry
- **WHEN** the Channels section is folded and a user opens a playlist's detail view
- **THEN** the "Channels" heading is not marked active

### Requirement: Remembered Sidebar Fold State
The application SHALL remember, per browser, whether each sidebar section is folded, and SHALL restore that state when the application is loaded again. When no state has been remembered, a section SHALL start unfolded. When the remembered state cannot be read, the section SHALL start unfolded without an error.

#### Scenario: Folded section survives a reload
- **WHEN** a user folds the Playlists section and reloads the page
- **THEN** the Playlists section is still folded, and the Channels section keeps its own fold state

#### Scenario: First visit
- **WHEN** a user loads the application with no remembered fold state
- **THEN** every sidebar section is unfolded

#### Scenario: Storage unavailable
- **WHEN** the browser's storage cannot be read
- **THEN** every sidebar section starts unfolded and folding still works until the page is reloaded

## MODIFIED Requirements

### Requirement: Sidebar Add Entries
The sidebar's Channels section SHALL offer an "Add channel" control and its Playlists section an "Add playlist" control, each placed between the section heading and the section's list and visible whenever the section is unfolded, regardless of the section's rows. A folded section SHALL NOT show its add control. "Add channel" SHALL open the add channel dialog and "Add playlist" SHALL open the add playlist dialog. On viewports narrower than the desktop breakpoint, activating either control SHALL close the sidebar before the dialog is shown.

#### Scenario: Opening the add channel dialog from the sidebar
- **WHEN** a user activates "Add channel" in the sidebar's Channels section
- **THEN** the add channel dialog opens

#### Scenario: Opening the add playlist dialog from the sidebar
- **WHEN** a user activates "Add playlist" in the sidebar's Playlists section
- **THEN** the add playlist dialog opens

#### Scenario: Add entries with nothing tracked
- **WHEN** no channels or playlists are tracked and both sections are unfolded
- **THEN** both sections still show their add control beneath their heading

#### Scenario: Folded section hides its add entry
- **WHEN** the Channels section is folded
- **THEN** no "Add channel" control is shown

#### Scenario: Adding from the mobile sidebar
- **WHEN** a user on a mobile-width viewport opens the sidebar and activates "Add channel" or "Add playlist"
- **THEN** the sidebar closes and the matching add dialog is shown

### Requirement: Sidebar Search
When the number of tracked channels plus tracked playlists is above 15, the sidebar SHALL show a single search field above its sections. At 15 or fewer the field SHALL NOT be shown. While the field contains text, each section SHALL show every entry whose name contains that text, ignoring case, in the section's usual order and regardless of whether the section is folded. A section with no matching entry SHALL be hidden. When no entry matches in either section, the sidebar SHALL say that nothing matches. Clearing the field SHALL restore each section's folded or unfolded view. Searching SHALL NOT change any section's remembered fold state.

#### Scenario: Search field hidden for a short list
- **WHEN** 10 channels and 5 playlists are tracked
- **THEN** the sidebar shows no search field

#### Scenario: Search field shown for a long list
- **WHEN** 12 channels and 4 playlists are tracked
- **THEN** the sidebar shows a search field above the Channels section

#### Scenario: Matching across both sections
- **WHEN** a user types "ver" and the channel "Veritasium" and the playlist "Universe Overview" are tracked
- **THEN** the Channels section shows "Veritasium" and the Playlists section shows "Universe Overview"

#### Scenario: Matches in a folded section
- **WHEN** the Channels section is folded and a user types text matching the channel "Veritasium"
- **THEN** the Channels section shows "Veritasium"

#### Scenario: Section without matches is hidden
- **WHEN** a user's search text matches only channels
- **THEN** the Playlists section is not shown

#### Scenario: Clearing the search
- **WHEN** a user clears the search field
- **THEN** both sections return to the folded or unfolded view they had before searching

## REMOVED Requirements

### Requirement: Collapsed Sidebar Sections
**Reason**: Replaced by user-driven folding of whole sections; an unfolded section now lists all its rows, ordered so unread channels lead.
**Migration**: Fold a section from its heading to hide its rows; use search to find an entry in a long section.

### Requirement: Remembered Sidebar Section State
**Reason**: There is no row-level expanded state left to remember.
**Migration**: The fold state is remembered instead (see "Remembered Sidebar Fold State"); old stored expanded states are ignored.

### Requirement: Active Sidebar Entry Always Visible
**Reason**: An unfolded section always shows every row, so the active entry is always visible there; a folded section marks its heading active instead.
**Migration**: See "Active Entry In A Folded Section".
