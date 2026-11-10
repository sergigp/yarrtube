use crate::domain::event::ScheduledEvent;
use crate::domain::task::{ScheduledTask, Task, TaskStatus};
use crate::domain::video::Video;
use std::path::Path;

/// Whether anything still writes into a show directory on its own: one of
/// its `videos` not Downloaded, Errored or Excluded, one of `tasks` (the
/// non-completed ones) a task writing into the show folder of one of
/// `videos` (`Task::show_writing_video_id`; a `sync_video` only while
/// running, as it may already have read `show_dir` while a pending one reads
/// the show's path once it runs) or a `delete_video_file` in `show_dir`, or
/// one of `events` (the pending ones) a `VideoDownloaded` in `show_dir`
/// whose `sync_video` task hasn't yet been scheduled. A source with work in
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
            .any(|task| task.task.output_dir().is_some_and(|dir| dir == show_dir))
        || tasks.iter().any(|task| {
            (task.status == TaskStatus::Running || !matches!(task.task, Task::SyncVideo { .. }))
                && task.task.show_writing_video_id().is_some_and(|video_id| {
                    videos.iter().any(|video| video.id.as_str() == video_id)
                })
        })
        || events
            .iter()
            .any(|event| event.output_dir().is_some_and(|dir| dir == show_dir))
}
