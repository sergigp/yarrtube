use crate::domain::event::ScheduledEvent;
use crate::domain::task::ScheduledTask;
use crate::domain::video::Video;
use std::path::Path;

/// Whether anything still writes into a show directory on its own: one of
/// its `videos` not Downloaded, Errored or Excluded, one of `tasks` (the
/// non-completed ones) a per-video task in `show_dir`, or one of `events`
/// (the pending ones) a `VideoDownloaded` in `show_dir` whose subscriber
/// hasn't yet scheduled its subtitles fetch there. A source with work in
/// progress can't be moved.
pub fn has_work_in_progress(
    videos: &[Video],
    tasks: &[ScheduledTask],
    events: &[ScheduledEvent],
    show_dir: &Path,
) -> bool {
    let show_dir = show_dir.to_string_lossy();
    videos.iter().any(|video| !video.is_download_settled())
        || tasks
            .iter()
            .any(|task| task.output_dir().is_some_and(|dir| dir == show_dir))
        || events
            .iter()
            .any(|event| event.output_dir().is_some_and(|dir| dir == show_dir))
}
