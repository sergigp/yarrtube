use crate::domain::shared::Quality;
use anyhow::{Result, anyhow};
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn ensure_output_dir(output_path: &Path) -> Result<()> {
    std::fs::create_dir_all(output_path)
        .map_err(|e| anyhow!("Failed to create output directory {output_path:?}: {e}"))
}

/// Maps a playlist's `quality` tier to the `yt-dlp` args that select its
/// resolution cap while softly preferring mp4/h264/aac (falling back to the
/// best available stream instead of hard-failing when no such stream
/// exists) — see design.md's "yt-dlp selector shape per tier" decision.
pub fn args_for_quality(quality: Quality) -> Vec<String> {
    let format = match quality {
        Quality::High => "bv*+ba/b".to_string(),
        Quality::Mid => "bv*[height<=720]+ba/b[height<=720]".to_string(),
        Quality::Low => "bv*[height<=480]+ba/b[height<=480]".to_string(),
    };
    vec![
        "-f".to_string(),
        format,
        "-S".to_string(),
        "codec:h264:aac,ext:mp4:m4a".to_string(),
        "--merge-output-format".to_string(),
        "mp4".to_string(),
    ]
}

/// Runs `yt-dlp <args> <video_url>` in `output_path`, saving it under
/// `desired_filename` (extension chosen by `yt-dlp`). If a file with that
/// stem already exists in `output_path`, `video_id` is appended to
/// disambiguate. Asks `yt-dlp` to print the exact filename it saved via
/// `--print after_move:filename`, in quiet mode so that's the only line on
/// stdout. Returns `Ok(Some(filename))` on a successful download,
/// `Ok(None)` for a clean `yt-dlp` failure (non-zero exit). Returns `Err`
/// only for a systemic problem: `yt-dlp` missing from `PATH`, or a
/// successful exit that didn't print a parseable filename.
pub fn download_video(
    video_url: &str,
    desired_filename: &str,
    video_id: &str,
    quality: Quality,
    output_path: &Path,
) -> Result<Option<String>> {
    let base = resolve_collision(output_path, desired_filename, video_id);
    let output_template = format!("{base}.%(ext)s");
    let mut args = args_for_quality(quality);
    args.extend([
        "--quiet".to_string(),
        "--no-warnings".to_string(),
        "--print".to_string(),
        "after_move:filename".to_string(),
    ]);
    println!(
        "Running: yt-dlp {} {video_url} -o \"{output_template}\" (in {})",
        args.join(" "),
        output_path.display()
    );
    let output = match Command::new("yt-dlp")
        .args(&args)
        .arg(video_url)
        .arg("-o")
        .arg(&output_template)
        .current_dir(output_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(output) => output,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(anyhow!(
                "`yt-dlp` was not found on PATH. Install yt-dlp and make sure it is available before running yarrtube."
            ));
        }
        Err(e) => return Err(anyhow!("Failed to run yt-dlp for {video_url}: {e}")),
    };

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    match stdout.lines().next_back().map(str::trim) {
        Some(filename) if !filename.is_empty() => Ok(Some(filename.to_string())),
        _ => Err(anyhow!(
            "yt-dlp exited successfully but did not print an output filename for {video_url}"
        )),
    }
}

