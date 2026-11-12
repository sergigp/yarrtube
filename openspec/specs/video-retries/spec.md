# video-retries Specification

## Purpose

One status vocabulary and one retry policy for every per-video step that has its own task (download, thumbnail, subtitles, chapters, quality check, summary, categorization): back off after each failure, give up after about a week, and let the user retry a video's retrying or failed steps at once.

## Requirements

### Requirement: Step Status
Every per-video step SHALL have a status from one vocabulary: Pending (not done yet, no failed attempt), Retrying (failed 1 to 10 consecutive times, next attempt scheduled), Done (succeeded), Unavailable (YouTube doesn't have it; final) or Failed (gave up after 11 consecutive failed attempts). The download additionally has In Progress while it runs, and calls Done "Downloaded". A step SHALL record its consecutive failed attempts and when the last one failed. In other specs, a video "Excluded" is an Unavailable download, "Errored" a Retrying or Failed download, subtitles or summary "Missing" are Pending, subtitles "Fetched" and a summary "Generated" are Done, an `unavailable` verdict is a `failed` verdict, and "settled" late data is Done, Unavailable or Failed.

#### Scenario: New video
- **WHEN** a video is added to a tracked playlist
- **THEN** its download, thumbnail, subtitles, chapters, quality check, summary and categorization are all Pending

#### Scenario: Step that failed twice
- **WHEN** a video's subtitle check failed twice in a row
- **THEN** its subtitles are Retrying with 2 failed attempts

#### Scenario: Old video without captions
- **WHEN** a conclusive check finds no captions for a video published 30 days ago
- **THEN** its subtitles are Unavailable with no failed attempts

### Requirement: Retry Backoff
A step's next attempt SHALL be due 10 minutes after its 1st consecutive failure, 1 hour after the 2nd, 6 hours after the 3rd, and 24 hours after each later one, counted from when the last attempt failed. A successful attempt SHALL make the step Done and clear its failed attempts.

#### Scenario: First failure
- **WHEN** a video's download fails at 10:00 for the first time
- **THEN** its download is Retrying with its next attempt due at 10:10

#### Scenario: Backoff grows
- **WHEN** a step's 2nd, 3rd and 4th consecutive attempts fail
- **THEN** its next attempt is due 1 hour, 6 hours and 24 hours after each failure respectively

#### Scenario: Success clears the failures
- **WHEN** a Retrying step with 3 failed attempts succeeds
- **THEN** it is Done with no failed attempts

### Requirement: What Counts As A Failed Attempt
A failed attempt SHALL be: for the download, any failure that is not permanently unavailable (see `video-download`), including `yt-dlp` failing to run and the files failing to be placed; for the thumbnail, a fetch that obtains no thumbnail; for subtitles, chapters and the quality check, a failed or inconclusive check (see `late-video-data`); for the summary and the categorization, a rejected call, an unusable answer, an unreadable transcript, or Ollama answering a server error. A check that conclusively finds data not on YouTube yet, a step skipped because a setting turns it off, Ollama being unavailable (see "Waiting For Ollama"), and a database error SHALL NOT count as failed attempts.

#### Scenario: Ollama down
- **WHEN** a summary runs while nothing answers at the Ollama URL
- **THEN** the summary keeps its status and failed attempts, and the summary is re-queued 6 hours later

#### Scenario: Ollama answers a server error
- **WHEN** a summary runs and Ollama answers 500 to its call
- **THEN** the summary is Retrying with one more failed attempt and its next attempt is scheduled

#### Scenario: AI turned off
- **WHEN** a summary runs while AI is off
- **THEN** the summary stays Pending with no failed attempt and nothing is scheduled

#### Scenario: Captions not on YouTube yet
- **WHEN** a conclusive subtitle check finds no captions for a video published 2 days ago
- **THEN** its subtitles stay Pending with no failed attempt

#### Scenario: Database unavailable
- **WHEN** a step cannot read or store the video because the database fails
- **THEN** no failed attempt is counted on the video, and the task fails for the task queue to retry and dead-letter it

### Requirement: Waiting For Ollama
Ollama SHALL be unavailable to a summary or categorization run when it can't be connected to (refused, unknown host, or no connection within 5 seconds), before or during the run, or when the saved model isn't among those it lists before the run. The run SHALL then stop, leave the step's status and failed attempts unchanged, and re-queue the step's task 6 hours later, with the reason as its last error, without failing the task. There SHALL be no limit on how many times a step waits.

#### Scenario: Pending summary waits
- **WHEN** a Pending summary runs at 10:00 while nothing answers at the Ollama URL
- **THEN** the summary is Pending with no failed attempts and its task is pending to run at 16:00

#### Scenario: Retrying step keeps its counter
- **WHEN** a summary Retrying with 4 failed attempts, last failed at 08:00 the day before, runs at 10:00 while Ollama can't be connected to
- **THEN** the summary is Retrying with 4 failed attempts, last failed at 08:00 the day before, and its task is pending to run at 16:00

#### Scenario: Last attempt not spent
- **WHEN** a categorization Retrying with 10 failed attempts runs while Ollama can't be connected to
- **THEN** it is Retrying with 10 failed attempts, no `failed` verdict is written, and it is re-queued 6 hours later

#### Scenario: Model not installed
- **WHEN** a summary runs and the saved model isn't among those Ollama lists
- **THEN** the summary keeps its status and failed attempts and is re-queued 6 hours later

#### Scenario: Slow answer still counts
- **WHEN** Ollama accepts the connection for a summary call but doesn't answer within the call's timeout
- **THEN** the call is rejected and the summary is Retrying with one more failed attempt

#### Scenario: Ollama back
- **WHEN** a summary that waited for Ollama runs again 6 hours later and Ollama answers usably
- **THEN** the summary is Done with no failed attempts

### Requirement: Retrying Steps Are Re-queued
After a failed attempt that leaves the step Retrying, the step's task SHALL be re-queued to run when the next attempt is due, without failing the task. A step's task that runs before its next attempt is due SHALL do nothing and be re-queued for that time. A video sync SHALL schedule a Retrying step whose next attempt is due and that has no task pending or running, so a lost task is recovered.

#### Scenario: Download re-queued
- **WHEN** a video's download fails at 10:00 for the first time
- **THEN** the download task is pending to run at 10:10, with the failure reason as its last error

#### Scenario: Task runs early
- **WHEN** a download task runs for a video whose download is Retrying with its next attempt due in 2 hours
- **THEN** nothing is downloaded and the task is pending to run in 2 hours

#### Scenario: Retry lost
- **WHEN** a video sync runs for a video whose summary is Retrying and due, with no summary pending or running
- **THEN** a summary of the video is scheduled to run immediately

### Requirement: Giving Up After A Week
A step SHALL become Failed after its 11th consecutive failed attempt (about 7 days after the first), and SHALL then never be attempted again until the user retries the video. When the categorization becomes Failed, every category still pending for the video SHALL get a `failed` verdict. A categorization MAY give a category a `failed` verdict before then, when a later attempt would only repeat the same calls (see `video-categorization`'s "Retry Without Thinking"); with nothing else left to retry it SHALL be Failed in that run.

#### Scenario: Download gives up
- **WHEN** a video's download fails for the 11th consecutive time
- **THEN** the video's download is Failed, no download of it is pending, and no video sync schedules one

#### Scenario: Summary gives up while Ollama is down
- **WHEN** a video's summary with 10 consecutive failed attempts runs while Ollama is down
- **THEN** it doesn't give up: its summary stays Retrying with 10 failed attempts and is re-queued 6 hours later (see "Waiting For Ollama")

#### Scenario: Summary gives up
- **WHEN** a video's summary call is rejected for the 11th consecutive time
- **THEN** its summary is Failed and no summary of it is scheduled again

#### Scenario: Categorization gives up
- **WHEN** a video's categorization fails for the 11th consecutive time with "Tech" and "Sicilian" pending
- **THEN** its categorization is Failed and the video has `failed` verdicts for "Tech" and "Sicilian"

#### Scenario: Categorization gives up at once
- **WHEN** on its first attempt a video's only pending category, "Tech", stops at the output limit with thinking and is rejected again without thinking
- **THEN** its categorization is Failed with one failed attempt, the video has a `failed` verdict for "Tech", and no categorization of it is scheduled again

### Requirement: Retry A Video
The system SHALL let a client retry a video (`POST /api/videos/{id}/retry`, answering 204). Retrying SHALL make every Retrying or Failed step of the video Pending with no failed attempts, delete the video's `failed` verdicts, and leave Pending, In Progress, Done and Unavailable steps and any data already obtained unchanged. The system SHALL then sync the video, so each step it needs is scheduled to run now, and a step whose retry was queued for later is moved to now. Retrying a video with nothing Retrying or Failed SHALL change nothing but still sync it. Retrying an unknown video SHALL answer 404.

#### Scenario: Retry a failed download
- **WHEN** a client retries a video whose download is Failed with 11 failed attempts
- **THEN** its download is Pending with no failed attempts and a download of it is scheduled to run now

#### Scenario: Retry a downloaded video whose AI failed
- **WHEN** a client retries a Downloaded video whose subtitles are Done, whose summary is Failed and which has a `failed` verdict for "Tech"
- **THEN** its file and subtitles are unchanged, its summary is Pending, "Tech" is pending again, and a summary is scheduled

#### Scenario: Retry a step waiting for its next attempt
- **WHEN** a client retries a video whose download is Retrying with its next attempt due in 6 hours
- **THEN** its download is Pending with no failed attempts and its queued download runs now

#### Scenario: Unavailable steps are kept
- **WHEN** a client retries a video whose subtitles are Unavailable and whose chapters are Failed
- **THEN** its subtitles stay Unavailable and its chapters are Pending

#### Scenario: Unknown video
- **WHEN** a client retries a video id that doesn't exist
- **THEN** the request answers 404
