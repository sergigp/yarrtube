use crate::domain::shared::Quality;
use anyhow::{Result, anyhow};
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn ensure_output_dir(output_path: &Path) -> Result<()> {
    std::fs::create_dir_all(output_path)
        .map_err(|e| anyhow!("Failed to create output directory {output_path:?}: {e}"))
}

/// Runs `cmd`, retrying briefly on a transient "text file busy" error. The
/// `yt-dlp` binary can be mid-replacement by a concurrent `update-ytdlp` run
/// (which swaps it in via an atomic rename, but the exec of the old inode
/// can still transiently race the kernel's write-lock teardown), and the
/// same race shows up in tests that write a fake binary and exec it right
/// away.
fn output_retrying_busy(cmd: &mut Command) -> io::Result<std::process::Output> {
    const MAX_ATTEMPTS: u32 = 5;
    const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(20);

    let mut attempt = 0;
    loop {
        attempt += 1;
        match cmd.output() {
            Err(e) if e.kind() == io::ErrorKind::ExecutableFileBusy && attempt < MAX_ATTEMPTS => {
                std::thread::sleep(RETRY_DELAY);
            }
            result => return result,
        }
    }
}

/// Maps a playlist's `quality` tier to the `yt-dlp` args that select its
/// resolution cap while softly preferring mp4/h264/aac (falling back to the
/// best available stream instead of hard-failing when no such stream
/// exists) — see design.md's "yt-dlp selector shape per tier" decision.
///
/// `--merge-output-format mp4` only takes effect when yt-dlp actually merges
/// separate video/audio streams; when the `b` fallback in the selector picks
/// a single already-muxed stream instead (e.g. because YouTube didn't offer
/// separate h264/aac DASH tracks for that video), no merge happens and the
/// file would otherwise keep its source container (webm/mkv), which browser
/// `<video>` players reject. `--remux-video mp4` repackages the container
/// into mp4 whenever it isn't already, independent of whether a merge
/// happened, so every download consistently lands as `.mp4`.
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
        "--remux-video".to_string(),
        "mp4".to_string(),
    ]
}

/// A video's `yt-dlp`-reported download result: the video's own output
/// folder name (relative to the container's output directory), the exact
/// filename it saved inside that folder, and its duration in whole seconds
/// when one could be determined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedVideo {
    pub folder: String,
    pub filename: String,
    pub duration_seconds: Option<i64>,
}

/// Parses `yt-dlp`'s `--print %(duration)s` line into whole seconds.
/// `yt-dlp` prints `NA` for an unknown duration; an empty line or anything
/// else unparsable as a number is likewise treated as unknown rather than
/// failing the download. A fractional value (seen for some extractors) is
/// truncated to whole seconds.
fn parse_duration_seconds(line: &str) -> Option<i64> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("na") {
        return None;
    }
    trimmed.parse::<f64>().ok().map(|seconds| seconds as i64)
}

