use anyhow::{Result, anyhow};
use serde::Deserialize;

const PLAYLIST_ITEMS_URL: &str = "https://www.googleapis.com/youtube/v3/playlistItems";

#[derive(Debug, Deserialize)]
struct PlaylistItemsResponse {
    items: Vec<PlaylistItem>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlaylistItem {
    snippet: PlaylistItemSnippet,
}

#[derive(Debug, Deserialize)]
struct PlaylistItemSnippet {
    title: String,
    #[serde(rename = "resourceId")]
    resource_id: ResourceId,
}

#[derive(Debug, Deserialize)]
struct ResourceId {
    #[serde(rename = "videoId")]
    video_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    pub url: String,
    pub title: String,
}

pub fn fetch_playlist_items_page(
    playlist_id: &str,
    api_key: &str,
    page_token: Option<&str>,
) -> Result<(Vec<Video>, Option<String>)> {
    let client = reqwest::blocking::Client::new();
    let mut query = vec![
        ("part", "snippet"),
        ("maxResults", "50"),
        ("playlistId", playlist_id),
        ("key", api_key),
    ];
    if let Some(token) = page_token {
        query.push(("pageToken", token));
    }

    let response = client.get(PLAYLIST_ITEMS_URL).query(&query).send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(anyhow!(
            "YouTube API request failed with status {status}: {body}"
        ));
    }

    let parsed: PlaylistItemsResponse = response.json()?;
    let videos = parsed
        .items
        .into_iter()
        .map(|item| Video {
            url: format!(
                "https://www.youtube.com/watch?v={}",
                item.snippet.resource_id.video_id
            ),
            title: item.snippet.title,
        })
        .collect();

    Ok((videos, parsed.next_page_token))
}

pub fn resolve_playlist(playlist_id: &str, api_key: &str) -> Result<Vec<Video>> {
    let mut all_videos = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let (mut videos, next_page_token) =
            fetch_playlist_items_page(playlist_id, api_key, page_token.as_deref())?;
        all_videos.append(&mut videos);

        match next_page_token {
            Some(token) => page_token = Some(token),
            None => break,
        }
    }

    Ok(all_videos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combines_multiple_pages_in_order() {
        fn fake_page(page_token: Option<&str>) -> (Vec<Video>, Option<String>) {
            match page_token {
                None => (
                    vec![
                        Video {
                            url: "https://www.youtube.com/watch?v=1".into(),
                            title: "One".into(),
                        },
                        Video {
                            url: "https://www.youtube.com/watch?v=2".into(),
                            title: "Two".into(),
                        },
                    ],
                    Some("page2".into()),
                ),
                Some("page2") => (
                    vec![Video {
                        url: "https://www.youtube.com/watch?v=3".into(),
                        title: "Three".into(),
                    }],
                    None,
                ),
                _ => panic!("unexpected page token"),
            }
        }

        let mut all_videos = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let (mut videos, next_page_token) = fake_page(page_token.as_deref());
            all_videos.append(&mut videos);
            match next_page_token {
                Some(token) => page_token = Some(token),
                None => break,
            }
        }

        assert_eq!(
            all_videos,
            vec![
                Video {
                    url: "https://www.youtube.com/watch?v=1".into(),
                    title: "One".into(),
                },
                Video {
                    url: "https://www.youtube.com/watch?v=2".into(),
                    title: "Two".into(),
                },
                Video {
                    url: "https://www.youtube.com/watch?v=3".into(),
                    title: "Three".into(),
                },
            ]
        );
    }
}
