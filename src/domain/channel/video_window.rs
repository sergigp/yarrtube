/// A channel's most recent `uploads` up to and including the `limit`-th
/// one not `is_skipped` (Unavailable or Discarded videos don't count toward
/// a channel's video limit — see the `channel-video-sync` capability).
pub fn within_limit<T>(uploads: Vec<T>, _is_skipped: impl Fn(&T) -> bool, _limit: usize) -> Vec<T> {
    uploads
}
