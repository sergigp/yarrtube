/// The two messages of one call to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryMessages {
    pub system: String,
    pub user: String,
}
