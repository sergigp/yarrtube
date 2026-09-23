use crate::domain::channel::ChannelHandle;
use std::path::PathBuf;

/// Downloads a channel's avatar image and stores/removes it under a local
/// avatars directory, injected into `ChannelService` alongside
/// `YoutubeChannelRepository`. Kept separate from that repository (see
/// design.md's "A new small port, not a method on `YoutubeChannelRepository`"
/// decision): resolving channel metadata from the YouTube Data API and
/// fetching-then-locally-storing an image are orthogonal infrastructure
/// concerns.
pub trait ChannelAvatarRepository: Send + Sync {
    /// Downloads the image at `url` and writes it to local storage as
    /// `<handle>.<ext>` (extension taken from the response's content type,
    /// defaulting to `jpg`), returning the stored filename. A network
    /// failure, a non-success HTTP status, or a local write failure returns
    /// `Ok(None)` rather than `Err` — mirrors the "thumbnail unavailable is
    /// not a download failure" precedent. `Err` is reserved for a systemic
    /// problem.
    fn store(&self, handle: &ChannelHandle, url: &str) -> anyhow::Result<Option<String>>;

    /// Deletes a previously stored avatar file, if present. A missing file
    /// is not an error.
    fn delete(&self, filename: &str) -> anyhow::Result<()>;
}

pub struct FilesystemChannelAvatarRepository {
    avatars_dir: PathBuf,
}

impl FilesystemChannelAvatarRepository {
    pub fn new(avatars_dir: PathBuf) -> Self {
        Self { avatars_dir }
    }
}

impl ChannelAvatarRepository for FilesystemChannelAvatarRepository {
    fn store(&self, handle: &ChannelHandle, url: &str) -> anyhow::Result<Option<String>> {
        let client = reqwest::blocking::Client::new();
        let response = match client.get(url).send() {
            Ok(response) => response,
            Err(e) => {
                tracing::warn!(handle = %handle, url = %url, error = %e, "failed to download channel avatar");
                return Ok(None);
            }
        };

        if !response.status().is_success() {
            tracing::warn!(handle = %handle, url = %url, status = %response.status(), "channel avatar download returned a non-success status");
            return Ok(None);
        }

        let extension = extension_from_content_type(
            response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
        );

        let bytes = match response.bytes() {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::warn!(handle = %handle, url = %url, error = %e, "failed to read channel avatar response body");
                return Ok(None);
            }
        };

        if let Err(e) = std::fs::create_dir_all(&self.avatars_dir) {
            tracing::warn!(dir = ?self.avatars_dir, error = %e, "failed to create avatars directory");
            return Ok(None);
        }

        let filename = format!("{}.{extension}", handle.as_str());
        let path = self.avatars_dir.join(&filename);
        if let Err(e) = std::fs::write(&path, &bytes) {
            tracing::warn!(path = ?path, error = %e, "failed to write channel avatar file");
            return Ok(None);
        }

        Ok(Some(filename))
    }

    fn delete(&self, filename: &str) -> anyhow::Result<()> {
        let path = self.avatars_dir.join(filename);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => {
                tracing::error!(path = ?path, error = %e, "failed to delete channel avatar file");
                Err(anyhow::anyhow!(
                    "failed to delete channel avatar file {path:?}: {e}"
                ))
            }
        }
    }
}

fn extension_from_content_type(content_type: Option<&str>) -> &'static str {
    match content_type.unwrap_or("").split(';').next().unwrap_or("") {
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "jpg",
    }
}

/// State-based stand-in for the avatars directory: the avatar files it
/// holds, each with the URL it was downloaded from. `store` writes
/// `<handle>.jpg` unless the fake is built to model an unavailable avatar
/// (`Ok(None)`) or a systemic failure (`Err`).
#[cfg(test)]
pub struct FakeChannelAvatarRepository {
    avatars: std::sync::Mutex<Vec<(String, String)>>,
    download: FakeAvatarDownload,
}

#[cfg(test)]
enum FakeAvatarDownload {
    Succeeds,
    Unavailable,
    Fails,
}

#[cfg(test)]
impl Default for FakeChannelAvatarRepository {
    fn default() -> Self {
        Self::with_avatars(&[])
    }
}

#[cfg(test)]
impl FakeChannelAvatarRepository {
    /// Seeds the directory with `(filename, source_url)` avatar files.
    pub fn with_avatars(avatars: &[(&str, &str)]) -> Self {
        Self {
            avatars: std::sync::Mutex::new(
                avatars
                    .iter()
                    .map(|(filename, url)| (filename.to_string(), url.to_string()))
                    .collect(),
            ),
            download: FakeAvatarDownload::Succeeds,
        }
    }

