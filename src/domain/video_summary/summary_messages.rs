/// The two messages of one call to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryMessages {
    pub system: String,
    pub user: String,
}

impl SummaryMessages {
    /// System and user together, in characters.
    pub fn chars(&self) -> usize {
        self.system.chars().count() + self.user.chars().count()
    }
}
