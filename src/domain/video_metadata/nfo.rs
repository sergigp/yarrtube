use super::VideoMetadata;
use quick_xml::escape::escape;

fn element(tag: &str, value: &str) -> String {
    format!("<{tag}>{}</{tag}>", escape(value))
}

/// Renders `movie.nfo`'s XML content from a video's `VideoMetadata`, with
/// every text value escaped so the result is always well-formed XML
/// regardless of characters present in the source fields.
pub fn render_movie_nfo(metadata: &VideoMetadata) -> String {
    let mut body = String::new();
    body.push_str(&element("title", &metadata.title));
    body.push_str(&element("sorttitle", &metadata.sorttitle));
    body.push_str(&element("plot", &metadata.plot));
    body.push_str(&element("studio", &metadata.studio));
    body.push_str(&element("director", &metadata.director));
    body.push_str(&element("premiered", &metadata.premiered()));
    body.push_str(&element("year", &metadata.year().to_string()));
    if let Some(genre) = &metadata.genre {
        body.push_str(&element("genre", genre));
    }
    for tag in &metadata.tags {
        body.push_str(&element("tag", tag));
    }
    body.push_str(&format!(
        r#"<uniqueid type="youtube">{}</uniqueid>"#,
        escape(&metadata.uniqueid)
    ));
    if let Some(thumb) = &metadata.thumb {
        body.push_str(&element("thumb", thumb));
    }

    format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><movie>{body}</movie>"#)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    fn metadata() -> VideoMetadata {
        VideoMetadata::new(
            "My Video",
            "A description",
            "My Channel",
            "My Channel",
            DateTime::parse_from_rfc3339("2024-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&Utc),
            Some("Music".to_string()),
            vec!["tag1".to_string(), "tag2".to_string()],
            "yt1",
            Some("My Video.jpg".to_string()),
            "0001 My Video",
            DateTime::<Utc>::UNIX_EPOCH,
        )
    }

    fn parses_as_valid_xml(xml: &str) {
        let mut reader = quick_xml::Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(e) => panic!("generated movie.nfo is not well-formed XML: {e}"),
            }
        }
    }

    #[test]
    fn it_should_render_every_field_when_present() {
        let xml = render_movie_nfo(&metadata());

        parses_as_valid_xml(&xml);
        assert!(xml.contains("<title>My Video</title>"));
        assert!(xml.contains("<sorttitle>0001 My Video</sorttitle>"));
        assert!(xml.contains("<plot>A description</plot>"));
        assert!(xml.contains("<studio>My Channel</studio>"));
        assert!(xml.contains("<director>My Channel</director>"));
        assert!(xml.contains("<premiered>2024-01-02</premiered>"));
        assert!(xml.contains("<year>2024</year>"));
        assert!(xml.contains("<genre>Music</genre>"));
        assert!(xml.contains("<tag>tag1</tag>"));
        assert!(xml.contains("<tag>tag2</tag>"));
        assert!(xml.contains(r#"<uniqueid type="youtube">yt1</uniqueid>"#));
        assert!(xml.contains("<thumb>My Video.jpg</thumb>"));
    }

    #[test]
    fn it_should_omit_thumb_when_absent() {
        let metadata = VideoMetadata {
            thumb: None,
            ..metadata()
        };

        let xml = render_movie_nfo(&metadata);

        parses_as_valid_xml(&xml);
        assert!(!xml.contains("<thumb>"));
    }

    #[test]
    fn it_should_omit_tag_elements_when_there_are_no_tags() {
        let metadata = VideoMetadata {
            tags: Vec::new(),
            ..metadata()
        };

        let xml = render_movie_nfo(&metadata);

        parses_as_valid_xml(&xml);
        assert!(!xml.contains("<tag>"));
    }

    #[test]
    fn it_should_omit_genre_when_unmapped() {
        let metadata = VideoMetadata {
            genre: None,
            ..metadata()
        };

        let xml = render_movie_nfo(&metadata);

        parses_as_valid_xml(&xml);
        assert!(!xml.contains("<genre>"));
    }

    #[test]
    fn it_should_escape_xml_significant_characters_and_remain_well_formed() {
        let metadata = VideoMetadata {
            title: "Title & <tags> \"quoted\" 'apos'".to_string(),
            plot: "Plot with & < > \" '".to_string(),
            tags: vec!["a&b".to_string()],
            ..metadata()
        };

        let xml = render_movie_nfo(&metadata);

        parses_as_valid_xml(&xml);
        assert!(!xml.contains("<tags>"));
        assert!(xml.contains("&amp;"));
        assert!(xml.contains("&lt;"));
        assert!(xml.contains("&gt;"));
        assert!(xml.contains("&quot;"));
        assert!(xml.contains("&apos;"));
    }
}
