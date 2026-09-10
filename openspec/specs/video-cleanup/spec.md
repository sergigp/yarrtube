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

### Requirement: File Located By Matching Sanitized Title
The system SHALL locate a removed video's file by searching its playlist's output directory for a file whose name matches the video's sanitized title, accounting for a `[video_id]` suffix the downloader may have appended to that filename to avoid a collision at download time.

#### Scenario: File found by title
- **WHEN** a removed video's playlist output directory contains a file whose name matches the video's sanitized title, with or without a `[video_id]` suffix
- **THEN** the system deletes that file

#### Scenario: No matching file found
- **WHEN** no file in the playlist's output directory matches the removed video's sanitized title
- **THEN** the system does not treat this as an error, and no file is deleted

### Requirement: File Deletion Skipped For a Deleted Playlist
The system SHALL NOT attempt to delete a video's file, or treat it as an error, when the deletion is attempted after the video's playlist no longer exists.

#### Scenario: Playlist deleted before its video's file deletion runs
- **WHEN** a video's file deletion is attempted for a playlist that no longer exists
- **THEN** the system makes no file system changes
