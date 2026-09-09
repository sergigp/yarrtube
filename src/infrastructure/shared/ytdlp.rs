use anyhow::{Result, anyhow};
use std::io;
use std::path::Path;
use std::process::Command;

pub fn ensure_output_dir(output_path: &Path) -> Result<()> {
    std::fs::create_dir_all(output_path)
        .map_err(|e| anyhow!("Failed to create output directory {output_path:?}: {e}"))
}

/// Runs `yt-dlp <video_url>` in `output_path`. Returns `Ok(true)`/`Ok(false)` for a
/// completed process based on its exit status. Returns `Err` only when `yt-dlp`
/// itself could not be spawned (e.g. not found on `PATH`) — a systemic setup
/// problem, distinct from a single video failing to download.
pub fn download_video(video_url: &str, output_path: &Path) -> Result<bool> {
    println!("Running: yt-dlp {video_url} (in {})", output_path.display());
    match Command::new("yt-dlp")
        .arg(video_url)
        .current_dir(output_path)
        .status()
    {
        Ok(status) => Ok(status.success()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(anyhow!(
            "`yt-dlp` was not found on PATH. Install yt-dlp and make sure it is available before running yarrtube."
        )),
        Err(e) => Err(anyhow!("Failed to run yt-dlp for {video_url}: {e}")),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::fs;
    use std::path::PathBuf;

    pub(crate) fn unique_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "yarrtube-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// `PATH` is process-global, so every test that mutates it (here and in
    /// `youtube_video_downloader_repository`'s tests, which build on this
    /// same helper) must serialize on this lock — otherwise two such tests
    /// running on different threads clobber each other's `PATH`.
    #[cfg(unix)]
    fn path_mutex() -> &'static std::sync::Mutex<()> {
        static PATH_MUTEX: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        PATH_MUTEX.get_or_init(|| std::sync::Mutex::new(()))
    }

    /// Puts a fake `yt-dlp` script (exiting with `exit_code`) on `PATH` for
    /// the duration of the guard, restoring the original `PATH` on drop.
    #[cfg(unix)]
    pub(crate) struct FakeYtDlpOnPath {
        _lock: std::sync::MutexGuard<'static, ()>,
        original_path: String,
        _bin_dir: PathBuf,
    }

    #[cfg(unix)]
    impl FakeYtDlpOnPath {
        pub(crate) fn with_exit_code(exit_code: i32) -> Self {
            use std::os::unix::fs::PermissionsExt;

            let lock = path_mutex()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            let bin_dir = unique_temp_dir("fake-ytdlp-bin");
            let script_path = bin_dir.join("yt-dlp");
            fs::write(&script_path, format!("#!/bin/sh\nexit {exit_code}\n")).unwrap();
            fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();

            let original_path = std::env::var("PATH").unwrap_or_default();
            let new_path = format!("{}:{original_path}", bin_dir.display());
            unsafe {
                std::env::set_var("PATH", new_path);
            }

            Self {
                _lock: lock,
                original_path,
                _bin_dir: bin_dir,
            }
        }
    }

    #[cfg(unix)]
    impl Drop for FakeYtDlpOnPath {
        fn drop(&mut self) {
            unsafe {
                std::env::set_var("PATH", &self.original_path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn it_should_map_the_yt_dlp_process_exit_status_to_a_bool() {
        use test_support::{FakeYtDlpOnPath, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output");

        {
            let _guard = FakeYtDlpOnPath::with_exit_code(0);
            assert!(download_video("https://example.com/video", &output_dir).unwrap());
        }
        {
            let _guard = FakeYtDlpOnPath::with_exit_code(1);
            assert!(!download_video("https://example.com/video", &output_dir).unwrap());
        }

        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn it_should_create_the_output_directory_when_it_does_not_exist() {
        let base = test_support::unique_temp_dir("ensure-output-dir");
        let output_dir = base.join("nested");
        assert!(!output_dir.exists());

        ensure_output_dir(&output_dir).unwrap();

        assert!(output_dir.is_dir());
        std::fs::remove_dir_all(&base).unwrap();
    }
}
