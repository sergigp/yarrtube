/// A video item Plex has scanned into the configured library section,
/// matched to yarrtube by the `youtube://<id>` GUID its NFO agent derives
/// from the `<uniqueid type="youtube">` element yarrtube writes. Without
/// that GUID (`None`) the item is unidentified, e.g. imported before its
/// `movie.nfo` existed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexItem {
    pub rating_key: String,
    pub youtube_video_id: Option<String>,
}
