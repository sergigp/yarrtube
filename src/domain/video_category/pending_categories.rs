use super::VideoCategoryVerdict;
use crate::domain::category::{Category, CategoryMember};

/// Categories with an AI prompt, not holding `source`, without a verdict in
/// `verdicts` — see the `video-categorization` capability.
pub fn pending_categories(
    categories: Vec<Category>,
    _source: &CategoryMember,
    verdicts: &[VideoCategoryVerdict],
) -> Vec<Category> {
    categories
        .into_iter()
        .filter(|category| {
            !verdicts
                .iter()
                .any(|verdict| verdict.category_id == category.id)
        })
        .collect()
}
