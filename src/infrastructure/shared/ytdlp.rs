use crate::domain::shared::Quality;
use crate::domain::video_transcript::{CaptionSource, CaptionTrack, CaptionTracks};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

/// A video's `yt-dlp`-reported download result, staged outside its show:
/// the media file's path, the converted thumbnail's path when `yt-dlp` wrote
/// one, its duration in whole seconds and its publish time, when known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedVideo {
    pub media_path: PathBuf,
    pub thumbnail_path: Option<PathBuf>,
    pub duration_seconds: Option<i64>,
    pub published_at: Option<DateTime<Utc>>,
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

/// Names every staged file after the video's YouTube id, so the media and
/// its thumbnail share one stem whatever the title.
const STAGED_OUTPUT_TEMPLATE: &str = "%(id)s.%(ext)s";

/// Runs `<ytdlp_path> <args> <video_url>` inside `staging_dir` (created
/// first), outside any show directory, so partial files never reach the
/// library. This is a pure process-invocation layer with no YouTube API or
/// XML knowledge. Asks `yt-dlp` to print its duration followed by the exact
/// filename it saved via `--print %(duration)s --print after_move:filename`,
/// in quiet mode so those are the only lines on stdout, filename last.
/// Returns `Ok(DownloadAttempt::Succeeded(..))` on a successful download,
/// `Ok(DownloadAttempt::Failed { stderr })` for a clean `yt-dlp` failure
/// (non-zero exit), carrying whatever `yt-dlp` wrote to its stderr. Returns
/// `Err` only for a systemic problem: no binary at `ytdlp_path`, a failure
/// creating the staging dir, or a successful exit that didn't print a
/// parseable filename. Discarding the staging dir is the caller's job.
pub fn download_video(
    ytdlp_path: &Path,
    video_url: &str,
    quality: Quality,
    staging_dir: &Path,
) -> Result<DownloadAttempt> {
    ensure_output_dir(staging_dir)?;
    let mut args = args_for_quality(quality);
    args.extend([
        "-o".to_string(),
        STAGED_OUTPUT_TEMPLATE.to_string(),
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
        "%(timestamp)s".to_string(),
        "--print".to_string(),
        "%(upload_date)s".to_string(),
        "--print".to_string(),
        "%(duration)s".to_string(),
        "--print".to_string(),
        "after_move:filename".to_string(),
    ]);
    println!(
        "Running: {} {} {video_url} (in {})",
        ytdlp_path.display(),
        args.join(" "),
        staging_dir.display()
    );
    let mut cmd = Command::new(ytdlp_path);
    cmd.args(&args)
        .arg(video_url)
        .current_dir(staging_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = match run(
        &mut cmd,
        || not_found_message(ytdlp_path),
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
            return Err(anyhow!(
                "yt-dlp exited successfully but did not print an output filename for {video_url}"
            ));
        }
    };
    let duration_seconds = line_before_last(&lines, 1).and_then(parse_duration_seconds);
    let published_at = parse_published_at(line_before_last(&lines, 3), line_before_last(&lines, 2));

    let media_path = staging_dir.join(filename);
    Ok(DownloadAttempt::Succeeded(DownloadedVideo {
        thumbnail_path: staged_thumbnail(&media_path),
        media_path,
        duration_seconds,
        published_at,
        sabr_notice,
    }))
}

/// A thumbnail-only fetch's result: the staged thumbnail's path and the
/// video's publish time, when `yt-dlp` reported one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedThumbnail {
    pub path: PathBuf,
    pub published_at: Option<DateTime<Utc>>,
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
/// for `video_url` inside `staging_dir` (created first), sibling to
/// `download_video`.
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
/// reaches the daemon's own output — this is expected and routine, not an
/// error. Returns `Err` only for a systemic problem: no binary at
/// `ytdlp_path`, or a failure creating the staging dir. Discarding the
/// staging dir is the caller's job.
pub fn fetch_thumbnail(
    ytdlp_path: &Path,
    video_url: &str,
    staging_dir: &Path,
) -> Result<ThumbnailFetch> {
    ensure_output_dir(staging_dir)?;
    let mut cmd = Command::new(ytdlp_path);
    cmd.args([
        "-o",
        STAGED_OUTPUT_TEMPLATE,
        "--skip-download",
        "--write-thumbnail",
        "--convert-thumbnails",
        "jpg",
        "--quiet",
        "--no-warnings",
        "--print",
        "%(timestamp)s",
        "--print",
        "%(upload_date)s",
        "--print",
        "after_video:thumbnails.-1.filepath",
    ])
    .arg(video_url)
    .current_dir(staging_dir)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    let output = match run(
        &mut cmd,
        || not_found_message(ytdlp_path),
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
    let lines: Vec<&str> = stdout.lines().collect();
    let printed = lines.last().map(|s| s.trim());
    let filename = match printed {
        Some(printed) if !printed.is_empty() && !printed.eq_ignore_ascii_case("na") => {
            Path::new(printed)
                .file_name()
                .and_then(|f| f.to_str())
                .map(str::to_string)
        }
        _ => None,
    };

    Ok(match filename {
        Some(filename) => ThumbnailFetch::Fetched(FetchedThumbnail {
            path: staging_dir.join(filename),
            published_at: parse_published_at(
                line_before_last(&lines, 2),
                line_before_last(&lines, 1),
            ),
        }),
        None => ThumbnailFetch::Unavailable { reason: None },
    })
}

/// Where `probe_captions` writes the video's info JSON in the staging dir.
const INFO_JSON_FILENAME: &str = "info.json";

/// The fields of yt-dlp's info JSON a caption probe reads. Track lists are
/// keyed by yt-dlp's `--sub-langs` key, sorted; absent or `null` when the
/// video has none.
#[derive(serde::Deserialize)]
struct CaptionInfo {
    language: Option<String>,
    timestamp: Option<i64>,
    upload_date: Option<String>,
    subtitles: Option<std::collections::BTreeMap<String, serde_json::Value>>,
    automatic_captions: Option<std::collections::BTreeMap<String, serde_json::Value>>,
}

/// A caption probe's result: the tracks yt-dlp's info JSON lists, the
/// video's publish time when reported, and the info JSON itself, written to
/// the staging dir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionProbe {
    pub tracks: CaptionTracks,
    pub published_at: Option<DateTime<Utc>>,
    /// Reused by `fetch_subtitles` (`--load-info-json`): one extraction per
    /// video, fewer YouTube requests.
    pub info_json: PathBuf,
    /// yt-dlp's stderr `WARNING:` lines that can affect caption tracks:
    /// any of them makes a probe without a selectable track inconclusive
    /// (yt-dlp withholds tracks silently, e.g. without a PO token). Cache
    /// write failures and skipped-formats notices are dropped.
    pub warnings: Vec<String>,
}

