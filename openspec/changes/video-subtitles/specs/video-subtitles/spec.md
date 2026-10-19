# Spec Delta

## Purpose

Fetches every downloaded video's original-language captions, writes them as an episode subtitle file for media servers, and stores them as a plain-text transcript, so videos can be watched with subtitles and later summarised or recommended from what they say.

## ADDED Requirements

### Requirement: Subtitle State
Every video SHALL record a subtitle state: Missing (no subtitles obtained yet), Fetched (subtitle file and transcript recorded) or Unavailable (checks spanning at least 7 days found no original-language captions; terminal). While Missing, a video SHALL also record when a subtitle fetch first found no selectable track for it, if one has. A new video, and every video already stored when this capability is introduced, SHALL start Missing with no such time recorded. A video reset for redownload SHALL return to Missing, with no subtitle filename and no such time recorded.

#### Scenario: New video
- **WHEN** a video is newly added to a tracked playlist or channel
- **THEN** its subtitle state is Missing and it has no recorded subtitle filename

#### Scenario: Library upgraded
- **WHEN** the system starts on a database whose videos were stored before subtitles existed
- **THEN** every such video's subtitle state is Missing

#### Scenario: Video reset for redownload
- **WHEN** a video whose subtitle state is Fetched or Unavailable is reset for redownload
- **THEN** its subtitle state is Missing, it has no recorded subtitle filename, and no first check without captions is recorded

### Requirement: Original-Language Caption Selection
A subtitle fetch SHALL select a single caption track in the video's original language as reported by the downloader, trying that language and then its base language (e.g. `es-US`, then `es`): an uploaded caption track in that language or in a regional variant of it (e.g. `en-US` for `en`) when one exists, otherwise YouTube's auto-generated track marked as the original in that language, otherwise its plain auto-generated track in that language, but only when no auto-generated track is marked as the original in any language (YouTube lists machine translations of the original under plain language codes). When none of those exists and exactly one auto-generated track is marked as the original, in another language, that track SHALL be selected and its own language recorded (the reported language was wrong). It SHALL NOT select a translated track. When the downloader reports no original language, the auto-generated track marked as the original SHALL be selected when exactly one exists; otherwise, when every uploaded track shares one base language, the uploaded track in that language (preferring the base code itself) SHALL be selected; otherwise none.

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

#### Scenario: Reported language differs from the auto-generated original
- **WHEN** a video's reported original language is `en`, it has no uploaded track, and its auto-generated tracks are the original `es` track plus plain `en`, `fr` and `de` translations
- **THEN** the auto-generated original `es` track is selected and the transcript's language is `es`

#### Scenario: Original not marked, no other original
- **WHEN** a video's original language is `yue`, it has no uploaded track, and its only auto-generated track is a plain `yue` track (no track is marked as the original)
- **THEN** the `yue` track is selected

#### Scenario: Language not reported, uploaded captions only
- **WHEN** the downloader reports no original language, no auto-generated track is marked as the original, and the only uploaded tracks are `en` and `en-GB`
- **THEN** the uploaded `en` track is selected and the transcript is recorded as manual

#### Scenario: Language not reported, uploaded captions in several languages
- **WHEN** the downloader reports no original language, no auto-generated track is marked as the original, and the uploaded tracks are `en` and `de`
- **THEN** no track is selected

### Requirement: Subtitle File Written Alongside Video
A successful subtitle fetch SHALL write the selected captions in SubRip format to the video's season folder as `<base name>.<lang>.srt`, where `<base name>` is the recorded filename of the video's downloaded file without its extension (titles can change after the download, files are not renamed, and media servers pair a subtitle with the media file by base name) and `<lang>` is the base code of the track's language (its primary subtag, e.g. `es` for `es-US`) without any original-track marker, and SHALL record that path (relative to the show directory) as the video's subtitle filename. The transcript SHALL keep the track's full language code.

#### Scenario: Channel episode subtitles
- **WHEN** subtitles in `en` are fetched for a video numbered `2026`/`031501` titled `Title`
- **THEN** its show directory contains `Season 2026/S2026E031501 - Title.en.srt` and the video's recorded subtitle filename is that path

#### Scenario: Regional track
- **WHEN** a video's original language is `es-US` and its uploaded `es-US` track is fetched for the video numbered `2026`/`031501` titled `Title`
- **THEN** its show directory contains `Season 2026/S2026E031501 - Title.es.srt` and its transcript's language is `es-US`

#### Scenario: Video retitled after its download
- **WHEN** `en` subtitles are fetched for a video whose recorded filename is `Season 2026/S2026E031501 - Old Title.mp4` and whose title has since changed to `New Title`
- **THEN** its show directory contains `Season 2026/S2026E031501 - Old Title.en.srt` and that is its recorded subtitle filename

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
A successful subtitle fetch SHALL store the video's transcript: the captions' plain text, without cue numbers, timings or formatting tags and with lines repeated by rolling auto-generated captions collapsed, together with its language and whether it came from uploaded (manual) or auto-generated captions. Only a cue's number line, its timing line and markup tags (e.g. `<i>`, `</i>`, `<font …>`, inline `<00:01:02.345>` timestamps) SHALL be removed; every other caption text SHALL be kept, including lines made only of digits, a literal `<` and text containing `-->`. A video SHALL have at most one transcript, and it SHALL be deleted whenever the video's record is deleted, including when its whole playlist or channel is deleted.

