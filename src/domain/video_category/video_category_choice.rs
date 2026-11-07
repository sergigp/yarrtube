use super::{Verdict, VerdictAuthor, VideoCategoryVerdict};
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

    fn of(author: VerdictAuthor) -> Self {
        match author {
            VerdictAuthor::Ai => Self::Ai,
            VerdictAuthor::User => Self::User,
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
        categories: &[Category],
        source: &CategoryMember,
        verdicts: &[VideoCategoryVerdict],
    ) -> Vec<Self> {
        categories
            .iter()
            .map(|category| {
                let verdict = verdicts
                    .iter()
                    .find(|verdict| verdict.category_id == category.id);
                Self::of(category, source, verdict)
            })
            .collect()
    }

    /// `category` for a video of `source` holding `verdict` for it.
    fn of(
        category: &Category,
        source: &CategoryMember,
        verdict: Option<&VideoCategoryVerdict>,
    ) -> Self {
        let (from, belongs) = match verdict {
            _ if category.has_member(source) => (Some(CategoryOrigin::Source), true),
            Some(verdict) => (
                Some(CategoryOrigin::of(verdict.decided_by)),
                verdict.is_yes(),
            ),
            None => (None, false),
        };
        Self {
            category: category.clone(),
            belongs,
            from,
            verdict: verdict.map(|verdict| verdict.verdict),
        }
    }
}
