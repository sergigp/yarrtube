use super::{Verdict, VideoCategoryVerdict};
use crate::domain::category::Category;

/// One categorization run's counts, for its summary line: the categories
/// asked about, how many got a yes or a no, and how many were rejected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CategorizationTally {
    pub asked: usize,
    pub yes: usize,
    pub no: usize,
    pub rejected: usize,
}

impl CategorizationTally {
    /// Counts `verdicts` about the `pending` categories the run asked about.
    pub fn new(pending: &[Category], verdicts: &[VideoCategoryVerdict], rejected: usize) -> Self {
        let answered = |wanted: Verdict| {
            verdicts
                .iter()
                .filter(|verdict| verdict.verdict == wanted)
                .filter(|verdict| {
                    pending
                        .iter()
                        .any(|category| category.id == verdict.category_id)
                })
                .count()
        };
        Self {
            asked: pending.len(),
            yes: answered(Verdict::Yes),
            no: answered(Verdict::No),
            rejected,
        }
    }
}