/// Runs `<ytdlp_path> <args> <video_url>` inside the video's own dedicated
/// folder under `output_path`, named from `desired_filename` (disambiguated
/// on collision by appending `video_id`, the same way a filename collision
/// used to be resolved — see `resolve_folder_collision`), and reused as the
/// saved file's base name too (extension chosen by `yt-dlp`). Creates that
/// folder before invoking `yt-dlp`. This is a pure process-invocation layer
/// with no YouTube API or XML knowledge — the video's `movie.nfo` metadata
/// sidecar is generated separately, after a successful download, by
/// `VideoDownloader` (see the `video-metadata` capability). Asks `yt-dlp` to
/// print its duration followed by the exact filename it saved via
/// `--print %(duration)s --print after_move:filename`, in quiet mode so
/// those are the only two lines on stdout, filename last; since `yt-dlp`
/// runs with the video's folder as its working directory, that printed
/// filename is bare (relative to the video's own folder).
/// Returns `Ok(Some(DownloadedVideo))` on a successful download, `Ok(None)`
/// for a clean `yt-dlp` failure (non-zero exit). Returns `Err` only for a
/// systemic problem: no binary at `ytdlp_path`, a failure creating the
/// video's folder, or a successful exit that didn't print a parseable
/// filename. On any of these non-success outcomes, the video's
/// folder (created up front, before it's known whether the download will
/// succeed) is removed again before returning, so a subsequent retry's
/// folder-collision check finds no stale entry and reuses the exact same
/// folder name — otherwise the retry would see that empty leftover folder
/// as a collision, append this video's own ID, and abandon it as an orphan.
pub fn download_video(
    ytdlp_path: &Path,
    video_url: &str,
    desired_filename: &str,
    video_id: &str,
    quality: Quality,
    output_path: &Path,
) -> Result<Option<DownloadedVideo>> {
    let folder = resolve_folder_collision(output_path, desired_filename, video_id);
    let video_dir = output_path.join(&folder);
    ensure_output_dir(&video_dir)?;

    let output_template = format!("{folder}.%(ext)s");
    let mut args = args_for_quality(quality);
    args.extend([
        "--embed-thumbnail".to_string(),
        "--write-thumbnail".to_string(),
        "--convert-thumbnails".to_string(),
        "jpg".to_string(),
        "--quiet".to_string(),
        "--no-warnings".to_string(),
        "--print".to_string(),
        "%(duration)s".to_string(),
        "--print".to_string(),
        "after_move:filename".to_string(),
    ]);
    println!(
        "Running: {} {} {video_url} -o \"{output_template}\" (in {})",
        ytdlp_path.display(),
        args.join(" "),
        video_dir.display()
    );
    let mut cmd = Command::new(ytdlp_path);
    cmd.args(&args)
        .arg(video_url)
        .arg("-o")
        .arg(&output_template)
        .current_dir(&video_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let output = match output_retrying_busy(&mut cmd) {
        Ok(output) => output,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            remove_video_dir_best_effort(&video_dir);
            return Err(anyhow!(
                "`yt-dlp` was not found at {}. Install yt-dlp there or run update-ytdlp before running yarrtube.",
                ytdlp_path.display()
            ));
        }
        Err(e) => {
            remove_video_dir_best_effort(&video_dir);
            return Err(anyhow!("Failed to run yt-dlp for {video_url}: {e}"));
        }
    };

    if !output.status.success() {
        remove_video_dir_best_effort(&video_dir);
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    let filename = match lines.last().map(|s| s.trim()) {
        Some(filename) if !filename.is_empty() => filename.to_string(),
        _ => {
            remove_video_dir_best_effort(&video_dir);
            return Err(anyhow!(
                "yt-dlp exited successfully but did not print an output filename for {video_url}"
            ));
        }
    };
    let duration_seconds = lines
        .len()
        .checked_sub(2)
        .and_then(|idx| lines.get(idx))
        .copied()
        .and_then(parse_duration_seconds);

    Ok(Some(DownloadedVideo {
        folder,
        filename,
        duration_seconds,
    }))
}

/// Removes a video's folder after a failed/errored download attempt,
/// logging rather than failing the whole operation if that cleanup itself
/// doesn't succeed — the download has already failed, so an inability to
/// tidy up after it shouldn't mask that failure or fail it differently.
fn remove_video_dir_best_effort(video_dir: &Path) {
    if let Err(e) = std::fs::remove_dir_all(video_dir) {
        tracing::warn!(video_dir = ?video_dir, error = %e, "failed to clean up video folder after a failed download attempt");
    }
}

/// One video discovered by `list_channel_videos`, in the order `yt-dlp`
/// printed it (newest first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelVideoEntry {
    pub video_id: String,
    pub title: String,
}

#[derive(serde::Deserialize)]
struct FlatPlaylistEntry {
    id: String,
    title: String,
}

