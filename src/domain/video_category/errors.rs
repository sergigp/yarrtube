use crate::domain::category::CategoryMember;
use std::fmt;

#[derive(Debug)]
pub enum EditVideoCategoriesError {
    VideoNotFound,
    CategoryNotFound,
    SourceNotFound,
    /// The video's source is a member of the category.
    FromSource(CategoryMember),
    Repository(anyhow::Error),
}

impl fmt::Display for EditVideoCategoriesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VideoNotFound => write!(f, "video not found"),
            Self::CategoryNotFound => write!(f, "category not found"),
            Self::SourceNotFound => write!(f, "source not found"),
            Self::FromSource(CategoryMember::Channel(_)) => {
                write!(f, "This category comes from the video's channel")
            }
            Self::FromSource(CategoryMember::Playlist(_)) => {
                write!(f, "This category comes from the video's playlist")
            }
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for EditVideoCategoriesError {}
