/// A video item Plex has scanned into the configured library section,
/// matched to yarrtube by the `youtube://<id>` GUID its NFO agent derives
/// from the `<uniqueid type="youtube">` element yarrtube writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexItem {
    pub rating_key: String,
    pub youtube_video_id: String,
}
