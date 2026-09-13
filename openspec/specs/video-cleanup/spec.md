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

### Requirement: Playlist Directory Deleted When Its Playlist Is Deleted
The system SHALL delete a playlist's entire output directory from disk once that playlist has been deleted, regardless of what videos it contained, their download status, or whether any files were ever written to it. This is separate from, and not conditioned on, per-video file deletion (see the video-removal requirements above), since deleting a playlist removes every video record for it in the same operation rather than removing them one at a time.

#### Scenario: Deleted playlist had downloaded videos
- **WHEN** a playlist with one or more downloaded videos is deleted
- **THEN** the system deletes that playlist's output directory, and every file in it, from disk

#### Scenario: Deleted playlist had no downloaded videos
- **WHEN** a playlist with no downloaded videos (or no videos at all) is deleted
- **THEN** the system does not treat a missing or empty output directory as an error

#### Scenario: Output directory already absent
- **WHEN** a deleted playlist's output directory does not exist on disk at the time this cleanup runs
- **THEN** the system does not treat this as an error, and makes no filesystem changes
