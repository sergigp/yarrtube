use std::path::Path;

/// Fetches and installs the latest `yt-dlp` binary at `target_path`. Backs
/// `UpdateYtdlpTask`'s recurring self-update; the `update-ytdlp` CLI command
/// and the daemon's startup update call `cli::ytdlp_update::update`
/// directly instead, since neither needs to be faked in tests.
pub trait YtdlpUpdater: Send + Sync {
    fn update(&self, target_path: &Path) -> anyhow::Result<()>;
}

pub struct RealYtdlpUpdater;

impl YtdlpUpdater for RealYtdlpUpdater {
    fn update(&self, target_path: &Path) -> anyhow::Result<()> {
        crate::cli::ytdlp_update::update(target_path)
    }
}

#[cfg(test)]
pub struct FakeYtdlpUpdater {
    pub succeeds: bool,
}

#[cfg(test)]
impl YtdlpUpdater for FakeYtdlpUpdater {
    fn update(&self, _target_path: &Path) -> anyhow::Result<()> {
        if self.succeeds {
            Ok(())
        } else {
            Err(anyhow::anyhow!("update failed"))
        }
    }
}
