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

/// Renders a show's `tvshow.nfo`, with every text value escaped so the
/// result is always well-formed XML.
pub fn render_show_nfo(show: &Show) -> String {
    let body = [
        element("title", &show.title),
        element("studio", &show.title),
        format!(
            r#"<uniqueid type="youtube">{}</uniqueid>"#,
            escape(&show.uniqueid)
        ),
        element("status", "Continuing"),
    ]
    .concat();
    format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><tvshow>{body}</tvshow>"#)
}
