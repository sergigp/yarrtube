/// Returns the first path component of a stored `filename`/`thumbnail_filename`:
/// the video's own output folder name for a new-style `"folder/file.ext"` path
/// (one video per folder, alongside its thumbnail and `meta.nfo`), or the bare
/// filename unchanged for a legacy flat path with no `/` (downloaded before
/// per-video folders were introduced). Either way, this is "the entry that
/// owns this stored path, at the container's output directory level" — what
/// deletion and reconciliation need to delete or protect as a unit.
pub fn top_level_entry(relative_path: &str) -> &str {
    relative_path
        .split_once('/')
        .map_or(relative_path, |(first, _)| first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_return_the_folder_name_for_a_nested_path() {
        assert_eq!(top_level_entry("My Video/My Video.mp4"), "My Video");
    }

    #[test]
    fn it_should_return_the_filename_unchanged_for_a_legacy_flat_path() {
        assert_eq!(top_level_entry("My Video.mp4"), "My Video.mp4");
    }

    #[test]
    fn it_should_only_split_on_the_first_separator() {
        assert_eq!(
            top_level_entry("My Video [vid1]/nested/file.mp4"),
            "My Video [vid1]"
        );
    }
}
