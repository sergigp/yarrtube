use crate::domain::video::{SHOW_NFO_FILENAME, SHOW_POSTER_FILENAME};
use std::path::Path;

/// Writes a show's files at the root of its playlist's or channel's
/// directory: `tvshow.nfo` and `poster.jpg`. Injected into `ShowFileWriter`.
pub trait ShowFileRepository: Send + Sync {
    /// Writes `nfo` as `show_dir/tvshow.nfo`, creating `show_dir` first and
    /// overwriting an existing file.
    fn write_show_nfo(&self, show_dir: &Path, nfo: &str) -> anyhow::Result<()>;

    /// Copies `source` to `show_dir/poster.jpg`, overwriting an existing one.
    fn copy_poster(&self, show_dir: &Path, source: &Path) -> anyhow::Result<()>;
}

pub struct FilesystemShowFileRepository;

impl ShowFileRepository for FilesystemShowFileRepository {
    fn write_show_nfo(&self, show_dir: &Path, nfo: &str) -> anyhow::Result<()> {
        create_show_dir(show_dir)?;
        let path = show_dir.join(SHOW_NFO_FILENAME);
        std::fs::write(&path, nfo)
            .inspect_err(|e| tracing::error!(path = ?path, error = %e, "failed to write show NFO"))
            .map_err(|e| anyhow::anyhow!("failed to write {path:?}: {e}"))
    }

    fn copy_poster(&self, show_dir: &Path, source: &Path) -> anyhow::Result<()> {
        create_show_dir(show_dir)?;
        let path = show_dir.join(SHOW_POSTER_FILENAME);
        std::fs::copy(source, &path)
            .map(|_| ())
            .inspect_err(|e| tracing::error!(source = ?source, path = ?path, error = %e, "failed to copy show poster"))
            .map_err(|e| anyhow::anyhow!("failed to copy {source:?} to {path:?}: {e}"))
    }
}

fn create_show_dir(show_dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(show_dir)
        .inspect_err(
            |e| tracing::error!(dir = ?show_dir, error = %e, "failed to create show directory"),
        )
        .map_err(|e| anyhow::anyhow!("failed to create {show_dir:?}: {e}"))
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeShowFileRepository {
    /// Every `tvshow.nfo` written, in order, with its show directory.
    pub(crate) show_nfos: std::sync::Mutex<Vec<(std::path::PathBuf, String)>>,
    /// Every poster copied, in order: show directory and source path.
    pub(crate) posters: std::sync::Mutex<Vec<(std::path::PathBuf, std::path::PathBuf)>>,
}

#[cfg(test)]
impl ShowFileRepository for FakeShowFileRepository {
    fn write_show_nfo(&self, show_dir: &Path, nfo: &str) -> anyhow::Result<()> {
        self.show_nfos
            .lock()
            .unwrap()
            .push((show_dir.to_path_buf(), nfo.to_string()));
        Ok(())
    }

    fn copy_poster(&self, show_dir: &Path, source: &Path) -> anyhow::Result<()> {
        self.posters
            .lock()
            .unwrap()
            .push((show_dir.to_path_buf(), source.to_path_buf()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::ytdlp::test_support::unique_temp_dir;

    #[test]
    fn it_should_write_the_show_nfo_and_copy_the_poster() {
        let dir = unique_temp_dir("show-file-repository-write");
        let show_dir = dir.join("channels/somechannel");
        std::fs::write(dir.join("@somechannel.jpg"), b"avatar").unwrap();

        FilesystemShowFileRepository
            .write_show_nfo(&show_dir, "<tvshow/>")
            .unwrap();
        FilesystemShowFileRepository
            .copy_poster(&show_dir, &dir.join("@somechannel.jpg"))
            .unwrap();

        assert_eq!(
            [
                std::fs::read(show_dir.join("tvshow.nfo")).ok(),
                std::fs::read(show_dir.join("poster.jpg")).ok(),
            ],
            [Some(b"<tvshow/>".to_vec()), Some(b"avatar".to_vec())]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_overwrite_an_existing_poster() {
        let dir = unique_temp_dir("show-file-repository-overwrite");
        let show_dir = dir.join("channels/somechannel");
        std::fs::create_dir_all(&show_dir).unwrap();
        std::fs::write(show_dir.join("poster.jpg"), b"old avatar").unwrap();
        std::fs::write(dir.join("@somechannel.jpg"), b"new avatar").unwrap();

        FilesystemShowFileRepository
            .copy_poster(&show_dir, &dir.join("@somechannel.jpg"))
            .unwrap();

        assert_eq!(
            std::fs::read(show_dir.join("poster.jpg")).ok(),
            Some(b"new avatar".to_vec())
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
