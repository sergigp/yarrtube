use super::VideoMetadata;
use crate::domain::show::Show;
use crate::domain::video::EpisodeNumber;
use quick_xml::escape::escape;

fn element(tag: &str, value: &str) -> String {
    format!("<{tag}>{}</{tag}>", escape(value))
}

/// Renders an episode NFO (`<episodedetails>`) from a video's
/// `VideoMetadata` and its episode number, with every text value escaped so
/// the result is always well-formed XML.
pub fn render_episode_nfo(_metadata: &VideoMetadata, _number: &EpisodeNumber) -> String {
    String::new()
}

/// Renders a show's `tvshow.nfo`.
pub fn render_show_nfo(_show: &Show) -> String {
    String::new()
}
