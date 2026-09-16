use std::path::Path;

/// Derives the thumbnail filename `yt-dlp` writes alongside a downloaded
/// video: `--convert-thumbnails jpg` always reuses the video's own
/// output-template base, so the thumbnail's name is the video filename's
/// stem with a `.jpg` extension, in the same output directory.
pub fn expected_thumbnail_filename(video_filename: &str) -> String {
    let stem = Path::new(video_filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(video_filename);
    format!("{stem}.jpg")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_swap_the_video_extension_for_jpg() {
        assert_eq!(expected_thumbnail_filename("My Video.mp4"), "My Video.jpg");
    }

    #[test]
    fn it_should_preserve_the_collision_suffix_in_the_stem() {
        assert_eq!(
            expected_thumbnail_filename("My Video [vid1].mp4"),
            "My Video [vid1].jpg"
        );
    }

    #[test]
    fn it_should_append_jpg_when_the_video_filename_has_no_extension() {
        assert_eq!(expected_thumbnail_filename("My Video"), "My Video.jpg");
    }
}
