use std::path::Path;

/// The path at which the Plex server sees `folder`, a video folder under
/// yarrtube's `videos_root`, given that Plex sees `videos_root` itself at
/// `plex_videos_root`. `None` when `folder` lies outside `videos_root`.
pub fn plex_folder_path(
    videos_root: &Path,
    plex_videos_root: &str,
    folder: &Path,
) -> Option<String> {
    folder
        .strip_prefix(videos_root)
        .ok()
        .map(|relative| Path::new(plex_videos_root).join(relative))
        .map(|path| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_map_a_folder_under_the_videos_root() {
        assert_eq!(
            plex_folder_path(
                Path::new("/videos"),
                "/volume1/media/yarrtube",
                Path::new("/videos/playlists/kids/Some video"),
            ),
            Some("/volume1/media/yarrtube/playlists/kids/Some video".to_string())
        );
    }

    #[test]
    fn it_should_map_regardless_of_trailing_slashes() {
        assert_eq!(
            plex_folder_path(
                Path::new("/videos/"),
                "/volume1/media/yarrtube/",
                Path::new("/videos/channels/Some video"),
            ),
            Some("/volume1/media/yarrtube/channels/Some video".to_string())
        );
    }

    #[test]
    fn it_should_skip_a_folder_outside_the_videos_root() {
        assert_eq!(
            plex_folder_path(
                Path::new("/videos"),
                "/volume1/media/yarrtube",
                Path::new("/elsewhere/Some video"),
            ),
            None
        );
    }

    #[test]
    fn it_should_skip_a_folder_in_a_sibling_with_the_same_prefix() {
        assert_eq!(
            plex_folder_path(
                Path::new("/videos"),
                "/volume1/media/yarrtube",
                Path::new("/videos2/Some video"),
            ),
            None
        );
    }
}
