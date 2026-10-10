## Purpose

Moves a library downloaded under the previous per-video-folder layout into the TV-show layout in one explicit, repeatable run, using only the database and the files on disk, so existing users upgrade without re-downloading anything.

## ADDED Requirements

### Requirement: Migrate-Layout Subcommand
The system SHALL provide a `migrate-layout` CLI subcommand that converts every downloaded video still stored in the previous layout (a per-video folder or a flat file directly under its source directory) into the TV-show layout. It SHALL use the same database and videos root configuration as `serve`, and SHALL be intended to run while the daemon is stopped. For each such video it SHALL: assign the video's episode number from the publish timestamp recorded in its metadata (in publish-time order within a day, so a batch published on one day is numbered in upload order), move its file, its thumbnail and its NFO to the episode base name inside the season folder, rewrite the NFO as an episode NFO, record the new filenames on the video, and remove the now-empty per-video folder. It SHALL write every source's show files (`tvshow.nfo`, `poster.jpg`). It SHALL print one line per moved video and a final summary of moved and skipped videos, and exit non-zero when any video failed to move.

#### Scenario: Library in the previous layout
- **WHEN** `migrate-layout` runs over a library where every downloaded video has metadata and its file is present in its per-video folder
- **THEN** every video's files end up under `Season <year>/S<year>E<MMDDii> - <title>.<ext>`, each video's recorded filenames point at them, every source directory has a `tvshow.nfo`, the old per-video folders are gone, and the summary reports every video as moved

#### Scenario: Batch published on one day
- **WHEN** a playlist holds videos published seconds apart on the same day
- **THEN** their same-day indexes follow publish time, the earliest published getting `01`

#### Scenario: Move fails for one video
- **WHEN** moving one video's files fails (for example a permission error)
- **THEN** that video's record is unchanged, the failure is printed, the remaining videos are still processed, and the command exits non-zero

### Requirement: Dry Run
The subcommand SHALL accept a `--dry-run` flag under which it prints exactly what it would do (each video's current and new path, each skip with its reason, and the summary) and changes nothing on disk or in the database.

#### Scenario: Dry run
- **WHEN** `migrate-layout --dry-run` runs
- **THEN** the plan is printed, no file is moved or written, and no video record changes

### Requirement: Videos The Migration Cannot Place Are Skipped
The subcommand SHALL skip, with a printed reason, a downloaded video that has no recorded metadata (so no publish timestamp), or whose recorded file is missing from disk, and SHALL leave its record and files unchanged; such videos are healed by later reconcile passes (metadata recovery assigns the number and moves the files; a missing file triggers a redownload). Videos that are not downloaded SHALL be ignored.

#### Scenario: Video without metadata
- **WHEN** a downloaded video has no recorded metadata
- **THEN** the migration prints a skip for it and leaves it in place

#### Scenario: Video file missing
- **WHEN** a downloaded video's recorded file does not exist on disk
- **THEN** the migration prints a skip for it and leaves its record unchanged

### Requirement: Migration Is Idempotent And Resumable
Running the subcommand again after a complete or interrupted run SHALL move only the videos still in the previous layout, SHALL leave already-migrated videos untouched, and SHALL rewrite show files without error. An already-migrated library SHALL report zero moved videos.

#### Scenario: Second run
- **WHEN** `migrate-layout` runs on a library it already migrated
- **THEN** it moves nothing, rewrites the show files, and reports zero moved videos

#### Scenario: Interrupted run
- **WHEN** a run is interrupted after moving some videos and the subcommand runs again
- **THEN** the videos moved by the first run are left as they are and the remaining ones are moved