/// Returns `desired_filename` unchanged, unless a file whose stem already
/// matches it exists in `output_path` — the extension isn't known until
/// `yt-dlp` picks a format, so the check is by stem, not exact path.
fn resolve_collision(output_path: &Path, desired_filename: &str, video_id: &str) -> String {
    let collides = std::fs::read_dir(output_path)
        .map(|entries| {
            entries.flatten().any(|entry| {
                entry.path().file_stem().and_then(|s| s.to_str()) == Some(desired_filename)
            })
        })
        .unwrap_or(false);

    if collides {
        format!("{desired_filename} [{video_id}]")
    } else {
        desired_filename.to_string()
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
    /// the duration of the guard, restoring the original `PATH` on drop. The
    /// script also records the arguments it was invoked with, retrievable
    /// via `captured_args`.
    #[cfg(unix)]
    pub(crate) struct FakeYtDlpOnPath {
        _lock: std::sync::MutexGuard<'static, ()>,
        original_path: String,
        _bin_dir: PathBuf,
        captured_args_path: PathBuf,
    }

    /// The filename `with_exit_code` prints on stdout when it succeeds,
    /// standing in for yt-dlp's `--print after_move:filename` output.
    pub(crate) const DEFAULT_PRINTED_FILENAME: &str = "fake-output.mp4";

    #[cfg(unix)]
    impl FakeYtDlpOnPath {
        pub(crate) fn with_exit_code(exit_code: i32) -> Self {
            Self::with_exit_code_and_printed_filename(exit_code, DEFAULT_PRINTED_FILENAME)
        }

        pub(crate) fn with_exit_code_and_printed_filename(
            exit_code: i32,
            printed_filename: &str,
        ) -> Self {
            use std::os::unix::fs::PermissionsExt;

            let lock = path_mutex()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            let bin_dir = unique_temp_dir("fake-ytdlp-bin");
            let script_path = bin_dir.join("yt-dlp");
            let captured_args_path = bin_dir.join("captured-args");
            let print_stmt = if exit_code == 0 {
                format!("printf '%s\\n' '{printed_filename}'\n")
            } else {
                String::new()
            };
            fs::write(
                &script_path,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"{}\"\n{print_stmt}exit {exit_code}\n",
                    captured_args_path.display()
                ),
            )
            .unwrap();
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
                captured_args_path,
            }
        }

        /// The arguments the fake `yt-dlp` was last invoked with, one per line.
        pub(crate) fn captured_args(&self) -> Vec<String> {
            fs::read_to_string(&self.captured_args_path)
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect()
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
    fn it_should_return_the_printed_filename_on_success() {
        use test_support::{DEFAULT_PRINTED_FILENAME, FakeYtDlpOnPath, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output");
        let _guard = FakeYtDlpOnPath::with_exit_code(0);

        let filename = download_video(
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap();

        assert_eq!(filename, Some(DEFAULT_PRINTED_FILENAME.to_string()));
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_none_on_a_clean_failed_exit() {
        use test_support::{FakeYtDlpOnPath, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output");
        let _guard = FakeYtDlpOnPath::with_exit_code(1);

        let filename = download_video(
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap();

        assert_eq!(filename, None);
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_pass_the_desired_filename_as_the_output_template_when_there_is_no_collision() {
        use test_support::{FakeYtDlpOnPath, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-no-collision");
        let guard = FakeYtDlpOnPath::with_exit_code(0);

        download_video(
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap();

        let mut expected = args_for_quality(Quality::High);
        expected.extend([
            "--quiet".to_string(),
            "--no-warnings".to_string(),
            "--print".to_string(),
            "after_move:filename".to_string(),
            "https://example.com/video".to_string(),
            "-o".to_string(),
            "My Video.%(ext)s".to_string(),
        ]);
        assert_eq!(guard.captured_args(), expected);
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_append_the_video_id_to_the_output_template_on_collision() {
        use test_support::{FakeYtDlpOnPath, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-collision");
        std::fs::write(output_dir.join("My Video.mp4"), b"").unwrap();
        let guard = FakeYtDlpOnPath::with_exit_code(0);

        download_video(
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap();

        let mut expected = args_for_quality(Quality::High);
        expected.extend([
            "--quiet".to_string(),
            "--no-warnings".to_string(),
            "--print".to_string(),
            "after_move:filename".to_string(),
            "https://example.com/video".to_string(),
            "-o".to_string(),
            "My Video [vid1].%(ext)s".to_string(),
        ]);
        assert_eq!(guard.captured_args(), expected);
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_error_when_a_successful_exit_prints_no_filename() {
        use test_support::{FakeYtDlpOnPath, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-no-print");
        let _guard = FakeYtDlpOnPath::with_exit_code_and_printed_filename(0, "");

        let result = download_video(
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        );

        assert!(result.is_err());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn it_should_map_high_quality_to_an_uncapped_selector() {
        assert_eq!(
            args_for_quality(Quality::High),
            vec![
                "-f".to_string(),
                "bv*+ba/b".to_string(),
                "-S".to_string(),
                "codec:h264:aac,ext:mp4:m4a".to_string(),
                "--merge-output-format".to_string(),
                "mp4".to_string(),
            ]
        );
    }

    #[test]
    fn it_should_map_mid_quality_to_a_720p_capped_selector() {
        assert_eq!(
            args_for_quality(Quality::Mid),
            vec![
                "-f".to_string(),
                "bv*[height<=720]+ba/b[height<=720]".to_string(),
                "-S".to_string(),
                "codec:h264:aac,ext:mp4:m4a".to_string(),
                "--merge-output-format".to_string(),
                "mp4".to_string(),
            ]
        );
    }

    #[test]
    fn it_should_map_low_quality_to_a_480p_capped_selector() {
        assert_eq!(
            args_for_quality(Quality::Low),
            vec![
                "-f".to_string(),
                "bv*[height<=480]+ba/b[height<=480]".to_string(),
                "-S".to_string(),
                "codec:h264:aac,ext:mp4:m4a".to_string(),
                "--merge-output-format".to_string(),
                "mp4".to_string(),
            ]
        );
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
