# layout-migration Specification

## Purpose

Moves a library downloaded under the previous per-video-folder layout into the TV-show layout in one explicit, repeatable run, using only the database and the files on disk, so existing users upgrade without re-downloading anything.

## Requirements

### Requirement: Migrate-Layout Subcommand
The system SHALL provide a `migrate-layout` CLI subcommand that converts every downloaded video still stored in the previous layout (a per-video folder or a flat file directly under its source directory) into the TV-show layout. It SHALL use the same database and videos root configuration as `serve`, and SHALL be intended to run while the daemon is stopped. For each such video it SHALL: take its episode number (a playlist video's is already recorded in playlist order, see `tv-show-layout`; a channel video's is assigned from the publish timestamp recorded in its metadata, in publish-time order within a day, so a batch published on one day is numbered in upload order), move its file, its thumbnail and its NFO to the episode base name inside the season folder, rewrite the NFO as an episode NFO, record the new filenames on the video, and remove the now-empty per-video folder. It SHALL write every source's show files (`tvshow.nfo`, `poster.jpg`); a source whose show files cannot be written SHALL be reported as a failure (so the command exits non-zero) and its videos SHALL still be migrated. It SHALL print one line per moved video and a final summary of moved and skipped videos, and exit non-zero when any video failed to move.

#### Scenario: Library in the previous layout
- **WHEN** `migrate-layout` runs over a library where every downloaded video has metadata and its file is present in its per-video folder
- **THEN** every channel video's files end up under `Season <year>/S<year>E<MMDDii> - <title>.<ext>` and every playlist video's under `Season 1/S01E<nnn> - <title>.<ext>`, each video's recorded filenames point at them, every source directory has a `tvshow.nfo`, the old per-video folders are gone, and the summary reports every video as moved

#### Scenario: Batch published on one day
- **WHEN** a channel holds videos published seconds apart on the same day
- **THEN** their same-day indexes follow publish time, the earliest published getting `01`

#### Scenario: Playlist in curated order
- **WHEN** a playlist's legacy videos sit at positions whose order differs from their publish order
- **THEN** each is moved under the episode number recorded for its position (`S01E001` for the first), not under a date-based number

#### Scenario: Move fails for one video
- **WHEN** moving one video's files fails (for example a permission error)
- **THEN** that video's recorded paths and status are unchanged (an episode number assigned before the failure is kept, so a rerun reuses it), the failure is printed, the remaining videos are still processed, and the command exits non-zero

### Requirement: Dry Run
The subcommand SHALL accept a `--dry-run` flag under which it prints exactly what it would do (each video's current and new path, each skip with its reason, and the summary) and moves, writes and records nothing; it only brings the database schema up to date, as `serve` does on its next start.

#### Scenario: Dry run
- **WHEN** `migrate-layout --dry-run` runs
- **THEN** the plan is printed, no file is moved or written, and no video record changes (pending schema migrations are applied)

#### Scenario: Dry run of a same-day batch
- **WHEN** `migrate-layout --dry-run` runs over videos of one channel published on the same day
- **THEN** the planned same-day indexes are exactly those the real run assigns (`01`, `02`, `03` in publish order), and no episode number is recorded

### Requirement: Videos The Migration Cannot Place Are Skipped
The subcommand SHALL skip, with a printed reason, a downloaded video that has neither a recorded episode number nor recorded metadata (so no publish timestamp; only a channel video can be in that state), or whose recorded file is missing from disk, and SHALL leave its record and files unchanged; such videos are healed by later reconcile passes (metadata recovery assigns the number and moves the files; a missing file triggers a redownload). Videos that are not downloaded SHALL be ignored.

#### Scenario: Channel video without metadata
- **WHEN** a downloaded channel video has no recorded metadata
- **THEN** the migration prints a skip for it and leaves it in place

#### Scenario: Playlist video without metadata
- **WHEN** a downloaded playlist video has no recorded metadata
- **THEN** the migration moves it under its recorded episode number without writing an NFO

#### Scenario: Show files cannot be written
- **WHEN** a source's show files cannot be written (for example a permission error)
- **THEN** the failure is printed for that source, its videos are still moved, and the command exits non-zero

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

#### Scenario: Run interrupted mid-video
- **WHEN** a previous run moved a video's media under its recorded episode number but not its thumbnail or its record, and the subcommand runs again
- **THEN** it completes that video's move under the same number, without reporting its file missing or redownloading it
