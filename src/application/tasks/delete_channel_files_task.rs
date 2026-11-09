use crate::domain::channel::ChannelHandle;
use crate::domain::services::{VideoFileDeleter, VideoFileDeleterApi};
use crate::domain::task::Task;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};

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
    fn handle(&self, task: &Task, _is_last_attempt: bool) -> anyhow::Result<()> {
        let Task::DeleteChannelFiles { channel_id, path } = task else {
            return Err(unexpected_task(task));
        };
        let Ok(channel_id) = ChannelHandle::new(channel_id.clone()) else {
            return Ok(());
        };
        self.video_file_deleter
            .delete_channel_video_files(channel_id, path.clone())
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

        let result = run(&task, &task_for("@somechannel", "creators/somechannel"));

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

        let result = run(&task, &task_for("", "creators/somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn it_should_fail_if_handed_another_task() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = DeleteChannelFilesTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &Task::UpdateYtdlp);

        assert_eq!(
            result,
            Err("unexpected update_ytdlp task for this handler".to_string())
        );
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            Vec::<PathBuf>::new()
        );
    }

    fn task_for(channel_id: &str, path: &str) -> Task {
        Task::DeleteChannelFiles {
            channel_id: channel_id.to_string(),
            path: path.to_string(),
        }
    }

    fn run(task: &DeleteChannelFilesTask, queued: &Task) -> Result<(), String> {
        task.handle(queued, false).map_err(|e| e.to_string())
    }
}