/// A caption-only download's result: the staged `.srt` and its contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedSubtitles {
    pub path: PathBuf,
    pub srt: String,
}

/// `-J --skip-download --write-subs` without `--no-warnings`: subtitle
/// warnings only appear when subtitles are requested, and `-J` writes no
/// files. Err on any failure (clean or systemic), so the caller leaves the
/// video Missing.
pub fn probe_captions(
    ytdlp_path: &Path,
    video_url: &str,
    staging_dir: &Path,
) -> Result<CaptionProbe> {
    ensure_output_dir(staging_dir)?;
    let mut cmd = Command::new(ytdlp_path);
    cmd.args(["-J", "--skip-download", "--write-subs"])
        .arg(video_url)
        .current_dir(staging_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = match run(
        &mut cmd,
        || not_found_message(ytdlp_path),
        |e| format!("Failed to run yt-dlp caption probe for {video_url}: {e}"),
    )? {
        RunOutcome::Success(output) => output,
        RunOutcome::CleanFailure { stderr_raw } => {
            let reason = failure_reason(&stderr_raw);
            return Err(anyhow!(
                "yt-dlp failed to probe captions for {video_url}: {reason}"
            ));
        }
    };

    let info_json = staging_dir.join(INFO_JSON_FILENAME);
    std::fs::write(&info_json, &output.stdout)
        .map_err(|e| anyhow!("Failed to write the info JSON to {info_json:?}: {e}"))?;
    let info: CaptionInfo = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow!("failed to parse yt-dlp info JSON for {video_url}: {e}"))?;
    Ok(CaptionProbe {
        published_at: info
            .timestamp
            .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
            .or_else(|| info.upload_date.as_deref().and_then(parse_upload_date)),
        tracks: CaptionTracks {
            original_language: info.language,
            manual: info.subtitles.unwrap_or_default().into_keys().collect(),
            automatic: info
                .automatic_captions
                .unwrap_or_default()
                .into_keys()
                .collect(),
        },
        info_json,
        warnings: caption_warning_lines(&String::from_utf8_lossy(&output.stderr)),
    })
}

