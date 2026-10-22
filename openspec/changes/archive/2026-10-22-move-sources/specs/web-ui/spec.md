# Spec Delta

## ADDED Requirements

### Requirement: Edit Dialogs Show The Location
The edit playlist and edit channel dialogs SHALL show a "Location" line with the source's current storage path and a "Move…" control next to it. Activating "Move…" SHALL close the edit dialog without saving and open the move dialog for that source.

#### Scenario: Location shown
- **WHEN** a user opens the edit dialog of a playlist stored at `music/chill`
- **THEN** the dialog shows "Location" with `music/chill` and a "Move…" control

#### Scenario: Move unavailable while work is in progress
- **WHEN** a user opens the edit dialog of a source listed with work in progress
- **THEN** the "Move…" control is disabled and the dialog says "Downloads in progress — you can move it once they finish."

#### Scenario: Opening the move dialog
- **WHEN** a user activates "Move…" in a channel's edit dialog
- **THEN** the edit dialog closes, nothing is saved, and the move dialog for that channel opens

### Requirement: Move Dialog
The move dialog SHALL be titled "Move <source name>" and pick the destination with the same "Save to" list, parent folder browser and folder name controls as the add dialogs, prefilled with the source's current parent and folder name, showing the resulting destination. Saving SHALL send the destination to the move endpoint; on success it SHALL close and refresh the library.

#### Scenario: Prefilled with the current location
- **WHEN** a user opens the move dialog for a playlist stored at `music/chill`
- **THEN** the parent folder is `music`, the folder name is `chill`, and the destination shown is `music/chill`

#### Scenario: Moving to a new folder
- **WHEN** a user browses to `archive`, names the folder `chill`, and saves
- **THEN** the application requests a move to `archive/chill`, closes the dialog, and the library shows the source at its new path

#### Scenario: Unchanged destination
- **WHEN** the destination equals the source's current path
- **THEN** the save control is disabled

#### Scenario: Folder name `.` or `..`
- **WHEN** a user names the folder `.` or `..`, or creates a new folder with either name in the folder browser
- **THEN** the dialog shows an error and the save control is disabled

#### Scenario: Move rejected
- **WHEN** the move is rejected, e.g. because the source still has downloads in progress or the destination is in use
- **THEN** the dialog stays open and shows the server's error message, and the source keeps its location

#### Scenario: Cancelling
- **WHEN** a user cancels the move dialog
- **THEN** no request is sent
