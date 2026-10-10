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
pub fn render_episode_nfo(metadata: &VideoMetadata, number: &EpisodeNumber) -> String {
    let body = [
        element("title", &metadata.title),
        element("plot", &metadata.plot),
        element("season", &number.season().to_string()),
        element("episode", &number.episode().to_string()),
        element("aired", &metadata.premiered()),
        element("premiered", &metadata.premiered()),
        element("year", &metadata.year().to_string()),
        element("studio", &metadata.studio),
        element("director", &metadata.director),
        format!(
            r#"<uniqueid type="youtube">{}</uniqueid>"#,
            escape(&metadata.uniqueid)
        ),
        metadata
            .thumb
            .as_deref()
            .map(|thumb| element("thumb", thumb))
            .unwrap_or_default(),
    ]
    .concat();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><episodedetails>{body}</episodedetails>"#
    )
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
