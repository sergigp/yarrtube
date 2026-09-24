use crate::domain::task::Task;
use crate::domain::video::{VideoFileDeleter, VideoFileDeleterApi};
use crate::infrastructure::repositories::task_handler::TaskHandler;
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
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let (filename, thumbnail_filename, output_dir) =
            Task::decode_delete_video_file_payload(payload)?;
        self.video_file_deleter.delete_video_file(
            filename,
            thumbnail_filename,
            Path::new(&output_dir),
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

        let result = run(&task, &payload_for(Some("My Video.mp4"), None));

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

        let result = run(&task, &payload_for(None, None));

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(true)));
        let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, "not json");

        assert_eq!(
            result,
            Err("invalid delete_video_file payload: expected ident at line 1 column 2".to_string())
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

        let result = run(&task, &payload_for(Some("My Video.mp4"), None));

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

        let result = run(
            &task,
            &payload_for(Some("My Video.mp4"), Some("My Video.jpg")),
        );

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

        let result = run(&task, &payload_for(Some("My Video.mp4"), None));

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

        let result = run(&task, &payload_for(None, Some("My Video.jpg")));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("My Video.jpg")]
        );
    }

    fn payload_for(filename: Option<&str>, thumbnail_filename: Option<&str>) -> String {
        Task::DeleteVideoFile {
            filename: filename.map(str::to_string),
            thumbnail_filename: thumbnail_filename.map(str::to_string),
            output_dir: "/videos/my-playlist".to_string(),
        }
        .payload()
        .to_string()
    }

    /// The `(output_dir, entry)` pair the fake records for one delete call.
    fn deleted(entry: &str) -> (PathBuf, String) {
        (PathBuf::from("/videos/my-playlist"), entry.to_string())
    }

    fn run(task: &DeleteVideoFileTask, payload: &str) -> Result<(), String> {
        task.handle(payload, false).map_err(|e| e.to_string())
    }
}