    pub fn unavailable() -> Self {
        Self {
            download: FakeAvatarDownload::Unavailable,
            ..Self::default()
        }
    }

    pub fn failing() -> Self {
        Self {
            download: FakeAvatarDownload::Fails,
            ..Self::default()
        }
    }

    /// Every avatar file currently stored, as `(filename, source_url)`.
    pub fn avatars(&self) -> Vec<(String, String)> {
        self.avatars.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl ChannelAvatarRepository for FakeChannelAvatarRepository {
    fn store(&self, handle: &ChannelHandle, url: &str) -> anyhow::Result<Option<String>> {
        match self.download {
            FakeAvatarDownload::Unavailable => Ok(None),
            FakeAvatarDownload::Fails => anyhow::bail!("avatars directory is not writable"),
            FakeAvatarDownload::Succeeds => {
                let filename = format!("{}.jpg", handle.as_str());
                let mut avatars = self.avatars.lock().unwrap();
                avatars.retain(|(stored, _)| *stored != filename);
                avatars.push((filename.clone(), url.to_string()));
                Ok(Some(filename))
            }
        }
    }

    fn delete(&self, filename: &str) -> anyhow::Result<()> {
        self.avatars
            .lock()
            .unwrap()
            .retain(|(stored, _)| stored != filename);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::ytdlp::test_support::unique_temp_dir;

    #[test]
    fn it_should_store_a_successfully_downloaded_avatar_and_return_its_filename() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/avatar.jpg")
            .with_status(200)
            .with_header("content-type", "image/jpeg")
            .with_body(b"fake image bytes")
            .create();
        let dir = unique_temp_dir("channel-avatar-store-success");
        let repo = FilesystemChannelAvatarRepository::new(dir.clone());
        let handle = ChannelHandle::new("@somechannel").unwrap();

        let filename = repo
            .store(&handle, &format!("{}/avatar.jpg", server.url()))
            .unwrap();

        assert_eq!(filename, Some("@somechannel.jpg".to_string()));
        assert_eq!(
            std::fs::read(dir.join("@somechannel.jpg")).unwrap(),
            b"fake image bytes"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_pick_the_extension_from_the_content_type() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/avatar")
            .with_status(200)
            .with_header("content-type", "image/png")
            .with_body(b"fake image bytes")
            .create();
        let dir = unique_temp_dir("channel-avatar-store-png");
        let repo = FilesystemChannelAvatarRepository::new(dir.clone());
        let handle = ChannelHandle::new("@somechannel").unwrap();

        let filename = repo
            .store(&handle, &format!("{}/avatar", server.url()))
            .unwrap();

        assert_eq!(filename, Some("@somechannel.png".to_string()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_return_none_on_an_http_error_status() {
        let mut server = mockito::Server::new();
        let _mock = server.mock("GET", "/avatar.jpg").with_status(404).create();
        let dir = unique_temp_dir("channel-avatar-store-http-error");
        let repo = FilesystemChannelAvatarRepository::new(dir.clone());
        let handle = ChannelHandle::new("@somechannel").unwrap();

        let filename = repo
            .store(&handle, &format!("{}/avatar.jpg", server.url()))
            .unwrap();

        assert_eq!(filename, None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_return_none_when_the_host_is_unreachable() {
        let dir = unique_temp_dir("channel-avatar-store-unreachable");
        let repo = FilesystemChannelAvatarRepository::new(dir.clone());
        let handle = ChannelHandle::new("@somechannel").unwrap();

        let filename = repo
            .store(&handle, "http://127.0.0.1:1/avatar.jpg")
            .unwrap();

        assert_eq!(filename, None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_delete_a_stored_avatar_file() {
        let dir = unique_temp_dir("channel-avatar-delete");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("@somechannel.jpg"), b"fake image bytes").unwrap();
        let repo = FilesystemChannelAvatarRepository::new(dir.clone());

        repo.delete("@somechannel.jpg").unwrap();

        assert!(!dir.join("@somechannel.jpg").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_succeed_when_deleting_a_missing_avatar_file() {
        let dir = unique_temp_dir("channel-avatar-delete-missing");
        let repo = FilesystemChannelAvatarRepository::new(dir);

        assert!(repo.delete("does-not-exist.jpg").is_ok());
    }
}
