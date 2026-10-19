# Spec Delta

## Purpose

Fetches every downloaded video's original-language captions, writes them as an episode subtitle file for media servers, and stores them as a plain-text transcript, so videos can be watched with subtitles and later summarised or recommended from what they say.

## ADDED Requirements

### Requirement: Subtitle State
Every video SHALL record a subtitle state: Missing (no subtitles obtained yet), Fetched (subtitle file and transcript recorded) or Unavailable (the video has no original-language captions; terminal). A new video, and every video already stored when this capability is introduced, SHALL start Missing.

#### Scenario: New video
- **WHEN** a video is newly added to a tracked playlist or channel
- **THEN** its subtitle state is Missing and it has no recorded subtitle filename

#### Scenario: Library upgraded
- **WHEN** the system starts on a database whose videos were stored before subtitles existed
- **THEN** every such video's subtitle state is Missing

### Requirement: Original-Language Caption Selection
A subtitle fetch SHALL select a single caption track in the video's original language as reported by the downloader, trying that language and then its base language (e.g. `es-US`, then `es`): an uploaded caption track in that language or in a regional variant of it (e.g. `en-US` for `en`) when one exists, otherwise YouTube's auto-generated track marked as the original in that language, otherwise its plain auto-generated track in that language. It SHALL NOT select a translated track. When the downloader reports no original language, the auto-generated track marked as the original SHALL be selected when exactly one exists.

#### Scenario: Uploaded captions in the original language
- **WHEN** a video's original language is `en` and it has an uploaded `en` track and an auto-generated `en` track
- **THEN** the uploaded `en` track is selected and the transcript is recorded as manual

#### Scenario: Only auto-generated captions
- **WHEN** a video's original language is `es` and it has no uploaded track in `es` but an auto-generated original `es` track
- **THEN** the auto-generated `es` track is selected and the transcript is recorded as automatic

#### Scenario: Only translated tracks
- **WHEN** a video's original language is `es` and its only tracks are uploaded or auto-translated tracks in other languages
- **THEN** no track is selected

#### Scenario: Regional original language
- **WHEN** a video's original language is `es-US` and its only track in that language is the auto-generated original `es` track
- **THEN** that track is selected

#### Scenario: Regional uploaded captions
- **WHEN** a video's original language is `en` and it has an uploaded `en-US` track and an auto-generated original `en` track
- **THEN** the uploaded `en-US` track is selected and the transcript is recorded as manual

#### Scenario: Original language not reported
- **WHEN** the downloader reports no original language for a video and one auto-generated track is marked as the original
- **THEN** that track is selected

### Requirement: Subtitle File Written Alongside Video
A successful subtitle fetch SHALL write the selected captions in SubRip format to the video's season folder as `<episode base name>.<lang>.srt`, where `<lang>` is the base code of the track's language (its primary subtag, e.g. `es` for `es-US`) without any original-track marker, and SHALL record that path (relative to the show directory) as the video's subtitle filename. The transcript SHALL keep the track's full language code.

#### Scenario: Channel episode subtitles
- **WHEN** subtitles in `en` are fetched for a video numbered `2026`/`031501` titled `Title`
- **THEN** its show directory contains `Season 2026/S2026E031501 - Title.en.srt` and the video's recorded subtitle filename is that path

#### Scenario: Regional track
- **WHEN** a video's original language is `es-US` and its uploaded `es-US` track is fetched for the video numbered `2026`/`031501` titled `Title`
- **THEN** its show directory contains `Season 2026/S2026E031501 - Title.es.srt` and its transcript's language is `es-US`

### Requirement: Subtitle Fetch Scheduled After Download
The system SHALL schedule a subtitle fetch to run immediately each time a video finishes downloading, without waiting for the next reconcile pass. It SHALL NOT schedule a second one for a video that already has a subtitle fetch pending or running. Reconcile passes keep scheduling fetches for videos whose subtitles are still Missing (see `playlist-reconciliation`, `channel-video-sync`).

#### Scenario: Video finishes downloading
- **WHEN** a video finishes downloading
- **THEN** a subtitle fetch task for that video is scheduled to run now

#### Scenario: Subtitle fetch already queued
- **WHEN** a video finishes downloading while a subtitle fetch for it is already pending or running
- **THEN** no second subtitle fetch is scheduled