/// `--load-info-json --skip-download --write-subs|--write-auto-subs
/// --sub-langs <key> --sub-format vtt --convert-subs srt`.
pub fn fetch_subtitles(
    ytdlp_path: &Path,
    probe: &CaptionProbe,
    track: &CaptionTrack,
    staging_dir: &Path,
) -> Result<FetchedSubtitles> {
    let write_flag = match track.source {
        CaptionSource::Manual => "--write-subs",
        CaptionSource::Automatic => "--write-auto-subs",
    };
    let mut cmd = Command::new(ytdlp_path);
    cmd.arg("--load-info-json")
        .arg(&probe.info_json)
        .args([
            "-o",
            STAGED_OUTPUT_TEMPLATE,
            "--skip-download",
            write_flag,
            "--sub-langs",
            &track.key,
            "--sub-format",
            "vtt",
            "--convert-subs",
            "srt",
            "--quiet",
            "--no-warnings",
        ])
        .current_dir(staging_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match run(
        &mut cmd,
        || not_found_message(ytdlp_path),
        |e| {
            format!(
                "Failed to run yt-dlp subtitle download for {}: {e}",
                track.key
            )
        },
    )? {
        RunOutcome::Success(_) => {}
        RunOutcome::CleanFailure { stderr_raw } => {
            let reason = failure_reason(&stderr_raw);
            return Err(anyhow!(
                "yt-dlp failed to download the {} subtitles: {reason}",
                track.key
            ));
        }
    }
    let path = staged_srt(staging_dir, &track.key)?.ok_or_else(|| {
        anyhow!(
            "yt-dlp wrote no SubRip file for the {} subtitles",
            track.key
        )
    })?;
    let srt = std::fs::read_to_string(&path)
        .map_err(|e| anyhow!("Failed to read the staged subtitles {path:?}: {e}"))?;
    Ok(FetchedSubtitles { path, srt })
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
            return Err(anyhow!(not_found_message(ytdlp_path)));
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
/// parsing one JSON object per stdout line. Empty output means "no videos"
/// (`Ok(vec![])`). Returns `Err` when `yt-dlp` exits non-zero (carrying its
/// warning-free stderr as the reason, so a failed listing is never mistaken
/// for an empty channel), when there's no binary at `ytdlp_path`, or for
/// output that doesn't parse as one JSON object per line. stderr is
/// captured, never inherited, so none of it reaches the daemon's own output.
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
    .stderr(Stdio::piped());
    let output = match output_retrying_busy(&mut cmd) {
        Ok(output) => output,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(anyhow!(not_found_message(ytdlp_path)));
        }
        Err(e) => return Err(anyhow!("Failed to run yt-dlp for {channel_url}: {e}")),
    };

    if !output.status.success() {
        let reason = failure_reason(&String::from_utf8_lossy(&output.stderr));
        return Err(anyhow!(
            "yt-dlp failed to list channel videos for {channel_url}: {reason}"
        ));
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

fn not_found_message(ytdlp_path: &Path) -> String {
    format!(
        "`yt-dlp` was not found at {}. Install yt-dlp there or run update-ytdlp before running yarrtube.",
        ytdlp_path.display()
    )
}

fn failure_reason(stderr: &str) -> String {
    reason_excluding_warnings(stderr).unwrap_or_else(|| "no error reported".to_string())
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
/// on a transient busy error. Returns `Err` only for a systemic problem
/// (missing binary, spawn failure).
fn run(
    cmd: &mut Command,
    not_found_msg: impl FnOnce() -> String,
    other_err_msg: impl FnOnce(&io::Error) -> String,
) -> Result<RunOutcome> {
    let output = match output_retrying_busy(cmd) {
        Ok(output) => output,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(anyhow!(not_found_msg())),
        Err(e) => return Err(anyhow!(other_err_msg(&e))),
    };

    if !output.status.success() {
        return Ok(RunOutcome::CleanFailure {
            stderr_raw: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(RunOutcome::Success(output))
}

/// `WARNING:` lines of a caption probe's stderr, without their
/// indentation and minus the harmless ones.
fn caption_warning_lines(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("WARNING:"))
        .filter(|line| !is_harmless_probe_warning(line))
        .map(str::to_string)
        .collect()
}

/// A cache write failure (`Writing cache to '...' failed`) or a
/// `formats have been skipped` notice (SABR): never about captions.
fn is_harmless_probe_warning(line: &str) -> bool {
    (line.starts_with("WARNING: Writing cache to ") && line.contains(" failed"))
        || line.contains("formats have been skipped")
}

fn ensure_output_dir(output_path: &Path) -> Result<()> {
    std::fs::create_dir_all(output_path)
        .map_err(|e| anyhow!("Failed to create output directory {output_path:?}: {e}"))
}

/// The `<media stem>.jpg` `--write-thumbnail --convert-thumbnails jpg` left
/// next to the staged media, when it is there.
fn staged_thumbnail(media_path: &Path) -> Option<PathBuf> {
    Some(media_path.with_extension("jpg")).filter(|thumbnail| thumbnail.is_file())
}

/// The `<id>.<key>.srt` `--convert-subs srt` left in `staging_dir` for the
/// track requested by its `--sub-langs` key, when it is there: yt-dlp names
/// a subtitle file after the output template's stem plus the key, and any
/// other track's file left in the staging dir is ignored.
fn staged_srt(staging_dir: &Path, key: &str) -> Result<Option<PathBuf>> {
    let suffix = format!(".{key}.srt");
    let entries = std::fs::read_dir(staging_dir)
        .map_err(|e| anyhow!("Failed to list the staging dir {staging_dir:?}: {e}"))?;
    Ok(entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with(&suffix))
        }))
}

/// The printed line `back` lines before the last one, when printed.
fn line_before_last<'a>(lines: &[&'a str], back: usize) -> Option<&'a str> {
    lines
        .len()
        .checked_sub(back + 1)
        .and_then(|idx| lines.get(idx))
        .copied()
}

/// The publish time from `yt-dlp`'s printed timestamp, else from its
/// printed upload date.
fn parse_published_at(timestamp: Option<&str>, upload_date: Option<&str>) -> Option<DateTime<Utc>> {
    timestamp
        .and_then(parse_timestamp)
        .or_else(|| upload_date.and_then(parse_upload_date))
}

/// Parses `yt-dlp`'s `--print %(timestamp)s` line (Unix seconds) into the
/// publish time; `NA` or anything unparsable is unknown.
fn parse_timestamp(line: &str) -> Option<DateTime<Utc>> {
    line.trim()
        .parse::<i64>()
        .ok()
        .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
}

