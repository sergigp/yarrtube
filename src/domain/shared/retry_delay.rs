const RETRY_DELAY_MULTIPLIER: i64 = 3;

/// Retry delay grows geometrically with the post-increment retry count:
/// `base_delay_seconds * 3^retries`. With the default base delay, the first
/// 4 failures of a task or a domain event (the 5th dead-letters instead of
/// retrying) span about 5 hours total end-to-end (~7.5min, ~22.5min, ~1.1h,
/// ~3.4h between successive attempts) — wide enough for the hourly `yt-dlp`
/// self-update loop to realistically fix a systemic problem, or for a briefly
/// unreachable dependency (Plex) to come back, before the attempts are
/// exhausted. Only `base_delay_seconds` is configurable; the multiplier stays
/// fixed.
pub fn retry_delay_seconds(retries: i64, base_delay_seconds: i64) -> i64 {
    base_delay_seconds * RETRY_DELAY_MULTIPLIER.pow(retries as u32)
}