/// Lists a channel's `limit` most recent uploads via
/// `yt-dlp --flat-playlist --print-json -I 1:<limit>` against `channel_url`,
/// parsing one JSON object per stdout line. A clean non-zero exit or empty
/// output means "no videos" (`Ok(vec![])`), not an error — mirroring
/// `download_video`'s error posture. Returns `Err` only for a systemic
/// problem: no binary at `ytdlp_path`, or output that doesn't parse as one
/// JSON object per line.
pub fn list_channel_videos(
    ytdlp_path: &Path,
    channel_url: &str,
    limit: u32,
) -> Result<Vec<ChannelVideoEntry>> {
    let mut cmd = Command::new(ytdlp_path);
    cmd.args([
        "--flat-playlist",
        "--print-json",
        "-I",
        &format!("1:{limit}"),
        "--quiet",
        "--no-warnings",
    ])
    .arg(channel_url)
    .stdout(Stdio::piped())
    .stderr(Stdio::inherit());
    let output = match output_retrying_busy(&mut cmd) {
        Ok(output) => output,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(anyhow!(
                "`yt-dlp` was not found at {}. Install yt-dlp there or run update-ytdlp before running yarrtube.",
                ytdlp_path.display()
            ));
        }
        Err(e) => return Err(anyhow!("Failed to run yt-dlp for {channel_url}: {e}")),
    };

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parsed: FlatPlaylistEntry = serde_json::from_str(line)
                .map_err(|e| anyhow!("failed to parse yt-dlp channel video listing line: {e}"))?;
            Ok(ChannelVideoEntry {
                video_id: parsed.id,
                title: parsed.title,
            })
        })
        .collect()
}

