use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

const RELEASES_API_URL: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
const DEFAULT_YTDLP_PATH: &str = "/app/bin/yt-dlp";

/// The standalone-binary release asset name for the given platform (the
/// production Docker image is always Linux, but `serve`/`update-ytdlp` also
/// run directly on a developer's machine — see `scripts/run-local.sh` —
/// where downloading the Linux asset would silently replace a working local
/// `yt-dlp` with one that can't execute). Linux additionally distinguishes
/// arm64: an x86_64 binary run there needs emulation the container/host may
/// not have (e.g. Docker Desktop's Rosetta layer on Apple Silicon), so the
/// startup self-update would otherwise clobber a correct arm64 binary — such
/// as the one the Dockerfile's `ytdlp-fetch` stage already fetches for the
/// same architecture — with one that can't run.
fn asset_name_for_platform(os: &str, arch: &str) -> &'static str {
    match os {
        "macos" => "yt-dlp_macos",
        _ => match arch {
            "aarch64" => "yt-dlp_linux_aarch64",
            _ => "yt-dlp_linux",
        },
    }
}

#[derive(Debug, Deserialize)]
struct Release {
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Fetches and installs the latest `yt-dlp` binary at `target_path`. Backs
/// `UpdateYtdlpTask`'s recurring self-update, the `update-ytdlp` CLI command,
/// and the daemon's startup update alike.
pub trait YtdlpUpdater: Send + Sync {
    fn update(&self, target_path: &Path) -> anyhow::Result<()>;
}

pub struct RealYtdlpUpdater;

impl YtdlpUpdater for RealYtdlpUpdater {
    fn update(&self, target_path: &Path) -> anyhow::Result<()> {
        update(target_path)
    }
}

pub fn target_path() -> PathBuf {
    std::env::var("YTDLP_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_YTDLP_PATH))
}

/// Downloads the latest yt-dlp standalone binary for the current platform
/// and replaces `target_path` with it.
fn update(target_path: &Path) -> Result<()> {
    let download_url = latest_binary_url()?;
    let bytes = download_bytes(&download_url)?;
    install_binary(&bytes, target_path)
}

/// Queries the yt-dlp GitHub releases API for the latest stable release and
/// returns the download URL of its standalone binary asset for the current
/// platform.
fn latest_binary_url() -> Result<String> {
    let asset_name = asset_name_for_platform(std::env::consts::OS, std::env::consts::ARCH);
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(RELEASES_API_URL)
        .header("User-Agent", "yarrtube")
        .send()
        .context("Failed to reach the GitHub releases API")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(anyhow!(
            "GitHub releases API request failed with status {status}: {body}"
        ));
    }

    let release: Release = response
        .json()
        .context("Failed to parse the GitHub releases API response")?;

    release
        .assets
        .into_iter()
        .find(|asset| asset.name == asset_name)
        .map(|asset| asset.browser_download_url)
        .ok_or_else(|| anyhow!("No '{asset_name}' asset found in the latest yt-dlp release"))
}

fn download_bytes(download_url: &str) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(download_url)
        .header("User-Agent", "yarrtube")
        .send()
        .context("Failed to download the yt-dlp binary")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Download of the yt-dlp binary failed with status {}",
            response.status()
        ));
    }

    Ok(response
        .bytes()
        .context("Failed to read the downloaded yt-dlp binary")?
        .to_vec())
}

/// Writes `bytes` to a temp file next to `target_path` and atomically renames
/// it over `target_path`, so a failure partway through never leaves the
/// existing binary at `target_path` partially overwritten.
fn install_binary(bytes: &[u8], target_path: &Path) -> Result<()> {
    let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
    let tmp_path = parent.join(".yt-dlp.download");

    let write_result: Result<()> = (|| {
        let mut file = File::create(&tmp_path)
            .with_context(|| format!("Failed to create temp file {tmp_path:?}"))?;
        file.write_all(bytes)
            .with_context(|| format!("Failed to write to temp file {tmp_path:?}"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o755))
                .with_context(|| format!("Failed to set permissions on {tmp_path:?}"))?;
        }

        Ok(())
    })();

    if let Err(e) = write_result {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }

    fs::rename(&tmp_path, target_path)
        .with_context(|| format!("Failed to replace {target_path:?} with the new binary"))?;

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_pick_the_macos_asset_regardless_of_architecture() {
        assert_eq!(asset_name_for_platform("macos", "aarch64"), "yt-dlp_macos");
        assert_eq!(asset_name_for_platform("macos", "x86_64"), "yt-dlp_macos");
    }

    #[test]
    fn it_should_pick_the_linux_aarch64_asset_on_arm64() {
        assert_eq!(
            asset_name_for_platform("linux", "aarch64"),
            "yt-dlp_linux_aarch64"
        );
    }

    #[test]
    fn it_should_pick_the_linux_x86_64_asset_by_default() {
        assert_eq!(asset_name_for_platform("linux", "x86_64"), "yt-dlp_linux");
    }

    #[test]
    #[cfg(unix)]
    fn it_should_leave_the_original_binary_untouched_when_the_write_fails() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!(
            "yarrtube-ytdlp-update-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("yt-dlp");
        fs::write(&target, b"original").unwrap();

        let original_perms = fs::metadata(&dir).unwrap().permissions();
        let mut readonly_perms = original_perms.clone();
        readonly_perms.set_mode(0o555);
        fs::set_permissions(&dir, readonly_perms).unwrap();

        let result = install_binary(b"new content", &target);

        fs::set_permissions(&dir, original_perms).unwrap();

        assert!(result.is_err());
        assert_eq!(fs::read(&target).unwrap(), b"original");

        fs::remove_dir_all(&dir).unwrap();
    }
}
