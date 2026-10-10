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
    fn write_show_nfo(&self, _show_dir: &Path, _nfo: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn copy_poster(&self, _show_dir: &Path, _source: &Path) -> anyhow::Result<()> {
        Ok(())
    }
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
