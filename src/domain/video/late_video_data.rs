//! The `late-video-data` policy's timings, shared by `Subtitles`,
//! `Chapters` and `DownloadedQuality`.

use chrono::Duration;

/// How long after its publish time a video counts as young: its late data
/// may still show up.
pub const YOUNG_FOR: Duration = Duration::days(7);
/// How long Pending data waits after a conclusive check before it is due
/// again.
pub const RECHECK_AFTER: Duration = Duration::hours(24);
