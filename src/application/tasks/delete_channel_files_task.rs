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
    use std::sync::Arc;

    fn payload_for(channel_id: &str, path: &str) -> String {
        Task::DeleteChannelFiles {
            channel_id: channel_id.to_string(),
            path: path.to_string(),
        }
        .payload()
        .to_string()
    }

    fn handler() -> (DeleteChannelFilesTask, Arc<FakeVideoFileRepository>) {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_file_deleter = VideoFileDeleter::new(video_file_repository.clone(), "/videos");

        (
            DeleteChannelFilesTask::new(video_file_deleter),
            video_file_repository,
        )
    }

    #[test]
    fn it_should_recursively_delete_the_channel_output_directory() {
        let (handler, video_file_repository) = handler();

        handler
            .handle(&payload_for("@somechannel", "creators/somechannel"), false)
            .unwrap();

        let deleted = video_file_repository.deleted_dirs.lock().unwrap();
        assert_eq!(
            *deleted,
            vec![std::path::PathBuf::from("/videos/creators/somechannel")]
        );
    }

    #[test]
    fn it_should_no_op_when_the_payload_channel_id_is_invalid() {
        let (handler, video_file_repository) = handler();

        handler
            .handle(&payload_for("", "creators/somechannel"), false)
            .unwrap();

        assert!(
            video_file_repository
                .deleted_dirs
                .lock()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let (handler, _video_file_repository) = handler();

        assert!(handler.handle("not json", false).is_err());
    }
}