/// Returns `desired_folder` unchanged, unless an entry (file or folder)
/// already exists at exactly that name in `output_path` — a video's own
/// folder has no extension, so the check is an exact path match rather than
/// a by-stem scan.
fn resolve_folder_collision(output_path: &Path, desired_folder: &str, video_id: &str) -> String {
    if output_path.join(desired_folder).exists() {
        format!("{desired_folder} [{video_id}]")
    } else {
        desired_folder.to_string()
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Tests run in parallel threads within one process, so the PID is the
    /// same for all of them; `SystemTime::now()`'s resolution isn't
    /// guaranteed to be finer than the gap between two threads calling this
    /// concurrently, and a collision here previously caused two tests'
    /// fake `yt-dlp` binaries and fixture files to land in the same
    /// directory and stomp on each other. A process-wide counter guarantees
    /// every call gets a distinct suffix regardless of clock resolution.
    static UNIQUE_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    pub(crate) fn unique_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "yarrtube-{name}-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            UNIQUE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A fake `yt-dlp` binary (a shell script exiting with `exit_code`) at
    /// its own path, passed explicitly to `download_video` in tests rather
    /// than relying on `PATH`. The script also records the arguments it was
    /// invoked with, retrievable via `captured_args`.
    #[cfg(unix)]
    pub(crate) struct FakeYtDlp {
        _bin_dir: PathBuf,
        pub(crate) path: PathBuf,
        captured_args_path: PathBuf,
    }

    /// The filename `with_exit_code` prints on stdout when it succeeds,
    /// standing in for yt-dlp's `--print after_move:filename` output.
    pub(crate) const DEFAULT_PRINTED_FILENAME: &str = "fake-output.mp4";

    #[cfg(unix)]
    impl FakeYtDlp {
        pub(crate) fn with_exit_code(exit_code: i32) -> Self {
            Self::with_exit_code_and_printed_filename(exit_code, DEFAULT_PRINTED_FILENAME)
        }

        pub(crate) fn with_exit_code_and_printed_filename(
            exit_code: i32,
            printed_filename: &str,
        ) -> Self {
            use std::os::unix::fs::PermissionsExt;

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

            Self {
                _bin_dir: bin_dir,
                path: script_path,
                captured_args_path,
            }
        }

        /// A fake `yt-dlp` that exits 0, prints `printed_filename`, and
        /// creates each of `extra_files` (relative to the directory it's
        /// invoked in) — stands in for `yt-dlp` actually writing a video
        /// file and its converted thumbnail sibling to `output_path`.
        pub(crate) fn with_downloaded_files(printed_filename: &str, extra_files: &[&str]) -> Self {
            use std::os::unix::fs::PermissionsExt;

            let bin_dir = unique_temp_dir("fake-ytdlp-bin");
            let script_path = bin_dir.join("yt-dlp");
            let captured_args_path = bin_dir.join("captured-args");
            let touch_stmts: String = extra_files
                .iter()
                .map(|f| format!("touch \"{f}\"\n"))
                .collect();
            fs::write(
                &script_path,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"{}\"\n{touch_stmts}printf '%s\\n' '{printed_filename}'\nexit 0\n",
                    captured_args_path.display()
                ),
            )
            .unwrap();
            fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();

            Self {
                _bin_dir: bin_dir,
                path: script_path,
                captured_args_path,
            }
        }

        /// A fake `yt-dlp` that exits 0 and prints `stdout` verbatim,
        /// written to a file and `cat`-ed rather than embedded in the
        /// script, so it's immune to shell quoting of its content (e.g. a
        /// `|` character or embedded newlines).
        pub(crate) fn with_stdout(stdout: &str) -> Self {
            use std::os::unix::fs::PermissionsExt;

            let bin_dir = unique_temp_dir("fake-ytdlp-bin");
            let script_path = bin_dir.join("yt-dlp");
            let captured_args_path = bin_dir.join("captured-args");
            let stdout_path = bin_dir.join("stdout-content");
            fs::write(&stdout_path, stdout).unwrap();
            fs::write(
                &script_path,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"{}\"\ncat \"{}\"\nexit 0\n",
                    captured_args_path.display(),
                    stdout_path.display()
                ),
            )
            .unwrap();
            fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();

            Self {
                _bin_dir: bin_dir,
                path: script_path,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn it_should_return_the_printed_filename_on_success() {
        use test_support::{DEFAULT_PRINTED_FILENAME, FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output");
        let fake = FakeYtDlp::with_exit_code(0);

        let result = download_video(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap();

        assert_eq!(
            result,
            Some(DownloadedVideo {
                folder: "My Video".to_string(),
                filename: DEFAULT_PRINTED_FILENAME.to_string(),
                duration_seconds: None,
            })
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_none_on_a_clean_failed_exit() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output");
        let fake = FakeYtDlp::with_exit_code(1);

        let filename = download_video(
            &fake.path,
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
    fn it_should_remove_the_video_folder_after_a_clean_failed_exit() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-cleanup-on-failure");
        let fake = FakeYtDlp::with_exit_code(1);

        download_video(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap();

        assert!(!output_dir.join("My Video").exists());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_reuse_the_same_folder_name_on_a_retry_after_a_failed_attempt() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-retry-reuse");
        let failing = FakeYtDlp::with_exit_code(1);
        download_video(
            &failing.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap();

        let succeeding = FakeYtDlp::with_exit_code(0);
        let result = download_video(
            &succeeding.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap()
        .unwrap();

        assert_eq!(result.folder, "My Video");
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_pass_the_desired_filename_as_the_output_template_when_there_is_no_collision() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-no-collision");
        let fake = FakeYtDlp::with_exit_code(0);

        let result = download_video(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap()
        .unwrap();

        let mut expected = args_for_quality(Quality::High);
        expected.extend([
            "--embed-thumbnail".to_string(),
            "--write-thumbnail".to_string(),
            "--convert-thumbnails".to_string(),
            "jpg".to_string(),
            "--quiet".to_string(),
            "--no-warnings".to_string(),
            "--print".to_string(),
            "%(duration)s".to_string(),
            "--print".to_string(),
            "after_move:filename".to_string(),
            "https://example.com/video".to_string(),
            "-o".to_string(),
            "My Video.%(ext)s".to_string(),
        ]);
        assert_eq!(fake.captured_args(), expected);
        assert_eq!(result.folder, "My Video");
        assert!(output_dir.join("My Video").is_dir());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_pass_thumbnail_embedding_and_conversion_flags_to_yt_dlp() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-thumbnail-flags");
        let fake = FakeYtDlp::with_exit_code(0);

        download_video(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap();

        let args = fake.captured_args();
        assert!(args.contains(&"--embed-thumbnail".to_string()));
        assert!(args.contains(&"--write-thumbnail".to_string()));
        assert!(args.contains(&"--convert-thumbnails".to_string()));
        assert!(args.contains(&"jpg".to_string()));
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_append_the_video_id_to_the_output_template_on_collision() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-collision");
        std::fs::create_dir_all(output_dir.join("My Video")).unwrap();
        let fake = FakeYtDlp::with_exit_code(0);

        let result = download_video(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap()
        .unwrap();

        let mut expected = args_for_quality(Quality::High);
        expected.extend([
            "--embed-thumbnail".to_string(),
            "--write-thumbnail".to_string(),
            "--convert-thumbnails".to_string(),
            "jpg".to_string(),
            "--quiet".to_string(),
            "--no-warnings".to_string(),
            "--print".to_string(),
            "%(duration)s".to_string(),
            "--print".to_string(),
            "after_move:filename".to_string(),
            "https://example.com/video".to_string(),
            "-o".to_string(),
            "My Video [vid1].%(ext)s".to_string(),
        ]);
        assert_eq!(fake.captured_args(), expected);
        assert_eq!(result.folder, "My Video [vid1]");
        assert!(output_dir.join("My Video [vid1]").is_dir());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_error_when_a_successful_exit_prints_no_filename() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-no-print");
        let fake = FakeYtDlp::with_exit_code_and_printed_filename(0, "");

        let result = download_video(
            &fake.path,
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
    #[cfg(unix)]
    fn it_should_parse_the_duration_line_into_whole_seconds() {
        let fake = test_support::FakeYtDlp::with_stdout("223\nMy Video.mp4\n");
        let output_dir = test_support::unique_temp_dir("ytdlp-output-duration");

        let result = download_video(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap();

        assert_eq!(
            result,
            Some(DownloadedVideo {
                folder: "My Video".to_string(),
                filename: "My Video.mp4".to_string(),
                duration_seconds: Some(223),
            })
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_truncate_a_fractional_duration_to_whole_seconds() {
        let fake = test_support::FakeYtDlp::with_stdout("223.9\nMy Video.mp4\n");
        let output_dir = test_support::unique_temp_dir("ytdlp-output-duration-fractional");

        let result = download_video(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap();

        assert_eq!(result.unwrap().duration_seconds, Some(223));
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_treat_an_na_duration_as_none_without_affecting_the_filename() {
        let fake = test_support::FakeYtDlp::with_stdout("NA\nMy Video.mp4\n");
        let output_dir = test_support::unique_temp_dir("ytdlp-output-duration-na");

        let result = download_video(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap()
        .unwrap();

        assert_eq!(result.duration_seconds, None);
        assert_eq!(result.filename, "My Video.mp4");
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_treat_a_missing_duration_line_as_none() {
        use test_support::{DEFAULT_PRINTED_FILENAME, FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-duration-missing");
        let fake = FakeYtDlp::with_exit_code(0);

        let result = download_video(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
        )
        .unwrap()
        .unwrap();

        assert_eq!(result.filename, DEFAULT_PRINTED_FILENAME);
        assert_eq!(result.duration_seconds, None);
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn it_should_error_when_no_binary_exists_at_the_configured_path() {
        use test_support::unique_temp_dir;

        let output_dir = unique_temp_dir("ytdlp-output-missing-binary");
        let missing_path = output_dir.join("does-not-exist");

        let result = download_video(
            &missing_path,
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
                "--remux-video".to_string(),
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
                "--remux-video".to_string(),
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
                "--remux-video".to_string(),
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

    #[test]
    #[cfg(unix)]
    fn it_should_parse_one_json_line_per_discovered_video() {
        let stdout =
            "{\"id\": \"vid1\", \"title\": \"One\"}\n{\"id\": \"vid2\", \"title\": \"Two\"}\n";
        let fake = test_support::FakeYtDlp::with_stdout(stdout);

        let videos = list_channel_videos(
            &fake.path,
            "https://www.youtube.com/@somechannel/videos",
            10,
        )
        .unwrap();

        assert_eq!(
            videos,
            vec![
                ChannelVideoEntry {
                    video_id: "vid1".to_string(),
                    title: "One".to_string()
                },
                ChannelVideoEntry {
                    video_id: "vid2".to_string(),
                    title: "Two".to_string()
                },
            ]
        );
    }

    #[test]
    #[cfg(unix)]
    fn it_should_pass_the_limit_through_the_index_range_flag() {
        let fake = test_support::FakeYtDlp::with_stdout("");

        list_channel_videos(&fake.path, "https://www.youtube.com/@somechannel/videos", 5).unwrap();

        assert!(fake.captured_args().contains(&"1:5".to_string()));
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_an_empty_list_on_a_clean_failed_exit() {
        let fake = test_support::FakeYtDlp::with_exit_code(1);

        let videos = list_channel_videos(
            &fake.path,
            "https://www.youtube.com/@somechannel/videos",
            10,
        )
        .unwrap();

        assert!(videos.is_empty());
    }

    #[test]
    fn it_should_error_when_no_binary_exists_at_the_configured_path_for_channel_video_listing() {
        use test_support::unique_temp_dir;

        let output_dir = unique_temp_dir("ytdlp-channel-videos-missing-binary");
        let missing_path = output_dir.join("does-not-exist");

        let result = list_channel_videos(
            &missing_path,
            "https://www.youtube.com/@somechannel/videos",
            10,
        );

        assert!(result.is_err());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    /// Regression test for the JSON-line parsing format (see design.md):
    /// a delimited `"%(title)s | ..."` format would corrupt parsing on a
    /// title containing `|`; JSON output doesn't have that failure mode.
    #[test]
    #[cfg(unix)]
    fn it_should_parse_a_title_containing_a_pipe_character_without_corrupting_adjacent_videos() {
        let stdout = serde_json::json!({"id": "vid1", "title": "Before | After"}).to_string()
            + "\n"
            + &serde_json::json!({"id": "vid2", "title": "Untouched"}).to_string()
            + "\n";
        let fake = test_support::FakeYtDlp::with_stdout(&stdout);

        let videos = list_channel_videos(
            &fake.path,
            "https://www.youtube.com/@somechannel/videos",
            10,
        )
        .unwrap();

        assert_eq!(
            videos,
            vec![
                ChannelVideoEntry {
                    video_id: "vid1".to_string(),
                    title: "Before | After".to_string()
                },
                ChannelVideoEntry {
                    video_id: "vid2".to_string(),
                    title: "Untouched".to_string()
                },
            ]
        );
    }

    #[test]
    #[cfg(unix)]
    fn it_should_parse_a_title_containing_an_embedded_newline() {
        let stdout =
            serde_json::json!({"id": "vid1", "title": "Line one\nLine two"}).to_string() + "\n";
        let fake = test_support::FakeYtDlp::with_stdout(&stdout);

        let videos = list_channel_videos(
            &fake.path,
            "https://www.youtube.com/@somechannel/videos",
            10,
        )
        .unwrap();

        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].title, "Line one\nLine two");
    }
}
