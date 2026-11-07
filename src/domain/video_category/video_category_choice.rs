use super::{Verdict, VideoCategoryVerdict};
use crate::domain::category::{Category, CategoryMember};

/// Where a video's membership of a category comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryOrigin {
    /// The video's channel or playlist is a member.
    Source,
    Ai,
    User,
}

impl CategoryOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Ai => "ai",
            Self::User => "user",
        }
    }
}

/// One category as offered for one video: whether the video belongs to it
/// and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoCategoryChoice {
    pub category: Category,
    pub belongs: bool,
    /// `Source` whenever the source is a member, whatever the verdict.
    pub from: Option<CategoryOrigin>,
    pub verdict: Option<Verdict>,
}

impl VideoCategoryChoice {
    /// Every category, name order, from the source's membership and the
    /// video's verdicts.
    pub fn list(
        _categories: &[Category],
        _source: &CategoryMember,
        _verdicts: &[VideoCategoryVerdict],
    ) -> Vec<Self> {
        Vec::new()
    }
}