#### Scenario: Auto-generated rolling captions
- **WHEN** a fetched auto-generated track repeats each line across consecutive cues as it scrolls
- **THEN** the stored transcript contains each spoken line once, in order

#### Scenario: Video removed
- **WHEN** a video with a stored transcript is removed from its playlist or channel and its record deleted
- **THEN** its transcript is deleted too

#### Scenario: Channel deleted
- **WHEN** a tracked channel whose videos have stored transcripts is deleted
- **THEN** those transcripts are deleted too

#### Scenario: Caption text that looks like markup
- **WHEN** a fetched track has cues whose text is `2024`, `if x < 5` and `use x --> y`
- **THEN** the stored transcript contains those three lines unchanged

### Requirement: Subtitle Fetch Outcome
A subtitle fetch SHALL set the video's subtitle state to Fetched when it writes the subtitle file and stores the transcript. A check is conclusive when the downloader successfully reports the video's caption tracks without reporting any warning (it can silently withhold tracks, e.g. when YouTube requires a token it does not have, and only says so in a warning). When a conclusive check finds no selectable track, the fetch SHALL record that time as the first check without captions if none is recorded yet, and SHALL set the state to Unavailable only when at least 7 days have passed both since the video was published (its record's creation time when no publish time is reported) and since that first check without captions. Otherwise the state SHALL stay Missing.

#### Scenario: Captions fetched
- **WHEN** a subtitle fetch selects a track and writes it
- **THEN** the video's subtitle state is Fetched

#### Scenario: Old video with no original-language captions
- **WHEN** a conclusive check finds no selectable track for a video published 30 days ago whose first check without captions was recorded 8 days ago
- **THEN** the video's subtitle state is Unavailable and no subtitle file is written

#### Scenario: Old video checked for the first time
- **WHEN** a conclusive check finds no selectable track for a video published 30 days ago (e.g. of a playlist added today) with no first check without captions recorded
- **THEN** its subtitle state stays Missing and now is recorded as its first check without captions, so later reconcile passes keep trying for 7 days

#### Scenario: Downloader warns while checking
- **WHEN** the downloader reports the caption tracks with a warning and none is selectable
- **THEN** the video's subtitle state stays Missing, no first check without captions is recorded, and the warning is logged

#### Scenario: Recent video with no captions yet
- **WHEN** a subtitle fetch finds no selectable track for a video published 2 days ago
- **THEN** the video's subtitle state stays Missing, so a later reconcile pass tries again

#### Scenario: Downloader fails
- **WHEN** the downloader fails while probing or downloading captions (e.g. rate limited, network error)
- **THEN** the video's subtitle state stays Missing and no subtitle file or transcript is recorded

### Requirement: Subtitle Fetch Is Best Effort
A subtitle fetch SHALL run as its own scheduled task, separately from the video download. Its failure SHALL NOT fail the task, SHALL NOT change the video's download status or any download field, and SHALL NOT be retried by the task queue; a later reconcile pass reschedules it. A subtitle fetch for a video that is not Downloaded, has no episode number, is still stored in the previous per-video-folder layout (it gets subtitles once `migrate-layout` has moved it into its season folder), or whose subtitle state is not Missing SHALL make no changes and SHALL NOT contact the downloader. A fetch SHALL place nothing when, by the time its subtitles are downloaded, the video is no longer Downloaded with the same recorded filename.

#### Scenario: Fetch runs for a video already Fetched
- **WHEN** a subtitle fetch runs for a video whose subtitle state is already Fetched or Unavailable
- **THEN** it makes no changes to the video or to disk

#### Scenario: Fetch runs for a video no longer downloaded
- **WHEN** a subtitle fetch runs for a video that has meanwhile been reset for redownload
- **THEN** it makes no changes and does not contact the downloader

#### Scenario: Fetch runs for a deleted video
- **WHEN** a subtitle fetch runs after its video's record was deleted
- **THEN** it completes without changes

#### Scenario: Video still in the previous layout
- **WHEN** a subtitle fetch runs for a Downloaded, numbered video whose recorded filename is still `My Video/My Video.mp4` or `My Video.mp4` (not yet migrated)
- **THEN** it makes no changes, writes no file and does not contact the downloader, and reconcile passes do not schedule a subtitle fetch for it

#### Scenario: Video reset for redownload while its subtitles are fetched
- **WHEN** a video is reset for redownload after a subtitle fetch for it has started but before it places the subtitle file
- **THEN** no subtitle file is placed, no transcript is stored, no subtitles-fetched event is published, and its subtitle state stays Missing

#### Scenario: Video deleted while its subtitles are fetched
- **WHEN** a video's record is deleted (or its playlist or channel deleted) after a subtitle fetch for it has started but before it places the subtitle file
- **THEN** no subtitle file is placed, no transcript is stored and no subtitles-fetched event is published

#### Scenario: Recording subtitles preserves download progress
- **WHEN** a video's download fields change between the time a subtitle fetch reads the video and the time it records the subtitles
- **THEN** the video keeps its new download fields and gains the recorded subtitle state and filename
