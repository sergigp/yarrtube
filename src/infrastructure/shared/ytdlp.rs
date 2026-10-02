use crate::domain::shared::Quality;
use crate::domain::video::video_filename::collision_suffixed_folder;
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
    /// `Some(reason)` when `yt-dlp` reported YouTube's SABR-only streaming
    /// experiment during this download — the better formats were skipped as
    /// missing a URL, so this success may be a lower-quality fallback. `None`
    /// when no SABR signal was reported. Carries the reason text for logging;
    /// it never affects the recorded status.
    pub sabr_notice: Option<String>,
}

/// The outcome of a `download_video` attempt: either a `DownloadedVideo`, or
/// a clean `yt-dlp` failure (non-zero exit) carrying whatever text `yt-dlp`
/// wrote to its stderr, trimmed and `None` if it wrote nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadAttempt {
    Succeeded(DownloadedVideo),
    Failed {
        stderr: Option<String>,
        /// `Some(reason)` when `yt-dlp` reported the SABR-only streaming
        /// experiment while failing; see `DownloadedVideo::sabr_notice`.
        sabr_notice: Option<String>,
    },
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
/// used to be resolved — see `create_fresh_video_dir`), and reused as the
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
/// Returns `Ok(DownloadAttempt::Succeeded(..))` on a successful download,
/// `Ok(DownloadAttempt::Failed { stderr })` for a clean `yt-dlp` failure
/// (non-zero exit), carrying whatever `yt-dlp` wrote to its stderr. Returns
/// `Err` only for a systemic problem: no binary at `ytdlp_path`, a failure
/// creating the video's folder, or a successful exit that didn't print a
/// parseable filename. On any of these non-success outcomes, the video's
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
    existing_folder: Option<&str>,
) -> Result<DownloadAttempt> {
    let VideoDir {
        folder,
        path: video_dir,
    } = prepare_video_dir(output_path, desired_filename, video_id, existing_folder)?;

    let output_template = format!("{folder}.%(ext)s");
    let mut args = args_for_quality(quality);
    args.extend([
        "--concurrent-fragments".to_string(),
        "4".to_string(),
        "--embed-thumbnail".to_string(),
        "--write-thumbnail".to_string(),
        "--convert-thumbnails".to_string(),
        "jpg".to_string(),
        // Note: unlike the thumbnail/diagnose/listing invocations, this one
        // deliberately keeps warnings on (no `--no-warnings`) so `yt-dlp`'s
        // SABR-only streaming warning reaches our captured stderr, where
        // `sabr_reason` can detect it. `--quiet` still gives us the clean
        // `--print` stdout. See the `log-sabr-detection` change.
        "--quiet".to_string(),
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
        .stderr(Stdio::piped());
    let output = match run_and_cleanup_on_failure(
        &mut cmd,
        &video_dir,
        existing_folder,
        || {
            format!(
                "`yt-dlp` was not found at {}. Install yt-dlp there or run update-ytdlp before running yarrtube.",
                ytdlp_path.display()
            )
        },
        |e| format!("Failed to run yt-dlp for {video_url}: {e}"),
    )? {
        RunOutcome::Success(output) => output,
        RunOutcome::CleanFailure { stderr_raw } => {
            return Ok(DownloadAttempt::Failed {
                stderr: reason_excluding_warnings(&stderr_raw),
                sabr_notice: sabr_reason(&stderr_raw),
            });
        }
    };

    let sabr_notice = sabr_reason(&String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    let filename = match lines.last().map(|s| s.trim()) {
        Some(filename) if !filename.is_empty() => filename.to_string(),
        _ => {
            remove_video_dir_unless_reused(&video_dir, existing_folder);
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

    Ok(DownloadAttempt::Succeeded(DownloadedVideo {
        folder,
        filename,
        duration_seconds,
        sabr_notice,
    }))
}

/// A thumbnail-only fetch's result: the video's own output folder name
/// (relative to the container's output directory, same naming as
/// `DownloadedVideo::folder`) and the thumbnail's filename inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedThumbnail {
    pub folder: String,
    pub filename: String,
}

/// A thumbnail-only fetch's outcome: the thumbnail was written, or none could
/// be obtained, carrying `yt-dlp`'s reported error text when it reported one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbnailFetch {
    Fetched(FetchedThumbnail),
    /// Clean yt-dlp failure, or success that printed no thumbnail.
    Unavailable {
        reason: Option<String>,
    },
}

/// Runs `yt-dlp --skip-download --write-thumbnail --convert-thumbnails jpg`
/// for `video_url` inside the video's own dedicated folder under
/// `output_path` — sibling to `download_video`, reusing the same
/// folder-collision resolution, folder creation, and folder cleanup on
/// failure, so the two never disagree on this video's folder name.
///
/// Asks `yt-dlp` to print the actually-written thumbnail's path via
/// `--print after_video:thumbnails.-1.filepath`: verified against real
/// `yt-dlp` that (a) this is the only print scope that still fires with
/// `--skip-download` (`post_process`/`after_move` never do, since no video
/// download happens for them to hook off), (b) it resolves to the exact
/// on-disk filename after `--convert-thumbnails` has run, and (c) it prints
/// the literal string `NA` — yt-dlp's own convention for an unset field —
/// when the extractor has no thumbnail to write, cleanly distinguishing
/// "no thumbnail for this video" from a systemic failure.
///
/// Returns `Ok(ThumbnailFetch::Fetched(..))` when a thumbnail was written,
/// `Ok(ThumbnailFetch::Unavailable { .. })` for a clean `yt-dlp` exit with
/// no thumbnail available (either a non-zero exit, carrying its
/// warning-free stderr as `reason`, or a successful exit that printed no
/// usable filename) — stderr is captured, never inherited, so none of it
/// reaches the daemon's own output —
/// this is expected and routine, not an error, since a thumbnail is
/// optional even on a clean run. Returns `Err` only for a systemic problem:
/// no binary at `ytdlp_path`, or a failure creating the video's folder.
/// Mirrors `download_video`'s folder cleanup: on any non-success outcome,
/// the folder created up front is removed again so a retry reuses the same
/// folder name instead of seeing a stale collision — unless `existing_folder`
/// was given, in which case it's a folder this same video already owns (e.g.
/// its real download's folder, for a missing-thumbnail recovery pass) and is
/// never removed by this function.
pub fn fetch_thumbnail(
    ytdlp_path: &Path,
    video_url: &str,
    desired_filename: &str,
    video_id: &str,
    output_path: &Path,
    existing_folder: Option<&str>,
) -> Result<ThumbnailFetch> {
    let VideoDir {
        folder,
        path: video_dir,
    } = prepare_video_dir(output_path, desired_filename, video_id, existing_folder)?;

    let output_template = format!("{folder}.%(ext)s");
    let mut cmd = Command::new(ytdlp_path);
    cmd.args([
        "--skip-download",
        "--write-thumbnail",
        "--convert-thumbnails",
        "jpg",
        "--quiet",
        "--no-warnings",
        "--print",
        "after_video:thumbnails.-1.filepath",
    ])
    .arg(video_url)
    .arg("-o")
    .arg(&output_template)
    .current_dir(&video_dir)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    let output = match run_and_cleanup_on_failure(
        &mut cmd,
        &video_dir,
        existing_folder,
        || {
            format!(
                "`yt-dlp` was not found at {}. Install yt-dlp there or run update-ytdlp before running yarrtube.",
                ytdlp_path.display()
            )
        },
        |e| format!("Failed to run yt-dlp thumbnail fetch for {video_url}: {e}"),
    )? {
        RunOutcome::Success(output) => output,
        RunOutcome::CleanFailure { stderr_raw } => {
            return Ok(ThumbnailFetch::Unavailable {
                reason: reason_excluding_warnings(&stderr_raw),
            });
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let printed = stdout.lines().next_back().map(|s| s.trim());
    let filename = match printed {
        Some(printed) if !printed.is_empty() && !printed.eq_ignore_ascii_case("na") => {
            Path::new(printed)
                .file_name()
                .and_then(|f| f.to_str())
                .map(str::to_string)
        }
        _ => None,
    };

    match filename {
        Some(filename) => Ok(ThumbnailFetch::Fetched(FetchedThumbnail {
            folder,
            filename,
        })),
        None => {
            remove_video_dir_unless_reused(&video_dir, existing_folder);
            Ok(ThumbnailFetch::Unavailable { reason: None })
        }
    }
}

/// Runs a simulate-only `yt-dlp` probe for `video_url`, forcing the
/// alternate player clients, to reveal the *precise* reason a download
/// failed — `yt-dlp`'s default clients collapse many distinct permanent
/// blocks into a bare "Video unavailable". The probe never downloads (it only
/// reveals the reason; a blocked video stays blocked), so it is safe and
/// cheap.
///
/// Returns `Ok(Some(reason))` with the reason `yt-dlp` printed to stderr
/// (its `ERROR: [<extractor>] <id>:` prefix stripped), `Ok(None)` when no
/// precise reason can be determined (the probe exited cleanly with no error,
/// or printed only a bare "Video unavailable" with no further detail).
/// Returns `Err` only for a systemic problem: no binary at `ytdlp_path`, or a
/// spawn failure — the caller swallows that and treats the reason as
/// undetermined.
pub fn diagnose(ytdlp_path: &Path, video_url: &str) -> Result<Option<String>> {
    let mut cmd = Command::new(ytdlp_path);
    cmd.args([
        "--simulate",
        "--no-warnings",
        "--extractor-args",
        "youtube:player_client=android,tv,ios,web_safari",
    ])
    .arg(video_url)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    let output = match output_retrying_busy(&mut cmd) {
        Ok(output) => output,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(anyhow!(
                "`yt-dlp` was not found at {}. Install yt-dlp there or run update-ytdlp before running yarrtube.",
                ytdlp_path.display()
            ));
        }
        Err(e) => {
            return Err(anyhow!(
                "Failed to run yt-dlp diagnostic probe for {video_url}: {e}"
            ));
        }
    };

    Ok(extract_reason(&output.stderr))
}

/// Extracts the human-readable failure reason from a diagnostic probe's
/// stderr. `yt-dlp` writes `ERROR: [<extractor>] <id>: <reason>`; this returns
/// `<reason>`, or `None` when there is no error line or the only reason is a
/// bare "Video unavailable" with no further detail (too generic to act on).
fn extract_reason(stderr: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stderr);
    let reason = text
        .lines()
        .filter_map(error_line_reason)
        .map(str::trim)
        .find(|reason| !reason.is_empty())?;
    (!is_bare_video_unavailable(reason)).then(|| reason.to_string())
}

/// The reason text of a single `ERROR:` stderr line, with the
/// `ERROR: [<extractor>] <id>: ` prefix stripped; `None` for a non-error line.
fn error_line_reason(line: &str) -> Option<&str> {
    let after_error = line.trim().strip_prefix("ERROR:")?.trim_start();
    Some(strip_extractor_prefix(after_error))
}

/// Strips a leading `[<extractor>] <id>: ` prefix, leaving the reason; any
/// text not in that shape is returned unchanged.
fn strip_extractor_prefix(text: &str) -> &str {
    text.strip_prefix('[')
        .and_then(|rest| rest.split_once("] "))
        .and_then(|(_, after)| after.split_once(": "))
        .map_or(text, |(_, reason)| reason)
}

fn is_bare_video_unavailable(reason: &str) -> bool {
    reason
        .trim_end_matches('.')
        .trim()
        .eq_ignore_ascii_case("video unavailable")
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

/// Claims `desired_folder` in `output_path` with a single atomic
/// `create_dir`, so two concurrent downloads of same-titled videos can't
/// both take it. If an entry (file or folder) already exists at exactly that
/// name, falls back to this video's own collision-suffixed folder instead.
fn create_fresh_video_dir(
    output_path: &Path,
    desired_folder: &str,
    video_id: &str,
) -> Result<String> {
    ensure_output_dir(output_path)?;
    match std::fs::create_dir(output_path.join(desired_folder)) {
        Ok(()) => Ok(desired_folder.to_string()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            let folder = collision_suffixed_folder(desired_folder, video_id);
            ensure_output_dir(&output_path.join(&folder))?;
            Ok(folder)
        }
        Err(e) => Err(anyhow!(
            "Failed to create video folder {:?}: {e}",
            output_path.join(desired_folder)
        )),
    }
}

/// The video's resolved per-video folder (`existing_folder` reused verbatim
/// when given, otherwise a fresh one from `create_fresh_video_dir`), created
/// on disk before either `download_video` or `fetch_thumbnail` invokes
/// `yt-dlp` inside it.
struct VideoDir {
    folder: String,
    path: std::path::PathBuf,
}

fn prepare_video_dir(
    output_path: &Path,
    desired_filename: &str,
    video_id: &str,
    existing_folder: Option<&str>,
) -> Result<VideoDir> {
    let folder = match existing_folder {
        Some(folder) => {
            ensure_output_dir(&output_path.join(folder))?;
            folder.to_string()
        }
        None => create_fresh_video_dir(output_path, desired_filename, video_id)?,
    };
    let path = output_path.join(&folder);
    Ok(VideoDir { folder, path })
}

/// Removes `video_dir`, unless it's a folder reused via `existing_folder`
/// (one this same video's earlier thumbnail fetch or download already
/// populated) — reusing it verbatim means it isn't ours to delete on
/// failure.
fn remove_video_dir_unless_reused(video_dir: &Path, existing_folder: Option<&str>) {
    if existing_folder.is_none() {
        remove_video_dir_best_effort(video_dir);
    }
}

/// The outcome of running a fully-configured `yt-dlp` invocation:
/// `Success` carries the process's output for the caller to parse,
/// `CleanFailure` a non-zero exit carrying `yt-dlp`'s full (lossy, untrimmed)
/// stderr for the caller to interpret — the download caller both derives its
/// recorded failure reason from it (warnings filtered out) and scans it for
/// the SABR signal.
enum RunOutcome {
    Success(std::process::Output),
    CleanFailure { stderr_raw: String },
}

/// The recorded failure reason for a clean `yt-dlp` failure: `stderr` with
/// every `WARNING:` block removed — the `WARNING:` line itself and any
/// indented continuation lines `yt-dlp` wraps it onto — so warnings it now
/// prints (the download invocation no longer passes `--no-warnings`) never
/// pollute the reason or the permanent-unavailability classification that
/// reads it. Non-warning text, including an unprefixed error line, is kept.
/// Trimmed, and `None` when nothing meaningful remains.
fn reason_excluding_warnings(stderr: &str) -> Option<String> {
    let mut in_warning_block = false;
    let text = stderr
        .lines()
        .filter(|line| {
            if line.trim_start().starts_with("WARNING:") {
                in_warning_block = true;
                return false;
            }
            // An indented line right after a warning is that warning wrapped
            // onto another line; drop it too. Any other line ends the block.
            if in_warning_block && line.starts_with([' ', '\t']) {
                return false;
            }
            in_warning_block = false;
            true
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    (!text.is_empty()).then_some(text)
}

/// Detects YouTube's SABR-only streaming experiment in a `yt-dlp` run's
/// stderr. `yt-dlp` announces it on a `WARNING:` line mentioning SABR (the
/// better formats skipped as "missing a URL"); keying on the case-insensitive
/// token `sabr` is specific enough not to false-positive and robust to minor
/// wording changes. Returns the trimmed matching line(s) joined, or `None`
/// when no such line is present.
fn sabr_reason(stderr: &str) -> Option<String> {
    let text = stderr
        .lines()
        .filter(|line| line.to_lowercase().contains("sabr"))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then_some(text)
}

/// Runs `cmd` (fully configured: args, stdio, working directory), retrying
/// on a transient busy error, and cleans up `video_dir` on any failure
/// unless it was reused via `existing_folder` — see
/// `remove_video_dir_unless_reused`. Returns `Err` only for a systemic
/// problem (missing binary, spawn failure).
fn run_and_cleanup_on_failure(
    cmd: &mut Command,
    video_dir: &Path,
    existing_folder: Option<&str>,
    not_found_msg: impl FnOnce() -> String,
    other_err_msg: impl FnOnce(&io::Error) -> String,
) -> Result<RunOutcome> {
    let output = match output_retrying_busy(cmd) {
        Ok(output) => output,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            remove_video_dir_unless_reused(video_dir, existing_folder);
            return Err(anyhow!(not_found_msg()));
        }
        Err(e) => {
            remove_video_dir_unless_reused(video_dir, existing_folder);
            return Err(anyhow!(other_err_msg(&e)));
        }
    };

    if !output.status.success() {
        remove_video_dir_unless_reused(video_dir, existing_folder);
        return Ok(RunOutcome::CleanFailure {
            stderr_raw: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(RunOutcome::Success(output))
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

        /// A fake `yt-dlp` that writes `stderr` verbatim to stderr and exits
        /// 1, `cat`-ing it from a file like `with_stdout` does.
        pub(crate) fn failing_with_stderr(stderr: &str) -> Self {
            use std::os::unix::fs::PermissionsExt;

            let bin_dir = unique_temp_dir("fake-ytdlp-bin");
            let script_path = bin_dir.join("yt-dlp");
            let captured_args_path = bin_dir.join("captured-args");
            let stderr_path = bin_dir.join("stderr-content");
            fs::write(&stderr_path, stderr).unwrap();
            fs::write(
                &script_path,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"{}\"\ncat \"{}\" >&2\nexit 1\n",
                    captured_args_path.display(),
                    stderr_path.display()
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

    fn unwrap_succeeded(attempt: DownloadAttempt) -> DownloadedVideo {
        match attempt {
            DownloadAttempt::Succeeded(video) => video,
            DownloadAttempt::Failed { stderr, .. } => {
                panic!(
                    "expected a successful download attempt, got Failed {{ stderr: {stderr:?} }}"
                )
            }
        }
    }

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
            None,
        )
        .unwrap();

        assert_eq!(
            result,
            DownloadAttempt::Succeeded(DownloadedVideo {
                folder: "My Video".to_string(),
                filename: DEFAULT_PRINTED_FILENAME.to_string(),
                duration_seconds: None,
                sabr_notice: None,
            })
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_the_stderr_text_on_a_clean_failed_exit() {
        use std::os::unix::fs::PermissionsExt;
        use test_support::unique_temp_dir;

        let output_dir = unique_temp_dir("ytdlp-output-stderr");
        let bin_dir = unique_temp_dir("fake-ytdlp-bin-stderr");
        let script_path = bin_dir.join("yt-dlp");
        std::fs::write(
            &script_path,
            "#!/bin/sh\nprintf '%s\\n' 'HTTP Error 403: Forbidden' >&2\nexit 1\n",
        )
        .unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let result = download_video(
            &script_path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
            None,
        )
        .unwrap();

        assert_eq!(
            result,
            DownloadAttempt::Failed {
                stderr: Some("HTTP Error 403: Forbidden".to_string()),
                sabr_notice: None,
            }
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
        std::fs::remove_dir_all(&bin_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_no_stderr_text_when_yt_dlp_writes_nothing_to_stderr() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output");
        let fake = FakeYtDlp::with_exit_code(1);

        let result = download_video(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
            None,
        )
        .unwrap();

        assert_eq!(
            result,
            DownloadAttempt::Failed {
                stderr: None,
                sabr_notice: None,
            }
        );
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
            None,
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
            None,
        )
        .unwrap();

        let succeeding = FakeYtDlp::with_exit_code(0);
        let result = unwrap_succeeded(
            download_video(
                &succeeding.path,
                "https://example.com/video",
                "My Video",
                "vid1",
                Quality::High,
                &output_dir,
                None,
            )
            .unwrap(),
        );

        assert_eq!(result.folder, "My Video");
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_pass_the_desired_filename_as_the_output_template_when_there_is_no_collision() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-no-collision");
        let fake = FakeYtDlp::with_exit_code(0);

        let result = unwrap_succeeded(
            download_video(
                &fake.path,
                "https://example.com/video",
                "My Video",
                "vid1",
                Quality::High,
                &output_dir,
                None,
            )
            .unwrap(),
        );

        let mut expected = args_for_quality(Quality::High);
        expected.extend([
            "--concurrent-fragments".to_string(),
            "4".to_string(),
            "--embed-thumbnail".to_string(),
            "--write-thumbnail".to_string(),
            "--convert-thumbnails".to_string(),
            "jpg".to_string(),
            "--quiet".to_string(),
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
            None,
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

        let result = unwrap_succeeded(
            download_video(
                &fake.path,
                "https://example.com/video",
                "My Video",
                "vid1",
                Quality::High,
                &output_dir,
                None,
            )
            .unwrap(),
        );

        let mut expected = args_for_quality(Quality::High);
        expected.extend([
            "--concurrent-fragments".to_string(),
            "4".to_string(),
            "--embed-thumbnail".to_string(),
            "--write-thumbnail".to_string(),
            "--convert-thumbnails".to_string(),
            "jpg".to_string(),
            "--quiet".to_string(),
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
            None,
        );

        assert!(result.is_err());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_parse_the_duration_line_into_whole_seconds() {
        let fake = test_support::FakeYtDlp::with_stdout("223\nMy Video.mp4\n");
        let output_dir = test_support::unique_temp_dir("ytdlp-output-duration");

        let result = unwrap_succeeded(
            download_video(
                &fake.path,
                "https://example.com/video",
                "My Video",
                "vid1",
                Quality::High,
                &output_dir,
                None,
            )
            .unwrap(),
        );

        assert_eq!(
            result,
            DownloadedVideo {
                folder: "My Video".to_string(),
                filename: "My Video.mp4".to_string(),
                duration_seconds: Some(223),
                sabr_notice: None,
            }
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_truncate_a_fractional_duration_to_whole_seconds() {
        let fake = test_support::FakeYtDlp::with_stdout("223.9\nMy Video.mp4\n");
        let output_dir = test_support::unique_temp_dir("ytdlp-output-duration-fractional");

        let result = unwrap_succeeded(
            download_video(
                &fake.path,
                "https://example.com/video",
                "My Video",
                "vid1",
                Quality::High,
                &output_dir,
                None,
            )
            .unwrap(),
        );

        assert_eq!(result.duration_seconds, Some(223));
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_treat_an_na_duration_as_none_without_affecting_the_filename() {
        let fake = test_support::FakeYtDlp::with_stdout("NA\nMy Video.mp4\n");
        let output_dir = test_support::unique_temp_dir("ytdlp-output-duration-na");

        let result = unwrap_succeeded(
            download_video(
                &fake.path,
                "https://example.com/video",
                "My Video",
                "vid1",
                Quality::High,
                &output_dir,
                None,
            )
            .unwrap(),
        );

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

        let result = unwrap_succeeded(
            download_video(
                &fake.path,
                "https://example.com/video",
                "My Video",
                "vid1",
                Quality::High,
                &output_dir,
                None,
            )
            .unwrap(),
        );

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
            None,
        );

        assert!(result.is_err());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_use_the_existing_folder_verbatim_without_a_collision_check() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-existing-folder");
        // An entry already exists at "My Video" — if `download_video` ran its
        // usual collision check it would suffix the folder with the video
        // ID; passing `existing_folder` must bypass that check entirely.
        std::fs::create_dir_all(output_dir.join("My Video")).unwrap();
        let fake = FakeYtDlp::with_exit_code(0);

        let result = unwrap_succeeded(
            download_video(
                &fake.path,
                "https://example.com/video",
                "My Video",
                "vid1",
                Quality::High,
                &output_dir,
                Some("My Video"),
            )
            .unwrap(),
        );

        assert_eq!(result.folder, "My Video");
        assert!(
            fake.captured_args()
                .contains(&"My Video.%(ext)s".to_string())
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_not_remove_a_reused_existing_folder_on_a_failed_download() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-failed-exit-reused-folder");
        // A thumbnail already fetched into this video's folder ahead of the
        // download — a failed download attempt must not delete it.
        std::fs::create_dir_all(output_dir.join("My Video")).unwrap();
        std::fs::write(output_dir.join("My Video").join("My Video.jpg"), b"thumb").unwrap();
        let fake = FakeYtDlp::with_exit_code(1);

        let result = download_video(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
            Some("My Video"),
        )
        .unwrap();

        assert_eq!(
            result,
            DownloadAttempt::Failed {
                stderr: None,
                sabr_notice: None,
            }
        );
        assert!(output_dir.join("My Video").join("My Video.jpg").exists());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[cfg(unix)]
    fn fake_ytdlp_writing_stderr(
        dir_name: &str,
        stderr: &str,
        exit_code: i32,
    ) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let bin_dir = test_support::unique_temp_dir(dir_name);
        let script_path = bin_dir.join("yt-dlp");
        let stderr_path = bin_dir.join("stderr-content");
        std::fs::write(&stderr_path, stderr).unwrap();
        std::fs::write(
            &script_path,
            format!(
                "#!/bin/sh\ncat \"{}\" >&2\nexit {exit_code}\n",
                stderr_path.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        script_path
    }

    /// A fake `yt-dlp` that prints `stdout` to stdout and `stderr` to stderr
    /// (each from a file, so content is immune to shell quoting) and exits 0 —
    /// for driving a successful download whose stderr still carries a warning.
    #[cfg(unix)]
    fn fake_ytdlp_writing_stdout_and_stderr(
        dir_name: &str,
        stdout: &str,
        stderr: &str,
    ) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let bin_dir = test_support::unique_temp_dir(dir_name);
        let script_path = bin_dir.join("yt-dlp");
        let stdout_path = bin_dir.join("stdout-content");
        let stderr_path = bin_dir.join("stderr-content");
        std::fs::write(&stdout_path, stdout).unwrap();
        std::fs::write(&stderr_path, stderr).unwrap();
        std::fs::write(
            &script_path,
            format!(
                "#!/bin/sh\ncat \"{}\"\ncat \"{}\" >&2\nexit 0\n",
                stdout_path.display(),
                stderr_path.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        script_path
    }

    const SABR_WARNING: &str = "WARNING: [youtube] abc123: Some ios client https formats have been skipped as they are missing a URL. YouTube may have enabled the SABR-only streaming experiment for the current session. See https://github.com/yt-dlp/yt-dlp/issues/12482 for more details";

    #[test]
    fn it_should_detect_the_sabr_only_streaming_warning_in_stderr() {
        let stderr = format!("{SABR_WARNING}\n[info] abc123: Downloading 1 format(s): 18\n");

        assert_eq!(sabr_reason(&stderr), Some(SABR_WARNING.to_string()));
    }

    #[test]
    fn it_should_not_detect_sabr_in_a_clean_run() {
        assert_eq!(
            sabr_reason("[info] abc123: Downloading 1 format(s): 399+251\n"),
            None
        );
    }

    #[test]
    fn it_should_not_detect_sabr_in_an_unrelated_warning() {
        assert_eq!(
            sabr_reason("WARNING: [youtube] abc123: Falling back to generic n-function search\n"),
            None
        );
    }

    #[test]
    fn it_should_exclude_warning_lines_from_the_recorded_failure_reason() {
        let stderr =
            format!("{SABR_WARNING}\nERROR: [youtube] abc123: This video is not available\n");

        assert_eq!(
            reason_excluding_warnings(&stderr),
            Some("ERROR: [youtube] abc123: This video is not available".to_string())
        );
    }

    #[test]
    fn it_should_report_no_recorded_reason_when_only_warnings_remain() {
        assert_eq!(
            reason_excluding_warnings(&format!("{SABR_WARNING}\n")),
            None
        );
    }

    #[test]
    fn it_should_exclude_an_indented_warning_continuation_line_from_the_recorded_reason() {
        let stderr = "WARNING: [youtube] abc123: this video may be blocked\n    because it is not available in your country\nERROR: [youtube] abc123: This video is not available\n";

        assert_eq!(
            reason_excluding_warnings(stderr),
            Some("ERROR: [youtube] abc123: This video is not available".to_string())
        );
    }

    #[test]
    #[cfg(unix)]
    fn it_should_report_a_sabr_notice_on_a_successful_but_degraded_download() {
        let script_path = fake_ytdlp_writing_stdout_and_stderr(
            "ytdlp-download-sabr-success",
            "My Video.mp4\n",
            &format!("{SABR_WARNING}\n"),
        );
        let output_dir = test_support::unique_temp_dir("ytdlp-download-sabr-success-out");

        let result = download_video(
            &script_path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
            None,
        )
        .unwrap();

        assert_eq!(
            result,
            DownloadAttempt::Succeeded(DownloadedVideo {
                folder: "My Video".to_string(),
                filename: "My Video.mp4".to_string(),
                duration_seconds: None,
                sabr_notice: Some(SABR_WARNING.to_string()),
            })
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_report_a_sabr_notice_and_a_warning_free_reason_on_a_failed_download() {
        let script_path = fake_ytdlp_writing_stderr(
            "ytdlp-download-sabr-failure",
            &format!("{SABR_WARNING}\nERROR: [youtube] abc123: This video is not available\n"),
            1,
        );
        let output_dir = test_support::unique_temp_dir("ytdlp-download-sabr-failure-out");

        let result = download_video(
            &script_path,
            "https://example.com/video",
            "My Video",
            "vid1",
            Quality::High,
            &output_dir,
            None,
        )
        .unwrap();

        assert_eq!(
            result,
            DownloadAttempt::Failed {
                stderr: Some("ERROR: [youtube] abc123: This video is not available".to_string()),
                sabr_notice: Some(SABR_WARNING.to_string()),
            }
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_the_precise_reason_a_diagnostic_probe_prints() {
        let script_path = fake_ytdlp_writing_stderr(
            "ytdlp-diagnose-reason",
            "ERROR: [youtube] abc123: Join this channel to get access to members-only content like this video, and other exclusive perks.\n",
            1,
        );

        let result = diagnose(&script_path, "https://example.com/video").unwrap();

        assert_eq!(
            result,
            Some(
                "Join this channel to get access to members-only content like this video, and other exclusive perks."
                    .to_string()
            )
        );
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_none_when_a_diagnostic_probe_finds_only_a_bare_video_unavailable() {
        let script_path = fake_ytdlp_writing_stderr(
            "ytdlp-diagnose-bare",
            "ERROR: [youtube] abc123: Video unavailable\n",
            1,
        );

        let result = diagnose(&script_path, "https://example.com/video").unwrap();

        assert_eq!(result, None);
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_none_when_a_diagnostic_probe_exits_cleanly() {
        let fake = test_support::FakeYtDlp::with_stdout("");

        let result = diagnose(&fake.path, "https://example.com/video").unwrap();

        assert_eq!(result, None);
    }

    #[test]
    #[cfg(unix)]
    fn it_should_pass_the_simulate_and_alternate_client_flags_to_the_diagnostic_probe() {
        let fake = test_support::FakeYtDlp::with_stdout("");

        diagnose(&fake.path, "https://example.com/video").unwrap();

        assert_eq!(
            fake.captured_args(),
            vec![
                "--simulate".to_string(),
                "--no-warnings".to_string(),
                "--extractor-args".to_string(),
                "youtube:player_client=android,tv,ios,web_safari".to_string(),
                "https://example.com/video".to_string(),
            ]
        );
    }

    #[test]
    fn it_should_error_when_no_binary_exists_at_the_configured_path_for_a_diagnostic_probe() {
        use test_support::unique_temp_dir;

        let dir = unique_temp_dir("ytdlp-diagnose-missing-binary");
        let missing_path = dir.join("does-not-exist");

        let result = diagnose(&missing_path, "https://example.com/video");

        assert!(result.is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_the_printed_thumbnail_filename_on_success() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-thumbnail-output");
        let fake = FakeYtDlp::with_stdout("My Video.jpg\n");

        let result = fetch_thumbnail(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            &output_dir,
            None,
        )
        .unwrap();

        assert_eq!(
            result,
            ThumbnailFetch::Fetched(FetchedThumbnail {
                folder: "My Video".to_string(),
                filename: "My Video.jpg".to_string(),
            })
        );
        assert!(output_dir.join("My Video").is_dir());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_pass_thumbnail_only_flags_to_yt_dlp() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-thumbnail-flags");
        let fake = FakeYtDlp::with_stdout("My Video.jpg\n");

        fetch_thumbnail(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            &output_dir,
            None,
        )
        .unwrap();

        let expected = vec![
            "--skip-download".to_string(),
            "--write-thumbnail".to_string(),
            "--convert-thumbnails".to_string(),
            "jpg".to_string(),
            "--quiet".to_string(),
            "--no-warnings".to_string(),
            "--print".to_string(),
            "after_video:thumbnails.-1.filepath".to_string(),
            "https://example.com/video".to_string(),
            "-o".to_string(),
            "My Video.%(ext)s".to_string(),
        ];
        assert_eq!(fake.captured_args(), expected);
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_unavailable_without_a_reason_when_yt_dlp_prints_na() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-thumbnail-na");
        let fake = FakeYtDlp::with_stdout("NA\n");

        let result = fetch_thumbnail(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            &output_dir,
            None,
        )
        .unwrap();

        assert_eq!(result, ThumbnailFetch::Unavailable { reason: None });
        assert!(!output_dir.join("My Video").exists());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_unavailable_with_the_reason_on_a_clean_failed_thumbnail_fetch() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-thumbnail-failed-exit");
        let fake = FakeYtDlp::failing_with_stderr(
            "WARNING: [youtube] x: some warning\nERROR: [youtube] x: Video unavailable\n",
        );

        let result = fetch_thumbnail(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            &output_dir,
            None,
        )
        .unwrap();

        assert_eq!(
            result,
            ThumbnailFetch::Unavailable {
                reason: Some("ERROR: [youtube] x: Video unavailable".to_string()),
            }
        );
        assert!(!output_dir.join("My Video").exists());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_not_remove_a_reused_existing_folder_on_a_clean_failed_exit() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-thumbnail-failed-exit-reused-folder");
        std::fs::create_dir_all(output_dir.join("My Video")).unwrap();
        std::fs::write(output_dir.join("My Video").join("My Video.mp4"), b"video").unwrap();
        let fake = FakeYtDlp::with_exit_code(1);

        let result = fetch_thumbnail(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            &output_dir,
            Some("My Video"),
        )
        .unwrap();

        assert_eq!(result, ThumbnailFetch::Unavailable { reason: None });
        assert!(output_dir.join("My Video").join("My Video.mp4").exists());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn it_should_error_when_no_binary_exists_at_the_configured_path_for_thumbnail_fetch() {
        use test_support::unique_temp_dir;

        let output_dir = unique_temp_dir("ytdlp-thumbnail-missing-binary");
        let missing_path = output_dir.join("does-not-exist");

        let result = fetch_thumbnail(
            &missing_path,
            "https://example.com/video",
            "My Video",
            "vid1",
            &output_dir,
            None,
        );

        assert!(result.is_err());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn it_should_give_concurrent_same_title_videos_distinct_folders() {
        use std::collections::HashSet;
        use std::sync::{Arc, Barrier};
        use test_support::unique_temp_dir;

        let outcomes: Vec<HashSet<String>> = (0..200)
            .map(|_| {
                let output_dir = unique_temp_dir("ytdlp-concurrent-folders");
                let barrier = Arc::new(Barrier::new(2));
                let folders: HashSet<String> = ["idA", "idB"]
                    .into_iter()
                    .map(|video_id| {
                        let (output_dir, barrier) = (output_dir.clone(), barrier.clone());
                        std::thread::spawn(move || {
                            barrier.wait();
                            prepare_video_dir(&output_dir, "Song", video_id, None)
                                .unwrap()
                                .folder
                        })
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|handle| handle.join().unwrap())
                    .collect();
                std::fs::remove_dir_all(&output_dir).unwrap();
                folders
            })
            .collect();

        let valid: [HashSet<String>; 2] = [
            HashSet::from(["Song".to_string(), "Song [idB]".to_string()]),
            HashSet::from(["Song [idA]".to_string(), "Song".to_string()]),
        ];
        assert_eq!(
            outcomes
                .into_iter()
                .filter(|folders| !valid.contains(folders))
                .collect::<Vec<_>>(),
            Vec::<HashSet<String>>::new()
        );
    }

    #[test]
    #[cfg(unix)]
    fn it_should_append_the_video_id_to_the_thumbnail_folder_on_collision() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-thumbnail-collision");
        std::fs::create_dir_all(output_dir.join("My Video")).unwrap();
        let fake = FakeYtDlp::with_stdout("My Video [vid1].jpg\n");

        let result = fetch_thumbnail(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            &output_dir,
            None,
        )
        .unwrap();

        assert_eq!(
            result,
            ThumbnailFetch::Fetched(FetchedThumbnail {
                folder: "My Video [vid1]".to_string(),
                filename: "My Video [vid1].jpg".to_string(),
            })
        );
        assert!(
            fake.captured_args()
                .contains(&"My Video [vid1].%(ext)s".to_string())
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_use_the_existing_folder_verbatim_for_a_thumbnail_fetch() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-thumbnail-existing-folder");
        // An entry already exists at "My Video" — if `fetch_thumbnail` ran its
        // usual collision check it would suffix the folder with the video ID;
        // passing `existing_folder` must bypass that check entirely.
        std::fs::create_dir_all(output_dir.join("My Video")).unwrap();
        let fake = FakeYtDlp::with_stdout("My Video.jpg\n");

        let result = fetch_thumbnail(
            &fake.path,
            "https://example.com/video",
            "My Video",
            "vid1",
            &output_dir,
            Some("My Video"),
        )
        .unwrap();

        assert_eq!(
            result,
            ThumbnailFetch::Fetched(FetchedThumbnail {
                folder: "My Video".to_string(),
                filename: "My Video.jpg".to_string(),
            })
        );
        assert!(
            fake.captured_args()
                .contains(&"My Video.%(ext)s".to_string())
        );
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
