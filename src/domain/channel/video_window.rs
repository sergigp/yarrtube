/// A channel's most recent `uploads` up to and including the `limit`-th
/// one not `is_skipped` (Unavailable or Discarded videos don't count toward
/// a channel's video limit — see the `channel-video-sync` capability).
pub fn within_limit<T>(uploads: Vec<T>, is_skipped: impl Fn(&T) -> bool, limit: usize) -> Vec<T> {
    uploads
        .into_iter()
        .scan(0, |counted, upload| {
            if *counted >= limit {
                return None;
            }
            if !is_skipped(&upload) {
                *counted += 1;
            }
            Some(upload)
        })
        .collect()
}
