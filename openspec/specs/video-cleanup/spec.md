# video-cleanup Specification

## Purpose

Deletes a video's downloaded file from disk once that video is no longer part of its tracked playlist, so videos removed from a playlist don't leave orphaned files behind forever.

## Requirements

### Requirement: File Deletion Triggered By Video Removal
The system SHALL delete a video's downloaded file from disk as soon as that video is removed from its tracked playlist, provided the video had reached the downloaded status. The system SHALL NOT attempt any file deletion for a video that never finished downloading.

#### Scenario: Downloaded video removed from its playlist
- **WHEN** a video with status downloaded is removed from its playlist
- **THEN** the system deletes that video's file from disk

#### Scenario: Never-downloaded video removed from its playlist
- **WHEN** a video that never reached the downloaded status is removed from its playlist
- **THEN** the system does not attempt any file deletion for that video

### Requirement: File Located By Its Recorded Filename
The system SHALL locate a removed video's file by its recorded filename
(the exact name recorded when the video was downloaded), rather than by
re-deriving a filename from its title.

#### Scenario: File found by recorded filename
- **WHEN** a removed video has a recorded filename and its playlist's output directory contains a file with that exact name
- **THEN** the system deletes that file

#### Scenario: No recorded filename
- **WHEN** a removed video has no recorded filename because it was never successfully downloaded
- **THEN** the system does not attempt any file deletion for it

#### Scenario: Recorded filename not found on disk
- **WHEN** a removed video has a recorded filename but no file with that exact name exists in its playlist's output directory
- **THEN** the system does not treat this as an error, and no file is deleted

### Requirement: File Deletion Skipped For a Deleted Playlist
The system SHALL NOT attempt to delete a video's file, or treat it as an error, when the deletion is attempted after the video's playlist no longer exists.

#### Scenario: Playlist deleted before its video's file deletion runs
- **WHEN** a video's file deletion is attempted for a playlist that no longer exists
- **THEN** the system makes no file system changes
