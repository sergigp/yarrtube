use crate::domain::services::{VideoFileDeleter, VideoFileDeleterApi};
use crate::domain::task::Task;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
use std::path::Path;

/// Deletes one video's downloaded file, scheduled by
/// `subscribers::delete_video_file_on_video_removed_from_playlist`/
/// `..._from_channel` whenever a downloaded video is removed from its
/// tracked container.
pub struct DeleteVideoFileTask {
    video_file_deleter: VideoFileDeleter,
}

impl DeleteVideoFileTask {
    pub fn new(video_file_deleter: VideoFileDeleter) -> Self {
        Self { video_file_deleter }
    }
}

impl TaskHandler for DeleteVideoFileTask {
    fn handle(&self, task: &Task, _is_last_attempt: bool) -> anyhow::Result<()> {
        let Task::DeleteVideoFile {
            filename,
            thumbnail_filename,
            subtitle_filename,
            output_dir,
        } = task
        else {
            return Err(unexpected_task(task));
        };
        self.video_file_deleter.delete_video_file(
            filename.clone(),
            thumbnail_filename.clone(),
            subtitle_filename.clone(),
            Path::new(output_dir),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn it_should_delete_the_video_file() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(true)));
        let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &task_for(Some("My Video.mp4"), None));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("My Video.mp4")]
        );
    }

    #[test]
    fn it_should_skip_if_video_has_no_filename() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(true)));
        let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &task_for(None, None));

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_fail_if_handed_another_task() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(true)));
        let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &Task::UpdateYtdlp);

        assert_eq!(
            result,
            Err("unexpected update_ytdlp task for this handler".to_string())
        );
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_if_video_file_not_found() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(false)));
        let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &task_for(Some("My Video.mp4"), None));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("My Video.mp4")]
        );
    }

    #[test]
    fn it_should_delete_the_video_and_thumbnail_files() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(true)));
        let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &task_for(Some("My Video.mp4"), Some("My Video.jpg")));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("My Video.mp4"), deleted("My Video.jpg")]
        );
    }

    #[test]
    fn it_should_skip_thumbnail_if_none_recorded() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(true)));
        let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &task_for(Some("My Video.mp4"), None));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("My Video.mp4")]
        );
    }

    #[test]
    fn it_should_skip_thumbnail_if_file_not_found() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(false)));
        let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &task_for(None, Some("My Video.jpg")));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("My Video.jpg")]
        );
    }

    #[test]
    fn it_should_delete_an_episodes_media_thumbnail_and_nfo() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(
            &task,
            &task_for(
                Some("Season 2026/S2026E031501 - My Video.mp4"),
                Some("Season 2026/S2026E031501 - My Video.jpg"),
            ),
        );

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![
                deleted("Season 2026/S2026E031501 - My Video.mp4"),
                deleted("Season 2026/S2026E031501 - My Video.nfo"),
                deleted("Season 2026/S2026E031501 - My Video.jpg"),
            ]
        );
    }

    #[test]
    fn it_should_delete_an_episodes_subtitle_file() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(
            &task,
            &Task::DeleteVideoFile {
                filename: Some("Season 2026/S2026E031501 - My Video.mp4".to_string()),
                thumbnail_filename: None,
                subtitle_filename: Some("Season 2026/S2026E031501 - My Video.en.srt".to_string()),
                output_dir: "/videos/my-playlist".to_string(),
            },
        );

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![
                deleted("Season 2026/S2026E031501 - My Video.mp4"),
                deleted("Season 2026/S2026E031501 - My Video.nfo"),
                deleted("Season 2026/S2026E031501 - My Video.en.srt"),
            ]
        );
    }

    #[test]
    fn it_should_delete_a_legacy_per_video_folder() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(
            &task,
            &task_for(Some("My Video/My Video.mp4"), Some("My Video/My Video.jpg")),
        );

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("My Video"), deleted("My Video")]
        );
    }

    fn task_for(filename: Option<&str>, thumbnail_filename: Option<&str>) -> Task {
        Task::DeleteVideoFile {
            filename: filename.map(str::to_string),
            thumbnail_filename: thumbnail_filename.map(str::to_string),
            subtitle_filename: None,
            output_dir: "/videos/my-playlist".to_string(),
        }
    }

    /// The `(output_dir, entry)` pair the fake records for one delete call.
    fn deleted(entry: &str) -> (PathBuf, String) {
        (PathBuf::from("/videos/my-playlist"), entry.to_string())
    }

    fn run(task: &DeleteVideoFileTask, queued: &Task) -> Result<(), String> {
        task.handle(queued, false).map_err(|e| e.to_string())
    }
}
