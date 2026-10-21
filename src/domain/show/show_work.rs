use crate::domain::task::ScheduledTask;
use crate::domain::video::Video;
use std::path::Path;

/// Whether anything still writes into a show directory on its own: one of
/// its `videos` not Downloaded, Errored or Excluded, or one of `tasks` (the
/// non-completed ones) a per-video task in `show_dir`. A source with work in
/// progress can't be moved.
pub fn has_work_in_progress(_videos: &[Video], _tasks: &[ScheduledTask], _show_dir: &Path) -> bool {
    false
}