/// Parses `yt-dlp`'s `--print %(upload_date)s` line (`YYYYMMDD`) into
/// midnight UTC of that day, the publish time when no timestamp is known.
fn parse_upload_date(line: &str) -> Option<DateTime<Utc>> {
    chrono::NaiveDate::parse_from_str(line.trim(), "%Y%m%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|midnight| midnight.and_utc())
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

        /// A fake `yt-dlp` that exits 0 after writing `contents` to
        /// `file_name` in the directory it's invoked in, copied from a file
        /// so it's immune to shell quoting — stands in for `yt-dlp` writing
        /// a converted subtitle file.
        pub(crate) fn writing_file(file_name: &str, contents: &str) -> Self {
            use std::os::unix::fs::PermissionsExt;

            let bin_dir = unique_temp_dir("fake-ytdlp-bin");
            let script_path = bin_dir.join("yt-dlp");
            let captured_args_path = bin_dir.join("captured-args");
            let contents_path = bin_dir.join("file-content");
            fs::write(&contents_path, contents).unwrap();
            fs::write(
                &script_path,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"{}\"\ncp \"{}\" \"{file_name}\"\nexit 0\n",
                    captured_args_path.display(),
                    contents_path.display()
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
    use crate::domain::video_transcript::CaptionLanguage;

    /// Trimmed `yt-dlp -J` output of real videos, captured to pin down how
    /// it reports caption tracks and languages.
    const UPLOADED_CAPTIONS_INFO_JSON: &str =
        include_str!("ytdlp_fixtures/uploaded_captions.info.json");
    const AUTO_ONLY_SPANISH_INFO_JSON: &str =
        include_str!("ytdlp_fixtures/auto_only_spanish.info.json");

    fn a_probe_in(staging_dir: &Path) -> CaptionProbe {
        CaptionProbe {
            tracks: CaptionTracks::default(),
            published_at: None,
            info_json: staging_dir.join("info.json"),
            warnings: vec![],
        }
    }

    fn caption_track(key: &str, language: &str, source: CaptionSource) -> CaptionTrack {
        CaptionTrack {
            key: key.to_string(),
            language: CaptionLanguage::new(language).unwrap(),
            source,
        }
    }

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
            Quality::High,
            &output_dir,
        )
        .unwrap();

        assert_eq!(
            result,
            DownloadAttempt::Succeeded(DownloadedVideo {
                media_path: output_dir.join(DEFAULT_PRINTED_FILENAME),
                thumbnail_path: None,
                duration_seconds: None,
                published_at: None,
                sabr_notice: None,
            })
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_download_into_the_staging_dir_with_the_video_id_as_name() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let staging_dir = unique_temp_dir("ytdlp-staging").join("yt1");
        let fake = FakeYtDlp::with_downloaded_files("yt1.mp4", &["yt1.mp4"]);

        let result = unwrap_succeeded(
            download_video(
                &fake.path,
                "https://example.com/video",
                Quality::High,
                &staging_dir,
            )
            .unwrap(),
        );

        assert_eq!(result.media_path, staging_dir.join("yt1.mp4"));
        assert!(staging_dir.join("yt1.mp4").is_file());
        assert!(
            fake.captured_args()
                .windows(2)
                .any(|pair| pair == ["-o", "%(id)s.%(ext)s"])
        );
        std::fs::remove_dir_all(staging_dir.parent().unwrap()).unwrap();
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
            Quality::High,
            &output_dir,
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
            Quality::High,
            &output_dir,
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
    fn it_should_pass_thumbnail_embedding_and_conversion_flags_to_yt_dlp() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-thumbnail-flags");
        let fake = FakeYtDlp::with_exit_code(0);

        download_video(
            &fake.path,
            "https://example.com/video",
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
    fn it_should_error_when_a_successful_exit_prints_no_filename() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-output-no-print");
        let fake = FakeYtDlp::with_exit_code_and_printed_filename(0, "");

        let result = download_video(
            &fake.path,
            "https://example.com/video",
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

        let result = unwrap_succeeded(
            download_video(
                &fake.path,
                "https://example.com/video",
                Quality::High,
                &output_dir,
            )
            .unwrap(),
        );

        assert_eq!(
            result,
            DownloadedVideo {
                media_path: output_dir.join("My Video.mp4"),
                thumbnail_path: None,
                duration_seconds: Some(223),
                published_at: None,
                sabr_notice: None,
            }
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_report_the_publish_timestamp_printed_by_ytdlp() {
        let fake = test_support::FakeYtDlp::with_stdout("1773568800\n20260315\n223\nyt1.mp4\n");
        let staging_dir = test_support::unique_temp_dir("ytdlp-output-timestamp");

        let result = unwrap_succeeded(
            download_video(
                &fake.path,
                "https://example.com/video",
                Quality::High,
                &staging_dir,
            )
            .unwrap(),
        );

        assert_eq!(
            result,
            DownloadedVideo {
                media_path: staging_dir.join("yt1.mp4"),
                thumbnail_path: None,
                duration_seconds: Some(223),
                published_at: Some(
                    DateTime::parse_from_rfc3339("2026-03-15T10:00:00Z")
                        .unwrap()
                        .to_utc()
                ),
                sabr_notice: None,
            }
        );
        assert!(
            fake.captured_args()
                .windows(4)
                .any(|args| args == ["--print", "%(timestamp)s", "--print", "%(upload_date)s"])
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_fall_back_to_the_upload_date_when_the_timestamp_is_na() {
        let fake = test_support::FakeYtDlp::with_stdout("NA\n20260315\n223\nyt1.mp4\n");
        let staging_dir = test_support::unique_temp_dir("ytdlp-output-upload-date");

        let result = unwrap_succeeded(
            download_video(
                &fake.path,
                "https://example.com/video",
                Quality::High,
                &staging_dir,
            )
            .unwrap(),
        );

        assert_eq!(
            result.published_at,
            Some(
                DateTime::parse_from_rfc3339("2026-03-15T00:00:00Z")
                    .unwrap()
                    .to_utc()
            )
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_report_the_written_thumbnail_path() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-thumbnail-staging");
        let fake = test_support::FakeYtDlp::with_stdout(&format!(
            "1773568800\n20260315\n{}\n",
            staging_dir.join("yt1.jpg").display()
        ));

        let result = fetch_thumbnail(&fake.path, "https://example.com/video", &staging_dir);

        assert_eq!(
            result.unwrap(),
            ThumbnailFetch::Fetched(FetchedThumbnail {
                path: staging_dir.join("yt1.jpg"),
                published_at: Some(
                    DateTime::parse_from_rfc3339("2026-03-15T10:00:00Z")
                        .unwrap()
                        .to_utc()
                ),
            })
        );
        assert!(
            fake.captured_args()
                .windows(2)
                .any(|pair| pair == ["-o", "%(id)s.%(ext)s"])
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_parse_caption_tracks_and_language_from_the_info_json() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-captions-staging");
        let fake = test_support::FakeYtDlp::with_stdout(AUTO_ONLY_SPANISH_INFO_JSON);

        let result = probe_captions(&fake.path, "https://example.com/video", &staging_dir);

        assert_eq!(
            result.unwrap(),
            CaptionProbe {
                tracks: CaptionTracks {
                    original_language: Some("es-US".to_string()),
                    manual: vec![],
                    automatic: ["de", "en", "en-orig", "es", "es-orig", "fr", "ja"]
                        .map(str::to_string)
                        .to_vec(),
                },
                published_at: Some(DateTime::from_timestamp(1792267205, 0).unwrap()),
                info_json: staging_dir.join("info.json"),
                warnings: vec![],
            }
        );
        assert_eq!(
            std::fs::read_to_string(staging_dir.join("info.json")).unwrap(),
            AUTO_ONLY_SPANISH_INFO_JSON
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_parse_the_publish_timestamp_from_the_info_json() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-captions-staging");
        let with_timestamp = test_support::FakeYtDlp::with_stdout(UPLOADED_CAPTIONS_INFO_JSON);
        let with_upload_date_only = test_support::FakeYtDlp::with_stdout(
            &UPLOADED_CAPTIONS_INFO_JSON
                .replace("\"timestamp\": 1256453853", "\"timestamp\": null"),
        );

        let published_at = [&with_timestamp, &with_upload_date_only].map(|fake| {
            probe_captions(&fake.path, "https://example.com/video", &staging_dir)
                .unwrap()
                .published_at
        });

        assert_eq!(
            published_at,
            [
                Some(DateTime::from_timestamp(1256453853, 0).unwrap()),
                Some(
                    DateTime::parse_from_rfc3339("2009-10-25T00:00:00Z")
                        .unwrap()
                        .to_utc()
                ),
            ]
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_request_subtitles_and_keep_warnings_when_probing() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-captions-staging");
        let fake = test_support::FakeYtDlp::with_stdout(UPLOADED_CAPTIONS_INFO_JSON);

        probe_captions(&fake.path, "https://example.com/video", &staging_dir).unwrap();

        assert_eq!(
            fake.captured_args(),
            [
                "-J",
                "--skip-download",
                "--write-subs",
                "https://example.com/video"
            ]
            .map(str::to_string)
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_collect_the_warnings_of_a_probe() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-captions-staging");
        let ytdlp = fake_ytdlp_writing_stdout_and_stderr(
            "ytdlp-captions-warning",
            UPLOADED_CAPTIONS_INFO_JSON,
            "[youtube] yt1: Downloading webpage\nWARNING: [youtube] yt1: Some subtitles may be missing\n",
        );

        let result = probe_captions(&ytdlp, "https://example.com/video", &staging_dir);

        assert_eq!(
            result.unwrap().warnings,
            vec!["WARNING: [youtube] yt1: Some subtitles may be missing".to_string()]
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_drop_a_cache_write_warning_from_a_probe() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-captions-staging");
        let ytdlp = fake_ytdlp_writing_stdout_and_stderr(
            "ytdlp-captions-cache-warning",
            UPLOADED_CAPTIONS_INFO_JSON,
            "WARNING: Writing cache to '/home/yarrtube/.cache/yt-dlp/youtube-sigfuncs/0229cd64-main-104.json' failed: Traceback (most recent call last):\n  File \"yt_dlp/cache.py\", line 41, in store\nPermissionError: [Errno 13] Permission denied: '/home/yarrtube'\n",
        );

        let result = probe_captions(&ytdlp, "https://example.com/video", &staging_dir);

        assert_eq!(result.unwrap().warnings, Vec::<String>::new());
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_drop_a_formats_skipped_warning_from_a_probe() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-captions-staging");
        let ytdlp = fake_ytdlp_writing_stdout_and_stderr(
            "ytdlp-captions-formats-warning",
            UPLOADED_CAPTIONS_INFO_JSON,
            &format!("{SABR_WARNING}\n"),
        );

        let result = probe_captions(&ytdlp, "https://example.com/video", &staging_dir);

        assert_eq!(result.unwrap().warnings, Vec::<String>::new());
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_keep_a_caption_warning_next_to_harmless_ones_in_a_probe() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-captions-staging");
        let ytdlp = fake_ytdlp_writing_stdout_and_stderr(
            "ytdlp-captions-mixed-warnings",
            UPLOADED_CAPTIONS_INFO_JSON,
            &format!(
                "WARNING: Writing cache to '/home/yarrtube/.cache/yt-dlp/youtube-sigfuncs/a.json' failed: Traceback (most recent call last):\n{SABR_WARNING}\nWARNING: [youtube] yt1: Some subtitles may be missing\n"
            ),
        );

        let result = probe_captions(&ytdlp, "https://example.com/video", &staging_dir);

        assert_eq!(
            result.unwrap().warnings,
            vec!["WARNING: [youtube] yt1: Some subtitles may be missing".to_string()]
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_keep_a_failure_warning_other_than_a_cache_write_in_a_probe() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-captions-staging");
        let ytdlp = fake_ytdlp_writing_stdout_and_stderr(
            "ytdlp-captions-other-failure-warning",
            UPLOADED_CAPTIONS_INFO_JSON,
            "WARNING: [youtube] yt1: Subtitles download failed: HTTP Error 429: Too Many Requests\n",
        );

        let result = probe_captions(&ytdlp, "https://example.com/video", &staging_dir);

        assert_eq!(
            result.unwrap().warnings,
            vec![
                "WARNING: [youtube] yt1: Subtitles download failed: HTTP Error 429: Too Many Requests"
                    .to_string()
            ]
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_fail_the_probe_on_a_clean_failed_exit() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-captions-staging");
        let ytdlp = fake_ytdlp_writing_stderr(
            "ytdlp-captions-failing",
            "ERROR: [youtube] yt1: HTTP Error 429: Too Many Requests",
            1,
        );

        let result = probe_captions(&ytdlp, "https://example.com/video", &staging_dir);

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err("yt-dlp failed to probe captions for https://example.com/video: ERROR: [youtube] yt1: HTTP Error 429: Too Many Requests".to_string())
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_pass_manual_subtitle_flags_for_a_manual_track() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-subtitles-staging");
        let fake = test_support::FakeYtDlp::with_downloaded_files("", &["yt1.en.srt"]);
        let probe = a_probe_in(&staging_dir);

        fetch_subtitles(
            &fake.path,
            &probe,
            &caption_track("en", "en", CaptionSource::Manual),
            &staging_dir,
        )
        .unwrap();

        assert_eq!(
            fake.captured_args(),
            [
                "--load-info-json",
                &staging_dir.join("info.json").to_string_lossy(),
                "-o",
                "%(id)s.%(ext)s",
                "--skip-download",
                "--write-subs",
                "--sub-langs",
                "en",
                "--sub-format",
                "vtt",
                "--convert-subs",
                "srt",
                "--quiet",
                "--no-warnings",
            ]
            .map(str::to_string)
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_pass_auto_subtitle_flags_for_an_automatic_track() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-subtitles-staging");
        let fake = test_support::FakeYtDlp::with_downloaded_files("", &["yt1.en-orig.srt"]);
        let probe = a_probe_in(&staging_dir);

        fetch_subtitles(
            &fake.path,
            &probe,
            &caption_track("en-orig", "en", CaptionSource::Automatic),
            &staging_dir,
        )
        .unwrap();

        assert_eq!(
            fake.captured_args(),
            [
                "--load-info-json",
                &staging_dir.join("info.json").to_string_lossy(),
                "-o",
                "%(id)s.%(ext)s",
                "--skip-download",
                "--write-auto-subs",
                "--sub-langs",
                "en-orig",
                "--sub-format",
                "vtt",
                "--convert-subs",
                "srt",
                "--quiet",
                "--no-warnings",
            ]
            .map(str::to_string)
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_the_written_srt_and_its_contents() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-subtitles-staging");
        std::fs::write(staging_dir.join("info.json"), "{}").unwrap();
        let srt = "1\n00:00:00,000 --> 00:00:02,000\nHello there\n";
        let fake = test_support::FakeYtDlp::writing_file("yt1.en.srt", srt);
        let probe = a_probe_in(&staging_dir);

        let result = fetch_subtitles(
            &fake.path,
            &probe,
            &caption_track("en", "en", CaptionSource::Manual),
            &staging_dir,
        );

        assert_eq!(
            result.unwrap(),
            FetchedSubtitles {
                path: staging_dir.join("yt1.en.srt"),
                srt: srt.to_string(),
            }
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_the_srt_of_the_requested_track_if_another_is_staged() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-subtitles-staging");
        std::fs::write(staging_dir.join("info.json"), "{}").unwrap();
        std::fs::write(staging_dir.join("yt1.es.srt"), "stale").unwrap();
        let srt = "1\n00:00:00,000 --> 00:00:02,000\nHello there\n";
        let fake = test_support::FakeYtDlp::writing_file("yt1.en.srt", srt);
        let probe = a_probe_in(&staging_dir);

        let result = fetch_subtitles(
            &fake.path,
            &probe,
            &caption_track("en", "en", CaptionSource::Manual),
            &staging_dir,
        );

        assert_eq!(
            result.unwrap(),
            FetchedSubtitles {
                path: staging_dir.join("yt1.en.srt"),
                srt: srt.to_string(),
            }
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_fail_when_no_srt_was_written() {
        let staging_dir = test_support::unique_temp_dir("ytdlp-subtitles-staging");
        std::fs::write(staging_dir.join("info.json"), "{}").unwrap();
        let fake = test_support::FakeYtDlp::with_exit_code(0);
        let probe = a_probe_in(&staging_dir);

        let result = fetch_subtitles(
            &fake.path,
            &probe,
            &caption_track("en", "en", CaptionSource::Manual),
            &staging_dir,
        );

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err("yt-dlp wrote no SubRip file for the en subtitles".to_string())
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_report_the_staged_thumbnail_written_next_to_the_download() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let staging_dir = unique_temp_dir("ytdlp-staged-thumbnail");
        let fake = FakeYtDlp::with_downloaded_files("yt1.mp4", &["yt1.mp4", "yt1.jpg"]);

        let result = unwrap_succeeded(
            download_video(
                &fake.path,
                "https://example.com/video",
                Quality::High,
                &staging_dir,
            )
            .unwrap(),
        );

        assert_eq!(result.thumbnail_path, Some(staging_dir.join("yt1.jpg")));
        std::fs::remove_dir_all(&staging_dir).unwrap();
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
                Quality::High,
                &output_dir,
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
                Quality::High,
                &output_dir,
            )
            .unwrap(),
        );

        assert_eq!(result.duration_seconds, None);
        assert_eq!(result.media_path, output_dir.join("My Video.mp4"));
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
                Quality::High,
                &output_dir,
            )
            .unwrap(),
        );

        assert_eq!(result.media_path, output_dir.join(DEFAULT_PRINTED_FILENAME));
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
            Quality::High,
            &output_dir,
        );

        assert!(result.is_err());
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn it_should_fail_with_an_install_hint_if_no_binary_exists() {
        use test_support::unique_temp_dir;

        let output_dir = unique_temp_dir("ytdlp-output-missing-binary-message");
        let missing_path = output_dir.join("does-not-exist");

        let result = download_video(
            &missing_path,
            "https://example.com/video",
            Quality::High,
            &output_dir,
        );

        assert_eq!(
            result.map(|_| ()).map_err(|e| e.to_string()),
            Err(format!(
                "`yt-dlp` was not found at {}. Install yt-dlp there or run update-ytdlp before running yarrtube.",
                missing_path.display()
            ))
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn it_should_fail_to_run_if_the_binary_is_not_executable() {
        use test_support::unique_temp_dir;

        let output_dir = unique_temp_dir("ytdlp-output-non-executable-binary");
        let non_executable_path = output_dir.join("yt-dlp");
        std::fs::write(&non_executable_path, "#!/bin/sh\nexit 0\n").unwrap();

        let result = download_video(
            &non_executable_path,
            "https://example.com/video",
            Quality::High,
            &output_dir,
        );

        assert_eq!(
            result.map(|_| ()).map_err(|e| e.to_string()),
            Err(
                "Failed to run yt-dlp for https://example.com/video: Permission denied (os error 13)"
                    .to_string()
            )
        );
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
            Quality::High,
            &output_dir,
        )
        .unwrap();

        assert_eq!(
            result,
            DownloadAttempt::Succeeded(DownloadedVideo {
                media_path: output_dir.join("My Video.mp4"),
                thumbnail_path: None,
                duration_seconds: None,
                published_at: None,
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
            Quality::High,
            &output_dir,
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
    fn it_should_fail_with_an_install_hint_if_no_binary_exists_for_a_diagnostic_probe() {
        use test_support::unique_temp_dir;

        let dir = unique_temp_dir("ytdlp-diagnose-missing-binary-message");
        let missing_path = dir.join("does-not-exist");

        let result = diagnose(&missing_path, "https://example.com/video");

        assert_eq!(
            result.map(|_| ()).map_err(|e| e.to_string()),
            Err(format!(
                "`yt-dlp` was not found at {}. Install yt-dlp there or run update-ytdlp before running yarrtube.",
                missing_path.display()
            ))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn it_should_fail_to_run_if_the_binary_is_not_executable_for_a_diagnostic_probe() {
        use test_support::unique_temp_dir;

        let dir = unique_temp_dir("ytdlp-diagnose-non-executable-binary");
        let non_executable_path = dir.join("yt-dlp");
        std::fs::write(&non_executable_path, "#!/bin/sh\nexit 0\n").unwrap();

        let result = diagnose(&non_executable_path, "https://example.com/video");

        assert_eq!(
            result.map(|_| ()).map_err(|e| e.to_string()),
            Err(
                "Failed to run yt-dlp diagnostic probe for https://example.com/video: Permission denied (os error 13)"
                    .to_string()
            )
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_the_printed_thumbnail_filename_on_success() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-thumbnail-output");
        let fake = FakeYtDlp::with_stdout("My Video.jpg\n");

        let result = fetch_thumbnail(&fake.path, "https://example.com/video", &output_dir).unwrap();

        assert_eq!(
            result,
            ThumbnailFetch::Fetched(FetchedThumbnail {
                path: output_dir.join("My Video.jpg"),
                published_at: None,
            })
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_pass_thumbnail_only_flags_to_yt_dlp() {
        use test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("ytdlp-thumbnail-flags");
        let fake = FakeYtDlp::with_stdout("My Video.jpg\n");

        fetch_thumbnail(&fake.path, "https://example.com/video", &output_dir).unwrap();

        let expected = vec![
            "-o".to_string(),
            "%(id)s.%(ext)s".to_string(),
            "--skip-download".to_string(),
            "--write-thumbnail".to_string(),
            "--convert-thumbnails".to_string(),
            "jpg".to_string(),
            "--quiet".to_string(),
            "--no-warnings".to_string(),
            "--print".to_string(),
            "%(timestamp)s".to_string(),
            "--print".to_string(),
            "%(upload_date)s".to_string(),
            "--print".to_string(),
            "after_video:thumbnails.-1.filepath".to_string(),
            "https://example.com/video".to_string(),
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

        let result = fetch_thumbnail(&fake.path, "https://example.com/video", &output_dir).unwrap();

        assert_eq!(result, ThumbnailFetch::Unavailable { reason: None });
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_return_unavailable_with_the_reason_on_a_clean_failed_thumbnail_fetch() {
        use test_support::unique_temp_dir;

        let output_dir = unique_temp_dir("ytdlp-thumbnail-failed-exit");
        let script_path = fake_ytdlp_writing_stderr(
            "ytdlp-thumbnail-failed-exit-bin",
            "WARNING: [youtube] x: some warning\nERROR: [youtube] x: Video unavailable\n",
            1,
        );

        let result =
            fetch_thumbnail(&script_path, "https://example.com/video", &output_dir).unwrap();

        assert_eq!(
            result,
            ThumbnailFetch::Unavailable {
                reason: Some("ERROR: [youtube] x: Video unavailable".to_string()),
            }
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn it_should_error_when_no_binary_exists_at_the_configured_path_for_thumbnail_fetch() {
        use test_support::unique_temp_dir;

        let output_dir = unique_temp_dir("ytdlp-thumbnail-missing-binary");
        let missing_path = output_dir.join("does-not-exist");

        let result = fetch_thumbnail(&missing_path, "https://example.com/video", &output_dir);

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
    fn it_should_error_with_the_reason_when_listing_channel_videos_fails() {
        let script_path = fake_ytdlp_writing_stderr(
            "ytdlp-channel-videos-failed-exit",
            "ERROR: [youtube:tab] @somechannel: This channel does not exist\n",
            1,
        );

        let result = list_channel_videos(
            &script_path,
            "https://www.youtube.com/@somechannel/videos",
            10,
        )
        .map_err(|e| e.to_string());

        assert_eq!(
            result,
            Err("yt-dlp failed to list channel videos for https://www.youtube.com/@somechannel/videos: ERROR: [youtube:tab] @somechannel: This channel does not exist".to_string())
        );
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

    #[test]
    fn it_should_fail_with_an_install_hint_if_no_binary_exists_for_channel_video_listing() {
        use test_support::unique_temp_dir;

        let dir = unique_temp_dir("ytdlp-channel-videos-missing-binary-message");
        let missing_path = dir.join("does-not-exist");

        let result = list_channel_videos(
            &missing_path,
            "https://www.youtube.com/@somechannel/videos",
            10,
        );

        assert_eq!(
            result.map(|_| ()).map_err(|e| e.to_string()),
            Err(format!(
                "`yt-dlp` was not found at {}. Install yt-dlp there or run update-ytdlp before running yarrtube.",
                missing_path.display()
            ))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn it_should_fail_to_run_if_the_binary_is_not_executable_for_channel_video_listing() {
        use test_support::unique_temp_dir;

        let dir = unique_temp_dir("ytdlp-channel-videos-non-executable-binary");
        let non_executable_path = dir.join("yt-dlp");
        std::fs::write(&non_executable_path, "#!/bin/sh\nexit 0\n").unwrap();

        let result = list_channel_videos(
            &non_executable_path,
            "https://www.youtube.com/@somechannel/videos",
            10,
        );

        assert_eq!(
            result.map(|_| ()).map_err(|e| e.to_string()),
            Err(
                "Failed to run yt-dlp for https://www.youtube.com/@somechannel/videos: Permission denied (os error 13)"
                    .to_string()
            )
        );
        std::fs::remove_dir_all(&dir).unwrap();
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
