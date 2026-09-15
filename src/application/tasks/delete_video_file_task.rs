use crate::domain::services::VideoFileDeleter;
use crate::domain::task::Task;
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
        let (filename, output_dir) = Task::decode_delete_video_file_payload(payload)?;
        self.video_file_deleter
            .delete_video_file(filename, Path::new(&output_dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;

    use std::sync::Arc;

    fn payload_for(filename: Option<&str>) -> String {
        Task::DeleteVideoFile {
            filename: filename.map(str::to_string),
            output_dir: "/videos/my-playlist".to_string(),
        }
        .payload()
        .to_string()
    }

    fn handler(
        video_file_repository: FakeVideoFileRepository,
    ) -> (DeleteVideoFileTask, Arc<FakeVideoFileRepository>) {
        let video_file_repository = Arc::new(video_file_repository);
        let video_file_deleter = VideoFileDeleter::new(video_file_repository.clone(), "/videos");

        (
            DeleteVideoFileTask::new(video_file_deleter),
            video_file_repository,
        )
    }

    #[test]
    fn it_should_delete_the_video_file() {
        let (handler, video_file_repository) = handler(FakeVideoFileRepository::new(Ok(true)));

        handler
            .handle(&payload_for(Some("My Video.mp4")), false)
            .unwrap();

        assert_eq!(video_file_repository.deleted_calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn it_should_no_op_when_the_video_has_no_recorded_filename() {
        let (handler, video_file_repository) = handler(FakeVideoFileRepository::new(Ok(true)));

        handler.handle(&payload_for(None), false).unwrap();

        assert!(
            video_file_repository
                .deleted_calls
                .lock()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let (handler, _video_file_repository) = handler(FakeVideoFileRepository::new(Ok(true)));

        assert!(handler.handle("not json", false).is_err());
    }

    #[test]
    fn it_should_no_op_when_no_matching_file_is_found() {
        let (handler, _video_file_repository) = handler(FakeVideoFileRepository::new(Ok(false)));

        let result = handler.handle(&payload_for(Some("My Video.mp4")), false);

        assert!(result.is_ok());
    }
}
