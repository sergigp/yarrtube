use crate::domain::channel::ChannelHandle;
use crate::domain::services::VideoFileDeleter;
use crate::domain::task::Task;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Recursively deletes a deleted channel's output directory, scheduled by
/// `subscribers::delete_channel_files_on_channel_deleted` whenever a channel
/// is deleted.
pub struct DeleteChannelFilesTask {
    video_file_deleter: VideoFileDeleter,
}

impl DeleteChannelFilesTask {
    pub fn new(video_file_deleter: VideoFileDeleter) -> Self {
        Self { video_file_deleter }
    }
}

impl TaskHandler for DeleteChannelFilesTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let (channel_id, path) = Task::decode_delete_channel_files_payload(payload)?;
        let Ok(channel_id) = ChannelHandle::new(channel_id) else {
            return Ok(());
        };
        self.video_file_deleter
            .delete_channel_video_files(channel_id, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn it_should_delete_the_channel_directory() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = DeleteChannelFilesTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &payload_for("@somechannel", "creators/somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from("/videos/creators/somechannel")]
        );
    }

    #[test]
    fn it_should_skip_if_invalid_channel_id_provided() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = DeleteChannelFilesTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &payload_for("", "creators/somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = DeleteChannelFilesTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, "not json");

        assert_eq!(
            result,
            Err(
                "invalid delete_channel_files payload: expected ident at line 1 column 2"
                    .to_string()
            )
        );
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            Vec::<PathBuf>::new()
        );
    }

    fn payload_for(channel_id: &str, path: &str) -> String {
        Task::DeleteChannelFiles {
            channel_id: channel_id.to_string(),
            path: path.to_string(),
        }
        .payload()
        .to_string()
    }

    fn run(task: &DeleteChannelFilesTask, payload: &str) -> Result<(), String> {
        task.handle(payload, false).map_err(|e| e.to_string())
    }
}