### Requirement: Show Folder Scanned After Subtitles Are Fetched
When the Plex scan integration is enabled (see `plex-library-scan`), the system SHALL, after each subtitle fetch that writes a subtitle file, ask Plex to scan the video's show directory, following the same section discovery, path mapping, asynchrony and bounded retries as the scan after a download. A failed or slow scan request MUST NOT affect the video's subtitle state. A subtitle fetch that writes no file SHALL NOT trigger a scan.

#### Scenario: Subtitles fetched with Plex enabled
- **WHEN** the integration is enabled and a subtitle fetch writes a subtitle file for a video of the channel stored at `channels/Some Channel`
- **THEN** Plex is asked to scan that show directory's Plex-side path in every section containing it

#### Scenario: No subtitle file written
- **WHEN** a subtitle fetch marks a video Unavailable, or leaves it Missing
- **THEN** no Plex scan is requested for it

#### Scenario: Scan request fails
- **WHEN** the scan request after a subtitle fetch fails
- **THEN** the video's subtitle state stays Fetched, and the failure is logged and retried

### Requirement: Transcript Stored
A successful subtitle fetch SHALL store the video's transcript: the captions' plain text, without timing or formatting markup and with lines repeated by rolling auto-generated captions collapsed, together with its language and whether it came from uploaded (manual) or auto-generated captions. A video SHALL have at most one transcript, and it SHALL be deleted when the video's record is deleted.

#### Scenario: Auto-generated rolling captions
- **WHEN** a fetched auto-generated track repeats each line across consecutive cues as it scrolls
- **THEN** the stored transcript contains each spoken line once, in order

#### Scenario: Video removed
- **WHEN** a video with a stored transcript is removed from its playlist or channel and its record deleted
- **THEN** its transcript is deleted too

### Requirement: Subtitle Fetch Outcome
A subtitle fetch SHALL set the video's subtitle state to Fetched when it writes the subtitle file and stores the transcript. It SHALL set it to Unavailable only when the downloader successfully reports the video's caption tracks, none is selectable, and the video was published at least 7 days before the fetch (its record's creation time when no publish time is reported). Otherwise the state SHALL stay Missing.

#### Scenario: Captions fetched
- **WHEN** a subtitle fetch selects a track and writes it
- **THEN** the video's subtitle state is Fetched

#### Scenario: Old video with no original-language captions
- **WHEN** a subtitle fetch finds no selectable track for a video published 30 days ago
- **THEN** the video's subtitle state is Unavailable and no subtitle file is written

#### Scenario: Recent video with no captions yet
- **WHEN** a subtitle fetch finds no selectable track for a video published 2 days ago
- **THEN** the video's subtitle state stays Missing, so a later reconcile pass tries again

#### Scenario: Downloader fails
- **WHEN** the downloader fails while probing or downloading captions (e.g. rate limited, network error)
- **THEN** the video's subtitle state stays Missing and no subtitle file or transcript is recorded

### Requirement: Subtitle Fetch Is Best Effort
A subtitle fetch SHALL run as its own scheduled task, separately from the video download. Its failure SHALL NOT fail the task, SHALL NOT change the video's download status or any download field, and SHALL NOT be retried by the task queue; a later reconcile pass reschedules it. A subtitle fetch for a video that is not Downloaded, has no episode number, or whose subtitle state is not Missing SHALL make no changes and SHALL NOT contact the downloader.

#### Scenario: Fetch runs for a video already Fetched
- **WHEN** a subtitle fetch runs for a video whose subtitle state is already Fetched or Unavailable
- **THEN** it makes no changes to the video or to disk

#### Scenario: Fetch runs for a video no longer downloaded
- **WHEN** a subtitle fetch runs for a video that has meanwhile been reset for redownload
- **THEN** it makes no changes and does not contact the downloader

#### Scenario: Fetch runs for a deleted video
- **WHEN** a subtitle fetch runs after its video's record was deleted
- **THEN** it completes without changes

#### Scenario: Recording subtitles preserves download progress
- **WHEN** a video's download fields change between the time a subtitle fetch reads the video and the time it records the subtitles
- **THEN** the video keeps its new download fields and gains the recorded subtitle state and filename
